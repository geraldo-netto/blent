use super::*;

#[test]
fn t1501_warning_window_counts_suppression_and_exact_boundary() {
    let start = Instant::now();
    let mut window = Window::default();
    assert_eq!(window.admit(start), Some(0));
    for delta in [0, 1, 999] {
        assert_eq!(window.admit(start + Duration::from_millis(delta)), None);
    }
    assert_eq!(window.admit(start + Duration::from_secs(1)), Some(3));
    assert_eq!(window.admit(start + Duration::from_secs(2)), Some(0));
}

#[test]
fn t1501_regressed_clock_and_counter_exhaustion_remain_bounded() {
    let start = Instant::now();
    let mut window = Window {
        last: Some(start + Duration::from_secs(1)),
        suppressed: u64::MAX,
    };
    assert_eq!(window.admit(start), None);
    assert_eq!(window.suppressed, u64::MAX);
    assert_eq!(window.admit(start + Duration::from_secs(2)), Some(u64::MAX));
    assert_eq!(window.suppressed, 0);
}

#[test]
fn t1501_preview_bounds_ascii_and_escapes_untrusted_control_text() {
    for length in [0, 1, 255, 256, 257, 65_536] {
        let output = preview(&"x".repeat(length));
        assert!(output.len() <= 259);
        assert_eq!(output.trim_end_matches('…').len(), length.min(256));
        assert_eq!(output.ends_with('…'), length > 256);
    }
    for text in ["\n\r\t\0\u{1b}", "😀é漢", "\"\\"] {
        let output = preview(text);
        assert_eq!(
            output,
            text.chars()
                .flat_map(char::escape_default)
                .collect::<String>()
        );
        assert!(output.bytes().all(|byte| (32..127).contains(&byte)));
    }
}

struct IgnoresFailure;
impl fmt::Display for IgnoresFailure {
    fn fmt(&self, writer: &mut fmt::Formatter<'_>) -> fmt::Result {
        for _ in 0..1000 {
            let _ = writer.write_str("0123456789");
        }
        Ok(())
    }
}

#[test]
fn t1501_preview_cannot_grow_when_formatter_ignores_truncation() {
    let output = preview(&IgnoresFailure);
    assert_eq!(output.len(), 259);
    assert_eq!(output.matches('…').count(), 1);
}

#[test]
fn t1501_unicode_expansion_obeys_the_same_output_bound() {
    for ch in ['\u{0}', '\n', '\u{7f}', 'é', '漢', '😀', '\u{10ffff}'] {
        for length in [1, 31, 64, 256, 1024] {
            let text: String = std::iter::repeat_n(ch, length).collect();
            let output = preview(&text);
            assert!(output.len() <= 259);
            assert!(!output.chars().any(char::is_control));
        }
    }
}
