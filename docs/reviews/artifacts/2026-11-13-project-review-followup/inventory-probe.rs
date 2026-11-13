fn main() {
    for serial in ["bad\u{1b}serial".to_owned(), "x".repeat(1025)] {
        let parsed = blent::adb_inventory::parse(&format!("List of devices attached\n{serial}\tdevice\n"));
        let route = blent::usb::Routes::for_transport(&serial, (8890,8891));
        println!("serial bytes={}: inventory confirmed={}, route admitted={}", serial.len(), parsed.is_some(), route.is_ok());
    }
}
