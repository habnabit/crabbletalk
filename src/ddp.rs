use crate::{addr::*, Result};
use packed_struct::prelude::*;
use tokio::sync::mpsc;

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
    pub typ: u8,
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
}

#[derive(Debug)]
pub struct DdpSocket {
    pub(crate) mine: AppletalkSocket,
    pub(crate) ddp_tx: mpsc::Sender<(Ddp, Vec<u8>)>,
    pub(crate) ddp_rx: mpsc::Receiver<(Ddp, Vec<u8>)>,
}

impl DdpSocket {
    pub async fn sendto(
        &self,
        buf: &[u8],
        dest: Appletalk,
        dest_socket: AppletalkSocket,
    ) -> Result<()> {
        let header = Ddp {
            _reserved: Default::default(),
            hop_count: 0,
            length: 0,
            checksum: 0,
            dest_net: dest.net,
            src_net: 0,
            dest_node: dest.node,
            src_node: AppletalkNode::Unknown,
            dest_socket,
            src_socket: self.mine,
            typ: 4,
        };
        self.ddp_tx
            .send((header, buf.to_owned()))
            .await
            .map_err(|_| crate::CrabbletalkError::Hangup)?;
        Ok(())
    }

    pub async fn recvfrom(&mut self, buf_out: &mut [u8]) -> Result<(usize, Appletalk, AppletalkSocket)> {
        let (ddp, buf_in) = self.ddp_rx.recv().await.ok_or(crate::CrabbletalkError::Hangup)?;
        let len = buf_in.len().min(buf_out.len());
        (&mut buf_out[..len]).copy_from_slice(&buf_in[..len]);
        Ok((len, ddp.source(), ddp.src_socket))
    }
}
