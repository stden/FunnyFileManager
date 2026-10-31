//! A private SSH server with throwaway keys and no access to host SSH config.

use std::fs;
use std::net::{TcpListener, TcpStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::models::{Auth, HostName, KeyFile, Port, Protocol, RemotePath, Session, UserName};
use tempfile::TempDir;

#[cfg(test)]
mod tests;

pub(super) struct Server {
    root: TempDir,
    child: Child,
    port: u16,
    user: String,
}

impl Server {
    pub(super) fn start() -> Self {
        let root = tempfile::tempdir().expect("private SFTP fixture");
        let user = command_text("id", &["-un"]);
        let user = user.trim().to_owned();
        assert_ne!(user, "", "id -un returned an empty username");
        create_keys(root.path());
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("allocate loopback port");
        let port = listener.local_addr().expect("loopback address").port();
        drop(listener);
        let config = root.path().join("sshd_config");
        fs::write(&config, configuration(root.path(), port, &user)).expect("private sshd config");
        let child = spawn(root.path(), &config);
        let mut server = Self {
            root,
            child,
            port,
            user,
        };
        server.wait_until_ready();
        server
    }

    pub(super) fn session(&self) -> Session {
        let mut session = Session::new(
            Protocol::Sftp,
            HostName::new("127.0.0.1").expect("loopback host"),
            Port::new(self.port).expect("port"),
            Some(UserName::new(&self.user).expect("current user")),
        );
        session.auth =
            Auth::KeyFile(KeyFile::new(&self.local("client").to_string_lossy()).expect("key file"));
        session
    }

    pub(super) fn local(&self, path: &str) -> PathBuf {
        self.root.path().join(path)
    }

    pub(super) fn path(&self, path: &str) -> RemotePath {
        RemotePath::parse(&self.local(path).to_string_lossy()).expect("fixture path")
    }

    pub(super) fn diagnostics(&self) -> String {
        fs::read_to_string(self.local("sshd.log")).unwrap_or_default()
    }

    pub(super) fn fingerprint(&self) -> String {
        let public_key = self.local("host.pub");
        let path = public_key.to_str().expect("UTF-8 host key path");
        let output = command_text("ssh-keygen", &["-lf", path, "-E", "sha256"]);
        output
            .split_whitespace()
            .nth(1)
            .expect("host key fingerprint")
            .to_owned()
    }

    fn wait_until_ready(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if self.is_ready() {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "private sshd failed to start: {}",
                self.diagnostics()
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn is_ready(&self) -> bool {
        self.assert_running();
        if !self.pidfile_ready() {
            return false;
        }
        let connected = TcpStream::connect_timeout(
            &([127, 0, 0, 1], self.port).into(),
            Duration::from_millis(50),
        )
        .is_ok();
        self.assert_running();
        connected
    }

    fn assert_running(&self) {
        assert!(
            !crate::platform::has_exited(&self.child).expect("sshd status"),
            "private sshd exited: {}",
            self.diagnostics()
        );
    }

    fn pidfile_ready(&self) -> bool {
        let Ok(text) = fs::read_to_string(self.local("sshd.pid")) else {
            return false;
        };
        let Some(line) = text.strip_suffix('\n') else {
            return false;
        };
        let pid = line.parse::<u32>().expect("invalid private sshd pidfile");
        assert_eq!(
            pid,
            self.child.id(),
            "unexpected sshd pid: {}",
            self.diagnostics()
        );
        true
    }
}

fn create_keys(root: &Path) {
    keygen(&root.join("host"));
    keygen(&root.join("client"));
    let public_key = fs::read(root.join("client.pub")).expect("fixture public key");
    fs::write(root.join("authorized_keys"), public_key).expect("authorized keys");
}

fn spawn(root: &Path, config: &Path) -> Child {
    let log = fs::File::create(root.join("sshd.log")).expect("private sshd log");
    Command::new("/usr/sbin/sshd")
        .args(["-D", "-e", "-f"])
        .arg(config)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log)
        .process_group(0)
        .spawn()
        .expect("sshd is required when FFM_TEST_SFTP_SERVER=1")
}

impl Drop for Server {
    fn drop(&mut self) {
        if matches!(crate::platform::has_exited(&self.child), Ok(false)) {
            drop(crate::platform::kill_process_group(&self.child));
        }
        drop(self.child.wait());
    }
}

fn keygen(path: &Path) {
    let output = Command::new("ssh-keygen")
        .args([
            "-q", "-t", "ecdsa", "-b", "256", "-m", "PEM", "-N", "", "-f",
        ])
        .arg(path)
        .output()
        .expect("ssh-keygen is required when FFM_TEST_SFTP_SERVER=1");
    assert!(
        output.status.success(),
        "ssh-keygen failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn command_text(program: &str, args: &[&str]) -> String {
    let output = Command::new(program)
        .args(args)
        .output()
        .expect("fixture command");
    assert!(output.status.success(), "{program} failed");
    String::from_utf8(output.stdout).expect("UTF-8 fixture command output")
}

fn configuration(root: &Path, port: u16, user: &str) -> String {
    let root = root.display();
    format!("Port {port}\nListenAddress 127.0.0.1\nHostKey \"{root}/host\"\nPidFile \"{root}/sshd.pid\"\nAuthorizedKeysFile \"{root}/authorized_keys\"\nAllowUsers {user}\nStrictModes no\nPermitRootLogin prohibit-password\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nPubkeyAuthentication yes\nUsePAM no\nAllowTcpForwarding no\nAllowAgentForwarding no\nX11Forwarding no\nPermitTTY no\nPermitUserEnvironment no\nUseDNS no\nSubsystem sftp internal-sftp\nForceCommand internal-sftp\nLogLevel ERROR\n")
}
