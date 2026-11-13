use super::*;

pub(super) struct State<E: Environment> {
    pub(super) environment: E,
    pub(super) status: Status,
    pub(super) session: Option<Session<E::Device>>,
}
impl<E: Environment> State<E> {
    pub(super) fn retire(&mut self) -> Result<()> {
        self.status.negotiated = false;
        if let Some(session) = &mut self.session {
            session.retire()?;
        }
        self.session = None;
        Ok(())
    }
    fn start(&mut self, mode: Mode) -> Result<()> {
        self.retire()?;
        ensure!(
            match mode {
                Mode::Touch => self.status.touch,
                Mode::DirectMouse => self.status.mouse,
            },
            "Input mode disabled"
        );
        let config = Config {
            monitor: self.status.monitor.clone(),
            mode,
        };
        let snapshot = self.environment.snapshot()?;
        snapshot.select(&config.monitor)?;
        self.session = Some(Session::new(
            self.environment.create(mode)?,
            &config,
            &snapshot,
        )?);
        self.environment.save(&config)?;
        self.status.mode = mode;
        self.status.negotiated = true;
        Ok(())
    }
    pub(super) fn command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::Negotiate { version } => {
                ensure!(version == 1, "Unsupported direct input protocol");
                self.start(self.status.mode)
            }
            Command::Select { mode } => {
                ensure!(self.status.negotiated, "Direct input must negotiate first");
                self.start(mode)
            }
            Command::Event { event } => {
                let snapshot = self.environment.snapshot()?;
                self.session
                    .as_mut()
                    .context("Direct input must negotiate first")?
                    .apply(&snapshot, event)
            }
        }
    }
}
