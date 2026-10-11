//! Audio preferences and explicit controls; native construction stays in factory().
use blent_config::audio::{
    AudioController, AudioOptions, AudioState, AudioStatus, Direction, Processing,
};
use eframe::egui;

pub(super) struct Panel {
    pub(super) backend: Option<Box<dyn AudioController>>,
    error: Option<String>,
    create: fn() -> Result<Box<dyn AudioController>, String>,
    transports: fn(Direction) -> blent_config::connection::media::Transports,
}
impl Default for Panel {
    fn default() -> Self {
        Self {
            backend: None,
            error: None,
            create: factory,
            transports: transport_capabilities,
        }
    }
}
impl Panel {
    pub(super) fn show(&mut self, ui: &mut egui::Ui, options: &mut AudioOptions) {
        let microphone = options.profile.direction == Direction::Microphone;
        let (label, selector) = if microphone {
            ("Microphone", "input")
        } else {
            ("Speakers", "output")
        };
        ui.heading(format!("{label} sharing"));
        ui.label(format!("Select Blent {label} in your computer’s {selector} selector. Existing defaults stay unchanged."));
        let status = self
            .backend
            .as_ref()
            .map_or_else(AudioStatus::default, |b| b.status());
        ui.label(&status.detail);
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::RED, error);
        }
        self.buttons(ui, options, status.state);
        super::settings_grid(
            ui,
            if microphone {
                "audio-microphone"
            } else {
                "audio-speakers"
            },
            |ui| {
                preferences(ui, options);
                connection(ui, options, (self.transports)(options.profile.direction));
            },
        );
        ui.label("Start applies these settings. Apply saves preferences; it never starts audio or restarts display sharing.");
        ui.label(if microphone { "Open Blent on the tablet for microphone permission and background consent. Raw mode may include acoustic echo." }
            else { "Open Blent on the tablet for speaker controls and background consent. Other apps may pause playback through audio focus." });
    }
    fn buttons(&mut self, ui: &mut egui::Ui, options: &AudioOptions, state: AudioState) {
        let active = matches!(
            state,
            AudioState::Starting | AudioState::Streaming | AudioState::Stopping
        );
        let label = if options.profile.direction == Direction::Microphone {
            "microphone"
        } else {
            "speakers"
        };
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !active && (self.transports)(options.profile.direction).available(),
                    egui::Button::new(format!("Start {label}")),
                )
                .clicked()
            {
                self.error = self.start(options.clone()).err();
            }
            if ui
                .add_enabled(active, egui::Button::new(format!("Stop {label}")))
                .clicked()
            {
                if let Some(backend) = &self.backend {
                    backend.stop();
                }
            }
        });
        if active {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
        if !(self.transports)(options.profile.direction).available() {
            ui.label("Audio backend unavailable on this operating system.");
        }
    }
    fn start(&mut self, options: AudioOptions) -> Result<(), String> {
        options.validate().map_err(|e| e.to_string())?;
        if !(self.transports)(options.profile.direction)
            .supports(&options.target().map_err(|e| e.to_string())?)
        {
            return Err("Selected audio transport is unavailable on this backend.".into());
        }
        if self.backend.is_none() {
            self.backend = Some((self.create)()?);
        }
        self.backend
            .as_ref()
            .unwrap()
            .start(options)
            .map_err(|e| format!("{e:#}"))
    }
}
fn transport_capabilities(direction: Direction) -> blent_config::connection::media::Transports {
    #[cfg(target_os = "linux")]
    return blent::audio::transport_capabilities(direction);
    #[cfg(not(target_os = "linux"))]
    {
        let _ = direction;
        blent_config::connection::media::Transports::default()
    }
}
fn connection(
    ui: &mut egui::Ui,
    options: &mut AudioOptions,
    transports: blent_config::connection::media::Transports,
) {
    use crate::media_target_settings::{self, Description, Selection};
    media_target_settings::show(ui, Selection {serial: &mut options.serial, usb_peer: &mut options.usb_peer}, transports,
        Description {label:"USB audio status", unavailable:"Direct USB audio is unavailable on this backend.", qualification:"Experimental: physical debugging-off audio acceptance is pending.",
            consent:"Pair and select this tablet in USB connection controls. Start this audio direction locally on the tablet; pairing grants no audio access."});
}

fn factory() -> Result<Box<dyn AudioController>, String> {
    #[cfg(target_os = "linux")]
    return blent::audio_control::Controller::new(blent::audio::run)
        .map(|c| Box::new(c) as Box<dyn AudioController>)
        .map_err(|e| e.to_string());
    #[cfg(not(target_os = "linux"))]
    Err("Audio backend unavailable".into())
}
fn preferences(ui: &mut egui::Ui, options: &mut AudioOptions) {
    ui.label("Processing");
    ui.horizontal(|ui| {
        ui.selectable_value(
            &mut options.profile.processing,
            Processing::Speech,
            "Speech",
        );
        ui.selectable_value(&mut options.profile.processing, Processing::Raw, "Raw");
    });
    ui.end_row();
    ui.label("Application buffering");
    ui.add(egui::Slider::new(&mut options.profile.buffer_ms, 20..=200).step_by(10.0).suffix(" ms"))
        .on_hover_text("Lower means less lag; higher tolerates more jitter. Native device buffers add their own delay.");
    ui.end_row();
    ui.label("Tablet background");
    ui.checkbox(
        &mut options.profile.background,
        "Continue while hidden (requires tablet consent)",
    );
    ui.end_row();
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use blent_config::audio::Direction;
    use std::{cell::RefCell, rc::Rc};
    struct Backend(Rc<RefCell<Vec<AudioOptions>>>);
    impl AudioController for Backend {
        fn start(&self, options: AudioOptions) -> blent_config::camera::BackendResult {
            self.0.borrow_mut().push(options);
            Ok(())
        }
        fn stop(&self) {
            self.0.borrow_mut().clear();
        }
        fn status(&self) -> AudioStatus {
            AudioStatus::default()
        }
    }
    #[test]
    fn t718_audio_controls_preserve_passive_preferences_and_validate_start() {
        let starts = Rc::new(RefCell::new(Vec::new()));
        let mut panel = Panel {
            backend: Some(Box::new(Backend(starts.clone()))),
            error: Some("fixture failure".into()),
            ..Default::default()
        };
        let mut options = AudioOptions::new(Direction::Microphone);
        let context = egui::Context::default();
        let _ = context.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| panel.show(ui, &mut options));
        });
        assert!(starts.borrow().is_empty());
        panel.start(options.clone()).unwrap();
        assert_eq!(starts.borrow()[0], options);
        options.profile.buffer_ms = 1;
        assert!(panel.start(options).is_err());
        drop(factory().unwrap());
    }
    #[test]
    fn t979_invalid_full_audio_targets_never_reach_even_a_permissive_backend() {
        let starts = Rc::new(RefCell::new(Vec::new()));
        let mut panel = Panel {
            backend: Some(Box::new(Backend(starts.clone()))),
            ..Default::default()
        };
        for direction in [Direction::Microphone, Direction::Speakers] {
            for length in 0..=128 {
                if length == 64 {
                    continue;
                }
                let mut options = AudioOptions::new(direction);
                options.usb_peer = Some("a".repeat(length));
                assert!(
                    panel.start(options).is_err(),
                    "T979 malformed full USB target reached backend"
                );
            }
            let mut options = AudioOptions::new(direction);
            options.usb_peer = Some("ab".repeat(32));
            options.serial = Some("other-tablet".into());
            assert!(
                panel.start(options).is_err(),
                "T979 ambiguous serial/peer reached backend"
            );
        }
        assert!(starts.borrow().is_empty());
    }
    #[test]
    fn t979_unavailable_direction_and_factory_failure_never_create_or_retain_backend() {
        let mut panel = Panel {
            create: || panic!("T979 unavailable target reached native factory"),
            transports: |_| Default::default(),
            ..Default::default()
        };
        for direction in [Direction::Microphone, Direction::Speakers] {
            for usb in [false, true] {
                let mut options = AudioOptions::new(direction);
                options.usb_peer = usb.then(|| "ab".repeat(32));
                assert_eq!(
                    panel.start(options).unwrap_err(),
                    "Selected audio transport is unavailable on this backend."
                );
            }
        }
        assert!(panel.backend.is_none());
        panel.transports = transport_capabilities;
        panel.create = || Err("T979 private constructor failed".into());
        assert_eq!(
            panel
                .start(AudioOptions::new(Direction::Speakers))
                .unwrap_err(),
            "T979 private constructor failed"
        );
        assert!(panel.backend.is_none());
        let mut microphone = AudioOptions::new(Direction::Microphone);
        microphone.usb_peer = Some("ab".repeat(32));
        panel.transports = |_| Default::default();
        assert_eq!(
            panel.start(microphone).unwrap_err(),
            "Selected audio transport is unavailable on this backend."
        );
        assert!(transport_capabilities(Direction::Speakers).usb);
        assert!(transport_capabilities(Direction::Microphone).usb);
    }
    #[test]
    fn t979_exact_speaker_usb_and_microphone_adb_options_reach_only_selected_backend() {
        let starts = Rc::new(RefCell::new(Vec::new()));
        let mut panel = Panel {
            backend: Some(Box::new(Backend(starts.clone()))),
            ..Default::default()
        };
        let mut speaker = AudioOptions::new(Direction::Speakers);
        speaker.usb_peer = Some("ab".repeat(32));
        speaker.profile.buffer_ms = 80;
        speaker.profile.background = true;
        speaker.profile.processing = Processing::Raw;
        panel.start(speaker.clone()).unwrap();
        let mut microphone = AudioOptions::new(Direction::Microphone);
        microphone.serial = Some("exact adb serial".into());
        panel.start(microphone.clone()).unwrap();
        let mut usb_microphone = AudioOptions::new(Direction::Microphone);
        usb_microphone.usb_peer = Some("cd".repeat(32));
        usb_microphone.profile.buffer_ms = 80;
        usb_microphone.profile.background = true;
        usb_microphone.profile.processing = Processing::Raw;
        panel.start(usb_microphone.clone()).unwrap();
        assert_eq!(*starts.borrow(), vec![speaker, microphone, usb_microphone]);
    }
    #[test]
    fn t979_bounded_profile_and_nonhex_target_inputs_never_reach_backend() {
        let starts = Rc::new(RefCell::new(Vec::new()));
        let mut panel = Panel {
            backend: Some(Box::new(Backend(starts.clone()))),
            ..Default::default()
        };
        for direction in [Direction::Microphone, Direction::Speakers] {
            for buffer_ms in 0..=240 {
                if (20..=200).contains(&buffer_ms) && buffer_ms % 10 == 0 {
                    continue;
                }
                let mut options = AudioOptions::new(direction);
                options.profile.buffer_ms = buffer_ms;
                assert!(panel.start(options).is_err());
            }
        }
        for byte in 0..=255u8 {
            if byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte) {
                continue;
            }
            let mut value = vec![b'a'; 64];
            value[31] = byte;
            let mut options = AudioOptions::new(Direction::Speakers);
            options.usb_peer = Some(String::from_utf8_lossy(&value).into());
            assert!(panel.start(options).is_err());
        }
        assert!(starts.borrow().is_empty());
    }
    #[test]
    fn t979_unavailable_saved_audio_targets_remain_visible_and_rendering_stays_passive() {
        let mut panel = Panel {
            transports: |_| Default::default(),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        for direction in [Direction::Microphone, Direction::Speakers] {
            for usb in [false, true] {
                let mut options = AudioOptions::new(direction);
                options.usb_peer = usb.then(|| "ab".repeat(32));
                let original = options.clone();
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(900.0, 1800.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| panel.show(ui, &mut options));
                    },
                );
                assert_eq!(options, original);
                assert!(panel.backend.is_none());
            }
        }
    }
}
