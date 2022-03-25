use pnet_base::MacAddr;
use pnet_macros::packet;
use pnet_macros_support::packet::PrimitiveValues;
use pnet_macros_support::types::{u1, u16be, u24be, u7};
use pnet_packet::ethernet::EtherTypes::AppleTalk;
use pnet_packet::ethernet::{EtherType, EtherTypes};
use pnet_packet::Packet;

#[packet]
pub struct Aarp {
    pub hardware: u16be,
    #[construct_with(u16be)]
    pub protocol: EtherType,
    pub hw_address_len: u8,
    pub protocol_address_len: u8,
    pub function: u16be,
    #[construct_with(u8, u8, u8, u8, u8, u8)]
    pub source_hw: MacAddr,
    #[construct_with(u16be, u16be)]
    pub source_appletalk: AppleTalkAddr,
    #[construct_with(u8, u8, u8, u8, u8, u8)]
    pub destination_hw: MacAddr,
    #[construct_with(u16be, u16be)]
    pub destination_appletalk: AppleTalkAddr,
    #[length = "0"]
    #[payload]
    pub payload: Vec<u8>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AppleTalkAddr(pub u16, pub u16);

impl AppleTalkAddr {
    pub fn new(a: u16, b: u16) -> Self {
        AppleTalkAddr(a, b)
    }
}

impl PrimitiveValues for AppleTalkAddr {
    type T = (u16, u16);
    fn to_primitive_values(&self) -> Self::T {
        (self.0, self.1)
    }
}
