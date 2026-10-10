use super::super::transport::deadline::Deadline;
use super::*;
use blent_config::connection::endpoint::Read;
use std::pin::Pin;

pub(super) struct Receive {
    pub(super) bytes: Vec<u8>,
    read: Option<Read>,
    written: usize,
    deadline: Deadline,
    timeout: Duration,
}
impl Receive {
    pub fn new(block: usize, timeout: Duration) -> Self {
        Self {
            bytes: vec![0; block],
            read: None,
            written: 0,
            deadline: Deadline::default(),
            timeout,
        }
    }
    pub fn poll<S: AsyncWrite + Unpin>(
        &mut self,
        cx: &mut Context<'_>,
        stream: &mut S,
        endpoint: &mut dyn Endpoint,
    ) -> Poll<Result<()>> {
        if self.read.is_none() {
            let Some(read) = endpoint.read(&mut self.bytes)? else {
                return Poll::Pending;
            };
            require(
                read.id > 0 && read.count > 0 && read.count <= self.bytes.len(),
                "invalid bridge consumer extent",
            )?;
            self.read = Some(read);
            self.written = 0;
            self.deadline.start(self.timeout);
        }
        self.deadline.poll(cx, "local consumer deadline expired")?;
        self.write(cx, stream, endpoint)
    }
    fn write<S: AsyncWrite + Unpin>(
        &mut self,
        cx: &mut Context<'_>,
        stream: &mut S,
        endpoint: &mut dyn Endpoint,
    ) -> Poll<Result<()>> {
        let read = self.read.expect("consumer owns bytes");
        if self.written < read.count {
            let count = std::task::ready!(
                Pin::new(&mut *stream).poll_write(cx, &self.bytes[self.written..read.count])
            )?;
            require(
                count > 0 && count <= read.count - self.written,
                "invalid local write progress",
            )?;
            self.written += count;
            return Poll::Ready(Ok(()));
        }
        std::task::ready!(Pin::new(stream).poll_flush(cx))?;
        endpoint.consumed(read)?;
        self.read = None;
        self.deadline.disarm();
        Poll::Ready(Ok(()))
    }
}
