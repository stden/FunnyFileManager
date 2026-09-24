//! Purpose: A fake machine for the MCP installer's tests — temp folders for the
//! home, the project, PATH and ffm's own folder, and a `claude` that edits its
//! config the way the real one does — so no test reads a real config or
//! starts a real client.

use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, PoisonError},
};

use serde_json::{Map, Value};
use tempfile::TempDir;

use super::write_executable;
use crate::mcp_clients::{Captured, CliCall, ClientId, Exit, Host, Runner, Scope, SERVER_NAME};

/// Temp folders standing in for a machine, and the `claude` on it.
#[derive(Debug)]
pub struct FakeMachine {
    root: TempDir,
    claude: Arc<ClaudeEmulator>,
}

impl FakeMachine {
    /// An empty machine: a home, a project folder, a PATH folder and ffm's
    /// own folder, with nothing installed in any of them.
    pub fn new() -> io::Result<Self> {
        let root = TempDir::new()?;
        for dir in ["home", "project", "bin", "exe"] {
            fs::create_dir(root.path().join(dir))?;
        }
        let claude = Arc::new(ClaudeEmulator::new(
            root.path().join("home").join(".claude.json"),
        ));
        Ok(Self { root, claude })
    }

    #[must_use]
    pub fn home(&self) -> PathBuf {
        self.root.path().join("home")
    }

    /// The project folder the active panel shows.
    #[must_use]
    pub fn folder(&self) -> PathBuf {
        self.root.path().join("project")
    }

    /// The one folder on the fake PATH.
    #[must_use]
    pub fn bin(&self) -> PathBuf {
        self.root.path().join("bin")
    }

    /// The folder the fake `ffm` runs from.
    #[must_use]
    pub fn exe_dir(&self) -> PathBuf {
        self.root.path().join("exe")
    }

    /// Where `ffm-mcp` sits next to `ffm`.
    #[must_use]
    pub fn server(&self) -> PathBuf {
        self.exe_dir().join(SERVER_NAME)
    }

    /// Put an inert program called `name` on PATH.
    pub fn install(&self, name: &str) -> io::Result<PathBuf> {
        let path = self.bin().join(name);
        write_executable(&path, b"fake")?;
        Ok(path)
    }

    /// Put every client's first program on PATH.
    pub fn install_clients(&self) -> io::Result<()> {
        for program in ["claude", "codex", "junie", "kiro"] {
            self.install(program)?;
        }
        Ok(())
    }

    /// Put `ffm-mcp` next to `ffm`.
    pub fn install_server(&self) -> io::Result<PathBuf> {
        write_executable(&self.server(), b"fake")?;
        Ok(self.server())
    }

    pub fn remove_server(&self) -> io::Result<()> {
        fs::remove_file(self.server())
    }

    /// The machine as the installer sees it.
    #[must_use]
    pub fn host(&self) -> Host {
        let runner: Arc<dyn Runner> = Arc::<ClaudeEmulator>::clone(&self.claude);
        Host {
            home: Some(self.home()),
            env: BTreeMap::new(),
            search_path: vec![self.bin()],
            program_suffixes: vec![String::new()],
            exe_dir: Some(self.exe_dir()),
            runner,
        }
    }

    #[must_use]
    pub fn claude(&self) -> &ClaudeEmulator {
        &self.claude
    }

    /// A client's config file at `scope`, where the table says it lives.
    #[must_use]
    pub fn config(&self, client: ClientId, scope: Scope) -> PathBuf {
        let row = client.client();
        let base = match scope {
            Scope::AllProjects => relative(&self.home(), row.home_dir),
            Scope::CurrentFolder => self.folder(),
        };
        relative(&base, row.config(scope))
    }

    /// Write `text` to a client's config, creating its folders.
    pub fn write_config(&self, client: ClientId, scope: Scope, text: &str) -> io::Result<PathBuf> {
        let path = self.config(client, scope);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, text)?;
        Ok(path)
    }

    /// Every file in the home and the project folder, with its bytes.
    pub fn files(&self) -> io::Result<BTreeMap<PathBuf, Vec<u8>>> {
        let mut files = BTreeMap::new();
        collect(&self.home(), &mut files)?;
        collect(&self.folder(), &mut files)?;
        Ok(files)
    }
}

/// What tells one version of a file from the next.
///
/// On Unix that is its inode, which every atomic replacement changes;
/// elsewhere its modification time, the best the platform offers. A
/// timestamp alone misses a rewrite within one tick of the file system's
/// clock, which is most of them.
#[cfg(unix)]
pub fn file_identity(path: &Path) -> io::Result<u64> {
    use std::os::unix::fs::MetadataExt;
    Ok(fs::metadata(path)?.ino())
}

/// See the Unix version.
#[cfg(not(unix))]
pub fn file_identity(path: &Path) -> io::Result<std::time::SystemTime> {
    fs::metadata(path)?.modified()
}

fn relative(base: &Path, relative: &str) -> PathBuf {
    relative
        .split('/')
        .filter(|part| !part.is_empty())
        .fold(base.to_path_buf(), |path, part| path.join(part))
}

fn collect(dir: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(&path, files)?;
        } else {
            files.insert(path.clone(), fs::read(&path)?);
        }
    }
    Ok(())
}

/// How the fake `claude` answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaudeBehaviour {
    /// Adds and removes the user-scope entry as the real CLI does.
    Obey,
    /// Runs past its time limit.
    TimeOut,
    /// Is gone by the time it is run: its program is deleted, and the run
    /// fails as a missing program does.
    Vanish,
    /// Exits 1 with this on stderr, whatever it was asked.
    Fail(String),
    /// Removes as asked, then fails the add with this on stderr.
    FailAdd(String),
    /// Exits 0 without writing anything.
    Pretend,
}

/// A `claude` that edits its user config — the fake home's `.claude.json`
/// unless told otherwise — and records every call it is given.
#[derive(Debug)]
pub struct ClaudeEmulator {
    config: Mutex<PathBuf>,
    behaviour: Mutex<ClaudeBehaviour>,
    calls: Mutex<Vec<CliCall>>,
}

impl ClaudeEmulator {
    #[must_use]
    pub fn new(config: PathBuf) -> Self {
        Self {
            config: Mutex::new(config),
            behaviour: Mutex::new(ClaudeBehaviour::Obey),
            calls: Mutex::new(Vec::new()),
        }
    }

    pub fn behave(&self, behaviour: ClaudeBehaviour) {
        *self
            .behaviour
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = behaviour;
    }

    /// Keep the user config at `config` from now on, as the real CLI does
    /// under `CLAUDE_CONFIG_DIR` or with a legacy `.config.json`.
    pub fn use_config(&self, config: PathBuf) {
        *self.config.lock().unwrap_or_else(PoisonError::into_inner) = config;
    }

    fn config(&self) -> PathBuf {
        self.config
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Every call so far, in order.
    #[must_use]
    pub fn calls(&self) -> Vec<CliCall> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn answer(&self, call: &CliCall, args: &[&str]) -> io::Result<Captured> {
        let behaviour = self
            .behaviour
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let adding = matches!(args.get(1), Some(&"add" | &"add-json"));
        match behaviour {
            ClaudeBehaviour::TimeOut => Err(io::Error::new(io::ErrorKind::TimedOut, "timed out")),
            ClaudeBehaviour::Vanish => {
                fs::remove_file(&call.program)?;
                Err(io::Error::new(io::ErrorKind::NotFound, "no claude"))
            }
            ClaudeBehaviour::Fail(stderr) => Ok(failed(&stderr)),
            ClaudeBehaviour::FailAdd(stderr) if adding => Ok(failed(&stderr)),
            ClaudeBehaviour::Obey | ClaudeBehaviour::FailAdd(_) => self.obey(args),
            ClaudeBehaviour::Pretend => Ok(succeeded()),
        }
    }

    fn obey(&self, args: &[&str]) -> io::Result<Captured> {
        let mut servers = self.servers()?;
        let done = match args {
            ["mcp", "add", "--scope", "user", "--transport", "stdio", name, "--", command] => {
                let entry = serde_json::json!({
                    "type": "stdio", "command": command, "args": [], "env": {}
                });
                servers.insert((*name).to_owned(), entry).is_none()
            }
            ["mcp", "add-json", "--scope", "user", name, json] => {
                let entry: Value = serde_json::from_str(json).map_err(io::Error::other)?;
                servers.insert((*name).to_owned(), entry).is_none()
            }
            ["mcp", "remove", "--scope", "user", name] => servers.shift_remove(*name).is_some(),
            _ => false,
        };
        if !done {
            return Ok(failed("claude: cannot do that"));
        }
        self.save(servers)?;
        Ok(succeeded())
    }

    /// The top-level `mcpServers` of the fake `.claude.json`.
    fn servers(&self) -> io::Result<Map<String, Value>> {
        let root = self.root()?;
        match root.get("mcpServers") {
            Some(Value::Object(servers)) => Ok(servers.clone()),
            _ => Ok(Map::new()),
        }
    }

    fn root(&self) -> io::Result<Map<String, Value>> {
        match fs::read_to_string(self.config()) {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(Value::Object(root)) => Ok(root),
                _ => Err(io::Error::other("claude: the config does not parse")),
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Map::new()),
            Err(error) => Err(error),
        }
    }

    fn save(&self, servers: Map<String, Value>) -> io::Result<()> {
        let mut root = self.root()?;
        root.insert("mcpServers".to_owned(), Value::Object(servers));
        fs::write(self.config(), format!("{:#}\n", Value::Object(root)))
    }
}

impl Runner for ClaudeEmulator {
    fn run(&self, call: &CliCall) -> io::Result<Captured> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(call.clone());
        let args: Vec<&str> = call.args.iter().map(String::as_str).collect();
        self.answer(call, &args)
    }
}

fn failed(stderr: &str) -> Captured {
    Captured {
        exit: Exit::Failure(Some(1)),
        stdout: String::new(),
        stderr: format!("{stderr}\n"),
    }
}

fn succeeded() -> Captured {
    Captured {
        exit: Exit::Success,
        stdout: "Added\n".to_owned(),
        stderr: String::new(),
    }
}
