use super::*;
const DISPLAY: &[Channel] = &[Channel::Control, Channel::Display];
pub(super) fn members(resource: Resource) -> &'static [Channel] {
    match resource {
        Resource::DisplaySession => DISPLAY,
        Resource::Camera => &[Channel::Camera],
        Resource::Microphone => &[Channel::Microphone],
        Resource::Speakers => &[Channel::Speaker],
    }
}
pub(super) fn belongs(resource: Resource, channel: Channel) -> bool {
    members(resource).contains(&channel)
}
pub(super) fn resource(channel: Channel) -> Result<Resource> {
    Ok(match channel {
        Channel::Control | Channel::Display => Resource::DisplaySession,
        Channel::Camera => Resource::Camera,
        Channel::Microphone => Resource::Microphone,
        Channel::Speaker => Resource::Speakers,
        Channel::Management => anyhow::bail!("management has no media consumer"),
    })
}
struct Buffer {
    bytes: VecDeque<u8>,
    borrowed: usize,
    revision: u64,
}
pub(super) struct Channels {
    buffers: [Option<Buffer>; 5],
    capacity: usize,
}
impl Channels {
    pub fn new(capacity: u32) -> Self {
        Self {
            buffers: Default::default(),
            capacity: capacity as usize,
        }
    }
    pub fn install(&mut self, resource: Resource) {
        for channel in members(resource) {
            self.buffers[*channel as usize - 1] = Some(Buffer {
                bytes: VecDeque::with_capacity(self.capacity),
                borrowed: 0,
                revision: 0,
            });
        }
    }
    fn buffer(&mut self, channel: Channel) -> Result<&mut Buffer> {
        self.buffers
            .get_mut(channel as usize - 1)
            .and_then(Option::as_mut)
            .ok_or_else(|| anyhow::anyhow!("consumer not installed"))
    }
    pub fn readable(&self, channel: Channel) -> bool {
        self.buffers
            .get(channel as usize - 1)
            .and_then(Option::as_ref)
            .is_some_and(|buffer| !buffer.bytes.is_empty() && buffer.borrowed == 0)
    }
    pub fn push(&mut self, channel: Channel, bytes: &[u8]) -> Result<()> {
        let capacity = self.capacity;
        let buffer = self.buffer(channel)?;
        ensure!(
            bytes.len() <= capacity - buffer.bytes.len() - buffer.borrowed,
            "consumer window exceeded"
        );
        buffer.bytes.extend(bytes);
        Ok(())
    }
    pub fn read(&mut self, channel: Channel, out: &mut [u8]) -> Result<Option<Read>> {
        ensure!(
            !out.is_empty() && out.len() <= MAX_PAYLOAD as usize,
            "invalid consumer buffer"
        );
        let buffer = self.buffer(channel)?;
        ensure!(buffer.borrowed == 0, "consumer still owns bytes");
        let count = out.len().min(buffer.bytes.len());
        if count == 0 {
            return Ok(None);
        }
        ensure!(buffer.revision < MAX_COUNTER, "consumer borrow exhausted");
        let (first, second) = buffer.bytes.as_slices();
        let first_count = count.min(first.len());
        out[..first_count].copy_from_slice(&first[..first_count]);
        out[first_count..count].copy_from_slice(&second[..count - first_count]);
        buffer.bytes.drain(..count);
        buffer.borrowed = count;
        buffer.revision += 1;
        Ok(Some(Read {
            id: buffer.revision,
            count,
        }))
    }
    pub fn consumed(&mut self, channel: Channel, read: Read) -> Result<()> {
        let buffer = self.buffer(channel)?;
        ensure!(
            read.count > 0 && buffer.borrowed == read.count && read.id == buffer.revision,
            "stale consumer completion"
        );
        buffer.borrowed = 0;
        Ok(())
    }
    /// Call only after actual consumer I/O retirement. Cancellation is not proof.
    pub fn retire(&mut self, resource: Resource) {
        for channel in members(resource) {
            self.buffers[*channel as usize - 1] = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t872_consumer_boundaries_wraparound_and_exhaustion() {
        let mut channels = Channels::new(128);
        for resource in [
            Resource::DisplaySession,
            Resource::Camera,
            Resource::Microphone,
            Resource::Speakers,
        ] {
            channels.install(resource);
        }
        assert!(super::resource(Channel::Management).is_err());
        assert!(channels.push(Channel::Management, b"x").is_err());
        channels.push(Channel::Camera, &[1; 128]).unwrap();
        assert!(channels.push(Channel::Camera, b"x").is_err());
        let first = channels
            .read(Channel::Camera, &mut [0; 100])
            .unwrap()
            .unwrap();
        assert!(channels.push(Channel::Camera, b"x").is_err());
        channels.consumed(Channel::Camera, first).unwrap();
        channels.push(Channel::Camera, &[2; 100]).unwrap();
        let mut bytes = [0; 128];
        let read = channels.read(Channel::Camera, &mut bytes).unwrap().unwrap();
        assert_eq!(&bytes[..28], &[1; 28]);
        assert_eq!(&bytes[28..], &[2; 100]);
        channels.consumed(Channel::Camera, read).unwrap();
        assert!(channels
            .read(Channel::Camera, &mut bytes)
            .unwrap()
            .is_none());
        for length in [0, MAX_PAYLOAD as usize + 1] {
            assert!(channels
                .read(Channel::Camera, &mut vec![0; length])
                .is_err());
        }
        channels.buffer(Channel::Camera).unwrap().revision = MAX_COUNTER;
        channels.push(Channel::Camera, b"x").unwrap();
        assert!(channels.read(Channel::Camera, &mut [0]).is_err());
        channels.retire(Resource::Camera);
        assert!(!channels.readable(Channel::Camera));
    }
}
