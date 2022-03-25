use pnet_base::MacAddr;
use pnet_macros::packet;
use pnet_macros_support::packet::PrimitiveValues;
use pnet_macros_support::types::{u1, u16be, u24be, u7};
use pnet_packet::ethernet::{EtherType, EtherTypes};
use pnet_packet::Packet;

#[packet]
pub struct EightOhTwo {
    #[construct_with(u8, u8, u8, u8, u8, u8)]
    pub destination: MacAddr,
    #[construct_with(u8, u8, u8, u8, u8, u8)]
    pub source: MacAddr,
    pub length: u16be,
    #[construct_with(u7)]
    pub dsap: SAP,
    pub ig: u1,
    #[construct_with(u7)]
    pub ssap: SAP,
    pub cr: u1,
    pub control: u8,
    pub oui: u24be,
    #[construct_with(u16be)]
    pub ethertype: EtherType,
    #[payload]
    pub payload: Vec<u8>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SAP(pub u8);

impl SAP {
    pub fn new(service: u7) -> Self {
        SAP(service)
    }
}

impl PrimitiveValues for SAP {
    type T = (u7,);
    fn to_primitive_values(&self) -> (u7,) {
        (self.0,)
    }
}

pub const SNAP: SAP = SAP(0xAA >> 1);
pub const APPLE_OUI: u32 = 0x08_00_07;

pub fn decode_appletalk<'p>(data: &'p [u8]) -> Option<EightOhTwoPacket<'p>> {
    let p = EightOhTwoPacket::new(data)?;
    if p.get_length() > 1600 || p.get_dsap() != SNAP || p.get_ssap() != SNAP {
        return None;
    }
    let typ = p.get_ethertype();
    if typ == EtherTypes::Aarp {
        let p = crate::aarp::AarpPacket::new(p.payload());
        println!("aarp: {:#?}", p);
    } else if typ == EtherTypes::AppleTalk {
        let p = crate::ddp::DdpPacket::new(p.payload());
        println!("ddp: {:#?}", p);
    } else {
        return None;
    }
    Some(p)
}
