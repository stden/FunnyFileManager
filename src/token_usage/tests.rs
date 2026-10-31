//! Purpose: Provider usage parses real response shapes and paginates without losing or doubling tokens.
use super::*;
use crate::models::token_usage::Tokens;
use serde_json::json;

mod collect;
mod invalid;

#[test]
fn openai_counts_cached_input_once_and_collects_all_pages() {
    let page = json!({"data":[{"results":[{"input_tokens":100,"output_tokens":20,"input_cached_tokens":40,"num_model_requests":2}]}],"has_more":true,"next_page":"next"});
    let mut http = Pages(vec![page, json!({"data":[],"has_more":false})].into_iter());
    let totals = report(&mut http, Provider::OpenAi).expect("report");
    assert_eq!(totals.input.count(), 100);
    assert_eq!(totals.output.count(), 20);
    assert_eq!(totals.cached, Some(Tokens::new(40)));
    assert_eq!(totals.requests, 2);
}
#[test]
fn anthropic_analytics_adds_input_cache_categories() {
    let body = json!({"data":[{"usage":{"uncached_input_tokens":100,"cache_read_input_tokens":40,"cache_creation":{"ephemeral_1h_input_tokens":20,"ephemeral_5m_input_tokens":30},"output_tokens":50,"requests":3}}],"has_more":false});
    let page = codec::parse(Provider::Anthropic, &body).expect("analytics");
    assert_eq!(page.totals.input.count(), 190);
    assert_eq!(page.totals.output.count(), 50);
    assert_eq!(page.totals.cached, Some(Tokens::new(40)));
}
#[test]
fn openrouter_reasoning_tokens_are_already_part_of_output() {
    let body = json!({"data":[{"prompt_tokens":100,"completion_tokens":50,"reasoning_tokens":20,"requests":2}]});
    let page = codec::parse(Provider::OpenRouter, &body).expect("activity");
    assert_eq!(page.totals.output.count(), 50);
    assert_eq!(page.totals.cached, None);
}
struct Pages<I>(I);
impl<I: Iterator<Item = serde_json::Value>> UsageHttp for Pages<I> {
    fn get(
        &mut self,
        _provider: Provider,
        _page: Option<&str>,
    ) -> Result<serde_json::Value, UsageError> {
        self.0.next().ok_or(UsageError::Network)
    }
}
#[test]
fn malformed_and_overflowing_reports_do_not_look_like_zero_usage() {
    assert!(codec::parse(
        Provider::OpenAi,
        &json!({"data":[{"results":[{}]}],"has_more":false})
    )
    .is_err());
    assert!(Tokens::new(u64::MAX).add(Tokens::new(1)).is_none());
}

#[test]
fn repeated_cursor_and_failed_later_page_refuse_partial_totals() {
    let page = json!({"data":[],"has_more":true,"next_page":"same"});
    let mut http = Pages(vec![page.clone(), page].into_iter());
    assert_eq!(report(&mut http, Provider::OpenAi), Err(UsageError::Limit));
    let first = json!({"data":[],"has_more":true,"next_page":"next"});
    assert_eq!(
        report(&mut Pages(vec![first].into_iter()), Provider::OpenAi),
        Err(UsageError::Network)
    );
    assert!(codec::parse(Provider::OpenAi, &json!({"data":[],"has_more":true})).is_err());
}
