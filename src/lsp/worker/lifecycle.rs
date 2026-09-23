//! Purpose: Require explicit session-scoped workspace trust before launching any server process.

use super::{
    report, workspace_root, Command, Connection, DocumentId, Envelope, Event, PathBuf,
    ServerSettings, ServerState, Snapshot, SyncSender,
};

#[derive(Default)]
enum Session {
    #[default]
    Closed,
    Untrusted(Box<Start>),
    Running(Box<Connection>),
}

struct Start {
    language: String,
    settings: ServerSettings,
    snapshot: Snapshot,
    // Canonical identity for trust; preserve the original spelling for external tools.
    root: PathBuf,
    workspace: PathBuf,
}

#[derive(Default)]
pub(super) struct Host {
    session: Session,
    trusted: Vec<PathBuf>,
}

impl Host {
    pub fn command(&mut self, command: Command, events: &SyncSender<Envelope>) {
        match command {
            Command::Open(language, settings, snapshot) => {
                self.session = Session::Closed;
                let owner = snapshot.stamp.id;
                let start = Start::prepare(language, settings, snapshot);
                match start {
                    Ok(start) => self.open(start, events),
                    Err(error) => failed(events, owner, &error),
                }
            }
            Command::Trust(owner) => self.trust(owner, events),
            Command::Close => self.session = Session::Closed,
            command => self.forward(command),
        }
    }

    fn forward(&mut self, command: Command) {
        match (&mut self.session, command) {
            (Session::Running(active), command) => active.command(command),
            (Session::Untrusted(start), Command::Update(snapshot))
                if snapshot.stamp.id == start.snapshot.stamp.id =>
            {
                start.snapshot = snapshot;
            }
            _ => {} // Closed/untrusted sessions cannot execute document requests.
        }
    }

    fn open(&mut self, start: Start, events: &SyncSender<Envelope>) {
        if self.trusted.contains(&start.root) {
            self.start(start, events);
        } else {
            report(
                events,
                start.snapshot.stamp.id,
                Event::TrustRequired(start.root.clone()),
            );
            self.session = Session::Untrusted(Box::new(start));
        }
    }

    fn trust(&mut self, owner: DocumentId, events: &SyncSender<Envelope>) {
        if !matches!(&self.session, Session::Untrusted(start) if start.snapshot.stamp.id == owner) {
            return;
        }
        if let Session::Untrusted(start) = std::mem::take(&mut self.session) {
            self.trusted.push(start.root.clone());
            self.start(*start, events);
        }
    }

    fn start(&mut self, start: Start, events: &SyncSender<Envelope>) {
        let owner = start.snapshot.stamp.id;
        report(events, owner, Event::State(ServerState::Starting));
        match Connection::start(
            start.language,
            &start.settings,
            start.snapshot,
            &start.workspace,
        ) {
            Ok(connection) => {
                self.session = Session::Running(Box::new(connection));
                report(events, owner, Event::State(ServerState::Initializing));
            }
            Err(error) => failed(events, owner, &error),
        }
    }

    pub fn pump(&mut self, events: &SyncSender<Envelope>) {
        if let Session::Running(active) = &mut self.session {
            if let Err(message) = active.pump(events) {
                crate::diagnostics::log_error(message);
                report(events, active.owner, Event::State(ServerState::Failed));
                self.session = Session::Closed;
            }
        }
    }
}

impl Start {
    fn prepare(
        language: String,
        settings: ServerSettings,
        mut snapshot: Snapshot,
    ) -> std::io::Result<Self> {
        snapshot.path = std::path::absolute(&snapshot.path)?;
        let workspace = workspace_root(&snapshot.path, &settings.root_markers);
        let root = workspace.canonicalize()?;
        Ok(Self {
            language,
            settings,
            snapshot,
            root,
            workspace,
        })
    }
}

fn failed(events: &SyncSender<Envelope>, owner: DocumentId, error: &std::io::Error) {
    crate::diagnostics::log_error(&format!("LSP start failed: {:?}", error.kind()));
    let state = if error.kind() == std::io::ErrorKind::NotFound {
        ServerState::Unavailable
    } else {
        ServerState::Failed
    };
    report(events, owner, Event::State(state));
}

#[cfg(test)]
mod tests;
