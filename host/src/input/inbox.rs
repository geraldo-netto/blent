//! A bounded read-ahead queue keeps peer close observable during native input.
//! Buffered frames preserve wire order and are never dispatched after cutoff.
use super::{Dispatch, InputSocket};
use futures_util::{stream::SplitStream, StreamExt};
use std::collections::VecDeque;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::{Error, Message};

type Incoming = std::result::Result<Message, Error>;
const PENDING_FRAMES: usize = 16;
pub(super) struct Inbox {
    receiver: SplitStream<InputSocket>,
    queued: VecDeque<Incoming>,
}
impl Inbox {
    pub(super) fn new(receiver: SplitStream<InputSocket>) -> Self {
        Self {
            receiver,
            queued: VecDeque::new(),
        }
    }
    pub(super) async fn next(&mut self) -> Option<Incoming> {
        match self.queued.pop_front() {
            Some(message) => Some(message),
            None => self.receiver.next().await,
        }
    }
    pub(super) async fn complete(
        &mut self,
        dispatch: Dispatch,
        ownership: &mut watch::Receiver<u64>,
    ) -> bool {
        let completion = dispatch.finish();
        tokio::pin!(completion);
        loop {
            tokio::select! {
                biased;
                _ = ownership.changed() => return false,
                accepted = &mut completion => return accepted,
                incoming = self.receiver.next() => {
                    if !self.buffer(incoming) { return false; }
                }
            }
        }
    }
    fn buffer(&mut self, incoming: Option<Incoming>) -> bool {
        let Some(Ok(message)) = incoming else {
            return false;
        };
        if matches!(message, Message::Close(_)) || self.queued.len() == PENDING_FRAMES {
            return false;
        }
        self.queued.push_back(Ok(message));
        true
    }
}
