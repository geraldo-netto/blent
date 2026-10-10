//! A retained listening descriptor, not a claimed port, proves endpoint ownership.
use anyhow::{ensure, Result};
use blent_config::connection::{
    management::Resource,
    routes::{RouteClaims, RouteId},
};
use std::net::{IpAddr, SocketAddr, TcpListener};
use std::os::fd::{AsRawFd, OwnedFd};

pub struct Endpoints {
    // Never accept from these duplicates. Retaining them pins the bound ports
    // until every bridge retires; the caller continues accepting its own sockets.
    listeners: Vec<TcpListener>,
    addresses: Vec<SocketAddr>,
    route: Option<RouteId>,
}
impl Endpoints {
    pub fn validate(resource: Resource, descriptors: Vec<OwnedFd>) -> Result<Self> {
        let expected = if resource == Resource::DisplaySession {
            2
        } else {
            1
        };
        ensure!(descriptors.len() == expected, "wrong local listener count");
        let listeners = descriptors
            .into_iter()
            .map(listener)
            .collect::<Result<Vec<_>>>()?;
        let addresses = listeners
            .iter()
            .map(TcpListener::local_addr)
            .collect::<std::io::Result<Vec<_>>>()?;
        ensure!(
            addresses.iter().all(valid_address),
            "endpoint must bind IPv4 localhost"
        );
        if expected == 2 {
            ensure!(
                addresses[0].port() != addresses[1].port(),
                "duplicate display listener"
            );
        }
        Ok(Self {
            listeners,
            addresses,
            route: None,
        })
    }
    pub fn addresses(&self) -> &[SocketAddr] {
        &self.addresses
    }
    /// Lazy connector; Bridges polls it only after current media admission.
    /// Every address came from retained listening descriptors, never peer bytes.
    pub fn connect(&self) -> Result<crate::broker::bridges::Connect> {
        ensure!(self.route.is_some(), "endpoint not registered");
        let addresses = self.addresses.clone();
        Ok(Box::pin(async move {
            let mut streams: Vec<Box<dyn crate::accessory::Stream>> = Vec::new();
            for address in addresses {
                let stream = tokio::net::TcpStream::connect(address).await?;
                stream.set_nodelay(true)?;
                streams.push(Box::new(stream));
            }
            Ok(streams)
        }))
    }
    pub fn reserve(&mut self, claims: &mut RouteClaims, owner: u64) -> Result<RouteId> {
        ensure!(self.route.is_none(), "endpoint already registered");
        let ports = self
            .addresses
            .iter()
            .map(|address| u32::from(address.port()))
            .collect::<Vec<_>>();
        let route = claims.reserve(owner, &ports)?;
        self.route = Some(route);
        Ok(route)
    }
    /// Release only after all bridge/socket I/O has actually retired. The caller
    /// may still own its original listener, but this broker's claim ends here.
    pub fn retire(&mut self, claims: &mut RouteClaims, route: RouteId) -> Result<()> {
        // Validate before dropping descriptors: a wrong/still-active claim cannot
        // release the capability accidentally. Registry enforces prior revoke.
        ensure!(self.route == Some(route), "wrong endpoint route");
        claims.retired(route)?;
        self.listeners.clear();
        self.addresses.clear();
        self.route = None;
        Ok(())
    }
}
fn valid_address(address: &SocketAddr) -> bool {
    address.ip() == IpAddr::V4(std::net::Ipv4Addr::LOCALHOST) && address.port() != 0
}
fn listener(fd: OwnedFd) -> Result<TcpListener> {
    let kind = option(&fd, libc::SO_TYPE)?;
    let protocol = option(&fd, libc::SO_PROTOCOL)?;
    let accepting = option(&fd, libc::SO_ACCEPTCONN)?;
    let reuse = option(&fd, libc::SO_REUSEPORT)?;
    validate_options(kind, protocol, accepting, reuse)?;
    Ok(TcpListener::from(fd))
}
fn option(fd: &OwnedFd, name: i32) -> Result<i32> {
    let mut value = 0i32;
    let mut length = std::mem::size_of::<i32>() as libc::socklen_t;
    let result = unsafe {
        libc::getsockopt(
            fd.as_raw_fd(),
            libc::SOL_SOCKET,
            name,
            (&mut value as *mut i32).cast(),
            &mut length,
        )
    };
    ensure!(
        result == 0 && length as usize == std::mem::size_of::<i32>(),
        "invalid listener metadata"
    );
    Ok(value)
}
fn validate_options(kind: i32, protocol: i32, accepting: i32, reuse: i32) -> Result<()> {
    ensure!(
        (kind, protocol, accepting, reuse) == (libc::SOCK_STREAM, libc::IPPROTO_TCP, 1, 0),
        "endpoint must be an exclusive TCP listener"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t871_socket_metadata_never_substitutes_another_protocol_or_shared_listener() {
        validate_options(libc::SOCK_STREAM, libc::IPPROTO_TCP, 1, 0).unwrap();
        for value in -1024..=1024 {
            assert_eq!(
                validate_options(value, libc::IPPROTO_TCP, 1, 0).is_ok(),
                value == libc::SOCK_STREAM
            );
            assert_eq!(
                validate_options(libc::SOCK_STREAM, value, 1, 0).is_ok(),
                value == libc::IPPROTO_TCP
            );
            assert_eq!(
                validate_options(libc::SOCK_STREAM, libc::IPPROTO_TCP, value, 0).is_ok(),
                value == 1
            );
            assert_eq!(
                validate_options(libc::SOCK_STREAM, libc::IPPROTO_TCP, 1, value).is_ok(),
                value == 0
            );
        }
    }
}
