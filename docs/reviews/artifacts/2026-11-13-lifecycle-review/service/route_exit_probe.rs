// Review-only fake adapter; no real ADB, device or service effects.
use blent::usb::{Adb, CommandFuture, Commands, Routes};
use std::{
    collections::BTreeMap,
    os::unix::process::ExitStatusExt,
    sync::{Arc, Mutex},
};
#[derive(Clone, Default)]
struct Fake(Arc<Mutex<BTreeMap<String, String>>>);
fn output(code: i32, text: String) -> std::process::Output {
    std::process::Output {
        status: std::process::ExitStatus::from_raw(code << 8),
        stdout: text.into_bytes(),
        stderr: vec![],
    }
}
impl Commands for Fake {
    fn execute(&self, args: Vec<String>, _: Option<Vec<u8>>) -> CommandFuture<'_> {
        Box::pin(async move {
            let mut routes = self.0.lock().unwrap();
            match args[2].as_str() {
                "shell" => Ok(output(0, "review-physical".into())),
                "reverse" => Ok(reverse(&mut routes, &args)),
                _ => panic!("unexpected fake command"),
            }
        })
    }
}
fn reverse(routes: &mut BTreeMap<String, String>, args: &[String]) -> std::process::Output {
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
        "--remove" if std::env::var_os("BLENT_REVIEW_SETTLED").is_some() => {
            routes.remove(&args[4]);
            output(0, String::new())
        }
        "--remove" => output(1, String::new()),
        _ => panic!("unexpected fake reverse command"),
    }
}
fn main() {
    let settled = std::env::var_os("BLENT_REVIEW_SETTLED").is_some();
    let fake = Fake::default();
    let inspect = fake.clone();
    let proof = blent::runtime_owner::run(async move {
        let adb = Adb(fake);
        let mut routes = Routes::new("REVIEWUSB", (19000, 19001)).unwrap();
        routes.prepare(&adb).await.unwrap();
        assert_eq!(routes.retire(&adb).await.is_ok(), settled);
        let proof = routes.completion().unwrap();
        drop(routes);
        proof
    })
    .unwrap();
    println!(
        "runtime_owner_returned=true completion={} remaining_fake_routes={}",
        proof.is_complete(),
        inspect.0.lock().unwrap().len()
    );
    assert_eq!(proof.is_complete(), settled);
    assert_eq!(inspect.0.lock().unwrap().len(), if settled { 0 } else { 2 });
}
