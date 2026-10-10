//! Async persistence contracts for event-loop policy. Native storage adapters
//! retain ownership independently of an awaiting caller's lifetime.
use blent_config::{
    model::FileConfig,
    persistence_contract::{Edit, PersistenceResult},
};
use std::{future::Future, pin::Pin};

pub type Reply<'a> = Pin<Box<dyn Future<Output = PersistenceResult<FileConfig>> + Send + 'a>>;
pub trait AsyncSnapshots: Send + Sync {
    fn snapshot(&self) -> Reply<'_>;
}
pub trait AsyncTransactions: Send + Sync {
    fn transact(&self, edit: Edit<'static>) -> Reply<'_>;
}
pub trait AsyncPersistence: AsyncSnapshots + AsyncTransactions {}
impl<T: AsyncSnapshots + AsyncTransactions> AsyncPersistence for T {}
