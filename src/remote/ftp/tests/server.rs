//! Purpose: Minimal deterministic FTP peer exercising the actual data and control sockets.

use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    thread::JoinHandle,
    time::Duration,
};

pub(super) struct Server {
    port: u16,
    worker: JoinHandle<Vec<String>>,
}

impl Server {
    pub(super) fn start(legacy: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listen");
        let port = listener.local_addr().expect("address").port();
        let worker = std::thread::spawn(move || serve(listener, legacy));
        Self { port, worker }
    }
    pub(super) const fn port(&self) -> u16 {
        self.port
    }
    pub(super) fn finish(self) -> Vec<String> {
        self.worker.join().expect("FTP fixture")
    }
}

struct Peer {
    control: TcpStream,
    passive: Option<TcpListener>,
    files: BTreeMap<String, Vec<u8>>,
    rename: Option<String>,
    commands: Vec<String>,
    legacy: bool,
}

fn serve(listener: TcpListener, legacy: bool) -> Vec<String> {
    let (control, _) = listener.accept().expect("accept");
    control
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("timeout");
    let mut input = BufReader::new(control.try_clone().expect("clone"));
    let mut peer = Peer {
        control,
        passive: None,
        files: BTreeMap::new(),
        rename: None,
        commands: Vec::new(),
        legacy,
    };
    peer.reply("220 fixture ready");
    let mut line = String::new();
    while input.read_line(&mut line).expect("command") > 0 {
        let command = line.trim_end_matches(['\r', '\n']).to_owned();
        peer.commands.push(command.clone());
        peer.command(&command);
        line.clear();
    }
    peer.commands
}

impl Peer {
    fn reply(&mut self, text: &str) {
        writeln!(self.control, "{text}\r").expect("reply");
    }

    fn command(&mut self, line: &str) {
        let (command, argument) = line.split_once(' ').unwrap_or((line, ""));
        match command {
            "USER" => self.reply("331 password"),
            "PASS" => self.reply("230 logged in"),
            "TYPE" | "SITE" => self.reply("200 accepted"),
            "PASV" | "EPSV" => self.passive(command),
            "MLSD" if self.legacy => self.reply("502 use LIST"),
            "MLSD" | "LIST" => self.list(command),
            "STOR" => self.upload(argument),
            "RETR" => self.download(argument),
            "RNFR" => {
                self.rename = Some(argument.to_owned());
                self.reply("350 destination");
            }
            "RNTO" => self.rename(argument),
            "DELE" => {
                self.files.remove(argument);
                self.reply("250 deleted");
            }
            _ => self.reply("502 unsupported"),
        }
    }

    fn passive(&mut self, command: &str) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("data listener");
        let port = listener.local_addr().expect("data address").port();
        self.passive = Some(listener);
        if command == "EPSV" {
            self.reply(&format!("229 extended (|||{port}|)"));
        } else {
            self.reply(&format!(
                "227 passive (127,0,0,1,{},{})",
                port / 256,
                port % 256
            ));
        }
    }

    fn data(&mut self) -> TcpStream {
        self.reply("150 data connection");
        let (socket, _) = self
            .passive
            .take()
            .expect("PASV first")
            .accept()
            .expect("data accept");
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("data timeout");
        socket
    }

    fn list(&mut self, command: &str) {
        let mut socket = self.data();
        for (name, bytes) in &self.files {
            let name = name.trim_start_matches('/');
            let line = if command == "MLSD" {
                format!(
                    "type=file;size={};modify=20250101000000;unix.mode=0644; {name}\r\n",
                    bytes.len()
                )
            } else {
                format!(
                    "-rw-r--r-- 1 1000 1000 {} Jan 1 2025 {name}\r\n",
                    bytes.len()
                )
            };
            socket.write_all(line.as_bytes()).expect("listing");
        }
        drop(socket);
        self.reply("226 complete");
    }

    fn upload(&mut self, name: &str) {
        let mut socket = self.data();
        let mut bytes = Vec::new();
        socket.read_to_end(&mut bytes).expect("uploaded bytes");
        self.files.insert(name.to_owned(), bytes);
        drop(socket);
        self.reply("226 complete");
    }

    fn download(&mut self, name: &str) {
        let mut socket = self.data();
        socket
            .write_all(self.files.get(name).expect("existing file"))
            .expect("download bytes");
        drop(socket);
        self.reply("226 complete");
    }

    fn rename(&mut self, name: &str) {
        let source = self.rename.take().expect("RNFR first");
        let bytes = self.files.remove(&source).expect("source");
        self.files.insert(name.to_owned(), bytes);
        self.reply("250 renamed");
    }
}
