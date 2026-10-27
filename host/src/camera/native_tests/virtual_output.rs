//! T1137: production camera serve path against two isolated real loopback devices.
//! Ordinary host runs only validate the guard, not native driver acceptance.
use super::*;
use std::{
    fs::File,
    io::{self, Read},
    os::unix::fs::OpenOptionsExt,
    path::Path,
    time::Duration,
};
use tokio::io::AsyncReadExt;

#[tokio::test]
async fn t1137_virtual_pair_uses_original_outputs_and_retires_before_reuse() {
    let Some(front) = std::env::var_os("BLENT_T1143_DEVICE") else {
        return;
    };
    let rear = std::env::var_os("BLENT_T1137_REAR").unwrap();
    let options = CameraOptions {
        profile: CameraProfile {
            width: 320,
            height: 240,
            ..Default::default()
        },
        front_device: front.into(),
        rear_device: rear.into(),
    };
    let (stop, mut stopped) = watch::channel(false);
    let report = tests::new_report();
    let devices = output_admission::admit(&options, &mut stopped, &report)
        .await
        .unwrap()
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let session = Session {
        listener: &listener,
        invitation: &retirement::Pending,
        token: "unused-no-tablet-invitation",
        ffmpeg: Path::new("/unused-decoder"),
        options: &options,
    };
    let observe = async {
        let result = observe_pair(&options, devices.files()).await;
        stop.send_replace(true);
        result
    };
    let (served, observed) = tokio::join!(
        serve(session, &mut stopped, &report, devices.files()),
        observe
    );
    drop(devices);
    report.wait_native().await;
    // Actual original native retirement precedes these fresh lock admissions.
    drop(outputs::open_device(&options.front_device, "Blent Front").unwrap());
    drop(outputs::open_device(&options.rear_device, "Blent Rear").unwrap());
    served.unwrap();
    observed.unwrap();
    println!("T1137 real production pair: both black frames/metadata matched; actual native join and original lease reuse passed");
}

async fn observe_pair(options: &CameraOptions, devices: &[Arc<File>; 2]) -> Result<()> {
    for (path, file) in [
        (&options.front_device, &devices[0]),
        (&options.rear_device, &devices[1]),
    ] {
        tokio::time::timeout(Duration::from_secs(5), capture_black(path)).await??;
        let actual = snapshot(file).await?;
        anyhow::ensure!(
            actual[..6] == [320, 240, u32::from_le_bytes(*b"YU12"), 1, 320, 115200],
            "T1137 native image metadata"
        );
        anyhow::ensure!(
            actual[6] != 0 && actual[6].checked_mul(30) == Some(actual[7]),
            "T1137 native frame rate"
        );
    }
    Ok(())
}
async fn capture_black(path: &Path) -> Result<()> {
    let mut bytes = vec![0; 115200];
    loop {
        let mut file = File::options()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)?;
        match file.read(&mut bytes) {
            Ok(115200) => {
                anyhow::ensure!(black(&bytes), "T1137 native black-frame bytes");
                return Ok(());
            }
            Ok(_) => anyhow::bail!("T1137 short native capture"),
            Err(error) => startup_read(error)?,
        }
        drop(file);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
fn startup_read(error: io::Error) -> Result<()> {
    // File-I/O capture is not initialized until the producer publishes a frame.
    if matches!(
        error.raw_os_error(),
        Some(libc::EAGAIN | libc::EINVAL | libc::EIO)
    ) {
        Ok(())
    } else {
        Err(error.into())
    }
}
async fn snapshot(file: &File) -> Result<[u32; 8]> {
    use blent_config::commands::OwnedChild;
    let oracle = std::env::var_os("BLENT_T1143_ORACLE").unwrap();
    let mut command = tokio::process::Command::new(oracle);
    command
        .arg("--snapshot")
        .stdin(std::process::Stdio::from(file.try_clone()?))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    let mut child = OwnedChild::spawn(&mut command)?;
    drop(command); // No parent-side Stdio copy can retain the original lease.
    let result = tokio::time::timeout(Duration::from_secs(5), read_snapshot(&mut child)).await;
    let retired = child.retire(Duration::from_secs(1)).await;
    while !retired.completion.is_complete() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    let bytes = result??;
    parse_snapshot(&bytes)
}
async fn read_snapshot(child: &mut blent_config::commands::OwnedChild) -> Result<Vec<u8>> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("T1137 metadata stdout missing"))?;
    let mut bytes = Vec::with_capacity(129);
    stdout.take(129).read_to_end(&mut bytes).await?;
    anyhow::ensure!(bytes.len() < 129, "T1137 metadata output bound");
    anyhow::ensure!(
        child.wait().await?.success(),
        "T1137 native metadata query failed"
    );
    Ok(bytes)
}

fn black(bytes: &[u8]) -> bool {
    // Borrowed native capture view: exact length guards both plane slices.
    bytes.len() == 115200
        && bytes[..76800].iter().all(|&byte| byte == 16)
        && bytes[76800..].iter().all(|&byte| byte == 128)
}

#[test]
fn t1137_capture_oracle_rejects_invalid_lengths_and_plane_boundary_corruption() {
    let mut bytes = vec![128; 115201];
    bytes[..76800].fill(16);
    for count in 0..=115201 {
        assert_eq!(black(&bytes[..count]), count == 115200);
    }
    for index in [0, 76799, 76800, 115199] {
        let expected = bytes[index];
        bytes[index] = 255;
        assert!(!black(&bytes[..115200]));
        bytes[index] = expected;
    }
}

fn parse_snapshot(bytes: &[u8]) -> Result<[u32; 8]> {
    // Synchronous borrowed development protocol: at most128 bytes and exactly
    // eight unsigned32 fields. No retained view or native resource is created.
    anyhow::ensure!(bytes.len() < 129, "T1137 metadata input bound");
    let mut words = std::str::from_utf8(bytes)?.split_whitespace();
    let mut values = [0; 8];
    for value in &mut values {
        *value = words
            .next()
            .ok_or_else(|| io::Error::other("T1137 metadata field missing"))?
            .parse()?;
    }
    anyhow::ensure!(words.next().is_none(), "T1137 extra metadata field");
    Ok(values)
}

#[test]
fn t1137_snapshot_parser_rejects_invalid_counts_bytes_ranges_and_bounds() {
    for count in 0..=16 {
        let input = "1 ".repeat(count);
        assert_eq!(parse_snapshot(input.as_bytes()).is_ok(), count == 8);
    }
    for field in ["-1", "4294967296", "NaN", "1.5", "0x10"] {
        let input = format!("{field} 1 1 1 1 1 1 1");
        assert!(parse_snapshot(input.as_bytes()).is_err());
    }
    assert_eq!(parse_snapshot(b"0 0 0 0 0 0 0 0").unwrap(), [0; 8]);
    assert_eq!(
        parse_snapshot("4294967295 ".repeat(8).as_bytes()).unwrap(),
        [u32::MAX; 8]
    );
    let mut limit = b"1 1 1 1 1 1 1 1".to_vec();
    limit.resize(128, b' ');
    assert!(parse_snapshot(&limit).is_ok());
    limit.push(b' ');
    assert!(parse_snapshot(&limit).is_err());
    limit[0] = 255;
    limit.truncate(128);
    assert!(parse_snapshot(&limit).is_err());
}
