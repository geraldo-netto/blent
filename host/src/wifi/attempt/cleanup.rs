//! One exclusively owned endpoint and one mutation at a time. Unknown removal
//! is observed, never retried; absence settles it only after native fences.
use super::*;
use blent_config::adb_reverse::temporary::{Cleanup, Creation, Route};

pub(super) async fn retire<C: Commands>(adb: &Adb<C>, address: &str, mut state: Creation) {
    loop {
        match once(adb, address, &mut state).await {
            Ok(true) => return,
            Ok(false) => {}
            Err(error) => tracing::warn!("Wi-Fi cleanup retained: {error:#}"),
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn once<C: Commands>(adb: &Adb<C>, address: &str, state: &mut Creation) -> Result<bool> {
    let present = observe(adb, address).await?.is_some();
    let route = if present {
        Route::Owned
    } else {
        Route::Missing
    };
    match state.cleanup(route) {
        Cleanup::Complete => Ok(true),
        Cleanup::Wait => Ok(false),
        Cleanup::Remove => {
            *state = Creation::RemovalPending;
            let command = adb
                .0
                .execute_route(vec!["disconnect".into(), address.into()])
                .await;
            if !command.started {
                *state = Creation::Acknowledged;
            }
            let output = command.output?;
            ensure!(
                output.status.success(),
                "ADB disconnect outcome unconfirmed"
            );
            Ok(false)
        }
    }
}

/// None means confirmed absent; Some(false) means present but not ready.
pub(super) async fn observe<C: Commands>(adb: &Adb<C>, address: &str) -> Result<Option<bool>> {
    let command = adb.0.execute_route(vec!["devices".into()]).await;
    let output = command.output?;
    ensure!(
        output.status.success() && output.stdout.len() <= 65536,
        "ADB transport inventory unavailable"
    );
    let entries = crate::adb_inventory::entries(std::str::from_utf8(&output.stdout)?)
        .context("Invalid ADB transport inventory")?;
    let key = crate::wifi::endpoint_key(address);
    Ok(entries
        .iter()
        .find(|(serial, _)| crate::wifi::endpoint_key(serial) == key)
        .map(|(_, state)| *state == "device"))
}

#[cfg(test)]
mod tests;
