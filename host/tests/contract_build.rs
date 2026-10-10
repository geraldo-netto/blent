//! T827: a separate crate compiles the actual contract sources without declaring
//! native capture, selection workers, sessions, queues or storage adapters.
#![allow(dead_code)]
#[path = "../src/capture_contract.rs"]
mod capture_contract;
#[path = "../src/encoder_worker_contract.rs"]
mod encoder_worker_contract;
#[path = "../src/media.rs"]
mod media;
#[path = "../src/media_storage.rs"]
mod media_storage;
#[path = "../src/selection_types.rs"]
mod selection;

#[test]
fn t827_contracts_build_without_native_owners() {
    assert!(std::mem::size_of::<media::EncoderSettings>() > 0);
    assert!(std::mem::size_of::<selection::Selected>() > 0);
}
