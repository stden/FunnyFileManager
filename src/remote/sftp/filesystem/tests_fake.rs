//! Cross-platform unit tests for `SftpFs` streaming operations and error translation.

use super::super::{metadata, ops::SftpOps};
use ssh2::{Error, ErrorCode, FileStat, OpenFlags, RenameFlags};
use std::{
    collections::BTreeMap,
    io::{self, Cursor, Read, Write},
    path::PathBuf,
    sync::Mutex,
};

#[derive(Default)]
pub(in crate::remote::sftp) struct FakeBackend {
    entries: Mutex<BTreeMap<PathBuf, (FileStat, Vec<u8>)>>,
    targets: Mutex<BTreeMap<PathBuf, Result<FileStat, i32>>>,
    fail_code: Mutex<Option<i32>>,
    fail_close: Mutex<bool>,
}

pub(in crate::remote::sftp) struct FakeFile {
    cursor: Cursor<Vec<u8>>,
    stat: FileStat,
    fail_close: bool,
}

impl Read for FakeFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.cursor.read(buf)
    }
}

impl Write for FakeFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.cursor.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn sftp_err(code: i32) -> Error {
    Error::from_errno(ErrorCode::SFTP(code))
}

fn check_fail(state: &FakeBackend) -> Result<(), Error> {
    state
        .fail_code
        .lock()
        .expect("lock")
        .map_or(Ok(()), |c| Err(sftp_err(c)))
}

pub(in crate::remote::sftp) static FAKE_SFTP_OPS: SftpOps<FakeBackend, FakeFile> = SftpOps {
    readdir: |s, _| {
        check_fail(s)?;
        Ok(s.entries
            .lock()
            .expect("lock")
            .iter()
            .map(|(p, (st, _))| (p.clone(), st.clone()))
            .collect())
    },
    lstat: |s, p| {
        check_fail(s)?;
        s.entries
            .lock()
            .expect("lock")
            .get(p)
            .map(|(st, _)| st.clone())
            .ok_or_else(|| sftp_err(2))
    },
    stat: |s, p| {
        s.targets
            .lock()
            .expect("lock")
            .get(p)
            .cloned()
            .unwrap_or(Err(2))
            .map_err(sftp_err)
    },
    open: |s, p| {
        check_fail(s)?;
        let (stat, bytes) = s
            .entries
            .lock()
            .expect("lock")
            .get(&p)
            .cloned()
            .ok_or_else(|| sftp_err(2))?;
        Ok(FakeFile {
            cursor: Cursor::new(bytes),
            stat,
            fail_close: *s.fail_close.lock().expect("lock"),
        })
    },
    file_stat: |f| Ok(f.stat.clone()),
    file_close: |f| {
        if f.fail_close {
            Err(sftp_err(4))
        } else {
            Ok(())
        }
    },
    open_mode: |s, p, flags, mode, _ty| {
        check_fail(s)?;
        if flags.contains(OpenFlags::EXCLUSIVE) && s.entries.lock().expect("lock").contains_key(&p)
        {
            return Err(sftp_err(11));
        }
        let mut stat = metadata::empty_stat();
        stat.perm = Some(0o100_000 | u32::try_from(mode).unwrap_or(0));
        s.entries
            .lock()
            .expect("lock")
            .insert(p, (stat.clone(), b"uploaded".to_vec()));
        Ok(FakeFile {
            cursor: Cursor::new(Vec::new()),
            stat,
            fail_close: *s.fail_close.lock().expect("lock"),
        })
    },
    mkdir: |s, p, _| {
        check_fail(s)?;
        let mut st = metadata::empty_stat();
        st.perm = Some(0o040_755);
        s.entries
            .lock()
            .expect("lock")
            .insert(p.to_path_buf(), (st, Vec::new()));
        Ok(())
    },
    unlink: |s, p| {
        check_fail(s)?;
        s.entries
            .lock()
            .expect("lock")
            .remove(p)
            .map(|_| ())
            .ok_or_else(|| sftp_err(2))
    },
    rmdir: |s, p| {
        check_fail(s)?;
        s.entries
            .lock()
            .expect("lock")
            .remove(p)
            .map(|_| ())
            .ok_or_else(|| sftp_err(2))
    },
    rename: |s, from, to, _flags: Option<RenameFlags>| {
        check_fail(s)?;
        let item = s
            .entries
            .lock()
            .expect("lock")
            .remove(from)
            .ok_or_else(|| sftp_err(2))?;
        s.entries
            .lock()
            .expect("lock")
            .insert(to.to_path_buf(), item);
        Ok(())
    },
    setstat: |s, p, stat| {
        check_fail(s)?;
        s.entries
            .lock()
            .expect("lock")
            .get_mut(p)
            .ok_or_else(|| sftp_err(2))?
            .0
            .perm = stat.perm.map(|m| 0o100_000 | m);
        Ok(())
    },
};

mod tests_ops;
