//! Purpose: Stub AI transport, response fixtures, and delay gate for AI assistant BDD.

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::ai::client::AiTransport;
use crate::ai::config::resolve_with;
use crate::ai::execute_with;
use crate::application::service_jobs::ServiceJobs;
use crate::application::tests::bdd::common::{await_gate, DelayGate};
use crate::values::ai::{AiError, AiProvider, AiRequest, AiResponse};

#[derive(Clone, Copy)]
pub enum Mode {
    Success,
    MissingKey,
    RateLimit,
    InvalidJson,
}

pub struct Fixture {
    pub jobs: ServiceJobs,
    pub mode: Mode,
    pub endpoints: Arc<Mutex<Vec<String>>>,
    pub gate: DelayGate,
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            jobs: ServiceJobs::default(),
            mode: Mode::Success,
            endpoints: Arc::new(Mutex::new(Vec::new())),
            gate: DelayGate::default(),
        }
    }
}

impl Fixture {
    pub(super) fn failing(kind: &str) -> Self {
        let mode = match kind {
            "missing key" => Mode::MissingKey,
            "rate limit" => Mode::RateLimit,
            "invalid response" => Mode::InvalidJson,
            other => panic!("unknown AI failure {other}"),
        };
        Self {
            mode,
            ..Self::default()
        }
    }

    pub(super) fn delayed() -> Self {
        Self {
            gate: DelayGate::delayed(),
            ..Self::default()
        }
    }

    pub(super) fn work(
        &mut self,
    ) -> impl FnOnce(AiRequest) -> Result<AiResponse, AiError> + Send + 'static {
        let mode = self.mode;
        let endpoints = Arc::clone(&self.endpoints);
        let blocker = self.gate.blocker();
        move |req| {
            await_gate(blocker);
            run_request(mode, &endpoints, &req)
        }
    }
}

struct StubTransport {
    mode: Mode,
    provider: AiProvider,
    endpoints: Arc<Mutex<Vec<String>>>,
}

impl AiTransport for StubTransport {
    fn post(&mut self, path: &str, _body: &Value) -> Result<Value, AiError> {
        self.endpoints
            .lock()
            .expect("endpoints")
            .push(path.to_owned());
        match self.mode {
            Mode::RateLimit => Err(AiError::Http(429, "too many requests".into())),
            Mode::InvalidJson => Ok(json!({})),
            Mode::Success | Mode::MissingKey => Ok(payload_for(self.provider)),
        }
    }
}

fn run_request(
    mode: Mode,
    endpoints: &Arc<Mutex<Vec<String>>>,
    req: &AiRequest,
) -> Result<AiResponse, AiError> {
    let resolved = resolve_with(req.provider, &req.settings, |name| {
        if matches!(mode, Mode::MissingKey) {
            return None;
        }
        match name {
            "ANTHROPIC_API_KEY" => Some("fixture-anthropic".into()),
            "OPENAI_API_KEY" => Some("fixture-openai".into()),
            "CODEX_API_KEY" => Some("fixture-codex".into()),
            _ => None,
        }
    })?;
    let mut transport = StubTransport {
        mode,
        provider: req.provider,
        endpoints: Arc::clone(endpoints),
    };
    execute_with(&mut transport, &resolved, &req.messages, &req.context)
}

fn payload_for(provider: AiProvider) -> Value {
    match provider {
        AiProvider::Anthropic => json!({
            "model": "claude-sonnet-4-5",
            "content": [{"type": "text", "text": "Run:\n```bash\nls -la\n```"}],
            "usage": {"input_tokens": 12, "output_tokens": 6}
        }),
        AiProvider::OpenAi => json!({
            "model": "gpt-5",
            "choices": [{"message": {"content": "```bash\nrg --files\n```"}}],
            "usage": {"prompt_tokens": 11, "completion_tokens": 5}
        }),
        AiProvider::Codex => json!({
            "model": "gpt-5.3-codex",
            "output_text": "```sh\nfind . -name '*.rs'\n```",
            "usage": {"input_tokens": 9, "output_tokens": 4}
        }),
    }
}
