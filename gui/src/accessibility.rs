//! Frame-local semantic adapters. egui owns nodes; no widget references escape.
use eframe::egui;

pub(super) fn field(
    response: egui::Response,
    label: egui::Id,
    description: &str,
) -> egui::Response {
    response.ctx.accesskit_node_builder(response.id, |node| {
        node.set_description(description);
    });
    response.labelled_by(label)
}

pub(super) fn combo(response: egui::Response, label: egui::Id, value: &str) {
    response
        .ctx
        .accesskit_node_builder(response.id, |node| node.set_value(value));
    response.labelled_by(label);
}

pub(super) fn selected(response: egui::Response, selected: bool) {
    response
        .ctx
        .accesskit_node_builder(response.id, |node| node.set_selected(selected));
}
