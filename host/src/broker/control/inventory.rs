//! Inventory is metadata only. Refreshes never release a live link or reuse a
//! retired target ID. The controller publishes a complete bounded replacement.
use super::*;

impl<K: Ord + Clone, T: Clone> Controller<K, T> {
    pub fn inventory_available(&self) -> bool {
        self.inventory_available
    }

    /// An unavailable scan cannot prove detach or release any original link.
    pub fn inventory_unavailable(&mut self) -> Result<()> {
        if !self.inventory_available && self.devices.is_empty() {
            return Ok(());
        }
        self.changed()?;
        self.devices.clear();
        self.inventory_available = false;
        Ok(())
    }

    pub fn replace_devices(&mut self, devices: Vec<Device<K, T>>) -> Result<()> {
        self.validate_inventory(&devices)?;
        let (replacement, next) = self.replacement(devices)?;
        if !self.inventory_available || self.inventory_changed(&replacement) {
            self.changed()?;
        }
        self.automatic
            .retain(|key, _| replacement.values().any(|device| &device.key == key));
        self.inventory_available = true;
        self.devices = replacement;
        self.next = next;
        Ok(())
    }

    fn validate_inventory(&self, devices: &[Device<K, T>]) -> Result<()> {
        ensure!(
            devices.len() <= self.limits.devices,
            "device capacity reached"
        );
        let keys: BTreeSet<_> = devices.iter().map(|device| &device.key).collect();
        ensure!(keys.len() == devices.len(), "duplicate physical device");
        Ok(())
    }

    fn replacement(
        &self,
        devices: Vec<Device<K, T>>,
    ) -> Result<(BTreeMap<u64, Device<K, T>>, u64)> {
        let targets: BTreeMap<_, _> = self
            .devices
            .iter()
            .map(|(&id, device)| (&device.key, id))
            .collect();
        let mut next = self.next;
        let mut replacement = BTreeMap::new();
        for device in devices {
            let id = match targets.get(&device.key) {
                Some(&id) => id,
                None => {
                    ensure!(next <= MAX_COUNTER, "broker identifiers exhausted");
                    let id = next;
                    next += 1;
                    id
                }
            };
            replacement.insert(id, device);
        }
        Ok((replacement, next))
    }

    fn inventory_changed(&self, replacement: &BTreeMap<u64, Device<K, T>>) -> bool {
        self.devices.len() != replacement.len()
            || replacement.iter().any(|(id, device)| {
                self.devices
                    .get(id)
                    .is_none_or(|old| !old.same_metadata(device))
            })
    }
}

impl<K, T> Device<K, T> {
    fn same_metadata(&self, other: &Self) -> bool {
        (
            self.vendor,
            self.product,
            self.accessory,
            self.availability,
            &self.names,
        ) == (
            other.vendor,
            other.product,
            other.accessory,
            other.availability,
            &other.names,
        )
    }
}
