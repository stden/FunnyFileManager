//! Unit tests for trace body read errors in `host::fetch_trace`.

use super::*;
use std::io::Write;
use std::net::TcpListener;

struct TraceUrlGuard;

impl Drop for TraceUrlGuard {
    fn drop(&mut self) {
        TEST_TRACE_URL.with(|cell| *cell.borrow_mut() = None);
    }
}

#[test]
fn truncated_trace_body_reports_network_error() {
    let _guard = TraceUrlGuard;
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let address = listener.local_addr().expect("address");
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("connection");
        let mut head = Vec::new();
        let mut byte = [0_u8; 1];
        while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap_or(0) > 0 {
            head.push(byte[0]);
        }
        drop(
            stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\nip=1",
            ),
        );
    });
    let url = format!("http://{address}/cdn-cgi/trace");
    TEST_TRACE_URL.with(|cell| *cell.borrow_mut() = Some(url));
    assert_eq!(fetch_trace(), Err(CloudflareError::Network));
}
