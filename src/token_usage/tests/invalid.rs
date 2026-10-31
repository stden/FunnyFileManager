//! Purpose: Reject malformed and unbounded usage reports without returning partial totals.
use super::*;

#[test]
fn page_limit_and_oversized_cursor_reject_partial_report() {
    let pages =
        (0..100).map(|index| json!({"data": [], "has_more": true, "next_page": index.to_string()}));
    assert_eq!(
        report(&mut Pages(pages), Provider::OpenAi),
        Err(UsageError::Limit)
    );
    let body = json!({"data": [], "has_more": true, "next_page": "a".repeat(2049)});
    assert_eq!(
        report(&mut Pages(vec![body].into_iter()), Provider::OpenAi),
        Err(UsageError::Limit)
    );
}

#[test]
fn malformed_page_metadata_is_rejected() {
    for body in [
        json!({"data": null, "has_more": false}),
        json!({"data": [], "has_more": null}),
        json!({"data": [], "has_more": true, "next_page": ""}),
        json!({"data": [{"results": null}], "has_more": false}),
    ] {
        assert_eq!(
            codec::parse(Provider::OpenAi, &body).err(),
            Some(UsageError::InvalidResponse)
        );
    }
}

#[test]
fn malformed_anthropic_cache_counts_and_router_rows_are_rejected() {
    let anthropic = json!({"data": [{"usage": {
        "uncached_input_tokens": 1,
        "cache_read_input_tokens": 1,
        "cache_creation": {"ephemeral_1h_input_tokens": 1},
        "output_tokens": 1, "requests": 1
    }}], "has_more": false});
    assert_eq!(
        codec::parse(Provider::Anthropic, &anthropic).err(),
        Some(UsageError::InvalidResponse)
    );
    let router = json!({"data": [{"prompt_tokens": 1, "completion_tokens": -1, "requests": 1}]});
    assert_eq!(
        codec::parse(Provider::OpenRouter, &router).err(),
        Some(UsageError::InvalidResponse)
    );
}

#[test]
fn overflow_in_any_aggregate_is_rejected() {
    for field in [
        "input_tokens",
        "output_tokens",
        "input_cached_tokens",
        "num_model_requests",
    ] {
        let mut first = json!({"input_tokens": 0, "output_tokens": 0, "input_cached_tokens": 0, "num_model_requests": 0});
        let mut second = first.clone();
        first[field] = json!(u64::MAX);
        second[field] = json!(1);
        let body = json!({"data": [{"results": [first, second]}], "has_more": false});
        assert_eq!(
            codec::parse(Provider::OpenAi, &body).err(),
            Some(UsageError::Limit),
            "{field}"
        );
    }
}
