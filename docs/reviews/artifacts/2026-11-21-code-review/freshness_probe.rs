// Original next/feed/write_frame bodies copied unchanged for isolated review.
// Native Writer and progress observer are fixture boundaries; no real devices.
use anyhow::Result;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    sync::watch,
    time::{Instant, Interval},
};
type Frames = watch::Receiver<Option<Arc<Vec<u8>>>>;
type Records = Arc<Mutex<Vec<(u128, u8)>>>;
struct Writer {
    origin: Instant,
    records: Records,
    events: tokio::sync::mpsc::UnboundedSender<(u128, u8)>,
}
impl Writer {
    fn write(&mut self, frame: &[u8]) -> std::io::Result<()> {
        let entry = (self.origin.elapsed().as_millis(), frame[0]);
        self.records.lock().unwrap().push(entry);
        self.events.send(entry).unwrap();
        Ok(())
    }
}
mod progress {
    pub type Sender = ();
    pub enum Kind {
        Write,
    }
    pub struct Span;
    impl Span {
        pub fn begin(_: &mut Sender, _: Kind) -> Self {
            Self
        }
    }
}
mod camera_control {
    pub async fn cancelled(stop: &mut tokio::sync::watch::Receiver<bool>) {
        if !*stop.borrow_and_update() {
            let _ = stop.changed().await;
        }
    }
}
mod outputs {
    use std::time::Duration;
    pub(super) const FRESH_FRAME_AGE: Duration = Duration::from_secs(2);

    pub(super) fn current_frame<'a>(
        frame: Option<&'a [u8]>,
        black: &'a [u8],
        age: Duration,
    ) -> &'a [u8] {
        match frame {
            Some(bytes) if age < FRESH_FRAME_AGE => bytes,
            _ => black,
        }
    }
}
enum Event {
    Frame,
    Tick,
    Stop,
}
async fn next(frames: &mut Frames, stop: &mut watch::Receiver<bool>, tick: &mut Interval) -> Event {
    tokio::select! {
        biased;
        _ = crate::camera_control::cancelled(stop) => Event::Stop,
        change = frames.changed() => if change.is_ok() { Event::Frame } else { Event::Stop },
        _ = tick.tick() => Event::Tick,
    }
}
async fn feed(
    mut writer: Writer,
    mut frames: Frames,
    mut stop: watch::Receiver<bool>,
    black: Vec<u8>,
    mut operation: progress::Sender,
) -> Result<()> {
    let mut last_frame = Instant::now();
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    loop {
        match next(&mut frames, &mut stop, &mut tick).await {
            Event::Stop => return write_frame(&mut writer, &black, &mut operation),
            Event::Frame => last_frame = Instant::now(),
            Event::Tick => {}
        }
        let newest = frames.borrow_and_update().clone();
        let bytes = outputs::current_frame(
            newest.as_deref().map(Vec::as_slice),
            &black,
            last_frame.elapsed(),
        );
        write_frame(&mut writer, bytes, &mut operation)?;
    }
}

fn write_frame(writer: &mut Writer, frame: &[u8], operation: &mut progress::Sender) -> Result<()> {
    let _operation = progress::Span::begin(operation, progress::Kind::Write);
    Ok(writer.write(frame)?)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tokio::time::pause();
    let origin = Instant::now();
    let records: Records = Default::default();
    let (frames, receiver) = watch::channel(None);
    let (stop, stopped) = watch::channel(false);
    frames.send_replace(Some(Arc::new(vec![99])));
    tokio::time::advance(Duration::from_millis(1500)).await;
    let (events, mut observed) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(feed(
        Writer {
            origin,
            records: records.clone(),
            events,
        },
        receiver,
        stopped,
        vec![16],
        (),
    ));
    loop {
        let entry = observed.recv().await.unwrap();
        if entry.0 >= 2500 {
            break;
        }
    }
    println!(
        "publication t=0 ms; writes (age_ms,luma)={:?}",
        records.lock().unwrap()
    );
    stop.send_replace(true);
    task.await.unwrap().unwrap();
    println!("after stop (age_ms,luma)={:?}", records.lock().unwrap());
}
