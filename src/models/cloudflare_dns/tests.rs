//! Purpose: The dialog's state: confirmation text, the form's fields, and the cursors and the single
//! outstanding request of the panel. Fixtures are shared here; the topics live in the children.

use super::*;

mod diffs;
mod form_fields;
mod panel_state;
mod selection;

pub(super) fn zone(id: &str, name: &str) -> Zone {
    Zone {
        id: ZoneId::new(id),
        name: name.to_owned(),
    }
}

pub(super) fn record(id: &str, kind: &str, proxy: Proxy) -> Record {
    Record {
        id: RecordId::new(id),
        kind: kind.to_owned(),
        name: "www.example.com".to_owned(),
        content: "203.0.113.5".to_owned(),
        ttl: Ttl::Seconds(300),
        proxy,
    }
}

pub(super) fn update(new_content: &str, new_ttl: Ttl, new_proxy: Proxy) -> RecordUpdate {
    RecordUpdate {
        zone: zone("0000000000000000000000000000000a", "example.com"),
        record: record("0000000000000000000000000000000b", "A", Proxy::Off),
        content: new_content.to_owned(),
        ttl: new_ttl,
        proxy: new_proxy,
    }
}

#[test]
fn ttl_and_proxy_labels_are_the_api_words() {
    assert_eq!(Ttl::Automatic.label(), "automatic");
    assert_eq!(Ttl::Seconds(300).label(), "300");
    assert_eq!(Proxy::On.label(), "on");
    assert_eq!(Proxy::Off.label(), "off");
    assert_eq!(Proxy::Unsupported.label(), "n/a");
    assert_eq!(Proxy::Off.toggled(), Proxy::On);
    assert_eq!(Proxy::On.toggled(), Proxy::Off);
    assert_eq!(Proxy::Unsupported.toggled(), Proxy::Unsupported);
}

#[test]
fn error_messages_name_the_variable_and_the_api_code_without_a_body() {
    assert_eq!(
        CloudflareError::MissingToken.message(),
        "set CLOUDFLARE_API_TOKEN"
    );
    let api = CloudflareError::Api {
        code: 9109,
        message: "Invalid access token".to_owned(),
    };
    assert_eq!(api.message(), "Cloudflare error 9109: Invalid access token");
    assert_eq!(
        CloudflareError::NoZone("host.example.org".to_owned()).message(),
        "No zone in this account matches hostname host.example.org"
    );
}
