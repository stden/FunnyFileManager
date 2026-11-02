//! Purpose: Provider report decoding and checked accumulation without double-counting cached tokens.
use crate::models::token_usage::{Provider, Tokens, UsageError, UsageTotals};
use serde_json::Value;

pub(super) struct Page {
    pub totals: UsageTotals,
    pub next: Option<String>,
}
pub(super) fn parse(provider: Provider, body: &Value) -> Result<Page, UsageError> {
    let mut totals = UsageTotals {
        cached: (provider != Provider::OpenRouter).then_some(Tokens::new(0)),
        ..UsageTotals::default()
    };
    for row in array(body, "data")? {
        if provider == Provider::OpenAi {
            for result in array(row, "results")? {
                merge(&mut totals, &openai(result)?)?;
            }
        } else {
            let value = match provider {
                Provider::Anthropic => {
                    anthropic(row.get("usage").ok_or(UsageError::InvalidResponse)?)?
                }
                Provider::OpenRouter => router(row)?,
                Provider::OpenAi => return Err(UsageError::InvalidResponse),
            };
            merge(&mut totals, &value)?;
        }
    }
    let next = if provider == Provider::OpenRouter {
        None
    } else {
        next(body)?
    };
    Ok(Page { totals, next })
}
fn next(body: &Value) -> Result<Option<String>, UsageError> {
    match body.get("has_more").and_then(Value::as_bool) {
        Some(false) => Ok(None),
        Some(true) => body
            .get("next_page")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(|text| Some(text.to_owned()))
            .ok_or(UsageError::InvalidResponse),
        None => Err(UsageError::InvalidResponse),
    }
}
fn array<'a>(body: &'a Value, name: &str) -> Result<&'a Vec<Value>, UsageError> {
    body.get(name)
        .and_then(Value::as_array)
        .ok_or(UsageError::InvalidResponse)
}
fn number(body: &Value, name: &str) -> Result<u64, UsageError> {
    body.get(name)
        .and_then(Value::as_u64)
        .ok_or(UsageError::InvalidResponse)
}
fn openai(row: &Value) -> Result<UsageTotals, UsageError> {
    Ok(UsageTotals {
        input: Tokens::new(number(row, "input_tokens")?),
        output: Tokens::new(number(row, "output_tokens")?),
        cached: Some(Tokens::new(number(row, "input_cached_tokens")?)),
        requests: number(row, "num_model_requests")?,
    })
}
fn anthropic(row: &Value) -> Result<UsageTotals, UsageError> {
    let cache = row
        .get("cache_creation")
        .ok_or(UsageError::InvalidResponse)?;
    let read = Tokens::new(number(row, "cache_read_input_tokens")?);
    let input = Tokens::new(number(row, "uncached_input_tokens")?)
        .add(read)
        .and_then(|count| {
            count.add(Tokens::new(
                number(cache, "ephemeral_1h_input_tokens").ok()?,
            ))
        })
        .and_then(|count| {
            count.add(Tokens::new(
                number(cache, "ephemeral_5m_input_tokens").ok()?,
            ))
        })
        .ok_or(UsageError::InvalidResponse)?;
    Ok(UsageTotals {
        input,
        output: Tokens::new(number(row, "output_tokens")?),
        cached: Some(read),
        requests: number(row, "requests")?,
    })
}
fn router(row: &Value) -> Result<UsageTotals, UsageError> {
    Ok(UsageTotals {
        input: Tokens::new(number(row, "prompt_tokens")?),
        output: Tokens::new(number(row, "completion_tokens")?),
        cached: None,
        requests: number(row, "requests")?,
    })
}
pub(super) fn merge(total: &mut UsageTotals, value: &UsageTotals) -> Result<(), UsageError> {
    total.input = total.input.add(value.input).ok_or(UsageError::Limit)?;
    total.output = total.output.add(value.output).ok_or(UsageError::Limit)?;
    total.requests = total
        .requests
        .checked_add(value.requests)
        .ok_or(UsageError::Limit)?;
    total.cached = match (total.cached, value.cached) {
        (Some(a), Some(b)) => Some(a.add(b).ok_or(UsageError::Limit)?),
        (None, cached) | (cached, None) => cached,
    };
    Ok(())
}
