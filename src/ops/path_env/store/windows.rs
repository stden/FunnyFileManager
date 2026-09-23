use crate::path_env::{Scope, Snapshot};
use serde_json::{json, Value};
use std::{
    io::{self, Write},
    process::{Command, Stdio},
};

pub fn read(scope: Scope) -> io::Result<Snapshot> {
    let result = invoke(&json!({"action": "read", "scope": scope_name(scope)}))?;
    Ok(Snapshot {
        value: field(&result, "value")?,
        revision: field(&result, "revision")?,
        location: field(&result, "location")?,
    })
}

pub fn write(scope: Scope, previous: &Snapshot, value: &str) -> io::Result<()> {
    invoke(&json!({"action": "write", "scope": scope_name(scope), "expected": previous.revision, "value": value})).map(|_| ())
}

fn scope_name(scope: Scope) -> &'static str {
    match scope {
        Scope::User => "User",
        Scope::System => "Machine",
    }
}

fn field(result: &Value, name: &str) -> io::Result<String> {
    result[name]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| io::Error::other("Invalid PATH response"))
}

fn invoke(input: &Value) -> io::Result<Value> {
    use std::os::windows::process::CommandExt;
    let system =
        std::env::var_os("SystemRoot").ok_or_else(|| io::Error::other("SystemRoot is missing"))?;
    let executable =
        std::path::PathBuf::from(system).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let mut child = Command::new(executable)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            include_str!("path.ps1"),
        ])
        .creation_flags(0x0800_0000)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut input_pipe = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("No PATH input pipe"))?;
    input_pipe.write_all(input.to_string().as_bytes())?;
    drop(input_pipe);
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    serde_json::from_slice(&output.stdout).map_err(io::Error::other)
}
