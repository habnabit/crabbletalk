use pnet_base::MacAddr;
use pnet_macros::packet;
use pnet_macros_support::packet::PrimitiveValues;
use pnet_macros_support::types::{u2, u4, u10be, u16be, u24be, u7};
use pnet_packet::ethernet::EtherTypes::AppleTalk;
use pnet_packet::ethernet::{EtherType, EtherTypes};
use pnet_packet::Packet;

#[packet]
pub struct Ddp {
    pub pad: u2,
    pub hop_count: u4,
    pub length: u10be,
    pub checksum: u16be,
    pub dest_net: u16be,
    pub src_net: u16be,
    pub dest_node: u8,
    pub src_node: u8,
    pub dest_socket: u8,
    pub src_socket: u8,
    pub typ: u8,
    #[payload]
    pub payload: Vec<u8>,
}
