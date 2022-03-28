use crate::addr::*;
use crate::addr::{Appletalk, Mac};
use packed_struct::prelude::*;

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
