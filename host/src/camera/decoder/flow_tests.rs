use super::*;

#[tokio::test]
async fn t1461_decoder_discards_exhausted_frames_and_releases_borrows_at_eof() {
    let mut pool = crate::camera::frame_pool::Pool::new(6, 2).unwrap();
    let held = pool.read(&mut [1u8; 6].as_slice()).await.unwrap().unwrap();
    let other = pool.read(&mut [1u8; 6].as_slice()).await.unwrap().unwrap();
    let (frames, received) = tokio::sync::watch::channel(None);
    assert!(decode_frames(&mut pool, &mut [2u8; 12].as_slice(), &frames)
        .await
        .is_err());
    assert!(received.borrow().is_none());
    assert_eq!(held.as_slice(), [1; 6]);
    drop(held);
    drop(other);
    assert!(decode_frames(&mut pool, &mut [3u8; 12].as_slice(), &frames)
        .await
        .is_err());
    assert_eq!(
        received.borrow().as_ref().unwrap().pixels.as_slice(),
        [3; 6]
    );
}

#[tokio::test]
async fn t1461_incomplete_decoder_pipes_fail_and_child_is_retired() {
    for piped in [false, true] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (_server, _) = listener.accept().await.unwrap();
        let mut command = Command::new("true");
        if piped {
            command.stdin(Stdio::piped());
        }
        let mut child = blent_config::commands::OwnedChild::spawn(&mut command).unwrap();
        let frames = tokio::sync::watch::channel(None).0;
        let error = transfer(&mut client, &mut child, &CameraProfile::default(), &frames)
            .await
            .unwrap_err();
        let retired = retire(child).await;
        assert!(retired.completion.is_complete());
        let pipe = if piped { "stdout" } else { "stdin" };
        assert_eq!(error.to_string(), format!("camera decoder {pipe} missing"));
    }
}
