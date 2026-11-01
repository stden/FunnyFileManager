//! Purpose: Exercise the bounded HTTP client against loopback only.
use super::*;

#[test]
fn tls_handshake_failure_returns_network_error_without_external_connection() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener");
    let address = listener.local_addr().expect("address");
    listener.set_nonblocking(true).expect("nonblocking");
    let server = std::thread::spawn(move || accept_local(listener));
    let mut client = Client {
        inner: HttpClient::builder()
            .no_proxy()
            .resolve("api.openai.com", address)
            .build()
            .expect("client"),
        end: Utc::now(),
        provider: Provider::OpenAi,
        key: ApiKey("fixture".to_owned().into()),
        deadline: Instant::now() + Duration::from_secs(5),
    };
    assert_eq!(client.get(Provider::OpenAi, None), Err(UsageError::Network));
    assert!(server.join().expect("server accepted a local connection"));
}

fn accept_local(listener: std::net::TcpListener) -> bool {
    let started = Instant::now();
    loop {
        match listener.accept() {
            Ok(_) => return true,
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    && started.elapsed() < Duration::from_secs(5) =>
            {
                std::thread::yield_now();
            }
            Err(_) => return false,
        }
    }
}
