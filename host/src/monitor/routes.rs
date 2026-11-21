//! Retain route ownership when an asynchronous mutation is cancelled or fails.
use super::*;
use blent::usb::{Adb, NativeCommands, Routes};

#[derive(Clone, Default)]
pub(crate) struct RouteOwner(Arc<tokio::sync::Mutex<Option<Routes>>>);

impl RouteOwner {
    pub(crate) async fn prepare(&self, serial: &str, ports: (u16, u16), adb: &str) -> Result<()> {
        let mut owned = self.0.lock().await;
        if owned.is_none() {
            *owned = Some(Routes::for_transport(serial, ports)?);
        }
        owned
            .as_mut()
            .unwrap()
            .prepare(&Adb(NativeCommands(adb.into())))
            .await
    }

    /// The display ticket remains owned until original native cleanup settles.
    /// Cancellation drops this view; Routes' bounded worker retains its debt
    /// and device/local-port leases independently of the monitor task/runtime.
    pub(super) async fn settle(&self, adb: &str, serial: &str) {
        let mut reported = false;
        loop {
            match self.retire(adb).await {
                Ok(()) => return,
                Err(error) if !reported => {
                    warn!("Display routes on {serial} remain owned: {error}");
                    reported = true;
                }
                Err(_) => {}
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub(crate) async fn retire(&self, adb: &str) -> Result<()> {
        let mut owned = self.0.lock().await;
        if let Some(routes) = owned.as_mut() {
            routes.retire(&Adb(NativeCommands(adb.into()))).await?;
        }
        Ok(())
    }
}
