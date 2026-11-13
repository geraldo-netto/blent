use super::State;
use blent::usb::{CommandFuture, Commands, RouteCommand, RouteFuture};
use std::{
    collections::BTreeMap,
    os::unix::process::ExitStatusExt,
    sync::{atomic::Ordering, Arc, Mutex},
};
#[derive(Clone)]
pub(super) struct Fake {
    routes: Arc<Mutex<BTreeMap<String, String>>>,
    state: Arc<State>,
}
impl Fake {
    pub(super) fn new(state: Arc<State>) -> Self {
        Self {
            routes: Default::default(),
            state,
        }
    }
    pub(super) fn count(&self) -> usize {
        self.routes.lock().unwrap().len()
    }
    fn reverse(&self, args: &[String]) -> std::process::Output {
        let mut routes = self.routes.lock().unwrap();
        match args[3].as_str() {
            "--list" => output(
                0,
                routes
                    .iter()
                    .map(|(a, b)| format!("UsbFfs {a} {b}\n"))
                    .collect(),
            ),
            "--no-rebind" => {
                routes.insert(args[4].clone(), args[5].clone());
                output(0, String::new())
            }
            "--remove" if self.state.release.load(Ordering::Acquire) => {
                routes.remove(&args[4]);
                output(0, String::new())
            }
            "--remove" => output(1, String::new()),
            _ => panic!("Unexpected fake reverse command"),
        }
    }
}
struct Lease(Arc<State>);
impl Drop for Lease {
    fn drop(&mut self) {
        self.0.route_lease_released.store(true, Ordering::Release);
    }
}
impl Commands for Fake {
    fn execute_route(&self, args: Vec<String>) -> RouteFuture<'_> {
        Box::pin(async move {
            if args.get(3).is_some_and(|arg| arg == "--remove")
                && !self.state.release.load(Ordering::Acquire)
            {
                return RouteCommand {
                    started: false,
                    output: Ok(output(1, String::new())),
                };
            }
            RouteCommand {
                started: true,
                output: self.execute(args, None).await,
            }
        })
    }

    fn display_route_lease(&self, _: &str, _: (u16, u16)) -> anyhow::Result<Box<dyn Send>> {
        Ok(Box::new(Lease(self.state.clone())))
    }
    fn execute(&self, args: Vec<String>, _: Option<Vec<u8>>) -> CommandFuture<'_> {
        Box::pin(async move {
            Ok(match args[2].as_str() {
                "shell" => output(0, "T1411physical".into()),
                "reverse" => self.reverse(&args),
                _ => panic!("Unexpected fake command"),
            })
        })
    }
}
fn output(code: i32, text: String) -> std::process::Output {
    std::process::Output {
        status: std::process::ExitStatus::from_raw(code << 8),
        stdout: text.into_bytes(),
        stderr: vec![],
    }
}
