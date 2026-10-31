//! Purpose: HTTP headers redact API keys and request periods remain stable across pages.
use super::*;
mod connection;
#[test]
fn credentials_are_sensitive_and_anthropic_uses_analytics_headers() {
    let key = ApiKey("fixture-secret".to_owned().into());
    for provider in Provider::ALL {
        let values = headers(provider, &key).expect("headers");
        let name = if provider == Provider::Anthropic {
            "x-api-key"
        } else {
            "authorization"
        };
        assert!(values.get(name).expect("credential").is_sensitive());
    }
    let values = headers(Provider::Anthropic, &key).expect("headers");
    assert_eq!(
        values.get("anthropic-version").expect("version"),
        "2023-06-01"
    );
    assert!(values.contains_key("anthropic-beta"));
}
#[test]
fn request_pages_keep_the_same_reporting_window_and_escape_cursors() {
    let end = chrono::DateTime::parse_from_rfc3339("2026-10-31T00:00:00Z")
        .expect("date")
        .to_utc();
    for provider in Provider::ALL {
        let (url, first) = request(provider, None, end);
        let (next_url, mut second) = request(provider, Some("page+/&"), end);
        assert_eq!(url, next_url);
        assert_eq!(second.pop(), Some(("page", "page+/&".to_owned())));
        assert_eq!(first, second);
        assert!(url.starts_with("https://"));
    }
}

#[test]
fn invalid_key_cannot_be_sent_as_an_http_header() {
    let key = ApiKey("bad\nheader".to_owned().into());
    for provider in Provider::ALL {
        assert!(matches!(
            headers(provider, &key),
            Err(UsageError::AccessDenied)
        ));
    }
}

#[test]
fn client_rejects_wrong_provider_and_expired_deadline_without_network() {
    let mut client = Client {
        inner: HttpClient::new(),
        end: Utc::now(),
        provider: Provider::OpenAi,
        key: ApiKey("fixture".to_owned().into()),
        deadline: Instant::now()
            .checked_sub(Duration::from_secs(1))
            .expect("deadline"),
    };
    assert_eq!(
        client.get(Provider::Anthropic, None),
        Err(UsageError::InvalidResponse)
    );
    assert_eq!(client.get(Provider::OpenAi, None), Err(UsageError::Limit));
    client.deadline = Instant::now() + Duration::from_secs(5);
    client.key = ApiKey("bad\nheader".to_owned().into());
    assert_eq!(
        client.get(Provider::OpenAi, None),
        Err(UsageError::AccessDenied)
    );
}

#[test]
fn constructor_uses_environment_key_without_requesting_network() {
    let _key =
        crate::test_support::EnvVarGuard::set("OPENAI_ADMIN_KEY", std::path::Path::new("fixture"));
    let client = Client::new(Provider::OpenAi).expect("client");
    assert_eq!(client.provider, Provider::OpenAi);
    assert_eq!(client.key.0.as_str(), "fixture");
    assert!(client.deadline > Instant::now());
}

#[test]
fn constructor_rejects_blank_environment_key() {
    let _key = crate::test_support::EnvVarGuard::set("OPENAI_ADMIN_KEY", std::path::Path::new(""));
    assert!(matches!(
        Client::new(Provider::OpenAi),
        Err(UsageError::MissingKey)
    ));
}

#[test]
fn responses_map_status_and_decode_json() {
    for (status, error) in [
        (401, UsageError::AccessDenied),
        (403, UsageError::AccessDenied),
        (429, UsageError::RateLimited),
        (500, UsageError::Network),
    ] {
        assert_eq!(read_response(response(status, b"{}")), Err(error));
    }
    assert_eq!(
        read_response(response(200, br#"{"data":[]}"#)),
        Ok(serde_json::json!({"data":[]}))
    );
    assert_eq!(
        read_response(response(200, b"not json")),
        Err(UsageError::InvalidResponse)
    );
}

#[test]
fn oversized_response_is_rejected_before_decoding() {
    let body = vec![b' '; 2 * 1024 * 1024 + 1];
    assert_eq!(read_response(response(200, &body)), Err(UsageError::Limit));
}

fn response(status: u16, body: &[u8]) -> reqwest::blocking::Response {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let address = listener.local_addr().expect("address");
    let body = body.to_vec();
    std::thread::spawn(move || serve(listener, status, &body));
    HttpClient::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client")
        .get(format!("http://{address}"))
        .send()
        .expect("local response")
}

fn serve(listener: std::net::TcpListener, status: u16, body: &[u8]) {
    use std::io::Write;
    let (mut stream, _) = listener.accept().expect("connection");
    read_request(&mut stream);
    write!(
        stream,
        "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .expect("headers");
    stream.write_all(body).expect("body");
}

fn read_request(stream: &mut std::net::TcpStream) {
    use std::io::{BufRead, BufReader};
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).expect("request");
        if line == "\r\n" || line.is_empty() {
            break;
        }
    }
}
