#[path = "/backups/disk2/projects/blent/gui/src/camera_preview.rs"]
mod camera_preview;
use blent_config::camera::{CameraPreview, Lens};
use eframe::egui;

fn main() {
    let context = egui::Context::default();
    let mut preview = camera_preview::Preview::default();
    let frame =
        std::sync::Arc::new(CameraPreview::new(Lens::Front, [2, 2], vec![127; 16]).unwrap());
    let weak = std::sync::Arc::downgrade(&frame);
    let _ = context.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| preview.show(ui, Some(frame.clone())));
    });
    drop(frame);
    let count = context.tex_manager().read().num_allocated();
    std::thread::sleep(std::time::Duration::from_millis(2100));
    let _ = context.run(Default::default(), |_| {}); // Another tab renders.
    println!(
        "hidden cache: retained={}, fresh={}, textures before={}, now={}",
        weak.upgrade().is_some(),
        weak.upgrade().unwrap().fresh(),
        count,
        context.tex_manager().read().num_allocated()
    );
    let _ = context.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| preview.show(ui, None));
    });
    println!(
        "camera tab revisited: retained={}, textures={}",
        weak.upgrade().is_some(),
        context.tex_manager().read().num_allocated()
    );
}
