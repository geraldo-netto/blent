//! One owned egui style per context. Fixed font/color/spacing values allocate no
//! native resources and retain no views beyond the context's own lifetime.
use eframe::egui::{self, Color32, FontId, TextStyle};

pub(super) const FORM_WIDTH: f32 = 960.0;

pub(super) fn configure(context: &egui::Context) {
    context.set_theme(egui::Theme::Light);
    context.set_zoom_factor(1.0);
    let mut style = (*context.style()).clone();
    for (kind, size) in [
        (TextStyle::Body, 15.0),
        (TextStyle::Button, 14.0),
        (TextStyle::Small, 12.0),
        (TextStyle::Heading, 21.0),
    ] {
        style.text_styles.insert(kind, FontId::proportional(size));
    }
    style.animation_time = egui::Style::default().animation_time;
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.interact_size.y = 32.0;
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.visuals.override_text_color = None;
    style.visuals.error_fg_color = Color32::from_rgb(160, 25, 35);
    style.visuals.weak_text_color = Some(Color32::from_gray(80));
    style.visuals.selection.bg_fill = Color32::from_rgb(38, 92, 165);
    style.visuals.selection.stroke.color = Color32::WHITE;
    context.set_style(style);
}

/// Left-align a bounded page using a frame-local borrowed UI. No retained state or
/// native resources; egui supplies finite viewport width and owns all shapes.
pub(super) fn content<R>(
    ui: &mut egui::Ui,
    maximum: f32,
    show: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let width = ui.available_width().min(maximum).max(0.0);
    let mut rect = ui.available_rect_before_wrap();
    rect.max.x = rect.min.x + width;
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), show)
        .inner
}

fn apply_native(context: &egui::Context, appearance: &crate::appearance_contract::Appearance) {
    configure(context);
    context.set_zoom_factor(appearance.text_scale);
    context.style_mut(|style| {
        native_palette(&mut style.visuals, appearance);
        let body = appearance.font_points * (4.0 / 3.0);
        for (kind, factor) in [
            (TextStyle::Body, 1.0),
            (TextStyle::Button, 1.0),
            (TextStyle::Small, 0.9),
            (TextStyle::Heading, 1.4),
        ] {
            style
                .text_styles
                .insert(kind, FontId::proportional(body * factor));
        }
        if !appearance.animations {
            style.animation_time = 0.0;
        }
    });
}

fn native_palette(visuals: &mut egui::Visuals, native: &crate::appearance_contract::Appearance) {
    let rgb = |value: [u8; 3]| Color32::from_rgb(value[0], value[1], value[2]);
    let background = rgb(native.background);
    let foreground = readable(rgb(native.foreground), background);
    visuals.panel_fill = background;
    visuals.window_fill = background;
    visuals.extreme_bg_color = rgb(native.surface);
    visuals.faint_bg_color = background;
    visuals.selection.bg_fill = rgb(native.accent);
    visuals.selection.stroke.color = readable(rgb(native.selected_text), visuals.selection.bg_fill);
    visuals.hyperlink_color = readable(rgb(native.accent), background);
    visuals.error_fg_color = readable(visuals.error_fg_color, background);
    visuals.weak_text_color = Some(foreground);
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.fg_stroke.color = foreground;
        widget.bg_fill = rgb(native.surface);
        widget.weak_bg_fill = background;
    }
    visuals.widgets.active.bg_stroke =
        egui::Stroke::new(2.0_f32, readable(rgb(native.accent), background));
}

fn luminance(color: Color32) -> f64 {
    [color.r(), color.g(), color.b()]
        .into_iter()
        .zip([0.2126, 0.7152, 0.0722])
        .map(|(byte, weight)| {
            let channel = f64::from(byte) / 255.0;
            weight
                * if channel <= 0.04045 {
                    channel / 12.92
                } else {
                    ((channel + 0.055) / 1.055).powf(2.4)
                }
        })
        .sum()
}

fn contrast(left: Color32, right: Color32) -> f64 {
    let (left, right) = (luminance(left), luminance(right));
    (left.max(right) + 0.05) / (left.min(right) + 0.05)
}

fn readable(requested: Color32, background: Color32) -> Color32 {
    if contrast(requested, background) >= 4.5 {
        return requested;
    }
    if contrast(Color32::BLACK, background) >= contrast(Color32::WHITE, background) {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}

pub(super) fn configure_desktop(context: &egui::Context) {
    #[cfg(target_os = "linux")]
    install_desktop(context, crate::platform::appearance::load());
    #[cfg(not(target_os = "linux"))]
    install_desktop(context, Err("Native appearance adapter unavailable".into()));
}

fn install_desktop(
    context: &egui::Context,
    result: Result<(crate::appearance_contract::Appearance, Vec<u8>), String>,
) {
    let description = match result {
        Ok((appearance, bytes)) => {
            apply_native(context, &appearance);
            install_font(context, bytes);
            format!(
                "{} · {} {} pt · text scale {}",
                appearance.source,
                appearance.font_family,
                appearance.font_points,
                appearance.text_scale
            )
        }
        Err(reason) => {
            configure(context);
            format!("Standard light defaults · {reason}")
        }
    };
    context.data_mut(|data| data.insert_temp(egui::Id::new("desktop-appearance"), description));
}

fn install_font(context: &egui::Context, bytes: Vec<u8>) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "desktop".into(),
        std::sync::Arc::new(egui::FontData::from_owned(bytes)),
    );
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "desktop".into());
    context.set_fonts(fonts);
}

pub(super) fn appearance_info(ui: &mut egui::Ui) {
    let description = ui
        .ctx()
        .data(|data| data.get_temp::<String>(egui::Id::new("desktop-appearance")))
        .unwrap_or_else(|| "Standard light defaults".into());
    ui.label(format!("Appearance: {description}"));
    ui.small("Desktop appearance is read when Blent opens. Theme changes apply the next time the Blent GUI starts.");
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> crate::appearance_contract::Appearance {
        crate::appearance_contract::Appearance {
            source: "Fixture light theme".into(),
            background: [248, 248, 249],
            surface: [255, 255, 255],
            foreground: [32, 32, 32],
            accent: [31, 158, 222],
            selected_text: [255, 255, 255],
            font_family: "Sans".into(),
            font_points: 10.0,
            text_scale: 1.0,
            animations: false,
        }
    }

    #[test]
    fn t1814_reenabling_desktop_animations_restores_animation_duration() {
        let context = egui::Context::default();
        let mut native = fixture();
        apply_native(&context, &native);
        assert_eq!(context.style().animation_time, 0.0);
        native.animations = true;
        apply_native(&context, &native);
        assert!(context.style().animation_time > 0.0);
    }

    #[test]
    fn t1786_native_palette_and_font_size_replace_custom_theme() {
        let context = egui::Context::default();
        apply_native(&context, &fixture());
        let style = context.style();
        assert_eq!(
            style.visuals.panel_fill,
            Color32::from_rgb(248, 248, 249),
            "T1786 native surface ignored"
        );
        assert_eq!(
            style.visuals.selection.bg_fill,
            Color32::from_rgb(31, 158, 222)
        );
        assert!((style.text_styles[&TextStyle::Body].size - 40.0 / 3.0).abs() < 0.01);
        assert_eq!(style.animation_time, 0.0);
    }
    #[test]
    fn t1786_semantic_contrast_and_large_text_remain_readable() {
        for background in [
            Color32::WHITE,
            Color32::BLACK,
            Color32::from_rgb(31, 158, 222),
        ] {
            for foreground in [Color32::WHITE, Color32::BLACK, Color32::GRAY] {
                assert!(contrast(readable(foreground, background), background) >= 4.5);
            }
        }
        let context = egui::Context::default();
        let mut native = fixture();
        native.text_scale = 2.0;
        native.animations = true;
        apply_native(&context, &native);
        let _ = context.run(Default::default(), |_| {});
        assert!(
            (context.style().text_styles[&TextStyle::Body].size * context.zoom_factor()
                - 80.0 / 3.0)
                .abs()
                < 0.01
        );
        assert!(
            contrast(
                context.style().visuals.selection.stroke.color,
                context.style().visuals.selection.bg_fill
            ) >= 4.5
        );
        install_desktop(&context, Err("Fixture unavailable".into()));
        let _ = context.run(Default::default(), |context| {
            egui::CentralPanel::default().show(context, appearance_info);
        });
    }
}
