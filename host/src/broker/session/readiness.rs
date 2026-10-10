use super::*;
impl<S: AsyncRead + AsyncWrite + Unpin> Session<S> {
    pub(super) fn mark_readable(&mut self, key: Key, channel: Channel) {
        if self.channels.readable(channel) {
            self.readable[channel as usize - 1] = Some(key);
        }
    }
    pub(super) fn next_notice(&mut self) -> Option<Event> {
        if let Some(event) = self.events.pop_front() {
            return Some(event);
        }
        for distance in 0..self.readable.len() {
            let index = (self.readable_cursor + distance) % self.readable.len();
            if let Some(event) = self.readiness_at(index) {
                self.readable_cursor = (index + 1) % self.readable.len();
                return Some(event);
            }
        }
        None
    }
    fn readiness_at(&mut self, index: usize) -> Option<Event> {
        let key = self.readable[index].take()?;
        let binding = self.slots[key.resource as usize - 1].as_ref()?;
        let channel = Channel::try_from(index as u8 + 1).expect("fixed media index");
        if binding.lease.key == key
            && self.current(&binding.lease)
            && self.channels.readable(channel)
        {
            Some(Event::Readable(key, channel))
        } else {
            None
        }
    }
}
