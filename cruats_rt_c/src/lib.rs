pub use cruats::at::sockaddr_at;
use libc::{c_int, c_void, size_t, socklen_t, ssize_t, c_uint};

#[no_mangle]
pub extern "C" fn cruats_ddp_open(addr: *mut sockaddr_at, bridge: *mut sockaddr_at) -> c_int {
    errno::set_errno(errno::Errno(libc::EACCES));
    println!(
        "*** hello from rust open: addr {:#?} bridge {:#?} ***",
        unsafe { addr.as_ref() },
        unsafe { bridge.as_ref() }
    );
    99
}

#[no_mangle]
pub extern "C" fn cruats_ddp_close(socket: c_int) -> c_int {
    println!(
        "*** hello from rust close: {} ***",
        socket
    );
    0
}

#[no_mangle]
pub extern "C" fn cruats_ddp_sendto(
    socket: c_int,
    buf: *const c_void,
    len: size_t,
    flags: c_int,
    addr: *const c_void,
    addrlen: c_uint,
) -> ssize_t {
    let data = if buf.is_null() {
        None
    } else {
        unsafe {
            Some(std::slice::from_raw_parts(buf as *const u8, len))
        }
    };
    let addr = if addr.is_null() {
        None
    } else {
        unsafe {
            Some(std::slice::from_raw_parts(addr as *const u8, addrlen as usize))
        }
    };
    println!(
        "*** hello from rust sendto: {} {} {}: {:?} \n {} vs {}: {:?} ***",
        socket,
        len,
        flags,
        data,
        addrlen,
        std::mem::size_of::<sockaddr_at>(),
        addr,
    );
    0
}

#[no_mangle]
pub extern "C" fn cruats_ddp_recvfrom(
    socket: c_int,
    buf: *mut c_void,
    len: size_t,
    flags: c_int,
    addr: *mut c_void,
    addrlen: *mut c_uint,
) -> ssize_t {
    println!(
        "*** hello from rust recvfrom: {} {} {} ***",
        socket, len, flags,
    );
    loop {}
}
