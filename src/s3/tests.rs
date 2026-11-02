//! Unit tests and shared test helpers for `s3`.

use super::*;

use aws_sdk_s3::config::{Credentials, Region, SharedCredentialsProvider};
use aws_smithy_http_client::test_util::{ReplayEvent, StaticReplayClient};
use aws_smithy_types::body::SdkBody;
use http_1x::{Request, Response};
use std::env;
use std::path::Path;
use std::sync::MutexGuard;

mod client;
mod download;
mod list_buckets;
mod list_objects;
mod list_objects_prefix;
mod profiles;
mod transfers;
mod value_types;
mod virtual_path_display;
mod virtual_path_parse;

pub fn build_test_client(events: Vec<ReplayEvent>) -> S3Client {
    let http_client = StaticReplayClient::new(events);
    let config = aws_sdk_s3::Config::builder()
        .credentials_provider(SharedCredentialsProvider::new(Credentials::for_tests()))
        .region(Region::new("us-east-1"))
        .http_client(http_client)
        .behavior_version_latest()
        .build();
    let client = Client::from_conf(config);
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    S3Client {
        client,
        runtime,
        profile: "test".to_owned(),
        current_bucket: None,
        current_prefix: String::new(),
    }
}

/// Sandboxes `$HOME` / `%USERPROFILE%` under the crate-wide env lock so S3
/// tests cannot race command/history/editor tests that also mutate the process
/// environment.
pub struct AwsHomeEnvGuard {
    _lock: MutexGuard<'static, ()>,
    original_home: Option<String>,
    original_userprofile: Option<String>,
}

impl AwsHomeEnvGuard {
    pub(crate) fn set(path: &Path) -> Self {
        let lock = crate::test_support::env_lock();
        let original_home = env::var("HOME").ok();
        let original_userprofile = env::var("USERPROFILE").ok();
        env::set_var("HOME", path);
        env::set_var("USERPROFILE", path);
        Self {
            _lock: lock,
            original_home,
            original_userprofile,
        }
    }
}

impl Drop for AwsHomeEnvGuard {
    fn drop(&mut self) {
        if let Some(home) = &self.original_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }

        if let Some(userprofile) = &self.original_userprofile {
            env::set_var("USERPROFILE", userprofile);
        } else {
            env::remove_var("USERPROFILE");
        }
    }
}

pub fn replay_event(xml_body: &str) -> ReplayEvent {
    ReplayEvent::new(
        Request::builder()
            .uri("https://s3.us-east-1.amazonaws.com/")
            .body(SdkBody::empty())
            .expect("request"),
        Response::builder()
            .status(200)
            .body(SdkBody::from(xml_body))
            .expect("response"),
    )
}

pub fn ok_event() -> ReplayEvent {
    ReplayEvent::new(
        Request::builder()
            .uri("https://s3.us-east-1.amazonaws.com/")
            .body(SdkBody::empty())
            .expect("request"),
        Response::builder()
            .status(200)
            .body(SdkBody::empty())
            .expect("response"),
    )
}

pub fn error_event(status: u16) -> ReplayEvent {
    ReplayEvent::new(
        Request::builder()
            .uri("https://s3.us-east-1.amazonaws.com/")
            .body(SdkBody::empty())
            .expect("request"),
        Response::builder()
            .status(status)
            .body(SdkBody::from(
                "<Error><Code>AccessDenied</Code><Message>Access Denied</Message></Error>",
            ))
            .expect("response"),
    )
}
