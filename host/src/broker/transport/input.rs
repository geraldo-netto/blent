use super::*;
use tokio::io::ReadBuf;

pub(super) struct Input {
    header_bytes: [u8; HEADER_BYTES],
    bytes: Box<[u8]>,
    offset: usize,
    header: Option<(Header, bool)>,
    timer: Deadline,
}
impl Input {
    pub fn new() -> Self {
        Self {
            header_bytes: [0; HEADER_BYTES],
            bytes: vec![0; MAX_PAYLOAD as usize].into_boxed_slice(),
            offset: 0,
            header: None,
            timer: Deadline::default(),
        }
    }
    pub fn payload(&self) -> &[u8] {
        let length = self.header.map_or(0, |(header, _)| header.length as usize);
        &self.bytes[..length]
    }
    pub fn clear(&mut self) {
        self.offset = 0;
        self.header = None;
        self.timer.disarm();
    }
    pub fn poll<S: AsyncRead + Unpin>(
        &mut self,
        cx: &mut Context<'_>,
        mut stream: Pin<&mut S>,
        mux: &mut Multiplexer,
        timeout: Duration,
    ) -> Poll<Result<(Header, bool)>> {
        self.timer.poll(cx, "receive frame deadline")?;
        if self.header.is_none() {
            std::task::ready!(self.read_header(cx, stream.as_mut(), timeout))?;
            let header = Header::decode(&self.header_bytes)?;
            let deliver = mux.receive(header)?;
            self.header = Some((header, deliver));
            self.offset = 0;
        }
        let (header, deliver) = self.header.expect("decoded header");
        std::task::ready!(read_part(
            cx,
            stream,
            &mut self.bytes[..header.length as usize],
            &mut self.offset
        ))?;
        self.timer.disarm();
        Poll::Ready(Ok((
            header,
            deliver && mux.accepts_data(header.channel, header.generation),
        )))
    }
    fn read_header<S: AsyncRead + Unpin>(
        &mut self,
        cx: &mut Context<'_>,
        stream: Pin<&mut S>,
        timeout: Duration,
    ) -> Poll<Result<()>> {
        let result = read_part(cx, stream, &mut self.header_bytes, &mut self.offset);
        if !self.timer.active() && self.offset > 0 {
            self.timer.start(timeout);
            self.timer.poll(cx, "receive frame deadline")?;
        }
        result
    }
}
fn read_part<S: AsyncRead + Unpin>(
    cx: &mut Context<'_>,
    mut stream: Pin<&mut S>,
    bytes: &mut [u8],
    offset: &mut usize,
) -> Poll<Result<()>> {
    if *offset == bytes.len() {
        return Poll::Ready(Ok(()));
    }
    let mut buffer = ReadBuf::new(&mut bytes[*offset..]);
    std::task::ready!(stream.as_mut().poll_read(cx, &mut buffer))?;
    if buffer.filled().is_empty() {
        return Poll::Ready(Err(anyhow::anyhow!("connection EOF in frame")));
    }
    *offset += buffer.filled().len();
    if *offset == bytes.len() {
        Poll::Ready(Ok(()))
    } else {
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}
