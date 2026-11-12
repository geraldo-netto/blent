//! Descriptive setup results; no native handles or cleanup authority escape.
#[derive(Debug, PartialEq, Eq)]
pub enum Setup {
    Connected(String),
    Disconnected,
    NoSavedAddress,
    AdbUnavailable,
}

impl Setup {
    pub fn message(&self) -> String {
        match self {
            Self::Connected(address) => format!("Connected to {address}. USB remains preferred; the daemon reconnects while Wi-Fi is configured."),
            Self::Disconnected => "Wi-Fi disconnected. Saved address forgotten; tablet network ADB listener is unchanged.".into(),
            Self::NoSavedAddress => "No saved Wi-Fi address; no disconnect was needed. Tablet network ADB listener is unchanged.".into(),
            Self::AdbUnavailable => "Saved Wi-Fi address forgotten. ADB is unavailable; transport disconnect is unconfirmed. Tablet network ADB listener is unchanged.".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t1374_messages_distinguish_forgetting_from_disconnect() {
        let outcomes = [
            (
                Setup::Connected("192.0.2.74:5555".into()),
                "Connected to 192.0.2.74:5555",
            ),
            (Setup::Disconnected, "Wi-Fi disconnected."),
            (Setup::NoSavedAddress, "no disconnect was needed"),
            (Setup::AdbUnavailable, "transport disconnect is unconfirmed"),
        ];
        for (outcome, expected) in outcomes {
            assert!(outcome.message().contains(expected));
        }
    }
}
