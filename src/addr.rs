use packed_struct::prelude::*;
use std::{fmt, ops::RangeInclusive};

const fn unpack_u24(n: u32) -> [u8; 3] {
    [
        ((n >> 16) & 0xff) as u8,
        ((n >> 8) & 0xff) as u8,
        (n & 0xff) as u8,
    ]
}

pub const APPLE_OUI: [u8; 3] = unpack_u24(0x08_00_07);
pub const APPLETALK_OUI: [u8; 3] = unpack_u24(0x09_00_07);
pub const LAA_OUI: [u8; 3] = unpack_u24(0x52_54_00);
pub const BROADCAST_NIC: [u8; 3] = unpack_u24(0xff_ff_ff);
pub const APPLETALK_BROADCAST_MAC: Mac = Mac {
    oui: APPLETALK_OUI,
    nic: BROADCAST_NIC,
};
pub const ZERO_OUI: [u8; 3] = unpack_u24(0);
pub const ZERO_MAC: Mac = Mac {
    oui: ZERO_OUI,
    nic: ZERO_OUI,
};

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

impl Mac {
    pub fn new_random() -> Self {
        use rand::rngs::OsRng;
        use rand::Rng;
        let mut nic = [0u8; 3];
        OsRng.fill(&mut nic[..]);
        Mac { oui: LAA_OUI, nic }
    }
}

#[derive(PrimitiveEnum_u8, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AppletalkNodePrim {
    Unknown = 0,
    Broadcast = 255,
}

pub type AppletalkNodeCatchall = EnumCatchAll<AppletalkNodePrim>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AppletalkNode {
    Unknown,
    Broadcast,
    Node(u8),
}

impl AppletalkNode {
    fn from_prim(val: AppletalkNodeCatchall) -> Self {
        use self::AppletalkNodePrim::*;
        use self::EnumCatchAll::*;
        match val {
            Enum(Unknown) => AppletalkNode::Unknown,
            Enum(Broadcast) => AppletalkNode::Broadcast,
            CatchAll(e) => AppletalkNode::Node(e),
        }
    }

    fn to_prim(self) -> AppletalkNodeCatchall {
        use self::AppletalkNode::*;
        use self::EnumCatchAll::*;
        match self {
            Unknown => Enum(AppletalkNodePrim::Unknown),
            Broadcast => Enum(AppletalkNodePrim::Broadcast),
            Node(e) => CatchAll(e),
        }
    }
}

impl PrimitiveEnum for AppletalkNode {
    type Primitive = u8;

    fn from_primitive(val: u8) -> Option<Self> {
        AppletalkNodeCatchall::from_primitive(val).map(Self::from_prim)
    }

    fn to_primitive(&self) -> u8 {
        self.to_prim().to_primitive()
    }

    fn from_str(s: &str) -> Option<Self> {
        AppletalkNodeCatchall::from_str(s).map(Self::from_prim)
    }

    fn from_str_lower(s: &str) -> Option<Self> {
        AppletalkNodeCatchall::from_str_lower(s).map(Self::from_prim)
    }
}

pub const APPLETALK_BROADCAST: Appletalk = Appletalk {
    net: 0,
    node: AppletalkNode::Broadcast,
};
pub const APPLETALK_STARTUP_NET_RANGE: RangeInclusive<u16> = 0xFF00..=0xFFFE;
pub const APPLETALK_USER_NODE_RANGE: RangeInclusive<u8> = 0x01..=0x7F;
pub const APPLETALK_SERVER_NODE_RANGE: RangeInclusive<u8> = 0x80..=0xFE;
pub const APPLETALK_ANY_NODE_RANGE: RangeInclusive<u8> = 0x01..=0xFE;

#[derive(PackedStruct, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[packed_struct(endian = "msb", bit_numbering = "msb0")]
pub struct Appletalk {
    pub net: u16,
    #[packed_field(element_size_bytes = "1", ty = "enum")]
    pub node: AppletalkNode,
}

impl Appletalk {
    pub fn new_random() -> Self {
        //return Appletalk { net: 0xff00, node: AppletalkNode::Node(0x80) };
        use rand::rngs::OsRng;
        use rand::Rng;
        let net = OsRng.gen_range(APPLETALK_STARTUP_NET_RANGE);
        let node = AppletalkNode::Node(OsRng.gen_range(APPLETALK_ANY_NODE_RANGE));
        Appletalk { net, node }
    }
}

impl fmt::Debug for Appletalk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AppleTalk<${:04x}.${:02x} ({:?})>",
            self.net,
            self.node.to_primitive(),
            self.node
        )
    }
}

pub const APPLETALK_DDP_DAS_RANGE: RangeInclusive<u8> = 0x80..=0xFD;

#[derive(PrimitiveEnum_u8, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AppletalkSocketPrim {
    Reserved0 = 0,
    SasNbp = 2,
    SasAep = 4,
    Reserved255 = 255,
}

pub type AppletalkSocketCatchall = EnumCatchAll<AppletalkSocketPrim>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sas {
    Nbp,
    Aep,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AppletalkSocket {
    StaticSas(Sas),
    Static(u8),
    Dynamic(u8),
    Reserved0,
    Reserved255,
}

impl AppletalkSocket {
    pub fn new_random_dynamic() -> Self {
        use rand::rngs::OsRng;
        use rand::Rng;
        let n = OsRng.gen_range(APPLETALK_DDP_DAS_RANGE);
        AppletalkSocket::Dynamic(n)
    }

    fn from_prim(val: AppletalkSocketCatchall) -> Self {
        use self::AppletalkSocketPrim::*;
        use self::EnumCatchAll::*;
        match val {
            Enum(Reserved0) => AppletalkSocket::Reserved0,
            Enum(Reserved255) => AppletalkSocket::Reserved255,
            Enum(SasNbp) => AppletalkSocket::StaticSas(Sas::Nbp),
            Enum(SasAep) => AppletalkSocket::StaticSas(Sas::Aep),
            CatchAll(e @ 0x7f..=0xfe) => AppletalkSocket::Dynamic(e),
            CatchAll(e) => AppletalkSocket::Static(e),
        }
    }

    fn to_prim(self) -> AppletalkSocketCatchall {
        use self::AppletalkSocket::*;
        use self::EnumCatchAll::*;
        match self {
            Reserved0 => Enum(AppletalkSocketPrim::Reserved0),
            Reserved255 => Enum(AppletalkSocketPrim::Reserved255),
            StaticSas(Sas::Nbp) => Enum(AppletalkSocketPrim::SasNbp),
            StaticSas(Sas::Aep) => Enum(AppletalkSocketPrim::SasAep),
            Static(e) | Dynamic(e) => CatchAll(e),
        }
    }
}

impl PrimitiveEnum for AppletalkSocket {
    type Primitive = u8;

    fn from_primitive(val: u8) -> Option<Self> {
        AppletalkSocketCatchall::from_primitive(val).map(Self::from_prim)
    }

    fn to_primitive(&self) -> u8 {
        self.to_prim().to_primitive()
    }

    fn from_str(s: &str) -> Option<Self> {
        AppletalkSocketCatchall::from_str(s).map(Self::from_prim)
    }

    fn from_str_lower(s: &str) -> Option<Self> {
        AppletalkSocketCatchall::from_str_lower(s).map(Self::from_prim)
    }
}

#[derive(PackedStruct, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DdpType {
    pub typ: u8,
}

impl fmt::Debug for DdpType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DdpType<${:02x}>", self.typ)
    }
}

#[derive(PackedStruct, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sap {
    pub protocol: u8,
}

impl fmt::Debug for Sap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sap<${:02x} ({})>", self.protocol << 1, self.protocol)
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
        write!(f, "Ethertype<${:04x}>", self.protocol)
    }
}

impl PartialEq<pnet_packet::ethernet::EtherType> for Ethertype {
    fn eq(&self, other: &pnet_packet::ethernet::EtherType) -> bool {
        self.protocol == other.0
    }
}

impl From<pnet_packet::ethernet::EtherType> for Ethertype {
    fn from(other: pnet_packet::ethernet::EtherType) -> Self {
        Self { protocol: other.0 }
    }
}
