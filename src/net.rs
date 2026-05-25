use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::mem::MaybeUninit;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};
use std::time::Duration;

const DEST_PORT: u16 = 33434;
const READ_TIMEOUT_SECS: u64 = 2;

pub fn probe_v4(target: Ipv4Addr, ttl: u32) -> std::io::Result<Option<Ipv4Addr>> {
    let send_sock = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    send_sock.set_ttl_v4(ttl)?;

    let recv_sock = Socket::new(
        Domain::IPV4,
        Type::from(libc::SOCK_RAW),
        Some(Protocol::ICMPV4),
    )?;
    recv_sock.set_read_timeout(Some(Duration::from_secs(READ_TIMEOUT_SECS)))?;

    let dest = SockAddr::from(SocketAddrV4::new(target, DEST_PORT));
    send_sock.send_to(&[0u8; 32], &dest)?;

    let mut buf = [MaybeUninit::<u8>::uninit(); 512];
    match recv_sock.recv(&mut buf) {
        Ok(n) => {
            let buf: &[u8] = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, n) };
            if buf.len() >= 20 {
                Ok(Some(Ipv4Addr::new(buf[12], buf[13], buf[14], buf[15])))
            } else {
                Ok(None)
            }
        }
        Err(_) => Ok(None),
    }
}

pub fn probe_v6(target: Ipv6Addr, hops: u32) -> std::io::Result<Option<Ipv6Addr>> {
    let send_sock = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?;
    send_sock.set_unicast_hops_v6(hops)?;

    let recv_sock = Socket::new(
        Domain::IPV6,
        Type::from(libc::SOCK_RAW),
        Some(Protocol::ICMPV6),
    )?;
    recv_sock.set_read_timeout(Some(Duration::from_secs(READ_TIMEOUT_SECS)))?;

    let dest = SockAddr::from(SocketAddrV6::new(target, DEST_PORT, 0, 0));
    send_sock.send_to(&[0u8; 32], &dest)?;

    let mut buf = [MaybeUninit::<u8>::uninit(); 1024];
    match recv_sock.recv(&mut buf) {
        Ok(n) => {
            let buf: &[u8] = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u8, n) };
            // IPv6 header source address is bytes 8..24
            if buf.len() >= 24 {
                let mut octets = [0u8; 16];
                octets.copy_from_slice(&buf[8..24]);
                Ok(Some(Ipv6Addr::from(octets)))
            } else {
                Ok(None)
            }
        }
        Err(_) => Ok(None),
    }
}
