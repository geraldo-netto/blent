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
