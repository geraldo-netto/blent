//! One asynchronous socket reader. GUI teardown cancels it; the existing process
//! retirement registry retains its worker and original socket until native join.
use std::{
    io,
    os::unix::net::UnixDatagram,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tokio::sync::oneshot;

pub(crate) struct Wake {
    pending: Arc<AtomicBool>,
    stop: Option<oneshot::Sender<()>>,
    _worker: blent::control_workers::Worker,
}
impl Wake {
    pub(super) fn start(
        socket: UnixDatagram,
        notify: impl Fn() + Send + 'static,
    ) -> io::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()?;
        let pending = Arc::new(AtomicBool::new(false));
        let event = pending.clone();
        let (stop, cancelled) = oneshot::channel();
        let pool = blent::control_workers::WorkerPool::new(1)?;
        let worker =
            blent::control_workers::Worker::spawn(pool, "blent-gui-activation", move || {
                if let Err(error) = runtime.block_on(receive(socket, cancelled, event, notify)) {
                    eprintln!("Blent activation reader failed: {error}");
                }
            })?;
        Ok(Self {
            pending,
            stop: Some(stop),
            _worker: worker,
        })
    }
    pub(crate) fn requested(&self) -> bool {
        self.pending.swap(false, Ordering::AcqRel)
    }
}
impl Drop for Wake {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
async fn receive(
    socket: UnixDatagram,
    mut cancelled: oneshot::Receiver<()>,
    pending: Arc<AtomicBool>,
    notify: impl Fn(),
) -> io::Result<()> {
    let socket = tokio::net::UnixDatagram::from_std(socket)?;
    let mut bytes = [0; 5];
    loop {
        tokio::select! {
            biased;
            _ = &mut cancelled => return Ok(()),
            result = socket.recv(&mut bytes) => {
                if result? == 4 && &bytes[..4] == b"show" && !pending.swap(true, Ordering::AcqRel) {
                    notify();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t1550_bounded_events_coalesce_and_shutdown_joins_original_socket_owner() {
        let (socket, sender) = UnixDatagram::pair().unwrap();
        socket.set_nonblocking(true).unwrap();
        let (sent, received) = std::sync::mpsc::channel();
        let wake = Wake::start(socket, move || {
            sent.send(()).unwrap();
        })
        .unwrap();
        for invalid in [b"".as_slice(), b"SHOW", b"show!", b"show trailing"] {
            sender.send(invalid).unwrap();
        }
        sender.send(b"show").unwrap();
        received
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        assert!(wake.requested());
        assert!(!wake.requested());
        sender.send(b"show").unwrap();
        received
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        for _ in 0..32 {
            sender.send(b"show").unwrap();
        }
        assert!(received
            .recv_timeout(std::time::Duration::from_millis(50))
            .is_err());
        let completion = wake._worker.completion();
        drop(wake);
        while !completion.is_complete() {
            std::thread::yield_now();
        }
        assert!(
            sender.send(b"show").is_err(),
            "T1550 retired worker retained its socket"
        );
    }
}
