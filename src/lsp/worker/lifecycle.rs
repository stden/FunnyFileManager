//! Purpose: Require explicit session-scoped workspace trust before launching any server process.

use super::{
    report, workspace_root, Command, Connection, DocumentId, Envelope, Event, Path, PathBuf,
    ServerSettings, ServerState, Snapshot, SyncSender,
};
mod install;
use install::Preparation;

#[derive(Default)]
enum Session {
    #[default]
    Closed,
    Untrusted(Box<Start>),
    Preparing(Box<Preparation>),
    Running(Box<Connection>),
}

struct Start {
    language: String,
    configured: ServerSettings,
    settings: ServerSettings,
    snapshot: Snapshot,
    // Canonical identity for trust; preserve the original spelling for external tools.
    root: PathBuf,
    workspace: PathBuf,
}

#[derive(Default)]
pub(super) struct Host {
    session: Session,
    pub(super) idle: Vec<Connection>,
    trusted: Vec<PathBuf>,
    pub(super) cache: Option<PathBuf>,
    pub(super) missing: Vec<(String, ServerSettings, &'static str)>,
}

impl Host {
    pub(super) const fn is_active(&self) -> bool {
        matches!(self.session, Session::Preparing(_) | Session::Running(_)) || !self.idle.is_empty()
    }

    pub fn command(&mut self, command: Command, events: &SyncSender<Envelope>) {
        match command {
            Command::Open(language, settings, snapshot) => {
                self.park();
                let owner = snapshot.stamp.id;
                match Start::prepare(language, settings, snapshot) {
                    Ok(start) => self.open(start, events),
                    Err(error) => failed(events, owner, &error),
                }
            }
            Command::Restart => self.restart(),
            Command::Trust(owner) => self.trust(owner, events),
            Command::Close => self.park(),
            command => self.forward(command),
        }
    }

    fn forward(&mut self, command: Command) {
        match (&mut self.session, command) {
            (Session::Running(active), command) => active.command(command),
            (Session::Untrusted(start), Command::Update(snapshot))
                if snapshot.stamp.id == start.snapshot.stamp.id =>
            {
                start.update(snapshot);
            }
            (Session::Preparing(preparing), Command::Update(snapshot))
                if snapshot.stamp.id == preparing.start.snapshot.stamp.id =>
            {
                preparing.start.update(snapshot);
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
        if let Some(message) = self.cached_missing(&start.language, &start.configured) {
            report(
                events,
                start.snapshot.stamp.id,
                Event::InstallationFailed(message),
            );
            return;
        }
        if let Some(mut connection) = self.take_idle(&start) {
            connection.reopen(start.snapshot, events);
            self.session = Session::Running(Box::new(connection));
            return;
        }
        self.launch(start, events);
    }

    pub fn pump(&mut self, events: &SyncSender<Envelope>) {
        self.installation(events);
        self.idle.retain_mut(|conn| conn.pump(events).is_ok());
        let pump_result = if let Session::Running(active) = &mut self.session {
            active.pump(events).err()
        } else {
            return;
        };
        let Some(failure) = pump_result else {
            return;
        };
        let Session::Running(active) = std::mem::take(&mut self.session) else {
            return;
        };
        self.fail_running(&active, failure, events);
    }
}

impl Start {
    fn update(&mut self, mut snapshot: Snapshot) {
        snapshot.path.clone_from(&self.snapshot.path);
        self.snapshot = snapshot;
    }

    fn prepare(
        language: String,
        settings: ServerSettings,
        mut snapshot: Snapshot,
    ) -> std::io::Result<Self> {
        snapshot.path = document_path(&snapshot.path)?;
        let workspace = workspace_root(&snapshot.path, &settings.root_markers);
        let root = workspace.canonicalize()?;
        Ok(Self {
            language,
            configured: settings.clone(),
            settings,
            snapshot,
            root,
            workspace,
        })
    }
}

fn document_path(path: &Path) -> std::io::Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    match absolute.canonicalize() {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(absolute),
        Err(error) => Err(error),
    }
}

pub(super) fn failed(events: &SyncSender<Envelope>, owner: DocumentId, error: &std::io::Error) {
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
