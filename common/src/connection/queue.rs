//! Bounded, fair payload scheduling. Queue owns buffers; writes borrow slices.
use super::wire::{Channel, CHANNELS, MAX_CREDIT, MAX_PAYLOAD};
use anyhow::{ensure, Result};
use std::collections::VecDeque;

struct Pending {
    bytes: Vec<u8>,
    offset: usize,
}
#[derive(Default)]
struct Queue {
    items: VecDeque<Pending>,
    retained: usize,
}
pub struct Slice<'a> {
    pub channel: Channel,
    pub bytes: &'a [u8],
}
/// One outstanding writer reservation. A stalled channel without credit is
/// skipped. Queued allocations remain charged until their final slice is sent.
/// Native adapters must bound I/O and retire a writer before cancelling its slot.
pub struct Scheduler {
    queues: [Queue; CHANNELS as usize],
    capacity: usize,
    messages: usize,
    quantum: usize,
    cursor: usize,
    active: Option<(usize, usize)>,
}
impl Scheduler {
    pub fn new(capacity: u32, messages: usize, quantum: u32) -> Result<Self> {
        ensure!(
            (1..=MAX_CREDIT).contains(&capacity),
            "invalid queue capacity"
        );
        ensure!((1..=1024).contains(&messages), "invalid message capacity");
        ensure!(
            (1..=MAX_PAYLOAD).contains(&quantum),
            "invalid scheduling quantum"
        );
        Ok(Self {
            queues: Default::default(),
            capacity: capacity as usize,
            messages,
            quantum: quantum as usize,
            cursor: 0,
            active: None,
        })
    }
    /// On rejection ownership is returned, with neither copy nor queue mutation.
    pub fn offer(&mut self, channel: Channel, bytes: Vec<u8>) -> std::result::Result<(), Vec<u8>> {
        let queue = &mut self.queues[channel as usize - 1];
        if bytes.is_empty()
            || bytes.len() > MAX_PAYLOAD as usize
            || bytes.capacity() > self.capacity - queue.retained
            || queue.items.len() >= self.messages
        {
            return Err(bytes);
        }
        queue.retained += bytes.capacity();
        queue.items.push_back(Pending { bytes, offset: 0 });
        Ok(())
    }
    pub fn next(&mut self, credit: &[u32; CHANNELS as usize]) -> Result<Option<Slice<'_>>> {
        ensure!(
            credit.iter().all(|value| *value <= MAX_CREDIT),
            "invalid scheduling credit"
        );
        if self.active.is_some() {
            return Ok(None);
        }
        for distance in 0..CHANNELS as usize {
            let index = (self.cursor + distance) % CHANNELS as usize;
            if credit[index] == 0 {
                continue;
            }
            if let Some(length) = self.slice_length(index, credit[index]) {
                return Ok(Some(self.reserve(index, length)));
            }
        }
        Ok(None)
    }
    fn slice_length(&self, index: usize, credit: u32) -> Option<usize> {
        self.queues[index].items.front().map(|item| {
            (item.bytes.len() - item.offset)
                .min(self.quantum)
                .min(credit as usize)
        })
    }
    fn reserve(&mut self, index: usize, length: usize) -> Slice<'_> {
        self.active = Some((index, length));
        self.cursor = (index + 1) % CHANNELS as usize;
        let item = self.queues[index]
            .items
            .front()
            .expect("selected nonempty queue");
        Slice {
            channel: Channel::try_from(index as u8 + 1).expect("bounded queue index"),
            bytes: &item.bytes[item.offset..item.offset + length],
        }
    }
    pub fn reserved(&self) -> Option<Slice<'_>> {
        let (index, length) = self.active?;
        let item = self.queues[index]
            .items
            .front()
            .expect("reserved queue remains owned");
        Some(Slice {
            channel: Channel::try_from(index as u8 + 1).expect("bounded queue index"),
            bytes: &item.bytes[item.offset..item.offset + length],
        })
    }
    /// Complete only after the complete slice is written. Cancellation preserves
    /// bytes for an unsent reservation; partial transport writes retire the stream.
    pub fn finish(&mut self, sent: bool) -> Result<()> {
        let (index, length) = self
            .active
            .take()
            .ok_or_else(|| anyhow::anyhow!("no reserved write"))?;
        if sent {
            let queue = &mut self.queues[index];
            let item = queue
                .items
                .front_mut()
                .expect("reserved queue remains owned");
            item.offset += length;
            if item.offset == item.bytes.len() {
                queue.retained -= item.bytes.capacity();
                queue.items.pop_front();
            }
        }
        Ok(())
    }
    pub fn clear(&mut self, channel: Channel) -> Result<()> {
        let index = channel as usize - 1;
        ensure!(
            self.active.is_none_or(|(active, _)| active != index),
            "writer still owns channel"
        );
        self.queues[index] = Queue::default();
        Ok(())
    }
}
