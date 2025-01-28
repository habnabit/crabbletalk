// © 2022 <_@habnab.it>
//
// SPDX-License-Identifier: MPL-2.0

use packed_struct::prelude::*;
use tokio::sync::mpsc;

use crate::{addr::*, Result};

pub fn ddp_checksum(bytes: &[u8]) -> u16 {
    let mut ret = 0u16;
    for &b in bytes {
        ret = ret.wrapping_add(b as u16).rotate_left(1);
    }
    if ret == 0 {
        0xffff
    } else {
        ret
    }
}

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
    #[packed_field(element_size_bytes = "1", ty = "enum")]
    pub dest_socket: AppletalkSocket,
    #[packed_field(element_size_bytes = "1", ty = "enum")]
    pub src_socket: AppletalkSocket,
    #[packed_field(element_size_bytes = "1")]
    pub typ: DdpType,
}

impl Ddp {
    pub fn source(&self) -> Appletalk {
        Appletalk {
            net: self.src_net,
            node: self.src_node,
        }
    }

    pub fn set_source(&mut self, addr: Appletalk) {
        self.src_net = addr.net;
        self.src_node = addr.node;
    }

    pub fn destination(&self) -> Appletalk {
        Appletalk {
            net: self.dest_net,
            node: self.dest_node,
        }
    }

    pub fn set_destination(&mut self, addr: Appletalk) {
        self.dest_net = addr.net;
        self.dest_node = addr.node;
    }

    pub fn set_checksum_from(&mut self, buf: &[u8]) {
        self.checksum = ddp_checksum(buf);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DdpHeader {
    pub addr: Appletalk,
    pub socket: AppletalkSocket,
    pub typ: DdpType,
}

#[derive(Debug)]
pub struct DdpSocket {
    pub(crate) addr: Appletalk,
    pub(crate) socket: AppletalkSocket,
    pub(crate) ddp_tx: mpsc::Sender<(Ddp, Vec<u8>)>,
    pub(crate) ddp_rx: mpsc::Receiver<(Ddp, Vec<u8>)>,
}

impl DdpSocket {
    pub fn local_addr(&self) -> Appletalk {
        self.addr
    }

    pub fn local_socket(&self) -> AppletalkSocket {
        self.socket
    }

    pub async fn sendto(&self, buf: &[u8], dest: DdpHeader) -> Result<()> {
        let header = Ddp {
            _reserved: Default::default(),
            hop_count: 0,
            length: 0,
            checksum: ddp_checksum(buf),
            dest_net: dest.addr.net,
            src_net: 0,
            dest_node: dest.addr.node,
            src_node: AppletalkNode::Unknown,
            dest_socket: dest.socket,
            src_socket: self.socket,
            typ: dest.typ,
        };
        self.ddp_tx
            .send((header, buf.to_owned()))
            .await
            .map_err(|_| crate::CrabbletalkError::Hangup)?;
        Ok(())
    }

    pub async fn recvfrom(&mut self, buf_out: &mut [u8]) -> Result<(usize, DdpHeader)> {
        let (ddp, buf_in) = self
            .ddp_rx
            .recv()
            .await
            .ok_or(crate::CrabbletalkError::Hangup)?;
        let len = buf_in.len().min(buf_out.len());
        (&mut buf_out[..len]).copy_from_slice(&buf_in[..len]);
        let header = DdpHeader {
            addr: ddp.source(),
            socket: ddp.src_socket,
            typ: ddp.typ,
        };
        Ok((len, header))
    }
}
