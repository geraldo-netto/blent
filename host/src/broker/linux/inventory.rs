//! Inventory startup and reads stay outside the shared client/media dispatcher.
use crate::{
    accessory::linux::discovery::{Candidate, Limits, Scan},
    owned_task::Task,
};
use anyhow::{Context, Result};
use blent_config::retirement::Completion;
use std::path::PathBuf;
use tokio::sync::mpsc;

enum Event {
    Started(Completion),
    Finished(Result<Vec<Candidate>>),
}
pub struct Inventory {
    task: Task,
    events: mpsc::Receiver<Event>,
    native: Option<Completion>,
    finished: bool,
    terminal: Option<Result<Vec<Candidate>>>,
}
impl Inventory {
    pub fn start(helper: PathBuf, limits: Limits) -> Result<Self> {
        let (output, events) = mpsc::channel(2);
        let task = Task::spawn(async move {
            let result = scan(helper, limits, &output).await;
            output
                .send(Event::Finished(result))
                .await
                .context("inventory receiver gone")?;
            Ok(())
        })?;
        Ok(Self {
            task,
            events,
            native: None,
            finished: false,
            terminal: None,
        })
    }
    pub async fn next(&mut self) -> Result<Vec<Candidate>> {
        anyhow::ensure!(!self.finished, "inventory already returned");
        loop {
            if self.terminal.is_some() {
                let joined = (&mut self.task).await;
                self.finished = true;
                joined??;
                return self.terminal.take().unwrap();
            }
            match self.events.recv().await {
                Some(Event::Started(native)) => self.native = Some(native),
                Some(Event::Finished(result)) => {
                    self.terminal = Some(result);
                }
                None => {
                    self.terminal = Some(Err(anyhow::anyhow!("inventory worker exited")));
                }
            }
        }
    }
    /// Keep this object until both the async worker and actual native helper
    /// retired. In particular, a read timeout cannot authorize another scan.
    pub fn retired(&self) -> bool {
        self.finished
            && self.task.completion().is_complete()
            && self.native.as_ref().is_none_or(Completion::is_complete)
    }
    pub fn finished(&self) -> bool {
        self.finished
    }
}
async fn scan(
    helper: PathBuf,
    limits: Limits,
    output: &mpsc::Sender<Event>,
) -> Result<Vec<Candidate>> {
    let scan = tokio::task::spawn_blocking(move || Scan::start(&helper, limits)).await??;
    output
        .send(Event::Started(scan.completion()))
        .await
        .context("inventory receiver gone")?;
    scan.finish().await
}

#[cfg(test)]
mod tests;
