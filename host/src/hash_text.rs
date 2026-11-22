//! Fixed digest formatting independent of a hashing library's output type.
//! The caller hands over 32 owned bytes; the result owns exactly 64 ASCII bytes.
//! No native resources, retained borrows, callbacks or deferred cleanup.

pub fn encode(bytes: [u8; 32]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(64);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t1732_hash_text_preserves_order_and_leading_zeroes() {
        let bytes = std::array::from_fn(|index| index as u8);
        assert_eq!(
            encode(bytes),
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
        );
    }

    #[test]
    fn t1732_hash_text_matches_independent_formatter_for_every_byte() {
        for byte in 0..=u8::MAX {
            assert_eq!(encode([byte; 32]), format!("{byte:02x}").repeat(32));
        }
    }
}
