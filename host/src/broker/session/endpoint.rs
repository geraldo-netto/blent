use super::*;
use blent_config::connection::endpoint::Endpoint as Contract;

pub struct Endpoint<'a, S> {
    session: &'a mut Session<S>,
    lease: &'a Lease,
    channel: Channel,
}
impl<S: AsyncRead + AsyncWrite + Unpin> Session<S> {
    pub fn endpoint<'a>(&'a mut self, lease: &'a Lease, channel: Channel) -> Endpoint<'a, S> {
        Endpoint {
            session: self,
            lease,
            channel,
        }
    }
}
impl<S: AsyncRead + AsyncWrite + Unpin> Contract for Endpoint<'_, S> {
    fn capacity(&self) -> usize {
        self.session.channels.capacity()
    }
    fn identity(&self) -> (&Lease, Channel) {
        (self.lease, self.channel)
    }
    fn active(&self) -> bool {
        self.session.current(self.lease) && channels::belongs(self.lease.key.resource, self.channel)
    }
    fn offer(&mut self, bytes: Vec<u8>) -> std::result::Result<(), Vec<u8>> {
        self.session.offer(self.lease, self.channel, bytes)
    }
    fn read(&mut self, out: &mut [u8]) -> Result<Option<Read>> {
        self.session.read(self.lease, self.channel, out)
    }
    fn consumed(&mut self, read: Read) -> Result<()> {
        self.session.consumed(self.lease, self.channel, read)
    }
}
