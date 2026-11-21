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
    fn set_property(
        &self,
        _request: Property,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>> {
        Box::pin(async { Err(io::ErrorKind::Unsupported.into()) })
    }
    /// Query-only ports cannot authorize a mutation without an owning sponsor.
    fn mutate<'a>(
        &'a self,
        _program: &'a str,
        _args: &'a [&'a str],
    ) -> Pin<Box<dyn Future<Output = io::Result<Output>> + Send + 'a>> {
        Box::pin(async { Err(io::ErrorKind::Unsupported.into()) })
    }
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

mod dbus;
mod mutation;
mod property;
pub(crate) use property::Property;

/// One command/mutation slot, funded by the owning input payload or OSK operation.
/// That owner must retain this port until complete(), including caller loss.
#[derive(Default)]
pub(crate) struct ObservedCommands {
    state: Mutex<State>,
}
#[derive(Default)]
struct State {
    originals: Observations<2>,
    mutation: Option<mutation::Operation>,
}
impl State {
    fn complete(&mut self) -> bool {
        if !self.originals.complete() {
            return false;
        }
        if let Some(operation) = self.mutation.as_mut() {
            if !operation.complete() {
                return false;
            }
            self.mutation = None;
        }
        true
    }
    fn reserve(&mut self) -> io::Result<[Reservation; 2]> {
        if !self.complete() {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let collector = self.originals.reserve()?;
        let child = self.originals.reserve()?;
        Ok([collector, child])
    }
}
impl ObservedCommands {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    pub(crate) fn complete(&self) -> bool {
        self.state().complete()
    }
    fn reserve(&self) -> io::Result<[Reservation; 2]> {
        self.state().reserve()
    }
    fn start_mutation(
        &self,
        program: &str,
        args: &[&str],
    ) -> io::Result<Option<tokio::sync::oneshot::Receiver<io::Result<Output>>>> {
        let mut state = self.state();
        if !state.complete() {
            return Ok(None);
        }
        let receipts = state.reserve()?;
        let (operation, result) = mutation::Operation::start(program, args, receipts);
        state.mutation = Some(operation);
        Ok(Some(result))
    }
    async fn mutation_result(&self, program: &str, args: &[&str]) -> io::Result<Output> {
        loop {
            if let Some(result) = self.start_mutation(program, args)? {
                return result
                    .await
                    .unwrap_or_else(|_| Err(io::Error::other("original mutation result lost")));
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }
    async fn mutate_bounded(&self, program: &str, args: &[&str]) -> io::Result<Output> {
        tokio::time::timeout(COMMAND_TIMEOUT, self.mutation_result(program, args))
            .await
            .unwrap_or_else(|_| {
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "mutation result deadline; original operation remains owned",
                ))
            })
    }
    fn start_property(
        &self,
        request: &mut Option<Property>,
    ) -> Option<tokio::sync::oneshot::Receiver<io::Result<()>>> {
        let mut state = self.state();
        if !state.complete() {
            return None;
        }
        let (operation, result) =
            mutation::Operation::property(request.take().expect("one property admission"));
        state.mutation = Some(operation);
        Some(result)
    }
    async fn property_result(&self, request: Property) -> io::Result<()> {
        let mut request = Some(request);
        loop {
            if let Some(result) = self.start_property(&mut request) {
                return result
                    .await
                    .unwrap_or_else(|_| Err(io::Error::other("original property result lost")));
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }
    async fn property_bounded(&self, request: Property) -> io::Result<()> {
        tokio::time::timeout(COMMAND_TIMEOUT, self.property_result(request))
            .await
            .unwrap_or_else(|_| {
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "property reply deadline; original operation remains owned",
                ))
            })
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
    fn set_property(
        &self,
        request: Property,
    ) -> Pin<Box<dyn Future<Output = io::Result<()>> + Send + '_>> {
        Box::pin(self.property_bounded(request))
    }
    fn output<'a>(
        &'a self,
        program: &'a str,
        args: &'a [&'a str],
    ) -> Pin<Box<dyn Future<Output = io::Result<Output>> + Send + 'a>> {
        Box::pin(self.run(program, args))
    }
    fn mutate<'a>(
        &'a self,
        program: &'a str,
        args: &'a [&'a str],
    ) -> Pin<Box<dyn Future<Output = io::Result<Output>> + Send + 'a>> {
        Box::pin(self.mutate_bounded(program, args))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mutation_tests;

#[cfg(test)]
mod test_port;
#[cfg(test)]
pub(crate) use test_port::AcknowledgedCommands;
#[cfg(test)]
mod osk_tests;
