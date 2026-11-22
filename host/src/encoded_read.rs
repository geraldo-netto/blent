//! Exact bounded encoded payload reads. The returned Vec owns initialized bytes;
//! Tokio fills spare capacity without an intermediate zero-filled allocation.
use anyhow::{ensure, Result};
use bytes::BufMut;
use tokio::io::{AsyncRead, AsyncReadExt};

pub async fn payload(input: &mut (impl AsyncRead + Unpin), size: usize) -> Result<Vec<u8>> {
    let limit = crate::video_queue::MAX_FRAME_BYTES;
    ensure!((1..=limit).contains(&size), "Invalid encoded payload size");
    let mut bytes = Vec::with_capacity(size);
    while bytes.len() < size {
        let remaining = size - bytes.len();
        let read = input.read_buf(&mut (&mut bytes).limit(remaining)).await?;
        ensure!(read != 0, "Truncated encoded payload");
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn t1495_exact_payload_preserves_next_frame_and_rejects_invalid_bounds() {
        for size in [1, 2, 31, 65536] {
            let data = vec![0x55; size + 1];
            {
                let mut input = data.as_slice();
                let read = payload(&mut input, size).await.unwrap();
                assert_eq!(read, data[..size]);
                assert_eq!(input, [0x55]);
                assert!(read.capacity() <= crate::video_queue::MAX_FRAME_BYTES);
            }
        }
        for size in [0, crate::video_queue::MAX_FRAME_BYTES + 1, usize::MAX] {
            assert!(payload(&mut b"x".as_slice(), size).await.is_err());
        }
        for length in 0..32 {
            assert!(payload(&mut vec![7; length].as_slice(), 32).await.is_err());
        }
    }
}
