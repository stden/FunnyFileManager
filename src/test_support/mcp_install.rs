//! Purpose: A fake machine for the MCP installer's tests — temp folders for the
//! home, the project, PATH and ffm's own folder, and a `claude` that edits its
//! config the way the real one does — so no test reads a real config or
//! starts a real client.

mod claude;

use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};

use tempfile::TempDir;

use super::write_executable;
use crate::mcp_clients::{ClientId, Host, Runner, Scope, SERVER_NAME};
pub use claude::{ClaudeBehaviour, ClaudeEmulator};

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
