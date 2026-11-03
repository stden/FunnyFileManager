//! Function-pointer tables binding `SftpFs` and `connect` to `ssh2` without wrapper closures.

use ssh2::{
    CheckResult, FileStat, HostKeyType, KnownHostFileKind, OpenFlags, OpenType, RenameFlags,
};
use std::{
    net::TcpStream,
    path::{Path, PathBuf},
};

type ReaddirFn<S> = fn(&S, PathBuf) -> Result<Vec<(PathBuf, FileStat)>, ssh2::Error>;
type HostKeyFn<S> = fn(&S) -> Option<(&[u8], HostKeyType)>;
type PubkeyFileFn<S> = fn(&S, &str, Option<&Path>, &Path, Option<&str>) -> Result<(), ssh2::Error>;

pub(super) struct SftpOps<S, F> {
    pub readdir: ReaddirFn<S>,
    pub lstat: fn(&S, &Path) -> Result<FileStat, ssh2::Error>,
    pub stat: fn(&S, &Path) -> Result<FileStat, ssh2::Error>,
    pub open: fn(&S, PathBuf) -> Result<F, ssh2::Error>,
    pub file_stat: fn(&mut F) -> Result<FileStat, ssh2::Error>,
    pub file_close: fn(&mut F) -> Result<(), ssh2::Error>,
    pub open_mode: fn(&S, PathBuf, OpenFlags, i32, OpenType) -> Result<F, ssh2::Error>,
    pub mkdir: fn(&S, &Path, i32) -> Result<(), ssh2::Error>,
    pub unlink: fn(&S, &Path) -> Result<(), ssh2::Error>,
    pub rmdir: fn(&S, &Path) -> Result<(), ssh2::Error>,
    pub rename: fn(&S, &Path, &Path, Option<RenameFlags>) -> Result<(), ssh2::Error>,
    pub setstat: fn(&S, &Path, FileStat) -> Result<(), ssh2::Error>,
}

pub(super) const SSH2_SFTP_OPS: SftpOps<ssh2::Sftp, ssh2::File> = SftpOps {
    readdir: ssh2::Sftp::readdir::<PathBuf>,
    lstat: ssh2::Sftp::lstat,
    stat: ssh2::Sftp::stat,
    open: ssh2::Sftp::open::<PathBuf>,
    file_stat: ssh2::File::stat,
    file_close: ssh2::File::close,
    open_mode: ssh2::Sftp::open_mode::<PathBuf>,
    mkdir: ssh2::Sftp::mkdir,
    unlink: ssh2::Sftp::unlink,
    rmdir: ssh2::Sftp::rmdir,
    rename: ssh2::Sftp::rename,
    setstat: ssh2::Sftp::setstat,
};

pub(super) struct ClientOps<S: 'static, K, B: 'static, F: 'static> {
    pub new_session: fn() -> Result<S, ssh2::Error>,
    pub set_timeout: fn(&S, u32),
    pub set_tcp_stream: fn(&mut S, TcpStream),
    pub handshake: fn(&mut S) -> Result<(), ssh2::Error>,
    pub host_key: HostKeyFn<S>,
    pub known_hosts: fn(&S) -> Result<K, ssh2::Error>,
    pub read_known_hosts: fn(&mut K, &Path, KnownHostFileKind) -> Result<u32, ssh2::Error>,
    pub check_port: fn(&K, &str, u16, &[u8]) -> CheckResult,
    pub auth_password: fn(&S, &str, &str) -> Result<(), ssh2::Error>,
    pub auth_pubkey_file: PubkeyFileFn<S>,
    pub auth_agent: fn(&S, &str) -> Result<(), ssh2::Error>,
    pub authenticated: fn(&S) -> bool,
    pub open_sftp: fn(&S) -> Result<B, ssh2::Error>,
    pub sftp_ops: &'static SftpOps<B, F>,
}

pub(super) const SSH2_CLIENT_OPS: ClientOps<
    ssh2::Session,
    ssh2::KnownHosts,
    ssh2::Sftp,
    ssh2::File,
> = ClientOps {
    new_session: ssh2::Session::new,
    set_timeout: ssh2::Session::set_timeout,
    set_tcp_stream: ssh2::Session::set_tcp_stream::<TcpStream>,
    handshake: ssh2::Session::handshake,
    host_key: ssh2::Session::host_key,
    known_hosts: ssh2::Session::known_hosts,
    read_known_hosts: ssh2::KnownHosts::read_file,
    check_port: ssh2::KnownHosts::check_port,
    auth_password: ssh2::Session::userauth_password,
    auth_pubkey_file: ssh2::Session::userauth_pubkey_file,
    auth_agent: ssh2::Session::userauth_agent,
    authenticated: ssh2::Session::authenticated,
    open_sftp: ssh2::Session::sftp,
    sftp_ops: &SSH2_SFTP_OPS,
};
