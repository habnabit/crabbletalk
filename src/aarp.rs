// © 2022 <_@habnab.it>
//
// SPDX-License-Identifier: MPL-2.0

use std::{cell::Cell, collections::BTreeMap, error::Error, fmt, sync::Arc};

use chrono::{DateTime, Utc};
use packed_struct::prelude::*;
use pnet_packet::ethernet::EtherTypes;
use tokio::{
    sync::{mpsc, oneshot, watch, Notify, OnceCell, RwLock},
    task,
};
use tokio_stream::StreamExt;

use crate::{
    addr::*,
    ddp::{Ddp, DdpSocket},
    link::{AppletalkPacket, Elap},
    Result, UnpackSplit,
};

#[derive(PrimitiveEnum_u16, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AarpHardware {
    Ethernet = 1,
    TokenRing = 2,
}

#[derive(PrimitiveEnum_u16, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AarpFunction {
    Request = 1,
    Response = 2,
    Probe = 3,
}

#[derive(PrimitiveEnum_u8, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AepFunction {
    Request = 1,
    Reply = 2,
}

#[derive(PackedStruct, Debug, Clone)]
#[packed_struct(endian = "msb", bit_numbering = "msb0")]
pub struct Aarp {
    #[packed_field(element_size_bytes = "2", ty = "enum")]
    pub hardware: AarpHardware,
    #[packed_field(element_size_bytes = "2")]
    pub protocol: Ethertype,
    pub hw_address_len: u8,
    pub protocol_address_len: u8,
    #[packed_field(element_size_bytes = "2", ty = "enum")]
    pub function: AarpFunction,
    #[packed_field(element_size_bytes = "6")]
    pub source_hw: Mac,
    #[packed_field(element_size_bytes = "1")]
    pub _pad1: ReservedZero<packed_bits::Bits<8>>,
    #[packed_field(element_size_bytes = "3")]
    pub source_appletalk: Appletalk,
    #[packed_field(element_size_bytes = "6")]
    pub destination_hw: Mac,
    #[packed_field(element_size_bytes = "1")]
    pub _pad2: ReservedZero<packed_bits::Bits<8>>,
    #[packed_field(element_size_bytes = "3")]
    pub destination_appletalk: Appletalk,
}

#[derive(Debug)]
enum AddressPhase {
    Uninitialized,
    Tentative {
        addr: Appletalk,
        conflict: std::sync::Arc<tokio::sync::Notify>,
    },
    Accepted {
        addr: Appletalk,
    },
}

impl AddressPhase {
    async fn acquire(phase_tx: mpsc::Sender<AddressPhase>) -> Result<()> {
        loop {
            let addr = Appletalk::new_random();
            let notifier = std::sync::Arc::new(tokio::sync::Notify::new());
            phase_tx
                .send(AddressPhase::Tentative {
                    addr,
                    conflict: notifier.clone(),
                })
                .await
                .map_err(|_| crate::CrabbletalkError::Hangup)?;
            tokio::select! {
                () = tokio::time::sleep(std::time::Duration::from_millis(1500)) => {
                    // we won!!
                    println!("haha we won");
                }
                () = notifier.notified() => {
                    // conflict; try again
                    println!("conflict");
                   continue;
                }
            }
            phase_tx
                .send(AddressPhase::Accepted { addr })
                .await
                .map_err(|_| crate::CrabbletalkError::Hangup)?;
            return Ok(());
        }
    }
}

#[derive(Default)]
struct AmtSynchronized {
    hw_table: BTreeMap<Mac, Arc<AmtEntry>>,
    atalk_table: BTreeMap<Appletalk, Arc<AmtEntry>>,
}

#[derive(Clone, Copy, Debug)]
struct AmtEntryCell {
    hw: Mac,
    atalk: Appletalk,
    set_at: DateTime<Utc>,
}

struct AmtEntry {
    entry_tx: watch::Sender<Option<AmtEntryCell>>,
    entry_rx: watch::Receiver<Option<AmtEntryCell>>,
}

impl Default for AmtEntry {
    fn default() -> Self {
        let (entry_tx, entry_rx) = watch::channel(None);
        AmtEntry { entry_tx, entry_rx }
    }
}

pub struct AarpStack {
    appletalk_tx: mpsc::Sender<AppletalkPacket>,
    my_addr_ethernet: Mac,
    my_addr_appletalk_tx: watch::Sender<Option<Appletalk>>,
    my_addr_appletalk_rx: watch::Receiver<Option<Appletalk>>,
    phase: AddressPhase,
    amt: RwLock<AmtSynchronized>,
}

impl fmt::Debug for AarpStack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AarpStack")
            .field("my_addr_ethernet", &self.my_addr_ethernet)
            .field("phase", &self.phase)
            .field("amt_hw2atalk", &"<opaque>")
            .field("amt_atalk2hw", &"<opaque>")
            .finish()
    }
}

impl AarpStack {
    pub fn new(hw: Mac) -> (Self, mpsc::Receiver<AppletalkPacket>) {
        let (appletalk_tx, appletalk_rx) = mpsc::channel(25);
        let (my_addr_appletalk_tx, my_addr_appletalk_rx) = watch::channel(None);
        let ret = AarpStack {
            appletalk_tx,
            my_addr_ethernet: hw,
            my_addr_appletalk_tx,
            my_addr_appletalk_rx,
            phase: AddressPhase::Uninitialized,
            amt: Default::default(),
        };
        (ret, appletalk_rx)
    }

    pub async fn process_aarp(&mut self, data: &[u8]) -> Result<()> {
        let (aarp, remainder) = Aarp::unpack_split(data)?;
        println!("  aarp: {:#?} trailer {:?}", aarp, remainder);
        use self::AarpFunction::*;
        match (aarp.function, &self.phase) {
            (Probe, AddressPhase::Tentative { addr, conflict })
                if addr == &aarp.destination_appletalk =>
            {
                println!("tentative conflict 1");
                conflict.notify_waiters();
            }
            (Response, AddressPhase::Tentative { addr, conflict })
                if addr == &aarp.source_appletalk =>
            {
                println!("tentative conflict 2");
                conflict.notify_waiters();
            }
            (Request | Probe, AddressPhase::Accepted { addr })
                if addr == &aarp.destination_appletalk =>
            {
                // TODO: send AARP reply
                println!("accepted reply");
                self.write_aarp((
                    Elap {
                        destination: aarp.source_hw,
                        source: self.my_addr_ethernet,
                        length: 0,
                        dsap: SNAP,
                        ig: false,
                        ssap: SNAP,
                        cr: false,
                        control: 3,
                        oui: ZERO_OUI,
                        ethertype: EtherTypes::Aarp.into(),
                    },
                    Aarp {
                        hardware: AarpHardware::Ethernet,
                        protocol: EtherTypes::AppleTalk.into(),
                        hw_address_len: 6,
                        protocol_address_len: 4,
                        function: AarpFunction::Response,
                        source_hw: self.my_addr_ethernet,
                        _pad1: Default::default(),
                        source_appletalk: *addr,
                        destination_hw: aarp.source_hw,
                        _pad2: Default::default(),
                        destination_appletalk: aarp.source_appletalk,
                    },
                ))
                .await?;
            }
            _ => {}
        }
        match aarp.function {
            Request | Response => {
                println!("aarp glean");
                self.add_addresses(aarp.source_hw, aarp.source_appletalk)
                    .await;
            }
            Probe => {}
        }
        Ok(())
    }

    pub async fn process_ddp(&mut self, elap: &Elap, data: &[u8]) -> Result<()> {
        let (ddp, payload) = crate::ddp::Ddp::unpack_split(data)?;
        match self.phase {
            AddressPhase::Accepted { addr, .. } => {
                let src = ddp.source();
                let dest = ddp.destination();
                if dest == addr {
                    println!("oh it's to me");
                } else if dest == APPLETALK_BROADCAST {
                } else {
                }
            }
            _ => {}
        }
        println!(
            "- ddp: {:?}#{:?} -> {:?}#{:?}; {}b (vs {}b)",
            ddp.source(),
            ddp.src_socket,
            ddp.destination(),
            ddp.dest_socket,
            ddp.length,
            payload.len()
        );
        Ok(())
    }

    async fn add_addresses(&self, hw: Mac, atalk: Appletalk) {
        let mut amt = self.amt.write().await;
        let entry = amt.atalk_table.entry(atalk).or_default().clone();
        amt.hw_table.entry(hw).or_insert_with(|| entry.clone());
        let new = Some(AmtEntryCell {
            hw,
            atalk,
            set_at: Utc::now(),
        });
        println!("new value for {:?}/{:?}: {:?}", hw, atalk, new);
        let old = entry.entry_tx.send_replace(new);
        println!("old value for {:?}/{:?}: {:?}", hw, atalk, old);
    }

    async fn hw_from_appletalk(&self, atalk: Appletalk) -> Result<Mac> {
        let mut rx = {
            let entry = {
                let mut amt = self.amt.write().await;
                println!("hw4a keys {:?}", amt.atalk_table.keys().collect::<Vec<_>>());
                amt.atalk_table.entry(atalk).or_default().clone()
            };
            let potential = entry.entry_rx.borrow();
            println!("hw4a for {:?} borrowed {:?}", atalk, potential);
            if let &Some(AmtEntryCell { hw, .. }) = &*potential {
                return Ok(hw);
            }
            entry.entry_rx.clone()
        };
        let mut addr_stream =
            tokio_stream::wrappers::WatchStream::new(self.my_addr_appletalk_rx.clone());
        let mut drive_aarp = true;
        let mut aarp_fut = async move {
            let addr = loop {
                println!("spin, spin,");
                match addr_stream.next().await {
                    Some(Some(x)) => break x,
                    Some(None) => continue,
                    None => return Err(crate::CrabbletalkError::Hangup),
                }
            };
            self.write_aarp((
                Elap {
                    destination: APPLETALK_BROADCAST_MAC,
                    source: self.my_addr_ethernet,
                    length: 0,
                    dsap: SNAP,
                    ig: false,
                    ssap: SNAP,
                    cr: false,
                    control: 3,
                    oui: ZERO_OUI,
                    ethertype: EtherTypes::Aarp.into(),
                },
                Aarp {
                    hardware: AarpHardware::Ethernet,
                    protocol: EtherTypes::AppleTalk.into(),
                    hw_address_len: 6,
                    protocol_address_len: 4,
                    function: AarpFunction::Request,
                    source_hw: self.my_addr_ethernet,
                    _pad1: Default::default(),
                    source_appletalk: addr,
                    destination_hw: ZERO_MAC,
                    _pad2: Default::default(),
                    destination_appletalk: atalk,
                },
            ))
            .await?;
            Ok(())
        };
        tokio::pin!(aarp_fut);
        loop {
            tokio::select! {
                res = rx.changed() => {
                    res.map_err(|_| crate::CrabbletalkError::Hangup)?;
                    let potential = rx.borrow_and_update();
                    if let &Some(AmtEntryCell { hw, .. }) = &*potential {
                        return Ok(hw);
                    }
                }
                res = &mut aarp_fut, if drive_aarp => {
                    println!("aarp_fut => {:?}", res);
                    drive_aarp = false;
                    return Err(crate::CrabbletalkError::Transient);
                }
            }
        }
    }

    pub async fn process_ethernet(&mut self, data: &[u8]) -> Result<()> {
        let (elap, payload) = Elap::unpack_split(data)?;
        if elap.length > 1600 || elap.dsap != SNAP || elap.ssap != SNAP {
            return Ok(());
        }
        if elap.ethertype == EtherTypes::Aarp {
            println!("\n==aarp elap: {:#?}", elap);
            self.process_aarp(payload).await?;
        } else if elap.ethertype == EtherTypes::AppleTalk {
            self.process_ddp(&elap, payload).await?;
        }
        Ok(())
    }

    async fn write_aarp(&self, mut payload: (Elap, Aarp)) -> Result<()> {
        payload.0.length =
            <(Elap, Aarp) as PackedStructSlice>::packed_bytes_size(Some(&payload))? as u16 - 14;
        self.appletalk_tx
            .send(AppletalkPacket(payload.pack_to_vec()?))
            .await
            .map_err(|_| crate::CrabbletalkError::Hangup)
    }

    async fn write_ddp(&self, mut header: (Elap, Ddp), payload: &[u8]) -> Result<()> {
        println!("write_ddp: {:#?}", header);
        let addr = match &self.phase {
            AddressPhase::Accepted { addr, .. } => addr,
            _ => {
                println!("erk");
                return Ok(());
            }
        };
        header.0.length = (<(Elap, Ddp) as PackedStructSlice>::packed_bytes_size(Some(&header))?
            - 14
            + payload.len()) as u16;
        header.1.set_source(*addr);
        header.1.length = (payload.len()
            + <Ddp as PackedStructSlice>::packed_bytes_size(Some(&header.1))?)
            as u16;
        let mut payload_vec = header.pack_to_vec()?;
        payload_vec.extend_from_slice(payload);
        println!("out to the wire? {:?}", payload_vec);
        let res = self.appletalk_tx
            .send(AppletalkPacket(payload_vec))
            .await
            .map_err(|_| crate::CrabbletalkError::Hangup);
        println!("  .. ok?");
        res
    }

    async fn maybe_probe_phase(&self, just_set: bool) -> Result<()> {
        match &self.phase {
            AddressPhase::Accepted { addr } if just_set => {
                self.my_addr_appletalk_tx.send_replace(Some(*addr));
            }
            AddressPhase::Tentative { addr, .. } => {
                let addr = *addr;
                self.write_aarp((
                    Elap {
                        destination: APPLETALK_BROADCAST_MAC,
                        source: self.my_addr_ethernet,
                        length: 0,
                        dsap: SNAP,
                        ig: false,
                        ssap: SNAP,
                        cr: false,
                        control: 3,
                        oui: ZERO_OUI,
                        ethertype: EtherTypes::Aarp.into(),
                    },
                    Aarp {
                        hardware: AarpHardware::Ethernet,
                        protocol: EtherTypes::AppleTalk.into(),
                        hw_address_len: 6,
                        protocol_address_len: 4,
                        function: AarpFunction::Probe,
                        source_hw: self.my_addr_ethernet,
                        _pad1: Default::default(),
                        source_appletalk: addr,
                        destination_hw: ZERO_MAC,
                        _pad2: Default::default(),
                        destination_appletalk: addr,
                    },
                ))
                .await?;
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn next_address(&self) -> Result<Appletalk> {
        let mut rx = tokio_stream::wrappers::WatchStream::new(self.my_addr_appletalk_rx.clone());
        loop {
            match rx.next().await {
                Some(Some(x)) => return Ok(x),
                Some(None) => continue,
                None => return Err(crate::CrabbletalkError::Hangup),
            }
        }
    }

    pub async fn spawn(
        mut self,
        mut buffer_rx: mpsc::Receiver<Vec<u8>>,
        mut ddp_control_rx: mpsc::Receiver<DdpControl>,
    ) {
        let (phase_tx, mut phase_rx) = mpsc::channel(1);
        let mut phase_fut = AddressPhase::acquire(phase_tx.clone());
        tokio::pin!(phase_fut);
        let mut drive_phase_fut = true;
        let mut join_set = tokio::task::JoinSet::new();
        let mut ddp_merge = tokio_stream::StreamMap::new();
        loop {
            tokio::select! {
                res = &mut phase_fut, if drive_phase_fut => {
                    println!("what phase my man {:?}", res);
                    drive_phase_fut = false;
                }
                next = phase_rx.recv() => {
                    self.phase = match next {
                        Some(p) => p,
                        None => {
                            println!("phase rx abort");
                            return;
                        },
                    };
                    let res = self.maybe_probe_phase(true).await;
                    println!("stepped phase: {:#?} {:?}", self, res);
                }
                next = buffer_rx.recv() => {
                    let buf = match next {
                        Some(b) => b,
                        None => {
                            println!("buffer rx abort");
                            return;
                        },
                    };
                    let res = self.process_ethernet(&buf[..]).await;
                    println!("did some ethernet: {:?}", res);
                }
                next = ddp_control_rx.recv() => {
                    let ctrl = match next {
                        Some(x) => x,
                        None => {
                            println!("ddp control rx abort");
                            return;
                        },
                    };
                    let (ddp_tx_in, ddp_rx_in) = mpsc::channel::<(Ddp, Vec<u8>)>(1);
                    let (ddp_tx_out, ddp_rx_out) = mpsc::channel::<(Ddp, Vec<u8>)>(1);
                    ddp_merge.insert(ctrl.bind, tokio_stream::wrappers::ReceiverStream::new(ddp_rx_in));
                    let mut addr_stream = tokio_stream::wrappers::WatchStream::new(self.my_addr_appletalk_rx.clone());
                    //let addr_fut = self.next_address();
                    join_set.spawn(async move {
                        let addr = loop {
                            println!("spin, spin,");
                            match addr_stream.next().await {
                                Some(Some(x)) => break x,
                                Some(None) => continue,
                                None => return Err(crate::CrabbletalkError::Hangup),
                            }
                        };
                        let ret = crate::ddp::DdpSocket { addr, socket: ctrl.bind, ddp_tx: ddp_tx_in, ddp_rx: ddp_rx_out };
                        println!("aight so we got {:#?}", ret);
                        let _ = ctrl.reply.send(ret);
                        Result::<_>::Ok(())
                    });
                }
                next = join_set.join_one(), if !join_set.is_empty() => {
                    println!("whoa stream step: {:?}", next);
                }
                next = ddp_merge.next(), if !ddp_merge.is_empty() => {
                    let (socket, (ddp, payload)) = match next {
                        Some(x) => x,
                        None => {
                            println!("ddp merge abort");
                            return;
                        },
                    };
                    let destination = match self.hw_from_appletalk(ddp.destination()).await {
                        Ok(x) => x,
                        Err(e) => {
                            println!("hw4a failure {:?}", e);
                            continue;
                        }
                    };
                    let res = self.write_ddp(
                        (
                            Elap {
                                destination,
                                source: self.my_addr_ethernet,
                                length: 0,  // filled in by write_ddp
                                dsap: SNAP,
                                ig: false,
                                ssap: SNAP,
                                cr: false,
                                control: 3,
                                oui: APPLE_OUI,
                                ethertype: EtherTypes::AppleTalk.into(),
                            },
                            ddp,
                        ),
                        &payload[..],
                    )
                    .await;
                }
                () = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                    match self.maybe_probe_phase(false).await {
                        Ok(()) => {}
                        Err(e) => {
                            println!("sent another probe? {:?}", e);
                        }
                    }
                }
            }
        }
    }
}

pub struct DdpControl {
    bind: AppletalkSocket,
    reply: oneshot::Sender<DdpSocket>,
}

#[derive(Debug, Clone)]
pub struct AarpStackHandle {
    buffer_tx: mpsc::Sender<Vec<u8>>,
    control_tx: mpsc::Sender<DdpControl>,
}

impl AarpStackHandle {
    pub fn spawn(hw: Mac) -> (Self, mpsc::Receiver<AppletalkPacket>) {
        let (stack, appletalk_rx) = AarpStack::new(hw);
        let (buffer_tx, buffer_rx) = mpsc::channel(1);
        let (control_tx, control_rx) = mpsc::channel(1);
        let handle = task::spawn(stack.spawn(buffer_rx, control_rx));
        (
            Self {
                buffer_tx,
                control_tx,
            },
            appletalk_rx,
        )
    }

    pub async fn process_ethernet(&self, data: &[u8]) -> Result<()> {
        self.buffer_tx
            .send(data.to_owned())
            .await
            .map_err(|_| crate::CrabbletalkError::Hangup)
    }

    pub async fn open_ddp(&self, bind: AppletalkSocket) -> Result<DdpSocket> {
        let (tx, rx) = oneshot::channel();
        self.control_tx
            .send(DdpControl { bind, reply: tx })
            .await
            .map_err(|_| crate::CrabbletalkError::Hangup)?;
        Ok(rx.await.map_err(|_| crate::CrabbletalkError::Hangup)?)
    }
}
