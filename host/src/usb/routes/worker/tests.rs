//! T1656: absent or unconfirmed transports retain ownership without device probes.
use super::*;
use std::collections::VecDeque;
use tokio::time::Instant;

struct Native {
    inventories: Mutex<VecDeque<std::process::Output>>,
    calls: Mutex<Vec<(Instant, Vec<String>)>>,
    mapped: AtomicBool,
    retarget: Option<Arc<Mutex<String>>>,
}

impl Commands for Native {
    fn execute(&self, _: Vec<String>, _: Option<Vec<u8>>) -> CommandFuture<'_> {
        unreachable!("every cleanup query must retain its original command fence")
    }

    fn execute_route(&self, args: Vec<String>) -> RouteFuture<'_> {
        Box::pin(async move {
            self.calls
                .lock()
                .unwrap()
                .push((Instant::now(), args.clone()));
            let output = if args == ["devices"] {
                if let Some(serial) = &self.retarget {
                    *serial.lock().unwrap() = "NEW".into();
                }
                self.inventories.lock().unwrap().pop_front().unwrap()
            } else {
                self.device(&args)
            };
            RouteCommand {
                output: Ok(output),
                started: true,
            }
        })
    }
}

impl Native {
    fn device(&self, args: &[String]) -> std::process::Output {
        assert_eq!(args[0], "-s");
        assert_eq!(args[1], "USB");
        match args[3].as_str() {
            "getprop" => output(0, "original"),
            "--list" => output(
                0,
                if self.mapped.load(Ordering::Acquire) {
                    "UsbFfs tcp:8890 tcp:18000\n"
                } else {
                    ""
                },
            ),
            "--remove" => {
                assert_eq!(args[4], "tcp:8890");
                self.mapped.store(false, Ordering::Release);
                output(0, "")
            }
            _ => panic!("unexpected command: {args:?}"),
        }
    }
}

fn output(code: u32, text: &str) -> std::process::Output {
    #[cfg(unix)]
    let status = {
        use std::os::unix::process::ExitStatusExt;
        std::process::ExitStatus::from_raw((code as i32) << 8)
    };
    #[cfg(windows)]
    let status = {
        use std::os::windows::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(code)
    };
    std::process::Output {
        status,
        stdout: text.as_bytes().to_vec(),
        stderr: Vec::new(),
    }
}

fn inventory(rows: &str) -> std::process::Output {
    output(0, &format!("List of devices attached\n{rows}"))
}

struct Lease(Arc<AtomicBool>);
impl Drop for Lease {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

fn fixture(inventories: Vec<std::process::Output>) -> (State<Native>, Arc<AtomicBool>) {
    let released = Arc::new(AtomicBool::new(false));
    let state = State {
        adb: Adb(Native {
            inventories: Mutex::new(inventories.into()),
            calls: Mutex::new(Vec::new()),
            mapped: AtomicBool::new(true),
            retarget: None,
        }),
        control: Arc::new(Control {
            serial: Arc::new(Mutex::new("USB".into())),
            expected: [(8890, 18000), (8891, 18001)],
            reservation: Mutex::new(Some(Box::new(Lease(released.clone())))),
            settled: AtomicBool::new(false),
            native: None,
        }),
        initial: "USB".into(),
        physical: Some("original".into()),
        created: [Creation::Acknowledged, Creation::NotStarted],
    };
    (state, released)
}

#[tokio::test]
async fn t1656_absence_retains_creator_states_and_issues_no_device_commands() {
    for creation in [
        Creation::Acknowledged,
        Creation::Uncertain,
        Creation::RemovalPending,
    ] {
        let (mut state, released) = fixture(vec![inventory("OTHER\tdevice\n")]);
        state.created[0] = creation;
        assert!(state.cleanup_once().await.is_err());
        assert_eq!(state.created[0], creation);
        assert!(state.adb.0.mapped.load(Ordering::Acquire));
        assert!(!released.load(Ordering::Acquire));
        assert_eq!(state.adb.0.calls.lock().unwrap()[0].1, ["devices"]);
        assert_eq!(state.adb.0.calls.lock().unwrap().len(), 1);
        drop(state);
        assert!(released.load(Ordering::Acquire));
    }
}

#[tokio::test]
async fn t1656_invalid_inventories_cannot_authorize_device_queries() {
    let mut replies = vec![
        output(1, "List of devices attached\nUSB\tdevice\n"),
        output(0, ""),
        inventory("USB\tdevice\nUSB\toffline\n"),
        inventory("USB\tbad state\n"),
        output(0, &"x".repeat(65537)),
    ];
    let mut invalid = inventory("");
    invalid.stdout.push(255);
    replies.push(invalid);
    for reply in replies {
        let (mut state, _) = fixture(vec![reply]);
        assert!(state.cleanup_once().await.is_err());
        assert_eq!(state.created[0], Creation::Acknowledged);
        assert_eq!(state.adb.0.calls.lock().unwrap().len(), 1);
        assert!(state.adb.0.mapped.load(Ordering::Acquire));
    }
}

#[tokio::test(start_paused = true)]
async fn t1656_offline_cleanup_backs_off_and_resumes_with_original_lease() {
    let mut inventories: Vec<_> = (0..8).map(|_| inventory("")).collect();
    inventories.extend([inventory("USB\tdevice\n"), inventory("USB\tdevice\n")]);
    let (mut state, released) = fixture(inventories);
    state.cleanup(&watch::channel(None).0).await;
    assert!(!released.load(Ordering::Acquire));
    assert_eq!(state.created, [Creation::NotStarted; 2]);
    let calls = state.adb.0.calls.lock().unwrap();
    let queries: Vec<_> = calls
        .iter()
        .filter(|(_, args)| args == &["devices"])
        .collect();
    assert_eq!(queries.len(), 10);
    assert_eq!(calls.len(), 15);
    for (pair, delay) in queries
        .windows(2)
        .zip([100, 200, 400, 800, 1600, 3200, 5000, 5000, 5000])
    {
        let expected = Duration::from_millis(delay);
        let elapsed = pair[1].0.duration_since(pair[0].0);
        assert!((expected..=expected + Duration::from_millis(1)).contains(&elapsed));
    }
    assert!(calls[..8].iter().all(|(_, args)| args == &["devices"]));
}

#[tokio::test]
async fn t1656_inventory_and_identity_use_one_serial_snapshot() {
    let (mut state, _) = fixture(vec![inventory("USB\tdevice\n")]);
    state.adb.0.retarget = Some(state.control.serial.clone());
    assert!(!state.cleanup_once().await.unwrap());
    assert_eq!(*state.control.serial.lock().unwrap(), "NEW");
    assert!(!state.adb.0.mapped.load(Ordering::Acquire));
    assert_eq!(state.adb.0.calls.lock().unwrap().len(), 4);
}

#[tokio::test]
async fn t1656_serial_only_owner_rejects_retarget_before_device_queries() {
    let (mut state, _) = fixture(vec![inventory("NEW\tdevice\n")]);
    state.physical = None;
    *state.control.serial.lock().unwrap() = "NEW".into();
    assert!(state.cleanup_once().await.is_err());
    assert_eq!(state.adb.0.calls.lock().unwrap().len(), 1);
    assert!(state.adb.0.mapped.load(Ordering::Acquire));
}
