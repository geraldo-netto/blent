use super::*;
use blent_config::commands::AsyncCommandExt;
use tokio::io::AsyncReadExt;

#[tokio::test]
async fn t1585_only_complete_valid_picture_headers_receive_a_delimiter() {
    for prefix in [&[0, 0, 1][..], &[0, 0, 0, 1][..]] {
        for header in 0..=255u8 {
            let packet = [prefix, &[header, 0x80]].concat();
            let mut output = Vec::new();
            write_access_unit(&mut output, &packet).await.unwrap();
            let picture = header < 128 && (1..=5).contains(&(header % 32));
            assert_eq!(output.starts_with(&packet), true);
            assert_eq!(output.len(), packet.len() + if picture { 6 } else { 0 });
        }
        for end in 0..=prefix.len() {
            let packet = &prefix[..end];
            let mut output = Vec::new();
            write_access_unit(&mut output, packet).await.unwrap();
            assert_eq!(output, packet);
        }
    }
}

#[tokio::test]
async fn t1585_pipe_failure_is_returned_without_retrying_the_packet() {
    let (mut input, output) = tokio::io::duplex(16);
    drop(output);
    assert_eq!(
        write_access_unit(&mut input, &[0, 0, 1, 0x65])
            .await
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::BrokenPipe
    );
}

async fn encoded_frame(ffmpeg: &Path) -> Vec<u8> {
    let encoded = Command::new(ffmpeg)
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=red:s=160x120:r=30",
            "-frames:v",
            "1",
            "-c:v",
            "libx264",
            "-threads",
            "1",
            "-tune",
            "zerolatency",
            "-f",
            "h264",
            "pipe:1",
        ])
        .output_bounded()
        .await
        .unwrap();
    assert!(encoded.status.success());
    encoded.stdout
}

async fn single_packet(
    client: &mut TcpStream,
    output: &mut tokio::process::ChildStdout,
    packet: &[u8],
    frame: &mut [u8],
) -> bool {
    client.write_all(&[0; 9]).await.unwrap();
    client.write_u32(packet.len() as u32).await.unwrap();
    client.write_all(packet).await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), output.read_exact(frame))
        .await
        .is_ok_and(|result| result.is_ok())
}

#[tokio::test]
async fn t1585_single_camera_packet_decodes_without_successor_or_eof() {
    let ffmpeg = crate::camera::executable("ffmpeg").unwrap();
    for threads in 1..=4 {
        single_frame_decodes(&ffmpeg, threads).await;
    }
}

async fn single_frame_decodes(ffmpeg: &Path, filter_threads: u8) {
    let packet = encoded_frame(ffmpeg).await;
    let profile = CameraProfile {
        filter_threads,
        ..crate::camera::tests::options().profile
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut client = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (mut socket, _) = listener.accept().await.unwrap();
    let mut child =
        blent_config::commands::OwnedChild::spawn(&mut command(ffmpeg, &profile).unwrap()).unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = child.stdout.take().unwrap();
    let live = crate::camera_live::Session::new(profile.clone());
    let mut frame = vec![0; profile.frame_bytes().unwrap()];
    let upload = async {
        let emitted = single_packet(&mut client, &mut output, &packet, &mut frame).await;
        client.shutdown().await.unwrap();
        emitted
    };
    let (received, emitted) = tokio::join!(
        receive_packets(&mut socket, &mut input, Duration::from_secs(3), &live),
        upload
    );
    drop(input);
    drop(output);
    assert!(retire(child).await.completion.is_complete());
    assert!(received.is_err());
    assert!(
        emitted,
        "T1585 single frame remained buffered until another access unit"
    );
    assert!(frame.iter().any(|byte| *byte != 0));
}
