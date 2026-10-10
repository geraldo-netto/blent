use super::*;
use std::future::Future;
use tokio::time::{sleep, Instant, Sleep};

/// Allocate once per direction, then reset in place. Disarming retains storage.
#[derive(Default)]
pub(crate) struct Deadline {
    timer: Option<Pin<Box<Sleep>>>,
    armed: bool,
}
impl Deadline {
    pub fn active(&self) -> bool {
        self.armed
    }
    pub fn start(&mut self, timeout: Duration) {
        match self.timer.as_mut() {
            Some(timer) => timer.as_mut().reset(Instant::now() + timeout),
            None => self.timer = Some(Box::pin(sleep(timeout))),
        }
        self.armed = true;
    }
    pub fn disarm(&mut self) {
        self.armed = false;
    }
    pub fn poll(&mut self, cx: &mut Context<'_>, message: &'static str) -> Result<()> {
        if self.armed {
            ensure!(
                self.timer
                    .as_mut()
                    .expect("armed deadline")
                    .as_mut()
                    .poll(cx)
                    .is_pending(),
                message
            );
        }
        Ok(())
    }
}
