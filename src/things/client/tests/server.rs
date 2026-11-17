//! A single HTTP exchange on an ephemeral loopback socket.

use std::{
    io::{Read, Write},
    net::TcpListener,
    thread::JoinHandle,
    time::Duration,
};

pub(super) fn response(status: u16, body: &[u8]) -> Vec<u8> {
    let mut reply = format!(
        "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    reply.extend_from_slice(body);
    reply
}

pub(super) fn serve(reply: Vec<u8>) -> (String, JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback listener");
    let address = listener.local_addr().expect("bound address");
    let worker = std::thread::spawn(move || {
        let (mut connection, _) = listener.accept().expect("catalog request");
        connection
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read deadline");
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            if connection.read(&mut byte).expect("request header") == 0 {
                break;
            }
            request.extend_from_slice(&byte);
        }
        drop(connection.write_all(&reply)); // The bounded reader may close early.
        String::from_utf8(request).expect("ASCII request")
    });
    (format!("http://{address}/api/v1/machines"), worker)
}
