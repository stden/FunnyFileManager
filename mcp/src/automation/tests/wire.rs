//! Purpose: Refuse malformed or oversized frames and interrupted exchanges.

use super::*;

#[tokio::test]
async fn distinct_json_lines_remain_distinct_responses() {
    let bytes = b"{\"one\":1}\n{\"two\":2}\n";
    let mut reader = BufReader::new(bytes.as_slice());
    assert_eq!(
        protocol::read_frame(&mut reader).await.unwrap(),
        json!({"one":1})
    );
    assert_eq!(
        protocol::read_frame(&mut reader).await.unwrap(),
        json!({"two":2})
    );
    assert!(protocol::read_frame(&mut reader).await.is_err());
}

#[tokio::test]
async fn an_unterminated_response_cannot_grow_beyond_the_frame_limit() {
    let bytes = vec![b'x'; protocol::MAX_FRAME_BYTES + 1];
    let mut reader = BufReader::new(bytes.as_slice());
    assert!(protocol::read_frame(&mut reader)
        .await
        .unwrap_err()
        .to_string()
        .contains("8 MiB"));
}

#[test]
fn incomplete_envelopes_are_transport_errors() {
    for frame in [
        json!({}),
        json!({"ok":true}),
        json!({"ok":false}),
        json!({"ok":false,"error":{"code":"busy"}}),
    ] {
        assert!(protocol::result(&frame).unwrap_err().is_fatal());
    }
}

#[tokio::test]
async fn cancelling_a_request_never_reuses_its_unread_response() {
    let (client, _peer) = tokio::io::duplex(4096);
    let (read, write) = tokio::io::split(client);
    let mut connection = connection::Connection::from_io(read, write);
    assert!(tokio::time::timeout(
        std::time::Duration::from_millis(1),
        connection.exchange(&Request::State)
    )
    .await
    .is_err());
    let error = connection.exchange(&Request::State).await.unwrap_err();
    drop(connection);
    assert!(error.to_string().contains("interrupted"));
}

#[tokio::test]
async fn oversized_encoded_input_is_refused_without_losing_the_session() {
    let reply: &'static [u8] = b"{\"ok\":true,\"result\":{}}\n";
    let mut connection = connection::Connection::from_io(reply, tokio::io::sink());
    let request = Request::Input {
        keys: Vec::new(),
        text: Some("\0".repeat(1024 * 1024)),
        mouse: None,
    };
    assert!(matches!(
        connection.exchange(&request).await,
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        connection.exchange(&Request::State).await.unwrap(),
        json!({})
    );
    drop(connection);
}

#[tokio::test]
async fn the_request_limit_includes_its_trailing_newline() {
    let reply: &'static [u8] = b"{\"ok\":true,\"result\":{\"accepted\":true}}\n{\"ok\":true,\"result\":{\"state\":true}}\n";
    let mut connection = connection::Connection::from_io(reply, tokio::io::sink());
    let too_large = encoded_input(protocol::MAX_REQUEST_BYTES);
    assert!(matches!(
        connection.exchange(&too_large).await,
        Err(Error::Invalid(_))
    ));
    let fits = encoded_input(protocol::MAX_REQUEST_BYTES - 1);
    assert_eq!(
        connection.exchange(&fits).await.unwrap(),
        json!({"accepted":true})
    );
    assert_eq!(
        connection.exchange(&Request::State).await.unwrap(),
        json!({"state":true})
    );
    drop(connection);
}

fn encoded_input(bytes: usize) -> Request {
    let empty = Request::Input {
        keys: Vec::new(),
        text: Some(String::new()),
        mouse: None,
    };
    let available = bytes - serde_json::to_vec(&empty).unwrap().len();
    let text = format!(
        "{}{}",
        "\0".repeat(available / 6),
        "x".repeat(available % 6)
    );
    assert!(
        text.len() <= 1024 * 1024,
        "raw paste remains within the public input bound"
    );
    let request = Request::Input {
        keys: Vec::new(),
        text: Some(text),
        mouse: None,
    };
    assert_eq!(serde_json::to_vec(&request).unwrap().len(), bytes);
    request
}
