//! Purpose: The fake machine's `claude`: it edits its user config the way the
//! real CLI does, or fails, times out or vanishes as a test scripts it, and
//! records every call it is given.

use std::{
    fs, io,
    path::PathBuf,
    sync::{Mutex, PoisonError},
};

use serde_json::{Map, Value};

use crate::mcp_clients::{Captured, CliCall, Exit, Runner};

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
