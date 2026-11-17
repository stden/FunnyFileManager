//! Purpose: Real provider response shapes pass through canonical pagination and aggregation.

use crate::models::token_usage::{Provider, ProviderUsage, UsageError};
use crate::token_usage::UsageHttp;
use serde_json::{json, Value};
use std::collections::VecDeque;

#[derive(Clone, Copy)]
pub(super) enum Edition {
    Original,
    Refreshed,
}
#[derive(Clone, Copy)]
pub(super) enum Reply {
    Success(Edition),
    Failure(UsageError),
}

pub(super) fn reports(reply: Reply) -> Vec<ProviderUsage> {
    Provider::ALL
        .into_iter()
        .map(|provider| {
            let result = match reply {
                Reply::Failure(error) => Err(error),
                Reply::Success(edition) => {
                    crate::token_usage::report(&mut Pages::new(provider, edition), provider)
                }
            };
            ProviderUsage { provider, result }
        })
        .collect()
}

struct Pages {
    provider: Provider,
    pages: VecDeque<(Option<&'static str>, Value)>,
}

impl Pages {
    fn new(provider: Provider, edition: Edition) -> Self {
        let pages = match provider {
            Provider::OpenAi => openai(edition),
            Provider::Anthropic => vec![(None, anthropic(edition))],
            Provider::OpenRouter => vec![(None, router(edition))],
        };
        Self {
            provider,
            pages: pages.into(),
        }
    }
}

impl UsageHttp for Pages {
    fn get(&mut self, provider: Provider, page: Option<&str>) -> Result<Value, UsageError> {
        assert_eq!(
            provider, self.provider,
            "the report requests its own provider"
        );
        let (expected, body) = self.pages.pop_front().expect("fixture page exists");
        assert_eq!(page, expected, "the report follows the provider cursor");
        Ok(body)
    }
}

fn openai(edition: Edition) -> Vec<(Option<&'static str>, Value)> {
    match edition {
        Edition::Original => vec![
            (
                None,
                json!({"data":[{"results":[{"input_tokens":70,"output_tokens":15,"input_cached_tokens":25,"num_model_requests":1}]}],"has_more":true,"next_page":"second-page"}),
            ),
            (
                Some("second-page"),
                json!({"data":[{"results":[{"input_tokens":30,"output_tokens":5,"input_cached_tokens":15,"num_model_requests":1}]}],"has_more":false}),
            ),
        ],
        Edition::Refreshed => vec![(
            None,
            json!({"data":[{"results":[{"input_tokens":200,"output_tokens":60,"input_cached_tokens":80,"num_model_requests":4}]}],"has_more":false}),
        )],
    }
}

fn anthropic(edition: Edition) -> Value {
    let scale = match edition {
        Edition::Original => 1,
        Edition::Refreshed => 2,
    };
    json!({"data":[{"usage":{"uncached_input_tokens":100 * scale,"cache_read_input_tokens":40 * scale,
        "cache_creation":{"ephemeral_1h_input_tokens":20 * scale,"ephemeral_5m_input_tokens":30 * scale},
        "output_tokens":50 * scale,"requests":3 * scale}}],"has_more":false})
}

fn router(edition: Edition) -> Value {
    let scale = match edition {
        Edition::Original => 1,
        Edition::Refreshed => 3,
    };
    json!({"data":[{"prompt_tokens":100 * scale,"completion_tokens":50 * scale,"reasoning_tokens":20 * scale,"requests":2 * scale}]})
}
