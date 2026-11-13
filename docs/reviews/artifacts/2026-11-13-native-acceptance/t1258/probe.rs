//! T1258 scoped physical acceptance; includes unchanged production owners.
//! Each run owns one mapping and observes its native settlement before exit.
#[allow(dead_code)]
#[path = "../../../../../host/src/adb_command.rs"]
mod adb_command;
#[allow(dead_code)]
#[path = "../../../../../host/src/adb_mapping/mod.rs"]
mod adb_mapping;

use anyhow::{ensure, Result};
use blent_config::retirement::Completion;
use std::{path::PathBuf, sync::{Arc, Mutex}, time::Duration};
use tokio::net::TcpListener;
type Proof = Arc<Mutex<Option<Completion>>>;

struct Trial {
    directory: PathBuf,
    proof: Proof,
    wrapper: PathBuf,
    serial: String,
}
impl Trial {
    fn record(&self, event: &str) {
        println!("{}", serde_json::json!({"event":event,"retired":self.retired()}));
    }
    fn retired(&self) -> bool {
        self.proof.lock().unwrap().as_ref().is_some_and(Completion::is_complete)
    }
    async fn create(&self, listener: &TcpListener) -> Result<adb_mapping::Lease> {
        let proof = self.proof.clone();
        adb_mapping::Lease::create(self.wrapper.clone(), Some(&self.serial), listener,
            &move |value| { *proof.lock().unwrap() = Some(value); Ok(()) }).await
    }
    async fn wait_file(&self, name: &str) {
        while !self.directory.join(name).exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    fn release(&self) -> Result<()> {
        std::fs::write(self.directory.join("release"), "release original command\n")?;
        Ok(())
    }
    async fn settle(&self) {
        if self.proof.lock().unwrap().is_none() { return; }
        let mut observations = 0;
        while !self.retired() {
            tokio::time::sleep(Duration::from_millis(100)).await;
            observations += 1;
            if observations % 50 == 0 { self.record("waiting_for_original_settlement"); }
        }
        self.record("original_worker_and_endpoints_retired");
    }
}

async fn ordinary(trial: &Trial, drop_owner: bool) -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let lease = trial.create(&listener).await?;
    println!("{}", serde_json::json!({"event":"created","remote":lease.target().remote,"local":lease.target().local}));
    ensure!(!trial.retired(), "live owner was retired");
    if !drop_owner { lease.close().await?; }
    drop(lease);
    trial.settle().await;
    Ok(())
}

async fn cancel_after_submission(trial: Arc<Trial>) -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let owner = trial.clone();
    let task = tokio::spawn(async move { owner.create(&listener).await });
    trial.wait_file("created").await;
    task.abort();
    let joined = task.await;
    ensure!(joined.is_err_and(|error| error.is_cancelled()), "caller did not cancel");
    ensure!(!trial.retired(), "late creator lost its owner");
    ensure!(std::net::TcpListener::bind(address).is_err(), "destination reused before settlement");
    trial.record("cancelled_caller_original_command_and_destination_retained");
    trial.release()?;
    trial.settle().await;
    drop(std::net::TcpListener::bind(address)?);
    Ok(())
}

async fn failed_ack(trial: &Trial) -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    ensure!(trial.create(&listener).await.is_err(), "lost acknowledgement unexpectedly succeeded");
    trial.record("lost_acknowledgement_reported");
    trial.settle().await;
    Ok(())
}

async fn delayed_removal(trial: Arc<Trial>, reconnect: bool) -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let lease = Arc::new(trial.create(&listener).await?);
    let closing = lease.clone();
    let task = tokio::spawn(async move { closing.close().await });
    trial.wait_file("remove_waiting").await;
    drop(listener);
    ensure!(!trial.retired(), "unsettled removal retired its owner");
    ensure!(std::net::TcpListener::bind(address).is_err(), "destination released during removal");
    trial.record("removal_pending_destination_retained");
    if reconnect { reconnect_transport(&trial).await?; }
    task.abort();
    let _ = task.await;
    drop(lease);
    trial.record("close_caller_lost_original_owner_retained");
    trial.release()?;
    trial.settle().await;
    drop(std::net::TcpListener::bind(address)?);
    Ok(())
}

async fn reconnect_transport(trial: &Trial) -> Result<()> {
    let result = tokio::process::Command::new("/usr/bin/adb")
        .args(["-s", &trial.serial, "reconnect"]).output().await?;
    ensure!(result.status.success(), "transport reconnect request failed");
    trial.record("native_transport_reconnect_requested");
    let result = tokio::process::Command::new("/usr/bin/adb")
        .args(["-s", &trial.serial, "wait-for-device"]).status().await?;
    ensure!(result.success(), "device did not return");
    ensure!(!trial.retired(), "original owner lost across reconnect");
    trial.record("native_transport_returned_original_owner_retained");
    Ok(())
}

async fn run(trial: Arc<Trial>, mode: &str) -> Result<()> {
    match mode {
        "normal" => ordinary(&trial, false).await,
        "drop" => ordinary(&trial, true).await,
        "cancel" => cancel_after_submission(trial).await,
        "lost_ack" => failed_ack(&trial).await,
        "remove_delay" => delayed_removal(trial, false).await,
        "reconnect" => delayed_removal(trial, true).await,
        _ => anyhow::bail!("unknown case"),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let trial = Arc::new(Trial {
        directory: std::env::var("T1258_RUN")?.into(),
        proof: Arc::new(Mutex::new(None)),
        wrapper: std::env::var("T1258_WRAPPER")?.into(),
        serial: std::env::var("T1258_SERIAL")?,
    });
    let result = run(trial.clone(), &std::env::var("T1258_MODE")?).await;
    trial.release()?;
    trial.settle().await;
    result
}
