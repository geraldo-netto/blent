//! One bounded warning window per input listener, including controller replacements.
use std::fmt::{self, Write};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct Invalid(Mutex<Window>);

#[derive(Default)]
struct Window {
    last: Option<Instant>,
    suppressed: u64,
}

impl Window {
    fn admit(&mut self, now: Instant) -> Option<u64> {
        if self
            .last
            .is_some_and(|last| now.saturating_duration_since(last) < Duration::from_secs(1))
        {
            self.suppressed = self.suppressed.saturating_add(1);
            return None;
        }
        self.last = Some(now);
        Some(std::mem::take(&mut self.suppressed))
    }
}

impl Invalid {
    pub(super) fn report(&self, error: &serde_json::Error, text: &str) {
        let admitted = self
            .0
            .lock()
            .unwrap()
            .admit(tokio::time::Instant::now().into_std());
        let Some(suppressed) = admitted else { return };
        tracing::warn!(error = %preview(error), frame = %preview(text), suppressed, "Invalid input");
    }
}

struct Preview {
    text: String,
    full: bool,
}

impl Write for Preview {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if self.full {
            return Err(fmt::Error);
        }
        for ch in text.chars().flat_map(char::escape_default) {
            if self.text.len() + ch.len_utf8() > 256 {
                self.full = true;
                self.text.push('…');
                return Err(fmt::Error);
            }
            self.text.push(ch);
        }
        Ok(())
    }
}

fn preview(value: &(impl fmt::Display + ?Sized)) -> String {
    let mut output = Preview {
        text: String::with_capacity(259),
        full: false,
    };
    let _ = write!(output, "{value}");
    output.text
}

#[cfg(test)]
mod tests;
