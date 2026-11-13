//! Original OSK operation ownership survives cancellation of its async caller.
use crate::native_commands::ObservedCommands;
use blent_config::retirement::{Admission, Completion, Observe, Pool};
use std::{
    io,
    sync::{Arc, OnceLock},
};
use tokio::sync::{Mutex, OwnedMutexGuard};

#[derive(Default)]
pub(super) struct Gate {
    serial: Arc<Mutex<()>>,
    pool: OnceLock<io::Result<Pool>>,
    pub(super) commands: Arc<ObservedCommands>,
}
impl Gate {
    fn pool(&self) -> io::Result<&Pool> {
        // One serialized operation plus its predecessor's completion publication.
        // Independent of task admission: cleanup must not compete with its sponsor.
        self.pool
            .get_or_init(|| Pool::new(2))
            .as_ref()
            .map_err(|error| io::Error::new(error.kind(), error.to_string()))
    }
    pub(super) async fn acquire(&self) -> io::Result<Operation> {
        let guard = self.serial.clone().lock_owned().await;
        let admission = self.pool()?.reserve()?;
        Ok(Operation {
            admission: Some(admission),
            retained: Some(Retained {
                commands: self.commands.clone(),
                _guard: guard,
            }),
        })
    }
}
pub(super) struct Operation {
    admission: Option<Admission>,
    retained: Option<Retained>,
}
impl Operation {
    pub(super) fn completion(&self) -> Completion {
        self.admission.as_ref().unwrap().completion()
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        self.admission
            .take()
            .unwrap()
            .retain(self.retained.take().unwrap());
    }
}
struct Retained {
    commands: Arc<ObservedCommands>,
    _guard: OwnedMutexGuard<()>,
}
impl Observe for Retained {
    fn poll_complete(&mut self) -> bool {
        self.commands.complete()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn t1433_operation_capacity_rejects_before_effects_and_recovers() {
        let gate = Gate::default();
        let first = gate.pool().unwrap().reserve().unwrap();
        let second = gate.pool().unwrap().reserve().unwrap();
        assert!(gate.acquire().await.is_err());
        assert!(gate.commands.complete());
        drop((first, second));
        let operation = gate.acquire().await.unwrap();
        let proof = operation.completion();
        drop(operation);
        while !proof.is_complete() {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
        assert_eq!(gate.pool().unwrap().active(), 0);
    }
}
