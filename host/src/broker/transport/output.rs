use super::*;

pub(super) struct Output {
    header: [u8; HEADER_BYTES],
    header_offset: usize,
    body_offset: usize,
    timer: Deadline,
}
impl Default for Output {
    fn default() -> Self {
        Self {
            header: [0; HEADER_BYTES],
            header_offset: 0,
            body_offset: 0,
            timer: Deadline::default(),
        }
    }
}
impl Output {
    pub fn poll<S: AsyncWrite + Unpin>(
        &mut self,
        cx: &mut Context<'_>,
        mut stream: Pin<&mut S>,
        mux: &mut Multiplexer,
        timeout: Duration,
    ) -> Poll<Result<Header>> {
        if !self.timer.active() {
            let Some(frame) = mux.next()? else {
                return Poll::Pending;
            };
            frame.header.encode(&mut self.header)?;
            self.timer.start(timeout);
        }
        self.timer.poll(cx, "write frame deadline")?;
        let frame = mux
            .reserved()
            .ok_or_else(|| anyhow::anyhow!("writer reservation lost"))?;
        std::task::ready!(write_part(
            cx,
            stream.as_mut(),
            &self.header,
            &mut self.header_offset
        ))?;
        std::task::ready!(write_part(
            cx,
            stream.as_mut(),
            frame.bytes,
            &mut self.body_offset
        ))?;
        std::task::ready!(stream.as_mut().poll_flush(cx))?;
        let header = frame.header;
        mux.finish(true)?;
        self.header_offset = 0;
        self.body_offset = 0;
        self.timer.disarm();
        Poll::Ready(Ok(header))
    }
}
fn write_part<S: AsyncWrite + Unpin>(
    cx: &mut Context<'_>,
    mut stream: Pin<&mut S>,
    bytes: &[u8],
    offset: &mut usize,
) -> Poll<Result<()>> {
    if *offset == bytes.len() {
        return Poll::Ready(Ok(()));
    }
    let count = std::task::ready!(stream.as_mut().poll_write(cx, &bytes[*offset..]))?;
    if count == 0 {
        return Poll::Ready(Err(anyhow::anyhow!("connection zero-byte write")));
    }
    *offset += count;
    if *offset == bytes.len() {
        Poll::Ready(Ok(()))
    } else {
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}
