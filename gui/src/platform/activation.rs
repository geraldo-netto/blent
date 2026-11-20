//! Linux GUI admission stays owned by the entry until native shutdown finishes.
//! Abstract sockets have kernel-managed lifetime: no stale paths or unlink races.
use std::{
    io,
    os::{
        linux::net::SocketAddrExt,
        unix::net::{SocketAddr, UnixDatagram},
    },
    rc::Rc,
};

pub(crate) struct Instance(UnixDatagram);

pub(crate) fn run(work: impl FnOnce(Rc<Instance>) -> eframe::Result) -> eframe::Result {
    let directory = blent_config::linux::runtime::runtime_dir()
        .map_err(|cause| error(io::Error::other(cause.to_string())))?;
    run_at(directory.join("gui").as_os_str().as_encoded_bytes(), work)
}

pub(crate) fn run_at(
    name: &[u8],
    work: impl FnOnce(Rc<Instance>) -> eframe::Result,
) -> eframe::Result {
    let address = SocketAddr::from_abstract_name(name).map_err(error)?;
    match UnixDatagram::bind_addr(&address) {
        Ok(socket) => {
            socket.set_nonblocking(true).map_err(error)?;
            let instance = Rc::new(Instance(socket));
            // Keep this reference outside the GUI/runtime closure through cleanup.
            work(instance.clone())
        }
        Err(cause) if cause.kind() == io::ErrorKind::AddrInUse => notify(&address).map_err(error),
        Err(cause) => Err(error(cause)),
    }
}

fn notify(address: &SocketAddr) -> io::Result<()> {
    let socket = UnixDatagram::unbound()?;
    socket.set_nonblocking(true)?;
    match socket.send_to_addr(b"show", address) {
        Ok(_) => Ok(()),
        // A full bounded queue already contains activation; never block a launch.
        Err(cause) if cause.kind() == io::ErrorKind::WouldBlock => Ok(()),
        Err(cause) => Err(cause),
    }
}

impl Instance {
    pub(crate) fn requested(&self) -> io::Result<bool> {
        let mut requested = false;
        // Bound work per frame even if another process floods the socket.
        for _ in 0..32 {
            let mut bytes = [0; 5];
            match self.0.recv(&mut bytes) {
                Ok(4) if &bytes[..4] == b"show" => requested = true,
                Ok(_) => {}
                Err(cause) if cause.kind() == io::ErrorKind::WouldBlock => break,
                Err(cause) => return Err(cause),
            }
        }
        Ok(requested)
    }
}

fn error(error: io::Error) -> eframe::Error {
    eframe::Error::AppCreation(Box::new(error))
}

#[cfg(test)]
mod tests;
