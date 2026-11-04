//! Scripted HTTP responses with captured SDK requests and bounded reads.

use std::{
    collections::VecDeque,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Clone)]
pub struct Request {
    pub headers: String,
    pub body: Vec<u8>,
}

impl Request {
    pub fn path(&self) -> &str {
        let target = self.headers.split_whitespace().nth(1).unwrap();
        target.split('?').next().unwrap()
    }
}

pub struct Server {
    endpoint: String,
    requests: Arc<Mutex<Vec<Request>>>,
    remaining: Arc<Mutex<VecDeque<Vec<u8>>>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Server {
    pub fn new(replies: Vec<Vec<u8>>) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let remaining = Arc::new(Mutex::new(VecDeque::from(replies)));
        let stop = Arc::new(AtomicBool::new(false));
        let worker = {
            let requests = Arc::clone(&requests);
            let remaining = Arc::clone(&remaining);
            let stop = Arc::clone(&stop);
            thread::spawn(move || serve(&listener, &requests, &remaining, &stop))
        };
        Self {
            endpoint,
            requests,
            remaining,
            stop,
            worker: Some(worker),
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.worker.take().unwrap().join().unwrap();
        if !thread::panicking() {
            assert!(
                self.remaining.lock().unwrap().is_empty(),
                "unused S3 replies"
            );
        }
    }
}

fn serve(
    listener: &TcpListener,
    requests: &Mutex<Vec<Request>>,
    replies: &Mutex<VecDeque<Vec<u8>>>,
    stop: &AtomicBool,
) {
    while !stop.load(Ordering::Acquire) {
        let Ok((mut stream, _)) = listener.accept() else {
            thread::sleep(Duration::from_millis(1));
            continue;
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream.set_nonblocking(false).unwrap();
        requests.lock().unwrap().push(read_request(&mut stream));
        let reply = replies
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected S3 request");
        stream.write_all(&reply).unwrap_or_else(|error| {
            assert!(
                matches!(
                    error.kind(),
                    std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset
                ),
                "{error}"
            );
        });
    }
}

fn read_request(stream: &mut TcpStream) -> Request {
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    let boundary = loop {
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
        let count = stream.read(&mut buffer).unwrap();
        assert_ne!(count, 0, "incomplete request headers");
        bytes.extend_from_slice(&buffer[..count]);
    };
    let headers = String::from_utf8(bytes[..boundary].to_vec()).unwrap();
    let length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    while bytes.len() - boundary < length {
        let count = stream.read(&mut buffer).unwrap();
        assert_ne!(count, 0, "incomplete request body");
        bytes.extend_from_slice(&buffer[..count]);
    }
    Request {
        headers,
        body: bytes[boundary..].to_vec(),
    }
}

pub fn response(status: u16, body: impl AsRef<[u8]>) -> Vec<u8> {
    let body = body.as_ref();
    let mut reply = format!("HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nContent-Type: application/xml\r\nConnection: close\r\n\r\n", body.len()).into_bytes();
    reply.extend_from_slice(body);
    reply
}
