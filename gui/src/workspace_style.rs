//! One owned egui style per context. Fixed font/color/spacing values allocate no
//! native resources and retain no views beyond the context's own lifetime.
use eframe::egui::{self, Color32, FontId, TextStyle};

pub(super) fn configure(context: &egui::Context) {
    let mut style = (*context.style()).clone();
    for (kind, size) in [
        (TextStyle::Body, 15.0),
        (TextStyle::Button, 14.0),
        (TextStyle::Small, 12.0),
        (TextStyle::Heading, 21.0),
    ] {
        style.text_styles.insert(kind, FontId::proportional(size));
    }
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.interact_size.y = 32.0;
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.visuals.override_text_color = Some(if style.visuals.dark_mode {
        Color32::from_gray(228)
    } else {
        Color32::from_gray(28)
    });
    style.visuals.weak_text_color = Some(if style.visuals.dark_mode {
        Color32::from_gray(180)
    } else {
        Color32::from_gray(80)
    });
    style.visuals.selection.bg_fill = Color32::from_rgb(38, 92, 165);
    style.visuals.selection.stroke.color = Color32::WHITE;
    context.set_style(style);
}
