//! In-process loopback PostgreSQL v3 wire-protocol test server.

mod catalog;
mod frames;
mod queries;
mod session;

use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, OnceLock};

use catalog::Catalog;
use frames::{
    read_cstring, write_data_row, write_error, write_frame, write_ready, write_text_rows,
};
use queries::query_catalog;
use session::ConnState;

const SSL_REQUEST_CODE: i32 = 80_877_103;

pub(in crate::ops::database) fn url() -> String {
    static SERVER_URL: OnceLock<String> = OnceLock::new();
    SERVER_URL
        .get_or_init(|| {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback postgres");
            let port = listener.local_addr().expect("local addr").port();
            let catalog = Arc::new(Mutex::new(Catalog::default()));
            std::thread::Builder::new()
                .name("mock-pg-wire".to_owned())
                .spawn(move || accept_loop(&listener, &catalog))
                .expect("spawn mock-pg-wire");
            format!("postgres://postgres:test-secret@127.0.0.1:{port}/postgres")
        })
        .clone()
}

fn accept_loop(listener: &TcpListener, catalog: &Arc<Mutex<Catalog>>) {
    for incoming in listener.incoming().flatten() {
        let shared = Arc::clone(catalog);
        std::thread::spawn(move || {
            drop(serve_client(incoming, shared));
        });
    }
}

fn serve_client(mut stream: TcpStream, catalog: Arc<Mutex<Catalog>>) -> io::Result<()> {
    let Some(pid) = handshake(&mut stream, &catalog)? else {
        return Ok(());
    };
    let mut state = ConnState {
        pid,
        read_only: false,
        error_until_sync: false,
        statements: BTreeMap::new(),
        portals: BTreeMap::new(),
        catalog,
    };
    loop {
        let mut tag = [0u8; 1];
        if stream.read_exact(&mut tag).is_err() || tag[0] == b'X' {
            break;
        }
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf)?;
        let msg_len = usize::try_from(i32::from_be_bytes(len_buf) - 4).unwrap_or(0);
        let mut body = vec![0u8; msg_len];
        stream.read_exact(&mut body)?;
        state.handle_message(tag[0], &body, &mut stream)?;
    }
    Ok(())
}

fn read_startup(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut header = [0u8; 8];
    stream.read_exact(&mut header)?;
    let mut len = i32::from_be_bytes([header[0], header[1], header[2], header[3]]);
    let code = i32::from_be_bytes([header[4], header[5], header[6], header[7]]);
    if len == 8 && code == SSL_REQUEST_CODE {
        stream.write_all(b"N")?;
        stream.flush()?;
        stream.read_exact(&mut header)?;
        len = i32::from_be_bytes([header[0], header[1], header[2], header[3]]);
    }
    let mut startup = vec![0u8; usize::try_from(len - 8).unwrap_or(0)];
    stream.read_exact(&mut startup)?;
    Ok(startup)
}

fn handshake(stream: &mut TcpStream, catalog: &Arc<Mutex<Catalog>>) -> io::Result<Option<i32>> {
    let startup = read_startup(stream)?;
    if startup_user(&startup) == "hunter2" {
        write_error(
            stream,
            "FATAL",
            "28P01",
            "password authentication failed for user \"hunter2\"",
        )?;
        stream.flush()?;
        return Ok(None);
    }
    let pid = catalog
        .lock()
        .expect("catalog lock")
        .register_backend(stream);
    write_frame(stream, b'R', &0i32.to_be_bytes())?;
    write_frame(stream, b'S', b"client_encoding\0UTF8\0")?;
    let mut key_data = Vec::with_capacity(8);
    key_data.extend_from_slice(&pid.to_be_bytes());
    key_data.extend_from_slice(&0i32.to_be_bytes());
    write_frame(stream, b'K', &key_data)?;
    write_ready(stream)?;
    stream.flush()?;
    Ok(Some(pid))
}

fn startup_user(startup: &[u8]) -> String {
    let mut pos = 0;
    while pos < startup.len() && startup[pos] != 0 {
        let key = read_cstring(startup, &mut pos);
        let val = read_cstring(startup, &mut pos);
        if key == "user" {
            return val;
        }
    }
    String::new()
}

fn execute_portal(
    pid: i32,
    catalog: &Arc<Mutex<Catalog>>,
    query: &str,
    params: &[Vec<u8>],
    fmt: i16,
    out: &mut impl Write,
) -> io::Result<()> {
    if query.contains("pg_backend_pid") {
        let cell = if fmt == 1 {
            pid.to_be_bytes().to_vec()
        } else {
            pid.to_string().into_bytes()
        };
        return write_data_row(out, &[Some(cell)]);
    }
    if query.contains("pg_terminate_backend") {
        let target = if params[0].len() == 4 {
            i32::from_be_bytes(params[0].as_slice().try_into().expect("pid bytes"))
        } else {
            String::from_utf8_lossy(&params[0]).parse().unwrap_or(0)
        };
        catalog
            .lock()
            .expect("catalog lock")
            .terminate_backend(target);
        let cell = if fmt == 1 { vec![1u8] } else { b"t".to_vec() };
        return write_data_row(out, &[Some(cell)]);
    }
    if query == "SHOW transaction_read_only" {
        return write_text_rows(out, &[vec![Some("on".to_owned())]]);
    }
    if query == "SHOW statement_timeout" {
        return write_text_rows(out, &[vec![Some("5s".to_owned())]]);
    }
    let rows = query_catalog(&catalog.lock().expect("catalog lock"), query, params);
    write_text_rows(out, &rows)
}
