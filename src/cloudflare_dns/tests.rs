//! Purpose: Cloudflare DNS client behaviour, checked without the network: decoding, pagination,
//! validation, zone choice, host detection, redaction, and the requests the client sends.

use super::*;
use crate::values::cloudflare_dns::CloudflareError;
use serde_json::Value;
use std::collections::VecDeque;

mod decoding;
mod host_detection;
mod http;
mod http_writes;
mod messages;
mod pagination;
mod paths;
mod requests;
mod token_environment;
mod token_redaction;
mod validation;
mod writes;
mod zone_choice;

/// A transport that answers from a queue and keeps every call it was sent.
#[derive(Default)]
pub(super) struct Fake {
    answers: VecDeque<Result<Value, CloudflareError>>,
    pub(super) calls: Vec<Call>,
}

impl Fake {
    pub(super) fn answering(
        answers: impl IntoIterator<Item = Result<Value, CloudflareError>>,
    ) -> Self {
        Self {
            answers: answers.into_iter().collect(),
            calls: Vec::new(),
        }
    }
}

impl Transport for Fake {
    fn send(&mut self, call: &Call) -> Result<Value, CloudflareError> {
        self.calls.push(call.clone());
        self.answers
            .pop_front()
            .unwrap_or(Err(CloudflareError::InvalidResponse))
    }
}

pub(super) const ZONE_ID: &str = "0000000000000000000000000000000a";
pub(super) const RECORD_ID: &str = "0000000000000000000000000000000b";
