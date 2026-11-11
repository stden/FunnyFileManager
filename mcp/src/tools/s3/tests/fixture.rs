//! Isolated AWS profile files and a loopback HTTP service, never real AWS.

use std::{future::Future, process::Command};

use rmcp::model::CallToolResult;
use serde_json::Value;
use tempfile::TempDir;

mod http;
pub use http::{response, Request};

pub struct Fixture {
    pub root: TempDir,
    http: http::Server,
}

impl Fixture {
    pub fn new(replies: Vec<Vec<u8>>) -> Self {
        assert_eq!(std::env::var("FFM_S3_HTTP_TEST_CHILD").as_deref(), Ok("1"));
        let root = tempfile::tempdir().unwrap();
        let http = http::Server::new(replies);
        let config = root.path().join("config");
        let credentials = root.path().join("credentials");
        std::fs::write(
            &config,
            format!(
                "[profile fixture]\nregion = us-east-1\nendpoint_url = {}\n",
                http.endpoint()
            ),
        )
        .unwrap();
        std::fs::write(
            &credentials,
            "[fixture]\naws_access_key_id = local\naws_secret_access_key = local\n",
        )
        .unwrap();
        std::env::set_var("AWS_CONFIG_FILE", config);
        std::env::set_var("AWS_SHARED_CREDENTIALS_FILE", credentials);
        std::env::set_var("AWS_ENDPOINT_URL_S3", http.endpoint());
        Self { root, http }
    }

    pub fn server(&self) -> crate::FfmServer {
        crate::FfmServer::with_config(self.root.path().canonicalize().unwrap(), false, false)
    }

    pub fn requests(&self) -> Vec<Request> {
        self.http.requests()
    }
}

pub fn run(check: impl Future<Output = ()>) {
    if std::env::var("FFM_S3_HTTP_TEST_CHILD").as_deref() == Ok("1") {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(check);
        println!("S3 fixture completed");
        return;
    }
    let thread = std::thread::current();
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", thread.name().unwrap(), "--nocapture"])
        .env("FFM_S3_HTTP_TEST_CHILD", "1")
        .env("AWS_MAX_ATTEMPTS", "1")
        .env("AWS_EC2_METADATA_DISABLED", "true")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "isolated S3 test failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("S3 fixture completed"));
}

pub fn json(result: &CallToolResult) -> Value {
    serde_json::from_str(&result.content[0].as_text().unwrap().text).unwrap()
}
