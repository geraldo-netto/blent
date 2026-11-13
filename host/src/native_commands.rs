//! Linux command port: reserve both exact child/collector observations before effects.
use blent_config::{
    commands::{AsyncCommandExt, COMMAND_TIMEOUT},
    retirement::{
        observations::{Observations, Reservation},
        Completion,
    },
};
use std::{
    future::Future,
    io,
    pin::Pin,
    process::Output,
    sync::{Mutex, MutexGuard},
    time::Duration,
};

pub(crate) trait CommandPort: Sync {
    fn output<'a>(
        &'a self,
        program: &'a str,
        args: &'a [&'a str],
    ) -> Pin<Box<dyn Future<Output = io::Result<Output>> + Send + 'a>>;
}

/// Ordinary command callers own their existing command adapter contract.
pub(crate) struct SystemCommands;
impl CommandPort for SystemCommands {
    fn output<'a>(
        &'a self,
        program: &'a str,
        args: &'a [&'a str],
    ) -> Pin<Box<dyn Future<Output = io::Result<Output>> + Send + 'a>> {
        Box::pin(async move {
            tokio::process::Command::new(program)
                .args(args)
                .output_bounded()
                .await
        })
    }
}

#[derive(Default)]
pub(crate) struct ObservedCommands {
    originals: Mutex<Observations<2>>,
}
impl ObservedCommands {
    fn originals(&self) -> MutexGuard<'_, Observations<2>> {
        self.originals
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    pub(crate) fn complete(&self) -> bool {
        self.originals().complete()
    }
    fn reserve(&self) -> io::Result<[Reservation; 2]> {
        let mut originals = self.originals();
        let collector = originals.reserve()?;
        let child = originals.reserve()?;
        Ok([collector, child])
    }
    async fn run(&self, program: &str, args: &[&str]) -> io::Result<Output> {
        // A command deadline does not authorize another effect while the old
        // original child or collector can still act. Cancellation stops this wait.
        while !self.complete() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        let receipts = Mutex::new(self.reserve()?.map(Some));
        let observe = |proof: Completion| {
            receipts
                .lock()
                .unwrap()
                .iter_mut()
                .find_map(Option::take)
                .expect("command contract reports at most collector and child")
                .bind(proof);
        };
        tokio::process::Command::new(program)
            .args(args)
            .output_input_timeout_observed(None, COMMAND_TIMEOUT, &observe)
            .await
    }
}
impl CommandPort for ObservedCommands {
    fn output<'a>(
        &'a self,
        program: &'a str,
        args: &'a [&'a str],
    ) -> Pin<Box<dyn Future<Output = io::Result<Output>> + Send + 'a>> {
        Box::pin(self.run(program, args))
    }
}

#[cfg(test)]
mod tests;
