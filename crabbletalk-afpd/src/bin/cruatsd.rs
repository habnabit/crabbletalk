use std::{collections::BTreeSet, fs::File, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use cruats::zerocopy::{FromBytes, LayoutVerified, Unalign};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{unix::SocketAddr, UnixDatagram, UnixStream},
};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    router_path: PathBuf,
    cruats_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let (ethertalk, _unlinker1) = crabbletalk_afpd::anonymous_datagram_client("cruatsd")?;
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
                aarp_stack.process_ethernet(data).await?;
            }
            accepted = cruats_control.accept() => {
                let (stream, addr) = accepted?;
                joinset.spawn(drive_stream(stream, addr));
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
    while !joinset.is_empty() {
        joinset.join_one().await?;
    }

    Ok(())
}

async fn drive_stream(mut stream: UnixStream, addr: SocketAddr) -> Result<()> {
    use cruats::at::sockaddr_at;
    use sendfd::SendWithFd;
    use std::mem::size_of;
    use std::os::unix::io::IntoRawFd;
    let cred = stream.peer_cred();
    println!("well who do we got here {:?} {:?}", addr, cred);
    let mut buffer = [0u8; 1600];
    let n_read = stream
        .read_exact(&mut buffer[..size_of::<sockaddr_at>()])
        .await?;
    let (mine, theirs) = UnixDatagram::pair()?;
    let theirs = theirs.into_std()?.into_raw_fd();
    println!("here's {} bytes: {:?}", n_read, &buffer[..n_read]);
    stream.writable().await?;
    let res = stream.send_with_fd(&buffer, &[theirs]);
    println!("1: {:?}", res);
    stream.shutdown().await?;
    drop(stream);

    loop {
        let n_read = mine.recv(&mut buffer[..]).await?;
        let (addr_in, payload) = LayoutVerified::<_, Unalign<sockaddr_at>>::new_unaligned_from_prefix(
            &buffer[..n_read],
        )
        .ok_or_else(|| {
            anyhow::anyhow!(
                "couldn't unpack the sockaddr_at? {}: {:?}",
                n_read,
                &buffer[..n_read]
            )
        })?;
        let addr_in = addr_in.into_ref().get();
        println!("from {:?} {:?}: {:?}", cred, addr_in, payload);
    }
    Ok(())
}
