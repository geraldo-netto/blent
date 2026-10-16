//! Optional display liveness acknowledgement, bound to one authenticated TCP stream.
pub const VERSION: u32 = 1;
pub const ACKNOWLEDGEMENT: [u8; 8] = *b"!BLENT1\n";

pub fn acknowledged(record: &[u8]) -> bool {
    record == ACKNOWLEDGEMENT
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn t1039_fixed_acknowledgement_has_bounded_strict_version_and_length() {
        assert!(acknowledged(b"!BLENT1\n"));
        for length in 0..=32 {
            assert!(!acknowledged(&vec![0; length]));
        }
        for offset in 0..8 {
            for value in 0..=255 {
                let mut bytes = *b"!BLENT1\n";
                bytes[offset] = value;
                assert_eq!(acknowledged(&bytes), value == b"!BLENT1\n"[offset]);
            }
        }
    }
}
