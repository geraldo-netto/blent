//! T977 private ADB pipeline fixtures. No device, real ADB or PipeWire access.
use super::*;
use std::os::unix::fs::PermissionsExt;
use tokio::io::AsyncReadExt;

#[test]
fn t992_audio_pre_stopped_never_creates_native_transport_or_helper() {
    if std::env::var_os("BLENT_T992_STOPPED_CHILD").is_some() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            for direction in [Direction::Microphone, Direction::Speakers] {
                let (_, stopped) = watch::channel(true);
                let (state, _) = watch::channel(Default::default());
                run(AudioOptions::new(direction), stopped, Report::new(state))
                    .await
                    .expect("T992 Stop must precede native dependency lookup or setup");
            }
        });
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "audio::connection_tests::t992_audio_pre_stopped_never_creates_native_transport_or_helper", "--nocapture"])
        .env("BLENT_T992_STOPPED_CHILD", "1").env("PATH", root.path())
        .env("XDG_RUNTIME_DIR", root.path().join("runtime"))
        .env("XDG_STATE_HOME", root.path().join("state"))
        .output().unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

fn script(root: &Path, name: &str, source: &str) {
    let path = root.join(name);
    std::fs::write(&path, format!("#!/usr/bin/python3\n{source}\n")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}
fn fixture(root: &Path) {
    script(
        root,
        "adb",
        r#"import json,pathlib,shlex,sys
root=pathlib.Path(__file__).parent
args=sys.argv[1:]
if args==['get-serialno']: print('original-tablet')
elif 'tcp:0' in args:
    (root/'port').write_text(args[-1].split(':')[1]); print('34567')
elif '--list' in args: print('original-tablet tcp:34567 tcp:'+(root/'port').read_text())
elif '--remove' in args: (root/'removed').write_text(args[-1])
elif 'shell' in args:
    words=shlex.split(sys.stdin.read())
    (root/'invitation.tmp').write_text(json.dumps(words)); (root/'invitation.tmp').replace(root/'invitation'); print('Broadcast completed: result=1')
else: raise RuntimeError(args)
"#,
    );
    script(
        root,
        "blent-audio",
        r#"import os,pathlib,sys,time
root=pathlib.Path(__file__).parent
sys.stdout.buffer.write(b'READY\n');sys.stdout.buffer.flush()
if sys.argv[3]=='microphone':
    data=sys.stdin.buffer.read(961)
    (root/'delivered.tmp').write_bytes(data); (root/'delivered.tmp').replace(root/'delivered')
    sys.stdin.buffer.read()
else:
    while True:
        sys.stdout.buffer.write(b'\x00'+b'\xd2\x04\x1f\xef'*480);sys.stdout.buffer.flush();time.sleep(.01)
"#,
    );
}

#[test]
fn t977_audio_adb_defaults_keep_full_pipeline_and_original_cleanup() {
    if std::env::var_os("BLENT_T977_ADB_CHILD").is_some() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(private_pipeline());
        return;
    }
    let root = tempfile::tempdir().unwrap();
    fixture(root.path());
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "audio::connection_tests::t977_audio_adb_defaults_keep_full_pipeline_and_original_cleanup", "--nocapture"])
        .env("BLENT_T977_ADB_CHILD", root.path()).env("PATH", root.path())
        .env("XDG_RUNTIME_DIR", root.path().join("runtime"))
        .env("XDG_STATE_HOME", root.path().join("state"))
        .output().unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
async fn private_pipeline() {
    let root = PathBuf::from(std::env::var_os("BLENT_T977_ADB_CHILD").unwrap());
    for direction in [Direction::Microphone, Direction::Speakers] {
        for mode in 0..4 {
            pipeline_once(&root, direction, mode).await;
        }
    }
    let capabilities = transport_capabilities(Direction::Microphone);
    assert!(capabilities.adb && !capabilities.usb);
    assert!(capabilities.available());
    assert!(capabilities.supports(&blent_config::audio::connection::Target::Adb(None)));
}
async fn pipeline_once(root: &Path, direction: Direction, mode: u8) {
    let pending = mode == 0;
    let _ = std::fs::remove_file(root.join("invitation"));
    let _ = std::fs::remove_file(root.join("delivered"));
    let _ = std::fs::remove_file(root.join("removed"));
    let (stop, stopped) = watch::channel(false);
    let (state, status) = watch::channel(blent_config::audio::AudioStatus::default());
    let options = AudioOptions::new(direction);
    let peer = async {
        wait_file(&root.join("invitation")).await;
        let socket = if pending {
            None
        } else {
            Some(transfer(root, direction).await)
        };
        if mode == 1 {
            while status.borrow().state != AudioState::Streaming {
                tokio::task::yield_now().await;
            }
        }
        if mode < 2 {
            stop.send_replace(true);
        } else {
            // This fixture runs only in its explicitly owned subprocess.
            let signal = if mode == 2 {
                libc::SIGTERM
            } else {
                libc::SIGINT
            };
            assert_eq!(unsafe { libc::raise(signal) }, 0);
        }
        // Keep the socket until Stop is observed rather than racing EOF.
        if !pending {
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
        drop(socket);
    };
    let (result, ()) = tokio::time::timeout(Duration::from_secs(4), async {
        let work = async {
            if mode < 2 {
                run(options, stopped, Report::new(state)).await
            } else {
                run_cli(options).await
            }
        };
        tokio::join!(work, peer)
    })
    .await
    .unwrap();
    result.unwrap();
    assert_eq!(
        std::fs::read_to_string(root.join("removed")).unwrap(),
        "tcp:34567"
    );
}
async fn wait_file(path: &Path) {
    while !path.is_file() {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
async fn transfer(root: &Path, direction: Direction) -> TcpStream {
    let words: Vec<String> =
        serde_json::from_slice(&std::fs::read(root.join("invitation")).unwrap()).unwrap();
    let token = &words[words.iter().position(|word| word == "token").unwrap() + 1];
    let port = std::fs::read_to_string(root.join("port")).unwrap();
    let mut socket = TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    socket
        .write_all(
            &[
                b"BLAUREQ1".as_slice(),
                token.as_bytes(),
                &[7, 1, direction as u8, 1],
            ]
            .concat(),
        )
        .await
        .unwrap();
    let mut bytes = [0; 64 + blent_config::audio::HELLO_BYTES];
    socket.read_exact(&mut bytes).await.unwrap();
    assert_eq!(&bytes[..64], token.as_bytes());
    assert_eq!(&bytes[64..72], b"BLAUD001");
    assert_eq!(bytes[144], direction as u8);
    assert_eq!(u16::from_be_bytes(bytes[154..156].try_into().unwrap()), 40);
    match direction {
        Direction::Microphone => microphone_packet(root, &mut socket, &bytes[136..144]).await,
        Direction::Speakers => {
            let mut packet = vec![0; 1948];
            socket.read_exact(&mut packet).await.unwrap();
            assert_eq!(&packet[..8], &bytes[136..144]);
            assert_eq!(&packet[28..], b"\xd2\x04\x1f\xef".repeat(480));
        }
    }
    // Original cleanup is driven by explicit Stop; retain transport until then.
    socket
}
async fn microphone_packet(root: &Path, socket: &mut TcpStream, generation: &[u8]) {
    let mut packet = vec![0; 28 + 960];
    packet[..8].copy_from_slice(generation);
    packet[16..24].copy_from_slice(&1u64.to_be_bytes());
    packet[24..26].copy_from_slice(&960u16.to_be_bytes());
    packet[26] = Direction::Microphone as u8;
    packet[28..].fill(123);
    socket.write_all(&packet).await.unwrap();
    wait_file(&root.join("delivered")).await;
    let native = std::fs::read(root.join("delivered")).unwrap();
    assert_eq!(native[0], 0);
    assert_eq!(&native[1..], &packet[28..]);
}

#[test]
fn t992_audio_explicit_usb_factory_never_looks_up_adb_or_selects_fallback() {
    if std::env::var_os("BLENT_T992_USB_FACTORY_CHILD").is_some() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut options = AudioOptions::new(Direction::Speakers);
                options.usb_peer = Some("ab".repeat(32));
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let error = connection(
                    &options,
                    &listener,
                    &"ab".repeat(32),
                    &Report::new(watch::channel(Default::default()).0),
                )
                .await
                .err()
                .unwrap();
                let expected = if cfg!(feature = "accessory-transport") {
                    "XDG_RUNTIME_DIR must be an absolute path"
                } else {
                    "direct-USB audio requires accessory-transport support"
                };
                assert_eq!(
                    error.to_string(),
                    expected,
                    "T992 USB chose ADB or touched a native source before validating private paths"
                );
            });
        return;
    }
    let root = tempfile::tempdir().unwrap();
    let result=std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact","audio::connection_tests::t992_audio_explicit_usb_factory_never_looks_up_adb_or_selects_fallback","--nocapture"])
        .env("BLENT_T992_USB_FACTORY_CHILD","1").env("PATH",root.path())
        .env("XDG_RUNTIME_DIR","relative-t992-private").env("XDG_STATE_HOME",root.path().join("state"))
        .output().unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
