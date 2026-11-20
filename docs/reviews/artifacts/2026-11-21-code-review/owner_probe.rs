#[path = "/backups/disk2/projects/blent/gui/src/media_owner.rs"]
mod media_owner;
use blent_config::{
    camera::{CameraBackend, CameraProfile, CameraState},
    workspace::Activity,
};

fn queued_owner() {
    let (completed, observed) = std::sync::mpsc::channel();
    let controller = blent::camera_control::Controller::with_pool(
        blent::control_workers::WorkerPool::new(1).unwrap(),
        move |_, _, _| {
            completed.send(()).unwrap();
            std::future::ready(Ok(()))
        },
    )
    .unwrap();
    let mut owner = media_owner::Owner::default();
    controller.start(CameraProfile::default()).unwrap();
    owner.admit(None);
    observed.recv().unwrap();
    while controller.state() != CameraState::Stopped {
        std::thread::yield_now();
    }
    println!(
        "backend={:?}, GUI={:?}",
        controller.state(),
        owner.activity(Activity::Off)
    );
    for _ in 0..4 {
        println!("later GUI poll={:?}", owner.activity(Activity::Off));
    }
    let fence = controller.shutdown();
    while !fence.is_complete() {
        std::thread::yield_now();
    }
    println!("original controller/native thread retired");
}

fn colors() {
    for matrix in ["bt601", "bt709"] {
        let bytes = std::fs::read(format!("/tmp/blent-review-20261121/{matrix}.yuv")).unwrap();
        let preview = blent_config::camera::CameraPreview::from_yuv420(
            blent_config::camera::Lens::Front,
            160,
            120,
            &bytes,
        )
        .unwrap();
        println!("actual CameraPreview {matrix}: {:?}", &preview.rgba()[..4]);
    }
}
fn main() {
    queued_owner();
    colors();
}
