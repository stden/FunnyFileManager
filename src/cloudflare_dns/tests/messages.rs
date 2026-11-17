//! Purpose: Every failure has a message the dialog can show, and a failed write is the answer of
//! that write.

use super::*;
use crate::values::cloudflare_dns::{Change, NewRecord, Proxy, Ttl, Zone, ZoneId};

fn create() -> Change {
    Change::Create(NewRecord {
        zone: Zone {
            id: ZoneId::new(ZONE_ID),
            name: "example.com".to_owned(),
        },
        name: "www".to_owned(),
        content: "203.0.113.5".to_owned(),
        ttl: Ttl::Seconds(300),
        proxy: Proxy::Off,
    })
}

#[test]
fn every_failure_has_a_message_the_dialog_can_show() {
    assert_eq!(
        CloudflareError::AccessDenied.message(),
        "Access denied: check the API token's permissions"
    );
    assert_eq!(
        CloudflareError::Status(503).message(),
        "Cloudflare answered HTTP 503"
    );
    assert_eq!(
        CloudflareError::InvalidResponse.message(),
        "Unsupported Cloudflare response"
    );
    assert_eq!(
        CloudflareError::Limit.message(),
        "Cloudflare response exceeded its page or size limit"
    );
    assert_eq!(
        CloudflareError::HostName.message(),
        "The hostname could not be read"
    );
}

#[test]
fn a_failed_write_is_the_answer_of_the_write() {
    let reply = Request::Write(create()).fail(CloudflareError::Network);
    assert!(matches!(
        reply,
        Reply::Written(Err(CloudflareError::Network))
    ));
}
