use super::*;
use blent_config::storage::ConfigStore;
use futures_util::FutureExt;
use std::collections::HashMap;
use tokio::time::Instant;
mod queries;

pub(super) struct Network {
    store: ConfigStore,
    queries: queries::Queries,
    identities: HashMap<String, String>,
    launches: crate::launch_policy::Launches,
    next_retry: Instant,
    reconnect: crate::device_tasks::DeviceTasks<()>,
}
impl Network {
    pub fn new(store: ConfigStore) -> Self {
        Self {
            store,
            queries: queries::Queries::new(),
            identities: HashMap::new(),
            launches: Default::default(),
            next_retry: Instant::now(),
            reconnect: crate::device_tasks::DeviceTasks::new(1),
        }
    }
    pub fn identity(&self, serial: &str) -> Option<&String> {
        self.identities.get(serial)
    }
    pub fn attachment_identity(&self, serial: &str) -> String {
        crate::transport::attachment_identity(serial, &self.identities)
    }
    pub fn ticket(&self, serial: &str) -> Option<crate::launch_policy::Ticket> {
        self.launches
            .observe(serial, self.identities.get(serial).map(String::as_str))
    }
    pub fn retarget(&self, routes: &mut Routes, identity: Option<&String>) -> bool {
        let Some(identity) = identity else {
            return true;
        };
        if let Some((serial, _)) = self
            .identities
            .iter()
            .filter(|(_, id)| *id == identity)
            .min_by_key(|(serial, _)| (transport_of(serial) != Transport::Usb, *serial))
        {
            return routes.retarget(serial).is_ok();
        }
        false
    }
    pub async fn can_retire<C: Commands>(
        &self,
        adb: &Adb<C>,
        routes: &mut Routes,
        identity: Option<&String>,
    ) -> bool {
        if self.retarget(routes, identity) {
            return true;
        }
        if self.identities.contains_key(routes.serial()) {
            return false;
        }
        // Inventory absence alone is not proof that a still-reachable route is
        // foreign. Recheck physical identity before touching its owned mapping.
        adb.identity(routes.serial()).await.as_ref() == identity
    }
    pub async fn devices<C: Commands + Clone + 'static>(
        &mut self,
        adb: &Adb<C>,
        current: &[String],
    ) -> Option<Vec<String>> {
        let mut devices = adb.all_devices().await?;
        if devices.len() > 256 {
            return None;
        }
        self.schedule_reconnect(adb, current, &devices);
        devices.sort_by_key(|serial| {
            (
                transport_of(serial) != Transport::Usb,
                !current.contains(serial),
            )
        });
        self.observe(adb, &devices).await;
        Some(self.select(devices))
    }
    fn schedule_reconnect<C: Commands + Clone + 'static>(
        &mut self,
        adb: &Adb<C>,
        current: &[String],
        devices: &[String],
    ) {
        // One original job survives poll cancellation. Attempt owns native
        // creation/compensation and its address until exact retirement, even
        // when this scheduler is stopped or dropped.
        let _ = self.reconnect.next().now_or_never();
        if self.reconnect.contains("saved")
            || Instant::now() < self.next_retry
            || current.iter().any(|s| devices.contains(s))
        {
            return;
        }
        let address = self.store.load().wifi_address;
        if !crate::wifi::valid_address(&address) || devices.contains(&address) {
            return;
        }
        self.next_retry = Instant::now() + Duration::from_secs(5);
        let adb = adb.clone();
        let store = self.store.clone();
        self.reconnect.schedule("saved".into(), async move {
            crate::wifi::reconnect(&adb, &store).await;
        });
    }
    fn select(&self, devices: Vec<String>) -> Vec<String> {
        let devices: Vec<_> = devices
            .into_iter()
            .filter(|serial| self.queries.ready(serial))
            .collect();
        crate::transport::select_device_transports(&devices, None, &self.identities)
    }
    async fn observe<C: Commands + Clone + 'static>(&mut self, adb: &Adb<C>, devices: &[String]) {
        self.identities.retain(|serial, _| devices.contains(serial));
        self.queries.refresh(adb, devices);
        self.launches.refresh(devices);
        // Let admitted workers progress; a pending query never forms a barrier.
        tokio::task::yield_now().await;
        while let Some((serial, identity)) = self.queries.next() {
            if let Some(identity) = identity {
                self.identities.insert(serial.clone(), identity);
            } else {
                self.identities.remove(&serial);
            }
            self.launches
                .observe(&serial, self.identities.get(&serial).map(String::as_str));
        }
        if self.queries.complete() {
            self.launches.retain(devices);
        }
    }
    pub async fn stop(&mut self) {
        self.reconnect.stop().await;
        self.queries.stop().await;
        self.identities.clear();
    }
}
