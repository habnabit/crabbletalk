use std::fmt;

use crate::link::AppletalkPacket;
use crate::{addr::*, Result, UnpackSplit};
use packed_struct::prelude::*;
use pnet_packet::ethernet::EtherTypes;
use tokio::sync::{mpsc, OnceCell};

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
    Tentative(tokio::sync::Notify),
    Accepted,
}

pub struct AarpStack {
    frame_tx: mpsc::Sender<AppletalkPacket>,
    my_addr_ethernet: Mac,
    my_addr_appletalk: OnceCell<Appletalk>,
    phase: AddressPhase,
    amt_hw2atalk: retainer::Cache<Mac, Appletalk>,
    amt_atalk2hw: retainer::Cache<Appletalk, Mac>,
}

impl fmt::Debug for AarpStack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AarpStack")
            .field("my_addr_ethernet", &self.my_addr_ethernet)
            .field("my_addr_appletalk", &self.my_addr_appletalk)
            .field("phase", &self.phase)
            .field("amt_hw2atalk", &"<opaque>")
            .field("amt_atalk2hw", &"<opaque>")
            .finish()
    }
}

impl AarpStack {
    pub fn new(hw: Mac) -> (Self, mpsc::Receiver<AppletalkPacket>) {
        let (frame_tx, rx) = mpsc::channel(25);
        let ret = AarpStack {
            frame_tx,
            my_addr_ethernet: hw,
            my_addr_appletalk: OnceCell::new(),
            phase: AddressPhase::Uninitialized,
            amt_hw2atalk: <retainer::Cache<_, _> as Default>::default().with_label("amt_hw2atalk"),
            amt_atalk2hw: <retainer::Cache<_, _> as Default>::default().with_label("amt_atalk2hw"),
        };
        (ret, rx)
    }

    pub async fn process_aarp(&mut self, data: &[u8]) -> Result<()> {
        let aarp = Aarp::unpack_from_slice(data)?;
        println!("  aarp: {:#?}", aarp);
        let my_hw = &self.my_addr_ethernet;
        let my_atalk = self.my_addr_appletalk.get();
        // let aarp_relevant = match my_hw {
        //     Some(addr) if &aarp.destination_hw == addr => true,
        //     _ if aarp.destination_hw == APPLETALK_BROADCAST => true,
        //     _ => false,
        // };
        use self::AarpFunction::*;
        match (my_atalk, aarp.function, &self.phase) {
            (Some(atalk), Probe, AddressPhase::Tentative(n))
                if atalk == &aarp.destination_appletalk =>
            {
                println!("tentative conflict");
                n.notify_waiters();
                self.phase = AddressPhase::Uninitialized;
            }
            (Some(atalk), Request | Probe, AddressPhase::Accepted)
                if atalk == &aarp.destination_appletalk =>
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
        let (elap, payload) = crate::link::Elap::unpack_split(data)?;
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
}
