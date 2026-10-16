//! Shared bounded admission for async and native work owners.
//! Adapters retain the same slot until actual join and dependent cleanup.
use blent_config::retirement::{Admission, Pool};
use std::{io, sync::OnceLock};

pub fn reserve() -> io::Result<Admission> {
    static POOL: OnceLock<io::Result<Pool>> = OnceLock::new();
    POOL.get_or_init(|| {
        Pool::new(blent_config::retirement_limits::configured(
            "BLENT_TASK_LIMIT",
            64,
            4096,
        )?)
    })
    .as_ref()
    .map_err(|error| io::Error::new(error.kind(), error.to_string()))?
    .reserve()
}
