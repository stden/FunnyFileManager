//! PostgreSQL v3 wire-protocol frame builders and binary readers.

use std::io::{self, Write};

pub(super) const OID_BOOL: u32 = 16;
pub(super) const OID_INT4: u32 = 23;
pub(super) const OID_TEXT: u32 = 25;

pub(super) fn write_frame(out: &mut impl Write, tag: u8, payload: &[u8]) -> io::Result<()> {
    let len = i32::try_from(payload.len() + 4).expect("frame length fits i32");
    out.write_all(&[tag])?;
    out.write_all(&len.to_be_bytes())?;
    out.write_all(payload)
}

pub(super) fn write_ready(out: &mut impl Write) -> io::Result<()> {
    write_frame(out, b'Z', b"I")
}

pub(super) fn write_command_complete(out: &mut impl Write, label: &str) -> io::Result<()> {
    let mut payload = label.as_bytes().to_vec();
    payload.push(0);
    write_frame(out, b'C', &payload)
}

pub(super) fn write_error(
    out: &mut impl Write,
    severity: &str,
    code: &str,
    message: &str,
) -> io::Result<()> {
    let mut payload = Vec::new();
    for (field, val) in [
        (b'S', severity),
        (b'V', severity),
        (b'C', code),
        (b'M', message),
    ] {
        payload.push(field);
        payload.extend_from_slice(val.as_bytes());
        payload.push(0);
    }
    payload.push(0);
    write_frame(out, b'E', &payload)
}

pub(super) fn write_param_desc(out: &mut impl Write, oids: &[u32]) -> io::Result<()> {
    let count = i16::try_from(oids.len()).expect("param count");
    let mut payload = count.to_be_bytes().to_vec();
    for &oid in oids {
        payload.extend_from_slice(&oid.to_be_bytes());
    }
    write_frame(out, b't', &payload)
}

pub(super) fn write_row_desc(out: &mut impl Write, columns: &[(&str, u32, i16)]) -> io::Result<()> {
    let count = i16::try_from(columns.len()).expect("col count");
    let mut payload = count.to_be_bytes().to_vec();
    for &(name, oid, format) in columns {
        payload.extend_from_slice(name.as_bytes());
        payload.push(0);
        payload.extend_from_slice(&0i32.to_be_bytes());
        payload.extend_from_slice(&0i16.to_be_bytes());
        payload.extend_from_slice(&oid.to_be_bytes());
        let size: i16 = match oid {
            OID_BOOL => 1,
            OID_INT4 => 4,
            _ => -1,
        };
        payload.extend_from_slice(&size.to_be_bytes());
        payload.extend_from_slice(&(-1i32).to_be_bytes());
        payload.extend_from_slice(&format.to_be_bytes());
    }
    write_frame(out, b'T', &payload)
}

pub(super) fn write_data_row(out: &mut impl Write, cells: &[Option<Vec<u8>>]) -> io::Result<()> {
    let count = i16::try_from(cells.len()).expect("cell count");
    let mut payload = count.to_be_bytes().to_vec();
    for cell in cells {
        match cell {
            None => payload.extend_from_slice(&(-1i32).to_be_bytes()),
            Some(bytes) => {
                let len = i32::try_from(bytes.len()).expect("cell len");
                payload.extend_from_slice(&len.to_be_bytes());
                payload.extend_from_slice(bytes);
            }
        }
    }
    write_frame(out, b'D', &payload)
}

pub(super) fn write_text_rows(
    out: &mut impl Write,
    rows: &[Vec<Option<String>>],
) -> io::Result<()> {
    for row in rows {
        let raw: Vec<Option<Vec<u8>>> = row
            .iter()
            .map(|c| c.as_ref().map(|s| s.as_bytes().to_vec()))
            .collect();
        write_data_row(out, &raw)?;
    }
    Ok(())
}

pub(super) fn read_cstring(buf: &[u8], pos: &mut usize) -> String {
    let end = buf[*pos..]
        .iter()
        .position(|&b| b == 0)
        .map_or(buf.len(), |off| *pos + off);
    let s = String::from_utf8_lossy(&buf[*pos..end]).into_owned();
    *pos = (end + 1).min(buf.len());
    s
}

pub(super) fn read_i16(buf: &[u8], pos: &mut usize) -> i16 {
    let bytes: [u8; 2] = buf[*pos..*pos + 2].try_into().expect("i16");
    *pos += 2;
    i16::from_be_bytes(bytes)
}

pub(super) fn read_i32(buf: &[u8], pos: &mut usize) -> i32 {
    let bytes: [u8; 4] = buf[*pos..*pos + 4].try_into().expect("i32");
    *pos += 4;
    i32::from_be_bytes(bytes)
}
