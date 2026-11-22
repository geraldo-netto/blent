//! Validated resource capacities and control-side reap budgets.
use std::{io, time::Duration};
pub fn configured(name: &str, default: usize, maximum: usize) -> io::Result<usize> {
    match std::env::var(name) {
        Ok(value) => parse(&value, maximum),
        Err(std::env::VarError::NotPresent) => parse(&default.to_string(), maximum),
        Err(error) => Err(io::Error::new(io::ErrorKind::InvalidInput, error)),
    }
}
fn parse(value: &str, maximum: usize) -> io::Result<usize> {
    let value: usize = value
        .parse()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    if value == 0 || value > maximum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Resource limit is out of range",
        ));
    }
    Ok(value)
}
/// Validate an observation deadline only; expiry never proves native retirement.
/// This pure contract retains no resource or borrowed input after returning.
pub fn observation_budget(budget: Duration) -> io::Result<Duration> {
    if budget.is_zero() || budget > Duration::from_secs(60) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid retirement observation budget",
        ));
    }
    Ok(budget)
}
pub const MAX_SESSION_STOP_MS: usize = 60_000;
pub const MAX_REAP_MS: usize = 60_000;
pub const DAEMON_REAP_MARGIN: Duration = Duration::from_secs(1);
pub const MAX_DAEMON_DRAIN: Duration =
    Duration::from_millis((MAX_SESSION_STOP_MS + MAX_REAP_MS) as u64 + 1000);

/// Session grace is shared by workers and their outer daemon observer.
pub fn session_stop_budget() -> io::Result<Duration> {
    Ok(Duration::from_millis(
        configured("BLENT_SESSION_STOP_MS", 5000, MAX_SESSION_STOP_MS)? as u64,
    ))
}
/// This observation budget never settles unknown native work.
pub fn daemon_drain_budget() -> io::Result<Duration> {
    Ok(session_stop_budget()? + reap_budget()? + DAEMON_REAP_MARGIN)
}

pub fn reap_budget() -> io::Result<Duration> {
    Ok(Duration::from_millis(
        configured("BLENT_REAP_TIMEOUT_MS", 100, MAX_REAP_MS)? as u64,
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t814_rejects_invalid_and_out_of_bounds_resource_limits() {
        for value in [
            "",
            "0",
            "-1",
            "1.5",
            "NaN",
            " 1",
            "1 ",
            "60001",
            "18446744073709551616",
        ] {
            assert!(parse(value, 60_000).is_err(), "{value}");
        }
        for value in 1..=4096 {
            assert_eq!(parse(&value.to_string(), 4096).unwrap(), value);
        }
        assert_eq!(parse("60000", 60_000).unwrap(), 60_000);
        assert_eq!(configured("BLENT_T814_ABSENT_LIMIT", 17, 20).unwrap(), 17);
        assert!(configured("BLENT_T814_ABSENT_LIMIT", 0, 20).is_err());
        assert!(reap_budget().is_ok());
    }
}

#[cfg(test)]
mod observation_tests {
    use super::*;
    #[test]
    fn t1101_observation_budget_preserves_valid_precision_and_rejects_boundaries() {
        for value in [
            Duration::from_nanos(1),
            Duration::from_millis(1),
            Duration::from_secs(5),
            Duration::from_secs(60),
        ] {
            assert_eq!(observation_budget(value).unwrap(), value);
        }
        for value in [
            Duration::ZERO,
            Duration::from_secs(60) + Duration::from_nanos(1),
            Duration::MAX,
        ] {
            assert_eq!(
                observation_budget(value).unwrap_err().kind(),
                io::ErrorKind::InvalidInput
            );
        }
        let mut seed = 1101u64;
        for _ in 0..4096 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let milliseconds = seed % 120_001;
            assert_eq!(
                observation_budget(Duration::from_millis(milliseconds)).is_ok(),
                (1..=60_000).contains(&milliseconds)
            );
        }
    }
}

#[cfg(test)]
mod shutdown_tests {
    use super::*;
    #[test]
    fn t1513_shutdown_budgets_cover_boundaries_and_reject_invalid_configuration() {
        if let Ok(expected) = std::env::var("BLENT_T1513_EXPECT") {
            let actual = daemon_drain_budget().map(|value| value.as_millis().to_string());
            assert_eq!(actual.unwrap_or_else(|_| "invalid".into()), expected);
            return;
        }
        for (grace, reap, expected) in [
            ("1", "1", "1002"),
            ("5000", "100", "6100"),
            ("60000", "60000", "121000"),
            ("0", "100", "invalid"),
            ("60001", "100", "invalid"),
            ("bad", "100", "invalid"),
            ("5000", "0", "invalid"),
            ("5000", "60001", "invalid"),
        ] {
            let result = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "retirement_limits::shutdown_tests::t1513_shutdown_budgets_cover_boundaries_and_reject_invalid_configuration"])
                .env("BLENT_SESSION_STOP_MS", grace)
                .env("BLENT_REAP_TIMEOUT_MS", reap)
                .env("BLENT_T1513_EXPECT", expected)
                .output().unwrap();
            assert!(
                result.status.success(),
                "{grace}/{reap}: {}",
                String::from_utf8_lossy(&result.stdout)
            );
        }
        assert_eq!(MAX_DAEMON_DRAIN, Duration::from_millis(121000));
    }
}
