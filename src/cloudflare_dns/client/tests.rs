//! Unit tests for HTTP body read errors in `Client`.

use super::*;
use crate::cloudflare_dns::call::zones_page;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

fn serve_truncated_body() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let address = listener.local_addr().expect("address");
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("connection");
        BufReader::new(&mut stream)
            .lines()
            .map_while(Result::ok)
            .take_while(|line| !line.is_empty())
            .for_each(drop);
        drop(stream.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\npartial",
        ));
    });
    format!("http://{address}/client/v4")
}

#[test]
fn truncated_response_body_maps_to_network_error() {
    let base = serve_truncated_body();
    let token = Token::from_value(Some("test-token-000".to_owned())).expect("token");
    let mut client = Client::with_base(token, &base).expect("client");
    assert_eq!(client.send(&zones_page(1)), Err(CloudflareError::Network));
}
