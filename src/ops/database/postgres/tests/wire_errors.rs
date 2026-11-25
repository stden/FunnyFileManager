//! Purpose: Ephemeral PostgreSQL wire-protocol error scenarios covering mid-query failures.

use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};

use super::super::{columns, connect, rows};
use crate::models::database::PostgresEndpoint;
use crate::ops::database::fixtures::table_ref;
use crate::ops::database::DbError;

#[derive(Clone, Copy, PartialEq, Eq)]
enum FailMode {
    PrimaryKeyNull,
    ColumnsQueryError,
    RowsSimpleError,
}

#[test]
fn mid_operation_driver_and_conversion_errors_are_reported() {
    let table = table_ref("public", "items");
    for mode in [FailMode::PrimaryKeyNull, FailMode::ColumnsQueryError] {
        let mut client = connect(&spawn_mock(mode), None).expect("connect");
        assert!(matches!(
            columns(&mut client, &table),
            Err(DbError::Driver(_))
        ));
    }
    let mut client = connect(&spawn_mock(FailMode::RowsSimpleError), None).expect("connect");
    assert!(matches!(
        rows(&mut client, &table, 0),
        Err(DbError::Driver(_))
    ));
}

fn spawn_mock(mode: FailMode) -> PostgresEndpoint {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        if let Ok((stream, _)) = listener.accept() {
            drop(serve(stream, mode));
        }
    });
    PostgresEndpoint {
        host: "127.0.0.1".to_owned(),
        port,
        database: "postgres".to_owned(),
        user: "postgres".to_owned(),
        password_env: None,
    }
}

fn serve(mut stream: TcpStream, mode: FailMode) -> io::Result<()> {
    handshake(&mut stream)?;
    let (mut parses, mut skip) = (0_usize, false);
    loop {
        let mut tag = [0_u8; 1];
        if stream.read_exact(&mut tag).is_err() || tag[0] == b'X' {
            break;
        }
        let mut len_buf = [0_u8; 4];
        stream.read_exact(&mut len_buf)?;
        let body_len = usize::try_from(i32::from_be_bytes(len_buf) - 4).unwrap_or(0);
        let mut body = vec![0_u8; body_len];
        stream.read_exact(&mut body)?;
        handle_msg(&mut stream, tag[0], &body, mode, &mut parses, &mut skip)?;
    }
    Ok(())
}

fn handshake(stream: &mut TcpStream) -> io::Result<()> {
    let mut header = [0_u8; 8];
    stream.read_exact(&mut header)?;
    let len = i32::from_be_bytes([header[0], header[1], header[2], header[3]]);
    let mut startup = vec![0_u8; usize::try_from(len - 8).unwrap_or(0)];
    stream.read_exact(&mut startup)?;
    frame(stream, b'R', &0_i32.to_be_bytes())?;
    frame(stream, b'S', b"client_encoding\0UTF8\0")?;
    frame(stream, b'K', &[0_u8; 8])?;
    frame(stream, b'Z', b"I")?;
    stream.flush()
}

fn handle_msg(
    out: &mut TcpStream,
    tag: u8,
    body: &[u8],
    mode: FailMode,
    parses: &mut usize,
    skip: &mut bool,
) -> io::Result<()> {
    if *skip {
        return finish_skip(out, tag, skip);
    }
    match tag {
        b'Q' => handle_query(out, body),
        b'P' => handle_parse(out, mode, parses, skip),
        b'D' => handle_describe(out, body[0], mode),
        b'B' => frame(out, b'2', &[]),
        b'E' => handle_execute(out, mode),
        b'C' => frame(out, b'3', &[]),
        b'S' => {
            frame(out, b'Z', b"I")?;
            out.flush()
        }
        _ => Ok(()),
    }
}

fn finish_skip(out: &mut TcpStream, tag: u8, skip: &mut bool) -> io::Result<()> {
    if tag == b'S' {
        *skip = false;
        frame(out, b'Z', b"I")?;
        out.flush()?;
    }
    Ok(())
}

fn handle_query(out: &mut TcpStream, body: &[u8]) -> io::Result<()> {
    if body.starts_with(b"SET SESSION") {
        frame(out, b'C', b"SET\0")?;
    } else {
        write_error(out)?;
    }
    frame(out, b'Z', b"I")?;
    out.flush()
}

fn handle_parse(
    out: &mut TcpStream,
    mode: FailMode,
    parses: &mut usize,
    skip: &mut bool,
) -> io::Result<()> {
    *parses += 1;
    if mode == FailMode::ColumnsQueryError && *parses == 2 {
        *skip = true;
        return write_error(out);
    }
    frame(out, b'1', &[])
}

fn handle_describe(out: &mut TcpStream, kind: u8, mode: FailMode) -> io::Result<()> {
    if kind == b'S' {
        if mode == FailMode::RowsSimpleError {
            frame(out, b't', &0_i16.to_be_bytes())?;
        } else {
            let mut params = 2_i16.to_be_bytes().to_vec();
            params.extend_from_slice(&25_u32.to_be_bytes());
            params.extend_from_slice(&25_u32.to_be_bytes());
            frame(out, b't', &params)?;
        }
    }
    write_one_text_col(out, "k")
}

fn handle_execute(out: &mut TcpStream, mode: FailMode) -> io::Result<()> {
    if mode == FailMode::PrimaryKeyNull {
        let mut row = 1_i16.to_be_bytes().to_vec();
        row.extend_from_slice(&(-1_i32).to_be_bytes());
        frame(out, b'D', &row)?;
        return frame(out, b'C', b"SELECT 1\0");
    }
    frame(out, b'C', b"SELECT 0\0")
}

fn write_one_text_col(out: &mut TcpStream, name: &str) -> io::Result<()> {
    let mut payload = 1_i16.to_be_bytes().to_vec();
    payload.extend_from_slice(name.as_bytes());
    payload.push(0);
    payload.extend_from_slice(&0_i32.to_be_bytes());
    payload.extend_from_slice(&0_i16.to_be_bytes());
    payload.extend_from_slice(&25_u32.to_be_bytes());
    payload.extend_from_slice(&(-1_i16).to_be_bytes());
    payload.extend_from_slice(&(-1_i32).to_be_bytes());
    payload.extend_from_slice(&0_i16.to_be_bytes());
    frame(out, b'T', &payload)
}

fn write_error(out: &mut TcpStream) -> io::Result<()> {
    frame(out, b'E', b"SERROR\0VERROR\0C42000\0Mmock error\0\0")
}

fn frame(out: &mut TcpStream, tag: u8, payload: &[u8]) -> io::Result<()> {
    let len = i32::try_from(payload.len() + 4).expect("len");
    out.write_all(&[tag])?;
    out.write_all(&len.to_be_bytes())?;
    out.write_all(payload)
}
