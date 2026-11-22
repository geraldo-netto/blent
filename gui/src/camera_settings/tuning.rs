use blent_config::camera::tuning::{Capabilities, Flicker, Metering, Sensor};
use eframe::egui;
mod noise;

pub(super) fn show(ui: &mut egui::Ui, sensor: &mut Sensor, caps: Option<Capabilities>) {
    ui.label("Tablet image controls");
    ui.label("Changes apply live; camera processing may take several frames.");
    ui.end_row();
    let Some(caps) = caps else {
        ui.label("Camera capabilities");
        ui.label("Start the camera to discover supported controls.");
        ui.end_row();
        return;
    };
    ui.label("Sensor defaults");
    if ui.button("Reset tablet adjustments").clicked() {
        *sensor = Sensor::default();
    }
    ui.end_row();
    exposure(ui, sensor, caps);
    metering(ui, sensor, caps);
    flicker(ui, sensor, caps);
    noise::show(ui, &mut sensor.noise, caps.noise_mask);
}
fn exposure(ui: &mut egui::Ui, sensor: &mut Sensor, caps: Capabilities) {
    let (minimum, maximum, step) = caps.exposure_bounds();
    let label = ui.label("Exposure compensation").id;
    let response = ui.add_enabled(step > 0.0 && minimum < maximum,
        egui::Slider::new(&mut sensor.exposure, (minimum*100.0).ceil() as i16..=(maximum*100.0).floor() as i16)
            .clamping(egui::SliderClamping::Edits)
            .step_by((step*100.0).max(1.0))
            .custom_formatter(|value, _| format!("{:+.2} EV", value/100.0))
            .custom_parser(parse_exposure))
        .on_hover_text("Positive values ask the camera for a brighter exposure; highlights can clip and motion can blur.");
    crate::accessibility::field(response, label, "Disabled when the tablet reports no adjustable exposure range.");
    ui.end_row();
}
fn parse_exposure(text: &str) -> Option<f64> {
    let text = text.trim().strip_suffix("EV").unwrap_or(text).trim();
    let value = text.parse::<f64>().ok()? * 100.0;
    value.is_finite().then_some(value)
}
fn metering(ui: &mut egui::Ui, sensor: &mut Sensor, caps: Capabilities) {
    let label = ui.label("Exposure metering").id;
    let value = match sensor.metering { Metering::Device => "Device default", Metering::Center => "Center weighted" };
    let response = egui::ComboBox::from_id_salt("camera-metering")
        .selected_text(value)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut sensor.metering, Metering::Device, "Device default");
            ui.add_enabled_ui(caps.center, |ui| {
                ui.selectable_value(&mut sensor.metering, Metering::Center, "Center weighted");
            });
        });
    crate::accessibility::combo(response.response, label, value);
    ui.end_row();
}
fn flicker(ui: &mut egui::Ui, sensor: &mut Sensor, caps: Capabilities) {
    let label = ui.label("Anti-flicker").id;
    let value = flicker_label(sensor.flicker).to_owned();
    let response = egui::ComboBox::from_id_salt("camera-flicker")
        .selected_text(flicker_label(sensor.flicker))
        .show_ui(ui, |ui| {
            for value in [Flicker::Device, Flicker::Auto, Flicker::Hz50, Flicker::Hz60] {
                ui.add_enabled_ui(caps.flicker_mask & (1 << value as u8) != 0, |ui| {
                    ui.selectable_value(&mut sensor.flicker, value, flicker_label(value));
                });
            }
        });
    crate::accessibility::combo(response.response, label, &value);
    ui.end_row();
}
fn flicker_label(value: Flicker) -> &'static str {
    match value {
        Flicker::Device => "Device default",
        Flicker::Auto => "Automatic",
        Flicker::Hz50 => "50 Hz",
        Flicker::Hz60 => "60 Hz",
    }
}
#[cfg(test)]
mod tests;
