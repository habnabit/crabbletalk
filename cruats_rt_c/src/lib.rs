use std::{
    io::{Read, Write},
    os::unix::{
        net::{UnixDatagram, UnixStream},
        prelude::FromRawFd,
    },
};

use anyhow::{Context, Result};
pub use cruats::at::{at_addr, sockaddr_at};
use cruats::zerocopy::{AsBytes, FromBytes, LayoutVerified};
use libc::{c_int, c_void, size_t, ssize_t};
use nix::sys::{socket::MsgFlags, uio::IoVec};
use sendfd::RecvWithFd;

const ADDRLEN: usize = std::mem::size_of::<sockaddr_at>();

fn open_cruats(bind: Option<sockaddr_at>) -> Result<(c_int, sockaddr_at)> {
    let cruats_path =
        std::env::var("CRUATS_CONTROL").context("whilst finding the cruats control socket")?;
    let mut cruats_control = UnixStream::connect(&cruats_path)
        .context("whilst connecting to the cruats control socket")?;
    let mut buffer = [0u8; ADDRLEN];
    if let Some(bind) = bind {
        let _ = bind.write_to(&mut buffer[..]);
    }
    cruats_control
        .write_all(&buffer[..])
        .context("whilst issuing cruats control request")?;
    let mut fds = [-1; 2];
    let (n_read, n_fds) = cruats_control.recv_with_fd(&mut buffer[..], &mut fds)?;
    if n_fds != 1 {
        return Err(anyhow::anyhow!(
            "cruats didn't send back the right reply: {:?}/{} {:?}",
            n_read,
            ADDRLEN,
            n_fds
        ));
    }
    let addr_out = sockaddr_at::read_from(&buffer[..])
        .ok_or_else(|| anyhow::anyhow!("couldn't unpack the sockaddr_at?"))?;
    Ok((fds[0], addr_out))
}

macro_rules! errno {
    ($e:expr) => {
        errno::set_errno(errno::Errno($e));
        return -1;
    };
}

#[no_mangle]
pub extern "C" fn cruats_ddp_open(addr: *mut sockaddr_at, bridge: *mut sockaddr_at) -> c_int {
    println!(
        "*** hello from rust open: addr {:#?} bridge {:#?} ***",
        unsafe { addr.as_ref() },
        unsafe { bridge.as_ref() }
    );
    let bind = unsafe { addr.as_ref() }.cloned();
    let (fd, addr_out) = match open_cruats(bind) {
        Ok(r) => r,
        Err(e) => {
            println!("*** rust fucked up: {:#?} ***", e);
            errno!(libc::EACCES);
        }
    };
    if let Some(dest) = unsafe { addr.as_mut() } {
        println!("*** ok rust open addr out: {:?} ***", addr_out);
        *dest = addr_out;
    }
    fd
}

#[no_mangle]
pub extern "C" fn cruats_ddp_close(socket: c_int) -> c_int {
    println!("*** hello from rust close: {} ***", socket);
    0
}

const NO_FLAGS: MsgFlags = MsgFlags::empty();

#[no_mangle]
pub extern "C" fn cruats_ddp_sendto(
    socket: c_int,
    buf: *const c_void,
    len: size_t,
    flags: c_int,
    addr: *const sockaddr_at,
    addrlen: size_t,
) -> ssize_t {
    let data = if buf.is_null() || len < 1 {
        println!("*** bail from rust: data {:p} {} ***", buf, len);
        errno!(libc::EINVAL);
    } else {
        unsafe { std::slice::from_raw_parts(buf as *const u8, len) }
    };
    let addr = if addr.is_null() || addrlen != ADDRLEN {
        println!("*** bail from rust: addr {:p} {} ***", addr, addrlen);
        errno!(libc::EINVAL);
    } else {
        unsafe { std::slice::from_raw_parts(addr as *const u8, addrlen as usize) }
    };
    let mut addr_local = [0u8; ADDRLEN];
    addr_local.copy_from_slice(addr);
    {
        let lv = LayoutVerified::<_, sockaddr_at>::new(&mut addr_local[..])
            .expect("rust internal error?")
            .into_mut();
        lv.sat_type = data[0] as i16;
        println!(
            "*** hello from rust sendto: {} {} {}: {:?} \n {} vs {}: {:?}\n {:#?} ***",
            socket,
            len,
            flags,
            data,
            addrlen,
            std::mem::size_of::<sockaddr_at>(),
            addr,
            lv,
        );
    }
    let iov = [
        IoVec::from_slice(&addr_local[..]),
        IoVec::from_slice(&data[1..]),
    ];
    match nix::sys::socket::sendmsg(socket, &iov, &[], NO_FLAGS, None) {
        Ok(n) => {
            println!("*** sendmsg back {:?} ***", n);
            len as isize
        }
        Err(e) => {
            println!("*** oh no sendmsg failure: {:?} ***", e);
            -1
        }
    }
}

#[no_mangle]
pub extern "C" fn cruats_ddp_recvfrom(
    socket: c_int,
    buf: *mut c_void,
    len: size_t,
    flags: c_int,
    addr: *mut sockaddr_at,
    addrlen: *mut size_t,
) -> ssize_t {
    println!(
        "*** hello from rust recvfrom: {} {} {} ***",
        socket, len, flags,
    );
    loop {}
}
