//! Bounded assignment batches, independent of native snapshot storage.
use super::{TabletSession, MAX_SESSIONS};
use anyhow::{ensure, Result};
use serde::de::{Error, IgnoredAny, SeqAccess, Visitor};

pub(crate) fn validate(sessions: &[TabletSession]) -> Result<()> {
    ensure!(
        sessions.len() <= MAX_SESSIONS,
        "too many display assignments"
    );
    let mut occupied = [false; MAX_SESSIONS];
    for session in sessions {
        session.validate()?;
        let slot = &mut occupied[session.instance as usize];
        ensure!(!*slot, "duplicate display instance");
        *slot = true;
    }
    Ok(())
}

pub(crate) fn deserialize<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Vec<TabletSession>, D::Error> {
    deserializer.deserialize_seq(BatchVisitor)
}

struct BatchVisitor;
impl<'de> Visitor<'de> for BatchVisitor {
    type Value = Vec<TabletSession>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "at most {MAX_SESSIONS} unique display assignments"
        )
    }

    fn visit_seq<A: SeqAccess<'de>>(
        self,
        mut sequence: A,
    ) -> std::result::Result<Self::Value, A::Error> {
        let mut sessions = Vec::new();
        for _ in 0..MAX_SESSIONS {
            let Some(session) = sequence.next_element()? else {
                validate(&sessions).map_err(A::Error::custom)?;
                return Ok(sessions);
            };
            sessions.push(session);
        }
        if sequence.next_element::<IgnoredAny>()?.is_some() {
            return Err(A::Error::invalid_length(MAX_SESSIONS + 1, &self));
        }
        validate(&sessions).map_err(A::Error::custom)?;
        Ok(sessions)
    }
}
