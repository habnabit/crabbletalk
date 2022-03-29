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
pub const LAA_OUI: [u8; 3] = unpack_u24(0x52_54_00);
pub const BROADCAST_NIC: [u8; 3] = unpack_u24(0xff_ff_ff);
pub const APPLETALK_BROADCAST: Mac = Mac {
    oui: APPLE_OUI,
    nic: BROADCAST_NIC,
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
        Mac {
            oui: LAA_OUI,
            nic,
        }
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
