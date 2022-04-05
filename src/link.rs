use std::fmt;

use packed_struct::prelude::*;

use crate::addr::*;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct AppletalkPacket(pub Vec<u8>);

impl fmt::Debug for AppletalkPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AppletalkPacket<{}b>", self.0.len())
    }
}

#[derive(PackedStruct, Debug, Clone)]
#[packed_struct(endian = "msb", bit_numbering = "msb0")]
pub struct Elap {
    #[packed_field(element_size_bytes = "6")]
    pub destination: Mac,
    #[packed_field(element_size_bytes = "6")]
    pub source: Mac,
    pub length: u16,
    #[packed_field(element_size_bits = "7")]
    pub dsap: Sap,
    #[packed_field(element_size_bits = "1")]
    pub ig: bool,
    #[packed_field(element_size_bits = "7")]
    pub ssap: Sap,
    #[packed_field(element_size_bits = "1")]
    pub cr: bool,
    pub control: u8,
    pub oui: [u8; 3],
    #[packed_field(element_size_bytes = "2")]
    pub ethertype: Ethertype,
}
