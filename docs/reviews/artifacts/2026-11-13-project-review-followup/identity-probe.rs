use blent_config::commands::AsyncCommandExt;
use std::{collections::HashMap, os::unix::fs::PermissionsExt};
include!("linux_identity_body.inc");
#[tokio::main]
async fn main() {
    let root = tempfile::tempdir().unwrap();
    let adb = root.path().join("adb");
    std::fs::write(&adb, "#!/bin/sh\ncat \"$0.$2\"\n").unwrap();
    std::fs::set_permissions(&adb, std::fs::Permissions::from_mode(0o700)).unwrap();
    let portable = blent::usb::Adb(blent::usb::NativeCommands(adb.clone()));
    let mut ids = HashMap::new();
    for (serial, bytes) in [("ONE", vec![255]), ("TWO", vec![254]), ("NUL", b"a\0b".to_vec()), ("LONG", vec![b'x';1025])] {
        std::fs::write(adb.with_extension(serial), bytes).unwrap();
        let linux = probe_device_identity(serial, adb.to_str().unwrap()).await;
        let other = portable.identity(serial).await;
        println!("{serial}: Linux identity length={:?}; portable identity={other:?}",linux.as_ref().map(|(_,id)|id.len()));
        if let Some((key,value)) = linux { ids.insert(key,value); }
    }
    let devices = ["ONE".to_string(), "TWO".to_string()];
    println!("Distinct invalid byte strings collapse: {}",ids["ONE"]==ids["TWO"]);
    println!("Selected endpoints from two devices: {:?}",blent::transport::select_device_transports(&devices,None,&ids));
}
