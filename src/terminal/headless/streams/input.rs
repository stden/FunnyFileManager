//! Purpose: Deliver terminal replies before user input can block `ConPTY` startup.

use super::super::{responses::Reply, state::Generation, HeadlessHost};
use std::{
    io::{self, Write},
    sync::mpsc::{Receiver, RecvTimeoutError},
    thread,
    time::Duration,
};

pub(in crate::terminal::headless) fn write(
    mut writer: Box<dyn Write + Send>,
    input: Receiver<Vec<u8>>,
    replies: Receiver<Reply>,
    host: HeadlessHost,
    generation: Generation,
) -> io::Result<()> {
    thread::Builder::new()
        .name("ffm-private-input".to_owned())
        .spawn(move || {
            if let Err(error) = pump(&mut *writer, &input, &replies, &host, generation) {
                crate::diagnostics::log_error(&format!("Private terminal input closed: {error}"));
            }
        })?;
    Ok(())
}

fn pump(
    writer: &mut dyn Write,
    input: &Receiver<Vec<u8>>,
    replies: &Receiver<Reply>,
    host: &HeadlessHost,
    generation: Generation,
) -> io::Result<()> {
    let mut starting = cfg!(windows);
    while host.active(generation) && !host.is_cancelled() {
        if matches!(replies.try_recv(), Ok(Reply::CursorPosition)) {
            deliver(writer, Reply::bytes(), host, generation)?;
            starting = false;
        } else if starting {
            // ConPTY needs its cursor reply before the child can read keys.
            thread::sleep(Duration::from_millis(10));
        } else {
            match input.recv_timeout(Duration::from_millis(10)) {
                Ok(input) => deliver(writer, &input, host, generation)?,
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }
    Ok(())
}

fn deliver(
    writer: &mut dyn Write,
    mut bytes: &[u8],
    host: &HeadlessHost,
    generation: Generation,
) -> io::Result<()> {
    while !bytes.is_empty() {
        if !host.active(generation) || host.is_cancelled() {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Private input cancelled",
            ));
        }
        match write_chunk(writer, bytes)? {
            Written::Count(count) => {
                bytes = bytes
                    .get(count..)
                    .ok_or_else(|| io::Error::other("Invalid terminal write count"))?;
            }
            Written::Retry => thread::sleep(Duration::from_millis(10)),
        }
    }
    writer.flush()
}

enum Written {
    Count(usize),
    Retry,
}

fn write_chunk(writer: &mut dyn Write, bytes: &[u8]) -> io::Result<Written> {
    match writer.write(bytes) {
        Ok(0) => Err(io::Error::new(
            io::ErrorKind::WriteZero,
            "Private input closed",
        )),
        Ok(count) => Ok(Written::Count(count)),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            ) =>
        {
            Ok(Written::Retry)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests;
