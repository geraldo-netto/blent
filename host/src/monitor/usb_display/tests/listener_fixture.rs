//! Test-owned sockets pin every derived endpoint during primary allocation.
use std::net::TcpListener;

pub(super) struct Reservation {
    pub(super) ports: (u16, u16),
    listeners: Vec<TcpListener>,
}

impl Reservation {
    pub(super) fn allocate(capacity: u32) -> Self {
        for _ in 0..64 {
            let video = TcpListener::bind("127.0.0.1:0").unwrap();
            let input = TcpListener::bind("127.0.0.1:0").unwrap();
            if let Ok(reservation) = Self::from_primary(video, input, capacity) {
                return reservation;
            }
        }
        panic!("T1267: no complete listener fixture available");
    }

    fn from_primary(video: TcpListener, input: TcpListener, capacity: u32) -> anyhow::Result<Self> {
        let ports = (video.local_addr()?.port(), input.local_addr()?.port());
        let pairs = blent_config::slot_ports(ports.0, ports.1, capacity)?;
        let mut listeners = vec![video, input];
        for (video, input) in pairs.into_iter().skip(1) {
            listeners.push(TcpListener::bind(("127.0.0.1", video))?);
            listeners.push(TcpListener::bind(("127.0.0.1", input))?);
        }
        Ok(Self { ports, listeners })
    }

    pub(super) fn release_primary(mut self) -> Vec<TcpListener> {
        self.listeners.drain(..2);
        self.listeners
    }
}

#[test]
fn t1267_fixture_rejects_an_occupied_derived_endpoint() {
    for endpoint in 0..2 {
        let reservation = Reservation::allocate(2);
        let ports = reservation.ports;
        let mut listeners = reservation.listeners.into_iter();
        let video = listeners.next().unwrap();
        let input = listeners.next().unwrap();
        drop(listeners);
        let port = [ports.0, ports.1][endpoint] + 2;
        let held = TcpListener::bind(("127.0.0.1", port)).unwrap();
        let rejected = Reservation::from_primary(video, input, 2);
        assert!(
            rejected.is_err(),
            "T1267 fixture admitted an occupied extra port"
        );
        assert!(TcpListener::bind(("127.0.0.1", ports.0)).is_ok());
        assert!(TcpListener::bind(("127.0.0.1", ports.1)).is_ok());
        assert!(TcpListener::bind(held.local_addr().unwrap()).is_err());
        let other = [ports.1, ports.0][endpoint] + 2;
        assert!(TcpListener::bind(("127.0.0.1", other)).is_ok());
    }
}

#[test]
fn t1267_fixture_pins_all_slots_and_releases_every_guard() {
    for capacity in 1..=4 {
        let reservation = Reservation::allocate(capacity);
        let pairs =
            blent_config::slot_ports(reservation.ports.0, reservation.ports.1, capacity).unwrap();
        for &(video, input) in &pairs {
            assert!(TcpListener::bind(("127.0.0.1", video)).is_err());
            assert!(TcpListener::bind(("127.0.0.1", input)).is_err());
        }
        let extra = reservation.release_primary();
        assert!(TcpListener::bind(("127.0.0.1", pairs[0].0)).is_ok());
        assert!(TcpListener::bind(("127.0.0.1", pairs[0].1)).is_ok());
        drop(extra);
        for (video, input) in pairs {
            assert!(TcpListener::bind(("127.0.0.1", video)).is_ok());
            assert!(TcpListener::bind(("127.0.0.1", input)).is_ok());
        }
    }
}
