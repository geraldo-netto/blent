use super::super::transport::deadline::Deadline;
use super::*;
use std::pin::Pin;
use tokio::io::ReadBuf;

pub(super) struct Send {
    bytes: Vec<u8>,
    pending: bool,
    block: usize,
    deadline: Deadline,
    timeout: Duration,
}
impl Send {
    pub fn new(block: usize, timeout: Duration) -> Self {
        Self {
            bytes: Vec::new(),
            pending: false,
            block,
            deadline: Deadline::default(),
            timeout,
        }
    }
    pub fn poll<S: AsyncRead + Unpin>(
        &mut self,
        cx: &mut Context<'_>,
        stream: &mut S,
        endpoint: &mut dyn Endpoint,
    ) -> Poll<Result<()>> {
        if !self.pending {
            if self.bytes.is_empty() {
                self.bytes = vec![0; self.block.min(endpoint.capacity())];
            }
            let mut buffer = ReadBuf::new(&mut self.bytes);
            std::task::ready!(Pin::new(stream).poll_read(cx, &mut buffer))?;
            let count = buffer.filled().len();
            require(count > 0, "local producer closed")?;
            self.bytes.truncate(count);
            self.pending = true;
            self.deadline.start(self.timeout);
        }
        self.deadline
            .poll(cx, "local producer backpressure deadline expired")?;
        match endpoint.offer(std::mem::take(&mut self.bytes)) {
            Ok(()) => {
                self.pending = false;
                self.deadline.disarm();
                Poll::Ready(Ok(()))
            }
            Err(bytes) => {
                self.bytes = bytes;
                Poll::Pending
            }
        }
    }
}
