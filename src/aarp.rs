use std::fmt;

use crate::link::{AppletalkPacket, Elap};
use crate::{addr::*, Result, UnpackSplit};
use packed_struct::prelude::*;
use pnet_packet::ethernet::EtherTypes;
use tokio::sync::{mpsc, watch, OnceCell};
use tokio::task;

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

pub struct AarpStack {
    appletalk_tx: mpsc::Sender<AppletalkPacket>,
    my_addr_ethernet: Mac,
    my_addr_appletalk_tx: watch::Sender<Option<Appletalk>>,
    my_addr_appletalk_rx: watch::Receiver<Option<Appletalk>>,
    phase: AddressPhase,
    amt_hw2atalk: retainer::Cache<Mac, Appletalk>,
    amt_atalk2hw: retainer::Cache<Appletalk, Mac>,
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
            amt_hw2atalk: <retainer::Cache<_, _> as Default>::default().with_label("amt_hw2atalk"),
            amt_atalk2hw: <retainer::Cache<_, _> as Default>::default().with_label("amt_atalk2hw"),
        };
        (ret, appletalk_rx)
    }

    pub async fn process_aarp(&mut self, data: &[u8]) -> Result<()> {
        let aarp = Aarp::unpack_from_slice(data)?;
        println!("  aarp: {:#?}", aarp);
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

    async fn add_addresses(&self, hw: Mac, atalk: Appletalk) {
        let expiry = || retainer::CacheExpiration::none();
        self.amt_hw2atalk.insert(hw, atalk, expiry()).await;
        self.amt_atalk2hw.insert(atalk, hw, expiry()).await;
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
            let (ddp, payload) = crate::ddp::Ddp::unpack_split(payload)?;
            println!(
                "- ddp: {:?} -> {:?}; {}b (vs {}b)",
                ddp.source(),
                ddp.destination(),
                ddp.length,
                payload.len()
            );
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

    async fn maybe_probe_phase(&self) -> Result<()> {
        let addr = match &self.phase {
            &AddressPhase::Tentative { addr, .. } => addr,
            _ => return Ok(()),
        };
        self.write_aarp((
            Elap {
                destination: APPLETALK_BROADCAST,
                source: self.my_addr_ethernet,
                length: 0,
                dsap: SNAP,
                ig: false,
                ssap: SNAP,
                cr: false,
                control: 3,
                oui: 0,
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
        .await
    }

    pub async fn spawn(mut self, mut buffer_rx: mpsc::Receiver<Vec<u8>>) {
        let (phase_tx, mut phase_rx) = mpsc::channel(1);
        let mut phase_fut = AddressPhase::acquire(phase_tx.clone());
        tokio::pin!(phase_fut);
        let mut drive_phase_fut = true;
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
                    let res = self.maybe_probe_phase().await;
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
                () = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                    match self.maybe_probe_phase().await {
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

pub struct AarpStackHandle {
    handle: task::JoinHandle<()>,
    buffer_tx: mpsc::Sender<Vec<u8>>,
}

impl AarpStackHandle {
    pub fn spawn(hw: Mac) -> (Self, mpsc::Receiver<AppletalkPacket>) {
        let (stack, appletalk_rx) = AarpStack::new(hw);
        let (buffer_tx, buffer_rx) = mpsc::channel(1);
        let handle = task::spawn(stack.spawn(buffer_rx));
        (Self { handle, buffer_tx }, appletalk_rx)
    }

    pub async fn process_ethernet(&self, data: &[u8]) -> Result<()> {
        self.buffer_tx
            .send(data.to_owned())
            .await
            .map_err(|_| crate::CrabbletalkError::Hangup)
    }
}
