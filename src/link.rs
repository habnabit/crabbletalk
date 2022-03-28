use packed_struct::prelude::*;
use pnet_packet::ethernet::EtherTypes;

use crate::addr::*;
use crate::Result;
use crate::UnpackSplit;

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
    #[packed_field(element_size_bytes = "3")]
    pub oui: u32,
    #[packed_field(element_size_bytes = "2")]
    pub ethertype: Ethertype,
}

pub fn decode_appletalk<'p>(data: &'p [u8]) -> Result<Option<()>> {
    let (elap, payload) = crate::link::Elap::unpack_split(data)?;
    if elap.length > 1600 || elap.dsap != SNAP || elap.ssap != SNAP {
        return Ok(None);
    }
    if elap.ethertype == EtherTypes::Aarp {
        let p = crate::aarp::Aarp::unpack_from_slice(payload);
        println!("\n==aarp: {:#?} {:#?}", elap, p);
    } else if elap.ethertype == EtherTypes::AppleTalk {
        let (ddp, _payload) = crate::ddp::Ddp::unpack_split(payload)?;
        println!("\n^-ddp: {:#?} {:#?}", elap, ddp);
    } else {
        return Ok(None);
    }
    Ok(None)
}
