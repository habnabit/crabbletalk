use packed_struct::prelude::*;
use std::fmt;

#[derive(PackedStruct, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[packed_struct(endian = "msb", bit_numbering = "msb0")]
pub struct Mac {
    pub oui: [u8; 3],
    pub nic: [u8; 3],
}

impl fmt::Debug for Mac {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Mac<{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}>",
            self.oui[0], self.oui[1], self.oui[2], self.nic[0], self.nic[1], self.nic[2]
        )
    }
}

#[derive(PackedStruct, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[packed_struct(endian = "msb", bit_numbering = "msb0")]
pub struct Appletalk {
    pub net: u16,
    pub node: u8,
}

impl fmt::Debug for Appletalk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AppleTalk<{}.{}>", self.net, self.node)
    }
}

#[derive(PackedStruct, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sap {
    pub protocol: u8,
}

impl fmt::Debug for Sap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sap<{:02X} ({})>", self.protocol << 1, self.protocol)
    }
}

pub const SNAP: Sap = Sap {
    protocol: 0xAA >> 1,
};

#[derive(PackedStruct, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[packed_struct(endian = "msb", bit_numbering = "msb0")]
pub struct Ethertype {
    pub protocol: u16,
}

impl fmt::Debug for Ethertype {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ethertype<{:04X}>", self.protocol)
    }
}

impl PartialEq<pnet_packet::ethernet::EtherType> for Ethertype {
    fn eq(&self, other: &pnet_packet::ethernet::EtherType) -> bool {
        self.protocol == other.0
    }
}
