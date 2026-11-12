use super::*;
use blent_config::connection::{
    control::Notice,
    control_client::{Outcome, SubmitError},
    control_view::{Action, Assembler, Limits, Model},
    pairing::PeerId,
};
use std::sync::{Arc, Mutex};

#[test]
fn t1273_only_current_authenticated_links_supply_workspace_tablets() {
    let (mut panel, handle) = fixture();
    assert!(Panel::default().tablets().is_empty());
    assert!(panel.tablets().is_empty());
    {
        let mut fake = handle.0.lock().unwrap();
        fake.status.view = Some(publish(&mut fake.model, 2, 1, State::Authenticated));
    }
    assert_eq!(
        panel.tablets()[0].identity(),
        &blent_config::tablets::Identity::UsbPeer(peer(2))
    );
    handle.0.lock().unwrap().status.phase = Phase::Closed;
    assert!(panel.tablets().is_empty());
    handle.0.lock().unwrap().status.phase = Phase::Busy;
    assert_eq!(panel.tablets().len(), 1);
    handle.0.lock().unwrap().status.view = None;
    assert!(panel.tablets().is_empty());
}

struct Fake {
    status: Status,
    model: Model,
    commands: Vec<Command>,
    retired: bool,
    full: bool,
    starts: usize,
}
#[derive(Clone)]
struct Handle(Arc<Mutex<Fake>>);
impl Client for Handle {
    fn status(&self) -> Status {
        self.0.lock().unwrap().status.clone()
    }
    fn submit(&self, action: Action) -> Result<u64, SubmitError> {
        let mut fake = self.0.lock().unwrap();
        if fake.full {
            return Err(SubmitError::Full);
        }
        let command = fake
            .model
            .command(&action)
            .map_err(|_| SubmitError::Closed)?
            .clone();
        fake.commands.push(command);
        Ok(fake.commands.len() as u64)
    }
    fn close(&self) {
        let mut fake = self.0.lock().unwrap();
        fake.status.phase = Phase::Closing;
        fake.status.view = None;
    }
    fn retired(&self) -> bool {
        self.0.lock().unwrap().retired
    }
}
impl backend::Source for Handle {
    fn start(&mut self) -> Result<Box<dyn Client>, String> {
        let mut fake = self.0.lock().unwrap();
        fake.starts += 1;
        fake.retired = false;
        fake.status.phase = Phase::Connecting;
        Ok(Box::new(self.clone()))
    }
}
fn peer(value: u64) -> PeerId {
    PeerId::parse(format!("{value:064x}")).unwrap()
}
fn publish(model: &mut Model, revision: u64, epoch: u64, state: State) -> View {
    publish_ownership(model, revision, epoch, state, None)
}
fn publish_ownership(
    model: &mut Model,
    revision: u64,
    epoch: u64,
    state: State,
    connection: Option<u64>,
) -> View {
    let mut assembler = Assembler::new(Limits::default()).unwrap();
    let mut snapshot = None;
    for notice in [
        Notice::Begin {
            revision,
            local: peer(1),
        },
        Notice::Device {
            revision,
            target: 1,
            vendor: 0x18d1,
            product: 0x2d00,
            accessory: true,
            names: blent_config::connection::control::DeviceNames::new(
                "Unisoc".into(),
                "RugKing Pad 2 Pro".into(),
            )
            .unwrap(),
            availability: blent_config::connection::control::Availability::Available,
            connection,
        },
        Notice::Link {
            revision,
            connection: 2,
            ticket: Some(Ticket {
                epoch,
                peer: peer(2),
            }),
            state,
            trusted: state == State::Authenticated,
        },
        Notice::Saved {
            revision,
            peer: peer(3),
        },
        Notice::End { revision },
    ] {
        snapshot = assembler.push(notice).unwrap();
    }
    model.publish(snapshot.unwrap()).unwrap()
}
fn fixture() -> (Panel, Handle) {
    let mut model = Model::default();
    let view = publish(&mut model, 1, 1, State::Pending);
    let handle = Handle(Arc::new(Mutex::new(Fake {
        status: Status {
            phase: Phase::Ready,
            view: Some(view),
            message: None,
            outcome: None,
        },
        model,
        commands: Vec::new(),
        retired: false,
        full: false,
        starts: 0,
    })));
    let panel = Panel {
        client: Some(Box::new(handle.clone())),
        source: Some(Box::new(handle.clone())),
        ..Default::default()
    };
    (panel, handle)
}
fn frame(
    panel: &mut Panel,
    ctx: &egui::Context,
    width: f32,
    events: Vec<egui::Event>,
) -> Vec<(String, egui::Rect)> {
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 1800.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| panel.show(ui));
        },
    );
    let mut text = Vec::new();
    for shape in output.shapes {
        collect_text(&shape.shape, &mut text);
    }
    text
}
fn collect_text(shape: &egui::epaint::Shape, text: &mut Vec<(String, egui::Rect)>) {
    match shape {
        egui::epaint::Shape::Text(shape) => text.push((
            shape.galley.job.text.clone(),
            shape.galley.rect.translate(shape.pos.to_vec2()),
        )),
        egui::epaint::Shape::Vec(shapes) => {
            for shape in shapes {
                collect_text(shape, text);
            }
        }
        _ => (),
    }
}
fn click(panel: &mut Panel, ctx: &egui::Context, label: &str) {
    let labels = frame(panel, ctx, 600.0, Vec::new());
    let position = labels
        .iter()
        .find(|(text, _)| text == label)
        .unwrap_or_else(|| panic!("missing {label}: {labels:?}"))
        .1
        .center();
    for pressed in [true, false] {
        frame(
            panel,
            ctx,
            600.0,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
}
fn complete(handle: &Handle, error: Option<&str>) {
    let mut fake = handle.0.lock().unwrap();
    fake.status.outcome = Some(Outcome {
        id: fake.commands.len() as u64,
        error: error.map(str::to_owned),
    });
}

#[test]
fn t894_widgets_match_fingerprint_labels_and_explicit_remembered_approval() {
    let (mut panel, handle) = fixture();
    let ctx = egui::Context::default();
    let labels = frame(&mut panel, &ctx, 600.0, Vec::new());
    for expected in [
        "Computer fingerprint (SHA-256)",
        "Tablet fingerprint (SHA-256)",
        peer(1).as_str(),
        peer(2).as_str(),
    ] {
        assert!(labels.iter().any(|(text, _)| text == expected));
    }
    assert!(handle.0.lock().unwrap().commands.is_empty());
    click(&mut panel, &ctx, "Remember this tablet");
    click(&mut panel, &ctx, "Approve tablet");
    assert_eq!(
        handle.0.lock().unwrap().commands,
        [Command::Decide {
            connection: 2,
            ticket: Ticket {
                epoch: 1,
                peer: peer(2)
            },
            allow: true,
            remember: true
        }]
    );
    assert_eq!(panel.pending, Some(1));
    click(&mut panel, &ctx, "Deny");
    assert_eq!(
        handle.0.lock().unwrap().commands.len(),
        1,
        "pending command disables further mutations"
    );
    complete(&handle, None);
    panel.refresh();
    assert!(panel.pending.is_none());
    click(&mut panel, &ctx, "Deny");
    assert!(matches!(
        handle.0.lock().unwrap().commands[1],
        Command::Decide {
            allow: false,
            remember: false,
            ..
        }
    ));
}

#[test]
fn t894_device_disconnect_and_saved_peer_buttons_issue_exact_commands() {
    let (mut panel, handle) = fixture();
    let ctx = egui::Context::default();
    click(&mut panel, &ctx, "Connect");
    assert_eq!(
        handle.0.lock().unwrap().commands[0],
        Command::Connect {
            revision: 1,
            target: 1
        }
    );
    complete(&handle, None);
    click(&mut panel, &ctx, "Disconnect");
    assert_eq!(
        handle.0.lock().unwrap().commands[1],
        Command::Disconnect {
            connection: 2,
            ticket: Some(Ticket {
                epoch: 1,
                peer: peer(2)
            })
        }
    );
    complete(&handle, None);
    click(&mut panel, &ctx, "Forget tablet");
    assert_eq!(
        handle.0.lock().unwrap().commands[2],
        Command::Revoke { peer: peer(3) }
    );
    complete(&handle, Some("durable storage refused revoke"));
    panel.refresh();
    assert_eq!(
        panel.error.as_deref(),
        Some("durable storage refused revoke")
    );
    panel.refresh();
    assert_eq!(
        panel.error.as_deref(),
        Some("durable storage refused revoke")
    );
}

#[test]
fn t894_remember_choice_is_scoped_to_exact_ticket_and_pending_state() {
    let (mut panel, handle) = fixture();
    let ctx = egui::Context::default();
    click(&mut panel, &ctx, "Remember this tablet");
    frame(&mut panel, &ctx, 600.0, Vec::new());
    assert!(
        panel.choices[&2].remember,
        "unchanged polling must preserve choice"
    );
    {
        let mut fake = handle.0.lock().unwrap();
        fake.status.view = Some(publish(&mut fake.model, 2, 2, State::Pending));
    }
    frame(&mut panel, &ctx, 600.0, Vec::new());
    assert!(!panel.choices[&2].remember);
    {
        let mut fake = handle.0.lock().unwrap();
        fake.status.view = Some(publish(&mut fake.model, 3, 2, State::Authenticated));
    }
    let labels = frame(&mut panel, &ctx, 600.0, Vec::new());
    assert!(panel.choices.is_empty());
    assert!(labels.iter().any(|(text, _)| text == "Tablet remembered"));
    assert!(!labels.iter().any(|(text, _)| text == "Approve tablet"));
}

#[test]
fn t894_failed_and_cancelled_commands_stay_unconfirmed() {
    for phase in [Phase::Failed, Phase::Closing, Phase::Closed] {
        let (mut panel, handle) = fixture();
        let view = handle.status().view.unwrap();
        panel.send(&view, Command::Inspect);
        {
            let mut fake = handle.0.lock().unwrap();
            fake.status.phase = phase;
            fake.status.view = None;
            fake.status.message = Some("transport lost".into());
        }
        panel.refresh();
        panel.refresh();
        assert!(panel.pending.is_none());
        assert!(
            panel.error.as_ref().unwrap().contains("unconfirmed"),
            "T894 preserve uncertain command result alongside transport error"
        );
    }
}

#[test]
fn t894_refresh_does_not_complete_queued_command_or_clear_rejection() {
    let (mut panel, handle) = fixture();
    let view = handle.status().view.unwrap();
    panel.send(&view, Command::Inspect);
    panel.refresh();
    assert_eq!(panel.pending, Some(1));
    handle.0.lock().unwrap().status.outcome = Some(Outcome {
        id: 99,
        error: None,
    });
    panel.refresh();
    assert_eq!(panel.pending, Some(1));
    complete(&handle, Some("rejected"));
    panel.refresh();
    handle.0.lock().unwrap().status.outcome = None;
    panel.refresh();
    assert_eq!(panel.error.as_deref(), Some("rejected"));
}

#[test]
fn t894_retry_requires_actual_retirement_and_reuses_source() {
    let (mut panel, handle) = fixture();
    let ctx = egui::Context::default();
    assert!(panel.start().is_err());
    click(&mut panel, &ctx, "Stop discovering");
    assert_eq!(handle.status().phase, Phase::Closing);
    assert!(panel.start().is_err());
    assert!(
        handle.0.lock().unwrap().commands.is_empty(),
        "stop controls never revokes independent media"
    );
    handle.0.lock().unwrap().retired = true;
    click(&mut panel, &ctx, "Find USB devices");
    assert_eq!(handle.0.lock().unwrap().starts, 1);
    assert!(panel.choices.is_empty());
    assert_eq!(handle.status().phase, Phase::Connecting);
}

#[test]
fn t894_rejected_admission_and_invalid_input_do_not_claim_success() {
    let (mut panel, handle) = fixture();
    let view = handle.status().view.unwrap();
    for target in [0, 2, u64::MAX] {
        panel.send(
            &view,
            Command::Connect {
                revision: 1,
                target,
            },
        );
        assert!(panel.pending.is_none());
        assert!(panel.error.is_some());
    }
    handle.0.lock().unwrap().full = true;
    panel.send(&view, Command::Inspect);
    assert!(panel.error.as_ref().unwrap().contains("Full"));
    panel.client = None;
    panel.send(&view, Command::Inspect);
    assert!(panel.error.as_ref().unwrap().contains("closed"));
    assert!(panel.refresh().is_none());
    panel.source = None;
    panel.create = || Err("service unavailable".into());
    assert_eq!(panel.start().unwrap_err(), "service unavailable");
}

#[test]
fn t894_empty_inventory_and_small_window_keep_labels_readable() {
    let (mut panel, handle) = fixture();
    let ctx = egui::Context::default();
    for width in [320.0, 600.0, 1200.0] {
        for (label, rect) in frame(&mut panel, &ctx, width, Vec::new()) {
            assert!(rect.right() <= width, "clipped {label}: {rect:?}");
        }
    }
    let mut assembler = Assembler::new(Limits::default()).unwrap();
    assembler
        .push(Notice::Begin {
            revision: 2,
            local: peer(1),
        })
        .unwrap();
    let snapshot = assembler
        .push(Notice::End { revision: 2 })
        .unwrap()
        .unwrap();
    {
        let mut fake = handle.0.lock().unwrap();
        fake.status.view = Some(fake.model.publish(snapshot).unwrap());
    }
    let labels = frame(&mut panel, &ctx, 600.0, Vec::new());
    assert!(labels
        .iter()
        .any(|(text, _)| text.starts_with("No USB devices")));
    assert!(labels
        .iter()
        .any(|(text, _)| text == "No tablets remembered."));
    for phase in [
        Phase::Connecting,
        Phase::Ready,
        Phase::Busy,
        Phase::Closing,
        Phase::Closed,
        Phase::Failed,
    ] {
        assert!(!phase_label(phase).is_empty());
    }
    for state in [
        State::Discovering,
        State::Pending,
        State::Approved,
        State::Authenticated,
        State::Closing,
        State::Quarantined,
    ] {
        assert!(!link_label(state).is_empty());
    }
}

#[test]
fn t894_terminal_status_does_not_hide_incomplete_cleanup() {
    let (mut panel, handle) = fixture();
    let ctx = egui::Context::default();
    for phase in [Phase::Failed, Phase::Closed] {
        {
            let mut fake = handle.0.lock().unwrap();
            fake.status.phase = phase;
            fake.status.view = None;
        }
        let labels = frame(&mut panel, &ctx, 600.0, Vec::new());
        assert!(
            labels
                .iter()
                .any(|(text, _)| text == "Waiting for native cleanup; retry unavailable."),
            "T894 terminal status is not retirement proof"
        );
    }
    handle.0.lock().unwrap().retired = true;
    let labels = frame(&mut panel, &ctx, 600.0, Vec::new());
    assert!(!labels
        .iter()
        .any(|(text, _)| text == "Waiting for native cleanup; retry unavailable."));
}

#[test]
fn t1336_usb_list_shows_model_and_manufacturer() {
    let (mut panel, _) = fixture();
    let labels = frame(&mut panel, &egui::Context::default(), 600.0, Vec::new());
    for expected in [
        "RugKing Pad 2 Pro",
        "Manufacturer: Unisoc",
        "Android accessory mode",
    ] {
        assert!(
            labels.iter().any(|(text, _)| text == expected),
            "T1336 missing {expected}"
        );
    }
}

fn replace_named_device(handle: &Handle, manufacturer: &str, model: &str, accessory: bool) {
    let mut assembler = Assembler::new(Limits::default()).unwrap();
    let mut snapshot = None;
    for notice in [
        Notice::Begin {
            revision: 2,
            local: peer(1),
        },
        Notice::Device {
            revision: 2,
            target: 7,
            vendor: 0x1234,
            product: 0x5678,
            accessory,
            names: blent_config::connection::control::DeviceNames::new(
                manufacturer.into(),
                model.into(),
            )
            .unwrap(),
            availability: blent_config::connection::control::Availability::Available,
            connection: None,
        },
        Notice::End { revision: 2 },
    ] {
        snapshot = assembler.push(notice).unwrap();
    }
    let mut fake = handle.0.lock().unwrap();
    fake.status.view = Some(fake.model.publish(snapshot.unwrap()).unwrap());
}

#[test]
fn t1336_unnamed_device_remains_explicit_and_selectable() {
    let (mut panel, handle) = fixture();
    replace_named_device(&handle, "", "", false);
    let ctx = egui::Context::default();
    let labels = frame(&mut panel, &ctx, 320.0, Vec::new());
    for expected in [
        "Model unavailable",
        "Manufacturer unavailable",
        "Tablet support unconfirmed",
        "USB 1234:5678 · Device 7",
    ] {
        assert!(labels.iter().any(|(text, _)| text == expected));
    }
    click(&mut panel, &ctx, "Connect");
    assert_eq!(
        handle.0.lock().unwrap().commands,
        [Command::Connect {
            revision: 2,
            target: 7
        }]
    );
}

#[test]
fn t1336_long_names_wrap_in_small_windows() {
    let (mut panel, handle) = fixture();
    replace_named_device(&handle, &"M".repeat(256), &"X".repeat(256), false);
    for width in [320.0, 600.0] {
        let ctx = egui::Context::default();
        for (label, rect) in frame(&mut panel, &ctx, width, Vec::new()) {
            assert!(rect.right() <= width, "T1336 clipped {label}: {rect:?}");
        }
    }
}

#[test]
fn t1337_owned_usb_row_shows_state_and_disables_connect() {
    let (mut panel, handle) = fixture();
    {
        let mut fake = handle.0.lock().unwrap();
        fake.status.view = Some(publish_ownership(
            &mut fake.model,
            2,
            1,
            State::Authenticated,
            Some(2),
        ));
    }
    let ctx = egui::Context::default();
    let labels = frame(&mut panel, &ctx, 600.0, Vec::new());
    assert!(
        labels.iter().any(|(text, _)| text == "Connected"),
        "T1337 active USB row needs Connected status"
    );
    click(&mut panel, &ctx, "Connect");
    assert!(handle.0.lock().unwrap().commands.is_empty());
}

#[test]
fn t1337_row_stays_locked_through_approval_and_cleanup() {
    for (index, state) in [
        State::Pending,
        State::Approved,
        State::Authenticated,
        State::Closing,
        State::Quarantined,
    ]
    .into_iter()
    .enumerate()
    {
        let (mut panel, handle) = fixture();
        {
            let mut fake = handle.0.lock().unwrap();
            fake.status.view = Some(publish_ownership(
                &mut fake.model,
                index as u64 + 2,
                1,
                state,
                Some(2),
            ));
        }
        let ctx = egui::Context::default();
        let labels = frame(&mut panel, &ctx, 320.0, Vec::new());
        assert!(labels
            .iter()
            .any(|(text, _)| text == device_state_label(state)));
        click(&mut panel, &ctx, "Connect");
        assert!(handle.0.lock().unwrap().commands.is_empty());
    }
    assert_eq!(device_state_label(State::Discovering), "Connecting");
}

fn reserved_view(
    model: &mut Model,
    availability: blent_config::connection::control::Availability,
) -> View {
    let mut assembler = Assembler::new(Limits::default()).unwrap();
    assembler
        .push(Notice::Begin {
            revision: 2,
            local: peer(1),
        })
        .unwrap();
    assembler
        .push(Notice::Device {
            revision: 2,
            target: 1,
            vendor: 0x18d1,
            product: 0x4ee8,
            accessory: false,
            names: Default::default(),
            availability,
            connection: None,
        })
        .unwrap();
    model
        .publish(
            assembler
                .push(Notice::End { revision: 2 })
                .unwrap()
                .unwrap(),
        )
        .unwrap()
}

#[test]
fn t1338_reserved_or_unknown_device_disables_connect() {
    use blent_config::connection::control::Availability;
    for (availability, label) in [
        (Availability::Debugging, "Reserved for USB debugging"),
        (Availability::Unknown, "USB availability unknown"),
    ] {
        let (mut panel, handle) = fixture();
        {
            let mut fake = handle.0.lock().unwrap();
            fake.status.view = Some(reserved_view(&mut fake.model, availability));
        }
        let ctx = egui::Context::default();
        let labels = frame(&mut panel, &ctx, 600.0, Vec::new());
        assert!(
            labels.iter().any(|(text, _)| text == label),
            "T1338 missing {label}"
        );
        click(&mut panel, &ctx, "Connect");
        assert!(handle.0.lock().unwrap().commands.is_empty());
    }
}

fn grid_devices(handle: &Handle) {
    use blent_config::connection::control::{Availability, DeviceNames};
    let mut assembler = Assembler::new(Limits::default()).unwrap();
    assembler
        .push(Notice::Begin {
            revision: 2,
            local: peer(1),
        })
        .unwrap();
    for target in 1..=6 {
        assembler
            .push(Notice::Device {
                revision: 2,
                target,
                vendor: 0x1234,
                product: 0x5678,
                accessory: false,
                names: DeviceNames::new("Maker".into(), format!("Tablet {target}")).unwrap(),
                availability: if target == 2 {
                    Availability::Debugging
                } else {
                    Availability::Available
                },
                connection: None,
            })
            .unwrap();
    }
    let snapshot = assembler
        .push(Notice::End { revision: 2 })
        .unwrap()
        .unwrap();
    let mut fake = handle.0.lock().unwrap();
    fake.status.view = Some(fake.model.publish(snapshot).unwrap());
}

fn label_rect(labels: &[(String, egui::Rect)], label: &str) -> egui::Rect {
    labels.iter().find(|(text, _)| text == label).unwrap().1
}

#[test]
fn t1339_usb_cards_use_columns_and_reflow_in_narrow_windows() {
    let (mut panel, handle) = fixture();
    grid_devices(&handle);
    let ctx = egui::Context::default();
    for (width, columns) in [(320.0, 1), (600.0, 2), (900.0, 3), (320.0, 1)] {
        let labels = frame(&mut panel, &ctx, width, Vec::new());
        let first = label_rect(&labels, "Tablet 1");
        let same_row = label_rect(&labels, &format!("Tablet {columns}"));
        let next_row = label_rect(&labels, &format!("Tablet {}", columns + 1));
        assert!(
            (first.top() - same_row.top()).abs() < 1.0,
            "T1339 missing {columns} columns"
        );
        assert!(next_row.top() > first.bottom());
        assert!(
            labels.iter().all(|(_, rect)| rect.right() <= width),
            "T1339 clipped card"
        );
    }
}

#[test]
fn t1339_column_counts_are_bounded_for_invalid_and_extreme_widths() {
    for width in [
        f32::NAN,
        f32::NEG_INFINITY,
        f32::INFINITY,
        -1.0,
        0.0,
        279.0,
        280.0,
        559.9,
    ] {
        assert_eq!(device_columns(width), 1);
    }
    for columns in 2..=8 {
        assert_eq!(device_columns(columns as f32 * 280.0), columns);
    }
    assert_eq!(device_columns(f32::MAX), 8);
}

fn pointer_click(panel: &mut Panel, ctx: &egui::Context, width: f32, position: egui::Pos2) {
    for pressed in [true, false] {
        frame(
            panel,
            ctx,
            width,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
}

#[test]
fn t1339_columns_keep_target_identity_and_reserved_buttons_disabled() {
    let (mut panel, handle) = fixture();
    grid_devices(&handle);
    let ctx = egui::Context::default();
    let labels = frame(&mut panel, &ctx, 900.0, Vec::new());
    let buttons: Vec<_> = labels
        .iter()
        .filter(|(text, _)| text == "Connect")
        .map(|(_, rect)| rect.center())
        .collect();
    assert_eq!(buttons.len(), 6);
    pointer_click(&mut panel, &ctx, 900.0, buttons[1]);
    assert!(handle.0.lock().unwrap().commands.is_empty());
    pointer_click(&mut panel, &ctx, 900.0, buttons[5]);
    assert_eq!(
        handle.0.lock().unwrap().commands,
        [Command::Connect {
            revision: 2,
            target: 6
        }]
    );
}

fn key_events(key: egui::Key) -> Vec<egui::Event> {
    [true, false]
        .into_iter()
        .map(|pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        })
        .collect()
}

#[test]
fn t1339_grid_connect_buttons_remain_keyboard_accessible() {
    let (mut panel, handle) = fixture();
    grid_devices(&handle);
    let ctx = egui::Context::default();
    let labels = frame(&mut panel, &ctx, 900.0, Vec::new());
    let target = labels
        .iter()
        .filter(|(text, _)| text == "Connect")
        .last()
        .unwrap()
        .1;
    for _ in 0..10 {
        frame(&mut panel, &ctx, 900.0, key_events(egui::Key::Tab));
        let focused = ctx
            .memory(|memory| memory.focused())
            .and_then(|id| ctx.read_response(id));
        if focused.is_some_and(|response| response.rect.contains(target.center())) {
            frame(&mut panel, &ctx, 900.0, key_events(egui::Key::Enter));
            assert_eq!(
                handle.0.lock().unwrap().commands,
                [Command::Connect {
                    revision: 2,
                    target: 6
                }]
            );
            return;
        }
    }
    panic!("T1339 last grid action inaccessible by keyboard");
}
