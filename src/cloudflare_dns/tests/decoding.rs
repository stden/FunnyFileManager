//! Purpose: API answers decode into zones and records, and an error envelope keeps only its first
//! error, sanitised.

use super::*;
use crate::models::cloudflare_dns::{Proxy, Ttl};
use serde_json::json;

#[test]
fn zones_decode_from_the_result_list() {
    let body = json!({
        "success": true,
        "errors": [],
        "result": [{"id": ZONE_ID, "name": "example.com", "status": "active"}],
        "result_info": {"page": 1, "total_pages": 1}
    });
    let zones = codec::zones(&body).expect("zones");
    assert_eq!(zones.len(), 1);
    assert_eq!(zones[0].name, "example.com");
    assert_eq!(zones[0].id.as_str(), ZONE_ID);
}

#[test]
fn records_decode_type_ttl_and_proxy_and_a_missing_proxy_is_unsupported() {
    let body = json!({
        "success": true,
        "result": [
            {"id": RECORD_ID, "type": "A", "name": "www.example.com",
             "content": "203.0.113.5", "ttl": 1, "proxied": true},
            {"id": "0000000000000000000000000000000c", "type": "TXT",
             "name": "example.com", "content": "v=spf1 -all", "ttl": 300},
        ],
        "result_info": {"page": 1, "total_pages": 1}
    });
    let records = codec::records(&body).expect("records");
    assert_eq!(records[0].kind, "A");
    assert_eq!(records[0].ttl, Ttl::Automatic);
    assert_eq!(records[0].proxy, Proxy::On);
    assert_eq!(records[1].ttl, Ttl::Seconds(300));
    assert_eq!(records[1].proxy, Proxy::Unsupported);
}

#[test]
fn a_create_or_update_answer_decodes_its_one_record() {
    let body = json!({
        "success": true,
        "result": {"id": RECORD_ID, "type": "A", "name": "host.example.com",
                   "content": "203.0.113.9", "ttl": 300, "proxied": false}
    });
    let record = codec::record(&body).expect("record");
    assert_eq!(record.content, "203.0.113.9");
    assert_eq!(record.proxy, Proxy::Off);
}

#[test]
fn a_record_without_its_content_is_an_invalid_response() {
    let body = json!({
        "success": true,
        "result": {"id": RECORD_ID, "type": "A", "name": "host.example.com", "ttl": 1}
    });
    assert_eq!(codec::record(&body), Err(CloudflareError::InvalidResponse));
}

#[test]
fn the_page_count_comes_from_result_info() {
    assert_eq!(
        codec::total_pages(&json!({"result_info": {"page": 1, "total_pages": 3}})),
        Ok(3)
    );
    assert_eq!(
        codec::total_pages(&json!({"result": []})),
        Err(CloudflareError::InvalidResponse)
    );
}

#[test]
fn an_error_envelope_reports_its_first_error_code_and_message() {
    let body = json!({
        "success": false,
        "errors": [{"code": 9109, "message": "Invalid access token"},
                   {"code": 1, "message": "second"}],
        "result": null
    });
    assert_eq!(
        codec::error_envelope(&body),
        Some(CloudflareError::Api {
            code: 9109,
            message: "Invalid access token".to_owned(),
        })
    );
    assert_eq!(
        codec::error_envelope(&json!({"success": true, "result": []})),
        None
    );
    assert_eq!(
        codec::error_envelope(&json!({"success": false})),
        Some(CloudflareError::InvalidResponse),
        "an envelope without errors says nothing useful"
    );
}

#[test]
fn an_error_message_is_stripped_of_control_characters_and_capped() {
    let text = format!("bad\u{1b}[31m{}", "x".repeat(400));
    let body = json!({"success": false, "errors": [{"code": 7003, "message": text}]});
    let Some(CloudflareError::Api { code, message }) = codec::error_envelope(&body) else {
        panic!("an API error");
    };
    assert_eq!(code, 7003);
    assert!(
        !message.contains('\u{1b}'),
        "no escape sequence reaches the screen"
    );
    assert!(message.chars().count() <= 200);
}
