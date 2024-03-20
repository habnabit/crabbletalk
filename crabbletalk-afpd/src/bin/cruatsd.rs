use std::{collections::BTreeSet, fs::File, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use crabbletalk::{
    aarp::AarpStackHandle,
    addr::{Appletalk, AppletalkNode, AppletalkSocket, DdpType},
    ddp::DdpHeader,
};
use cruats::{
    at::at_addr,
    zerocopy::{FromBytes, LayoutVerified, Unalign, Unaligned},
};
use packed_struct::PrimitiveEnum;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{unix::SocketAddr, UnixDatagram, UnixStream},
};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(short, long)]
    tmpdir: Option<PathBuf>,
    router_path: PathBuf,
    cruats_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    console_subscriber::init();
    let cli = Cli::parse();
    let (ethertalk, _unlinker1) = crabbletalk_afpd::anonymous_datagram_client(
        "cruatsd",
        cli.tmpdir.as_ref().map(|x| x.as_ref()),
    )?;
    let ethertalk = tokio::net::UnixDatagram::from_std(ethertalk)?;
    ethertalk.connect(&cli.router_path)?;
    ethertalk.send(b"").await?;

    let cruats_control = tokio::net::UnixListener::bind(&cli.cruats_path)?;
    let _unlinker2 = crabbletalk_afpd::UnlinkOnDrop::new(cli.cruats_path);

    let mut ethertalk_buf = vec![0u8; 1600];
    let mac = crabbletalk::addr::Mac::new_random();
    println!("cruatsd starting up on {:?}", mac);
    let (aarp_stack, mut atalk_rx) = crabbletalk::aarp::AarpStackHandle::spawn(mac);
    let mut joinset = tokio::task::JoinSet::new();

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => { break }
            joined = joinset.join_one(), if !joinset.is_empty() => {
                println!("ok well we joined {:?}", joined);
            }
            recvd = ethertalk.recv_from(&mut ethertalk_buf) => {
                let (n_read, addr) = recvd?;
                let data = &ethertalk_buf[..n_read];
                println!("ethertalk in in");
                aarp_stack.process_ethernet(data).await?;
                println!("ethertalk in out");
            }
            accepted = cruats_control.accept() => {
                let (stream, addr) = accepted?;
                joinset.spawn(drive_stream(aarp_stack.clone(), stream, addr));
            }
            atalk = atalk_rx.recv() => {
                match &atalk {
                    Some(p) => {
                        println!("oh no {:?}", p);
                        ethertalk.send(&p.0[..]).await?;
                    }
                    None => {
                        println!("oh no bye??");
                        break;
                    }
                }
            }
        }
    }

    println!("draining joinset of {:?}", joinset.len());
    joinset.abort_all();
    while !joinset.is_empty() {
        joinset.join_one().await?;
    }

    Ok(())
}

async fn drive_stream(
    aarp_stack: AarpStackHandle,
    mut stream: UnixStream,
    addr: SocketAddr,
) -> Result<()> {
    use std::{mem::size_of, os::unix::io::IntoRawFd};

    use cruats::at::sockaddr_at;
    use sendfd::SendWithFd;
    let cred = stream.peer_cred();
    println!("well who do we got here {:?} {:?}", addr, cred);
    let mut buffer = [0u8; 1600];
    let ddp_socket;
    let sock;
    let mine;
    let my_addr;
    {
        let addr_buf = &mut buffer[..size_of::<sockaddr_at>()];
        let n_read = stream.read_exact(addr_buf).await?;
        ddp_socket = AppletalkSocket::new_random_dynamic();
        sock = aarp_stack.open_ddp(ddp_socket).await?;
        let (mine_, theirs) = UnixDatagram::pair()?;
        mine = mine_;
        let theirs = theirs.into_std()?.into_raw_fd();
        println!("here's {} bytes: {:?}", n_read, addr_buf);
        {
            let lv = LayoutVerified::<_, Unalign<sockaddr_at>>::new(&mut addr_buf[..])
                .expect("rust internal error?")
                .into_mut();
            my_addr = lv.into_inner();
            let addr = sock.local_addr();
            *lv = Unalign::new(sockaddr_at {
                sat_addr: at_addr {
                    s_net: addr.net,
                    s_node: addr.node.to_primitive() as u16,
                },
                sat_port: sock.local_socket().to_primitive() as i16,
                ..Default::default()
            });
            println!("in: {:#?} out: {:#?}", my_addr, lv.into_inner());
        }
        stream.writable().await?;
        let res = stream.send_with_fd(addr_buf, &[theirs]);
        println!("fd/buf: {:?}", res);
    }
    stream.shutdown().await?;
    drop(stream);

    loop {
        let n_read = mine.recv(&mut buffer[..]).await?;
        let (addr_in, payload) =
            LayoutVerified::<_, Unalign<sockaddr_at>>::new_unaligned_from_prefix(&buffer[..n_read])
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "couldn't unpack the sockaddr_at? {}: {:?}",
                        n_read,
                        &buffer[..n_read]
                    )
                })?;
        let addr_in = addr_in.into_ref().get();
        println!("from {:?} {:?}: {:?}", cred, addr_in, payload);
        sock.sendto(
            &payload[..],
            DdpHeader {
                addr: Appletalk {
                    net: addr_in.sat_addr.s_net,
                    node: AppletalkNode::Node(addr_in.sat_addr.s_node as u8),
                },
                socket: ddp_socket,
                typ: DdpType {
                    typ: addr_in.sat_type as u8,
                },
            },
        )
        .await?;
    }
    Ok(())
}
