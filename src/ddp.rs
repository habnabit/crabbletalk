use packed_struct::prelude::*;

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
    pub dest_node: u8,
    pub src_node: u8,
    pub dest_socket: u8,
    pub src_socket: u8,
    pub typ: u8,
}
