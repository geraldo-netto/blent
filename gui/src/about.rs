//! Passive project information. Static notice bytes live for the process lifetime;
//! egui owns only the selected document index and the last bounded adapter error.
use eframe::egui;
mod documents;
use documents::DOCUMENTS;

pub(crate) fn identity(ui: &mut egui::Ui) {
    let identity = build_identity();
    ui.label(&identity);
    if ui.button("Copy version details").clicked() {
        ui.ctx().copy_text(identity);
    }
    ui.add_space(8.0);
    ui.strong("Credits");
    for paragraph in DOCUMENTS[1].1.split("\n\n").skip(1).take(4) {
        ui.label(paragraph);
    }
    ui.add_space(8.0);
}

fn build_identity() -> String {
    format!(
        "Version {} · {} / {}\nBuild revision: {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        option_env!("BLENT_BUILD_REVISION").unwrap_or("not recorded")
    )
}

pub(crate) fn notices(ui: &mut egui::Ui) {
    ui.add_space(12.0);
    ui.heading("Licenses and notices");
    ui.label("Blent's own code uses the MIT license: you may use, modify and share it, including commercially, while retaining the copyright and permission notices. It is provided without warranty. Third-party components retain their own terms.");
    ui.label("Full project and tracked component notices are embedded below and work offline. Android notices describe the APK; they do not identify this computer's libraries.");
    document_viewer(ui);
    ui.add_space(8.0);
    bundle_link(ui, crate::platform::bundled_notices_url);
}

fn document_viewer(ui: &mut egui::Ui) {
    let id = ui.make_persistent_id("about-document");
    let mut index = ui.data(|data| data.get_temp::<usize>(id)).unwrap_or(0);
    index = index.min(DOCUMENTS.len() - 1);
    let width = ui.available_width().min(520.0);
    egui::ComboBox::from_id_salt("license-selector")
        .selected_text(DOCUMENTS[index].0)
        .wrap()
        .width(width)
        .show_ui(ui, |ui| {
            ui.set_max_width((width - 24.0).max(40.0));
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            for (candidate, (name, _)) in DOCUMENTS.iter().enumerate() {
                ui.selectable_value(&mut index, candidate, *name);
            }
        });
    ui.data_mut(|data| data.insert_temp(id, index));
    notice_text(ui, index);
}

fn notice_text(ui: &mut egui::Ui, index: usize) {
    let (name, text) = DOCUMENTS[index];
    if ui.button("Copy full notice").clicked() {
        ui.ctx().copy_text(text.to_owned());
    }
    egui::ScrollArea::vertical()
        .id_salt(("license-text", name))
        .max_height(260.0)
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.add(egui::Label::new(text).selectable(true).wrap());
        });
}

fn bundle_link(ui: &mut egui::Ui, locate: impl FnOnce() -> Result<String, String>) {
    ui.label("Packaged builds may include additional libraries and tools. Their exact notices and source manifests belong to that installed bundle, separately from the embedded project notices.");
    let id = ui.make_persistent_id("bundle-notice-error");
    if ui.button("Open installed bundle notices").clicked() {
        match locate() {
            Ok(url) => {
                ui.data_mut(|data| data.remove::<String>(id));
                ui.ctx().open_url(egui::OpenUrl::new_tab(url));
            }
            Err(error) => ui.data_mut(|data| data.insert_temp(id, error)),
        }
    }
    if let Some(error) = ui.data(|data| data.get_temp::<String>(id)) {
        ui.label(error);
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests;
