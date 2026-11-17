//! Purpose: Bound protocol waits and always reap the owned automation subprocess.
#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::redundant_pub_crate,
    clippy::needless_pass_by_value,
    reason = "integration-test helper: panics on fixture failure are correct behavior for test infrastructure"
)]

use std::{
    io::Write,
    path::Path,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::Receiver,
    thread::JoinHandle,
    time::{Duration, Instant},
};

use serde_json::{json, Value};

#[path = "session/transport.rs"]
mod transport;

pub(super) struct Session {
    child: Child,
    input: Option<ChildStdin>,
    replies: Option<Receiver<String>>,
    reader: Option<JoinHandle<()>>,
    pub actions: Value,
}

impl Session {
    pub fn start(root: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ffm"))
            .args(["--automation", "--root"])
            .arg(root)
            .args(["--isolated", "--width", "80", "--height", "24"])
            .env("FFM_LOG", "0")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let (replies, reader) = transport::reader(child.stdout.take().unwrap());
        let mut session = Self {
            child,
            input,
            replies: Some(replies),
            reader: Some(reader),
            actions: Value::Null,
        };
        let hello = session.receive();
        assert_eq!(hello["ok"], true, "{hello}");
        assert_eq!(hello["result"]["protocol_version"], 1);
        assert_eq!(hello["result"]["state"]["screen"]["width"], 80);
        let catalogue = session.request(json!({"operation":"catalog"}));
        assert_eq!(catalogue["ok"], true, "{catalogue}");
        session.actions = catalogue["result"]["actions"].clone();
        session
    }

    pub fn request(&mut self, request: Value) -> Value {
        let input = self.input.as_mut().unwrap();
        writeln!(input, "{request}").unwrap();
        input.flush().unwrap();
        self.receive()
    }

    fn receive(&self) -> Value {
        let line = self
            .replies
            .as_ref()
            .unwrap()
            .recv_timeout(Duration::from_secs(10))
            .expect("automation reply timed out or process exited");
        serde_json::from_str(&line).expect("stdout must contain only JSON protocol frames")
    }

    pub fn perform(&mut self, request: Value) -> Value {
        let response = self.request(request);
        assert_eq!(response["ok"], true, "{response}");
        let state = self.settle(response["result"]["state"].clone());
        assert!(state["job"]["error"].is_null(), "{state}");
        state
    }

    pub fn settle(&mut self, mut state: Value) -> Value {
        let deadline = Instant::now() + Duration::from_secs(5);
        while state["busy"] == true {
            assert!(
                Instant::now() < deadline,
                "automation job failed to finish: {state}"
            );
            let response = self.request(json!({"operation":"wait","timeout_ms":100}));
            assert_eq!(response["ok"], true, "{response}");
            state = response["result"]["state"].clone();
        }
        assert_eq!(
            state["busy"], false,
            "state must explicitly report job completion"
        );
        state
    }

    pub fn state(&mut self) -> Value {
        self.perform(json!({"operation":"state"}))
    }

    pub fn action(&mut self, context: &str, action: &str) -> Value {
        assert!(
            self.actions
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["context"] == context && row["action"] == action),
            "unknown test action {context}/{action}"
        );
        self.perform(json!({"operation":"action","context":context,"action":action}))
    }

    pub fn keys(&mut self, keys: &[&str]) -> Value {
        self.perform(json!({"operation":"input","keys":keys}))
    }

    pub fn text(&mut self, text: &str) -> Value {
        self.perform(json!({"operation":"input","text":text}))
    }

    pub fn select(&mut self, name: &str) -> Value {
        let state = self.state();
        let side = state["panels"]["active"].as_str().unwrap();
        let index = state["panels"][side]["entries"]
            .as_array()
            .unwrap()
            .iter()
            .position(|entry| entry["name"] == name)
            .expect("fixture entry is listed");
        let mut keys = vec!["Home"];
        keys.extend(std::iter::repeat_n("Down", index));
        self.keys(&keys)
    }

    pub fn enter(&mut self, directory: &str) -> Value {
        self.select(directory);
        self.keys(&["Enter"])
    }

    pub fn close(&mut self) {
        let response = self.request(json!({"operation":"close"}));
        assert_eq!(response["ok"], true, "{response}");
        self.eof();
    }

    pub fn eof(&mut self) {
        self.input.take();
        transport::wait_for_exit(&mut self.child);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.input.take();
        // Release a reader blocked on a full channel before joining it.
        self.replies.take();
        drop(self.child.kill());
        drop(self.child.wait());
        if let Some(reader) = self.reader.take() {
            drop(reader.join());
        }
    }
}
