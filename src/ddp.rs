use packed_struct::prelude::*;
use crate::{addr::*, };

#[derive(PackedStruct, Debug, Clone)]
#[packed_struct(endian = "msb", bit_numbering = "msb0")]
pub struct Ddp {
    #[packed_field(element_size_bits = "2")]
    pub _reserved: ReservedZero<packed_bits::Bits<2>>,
    #[packed_field(element_size_bits = "4")]
    pub hop_count: u8,
    #[packed_field(element_size_bits = "10")]
    pub length: u16,
    pub checksum: u16,
    pub dest_net: u16,
    pub src_net: u16,
    #[packed_field(element_size_bytes = "1", ty = "enum")]
    pub dest_node: AppletalkNode,
    #[packed_field(element_size_bytes = "1", ty = "enum")]
    pub src_node: AppletalkNode,
    pub dest_socket: u8,
    pub src_socket: u8,
    pub typ: u8,
}

impl Ddp {
    pub fn source(&self) -> Appletalk {
        Appletalk { net: self.src_net, node: self.src_node }   
    }

    pub fn destination(&self) -> Appletalk {
        Appletalk { net: self.dest_net, node: self.dest_node }   
    }
}
