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
pub fn reap_budget() -> io::Result<Duration> {
    Ok(Duration::from_millis(
        configured("BLENT_REAP_TIMEOUT_MS", 100, 60_000)? as u64,
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
