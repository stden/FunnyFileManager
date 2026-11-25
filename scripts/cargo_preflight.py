#!/usr/bin/env python3
"""Fail within seconds when Cargo's compiler wrapper stalls or is broken.

A wrapper that never answers (typically a shared sccache server holding
Cargo's output pipes open) turns every build into a wait of many minutes with
no rustc running. This asks for `rustc -vV`, the first thing every build
runs, through the wrapper this repository's Cargo configuration and the
caller's environment select. It uses a throwaway crate with its own target
dir, so Cargo's cached rustc info cannot skip the call.

Usage: scripts/cargo_preflight.py    (arguments are ignored unless the first is -h or --help)
  FFM_WRAPPER_PROBE_SECS  seconds before the wrapper counts as stalled (20; 0 = no limit;
                          any duration timeout(1) takes, such as 1.5, 2m or 1h)
  CARGO                   Cargo to probe with (default: `cargo` on PATH)
  TMPDIR                  where the throwaway crate goes (default: /tmp on POSIX,
                            the system temporary directory on Windows)

It times Cargo itself, so it needs no GNU timeout(1) and probes on every host.
"""

import io
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile

# lint.py, beside this file, runs a step and ends the process as the bash
# scripts did; shell_path.py finds programs as bash does; preflight_limit.py
# reads FFM_WRAPPER_PROBE_SECS. Explicitly first on sys.path: `python3 -P` or
# PYTHONSAFEPATH=1 leave this file's directory out, and no other `lint`,
# `shell_path` or `preflight_limit` module may stand in.
sys.path.insert(0, str(Path(__file__).resolve().parent))
import lint  # noqa: E402
import shell_path  # noqa: E402
from preflight_limit import limit_seconds, limit_text  # noqa: E402


ROOT = Path(__file__).resolve().parents[1]
HELP_FLAGS = ("-h", "--help")
PROBE_PREFIX = "ffm-wrapper-probe."
PROBE_MANIFEST = '[package]\nname = "wrapper-probe"\nversion = "0.0.0"\nedition = "2021"\n\n[workspace]\n'
# Cargo queries the compiler before it takes the package-cache lock, so a wait
# on that lock means the wrapper answered and another Cargo is busy.
LOCK_WAIT = b"Blocking waiting for file lock"
SHOWN_STDERR_LINES = 20


class Terminated(Exception):
    """SIGTERM arrived; raised so that the probe crate is removed on the way out."""


def raise_terminated(_signal_number, _frame):
    raise Terminated


def die_of_sigterm():
    """End this process killed by SIGTERM, as bash ended once its EXIT trap had run."""
    signal.signal(signal.SIGTERM, signal.SIG_DFL)
    os.kill(os.getpid(), signal.SIGTERM)


def probe_parent():
    """The configured probe parent, else /tmp on POSIX or the system temp directory."""
    if directory := os.environ.get("TMPDIR"):
        return ROOT / directory
    default = "/tmp" if os.name == "posix" else tempfile.gettempdir()
    return ROOT / default


def make_probe_directory():
    """A fresh directory for the probe crate, as mktemp -d made one; None, said why, if it cannot be made."""
    parent = probe_parent()
    try:
        return tempfile.TemporaryDirectory(prefix=PROBE_PREFIX, dir=parent)
    except OSError as error:
        template = parent / f"{PROBE_PREFIX}XXXXXX"
        print(f"mktemp: failed to create directory via template '{template}': {error.strerror}", file=sys.stderr)
        return None


def write_probe_crate(probe):
    """An empty library crate in `probe`, outside every workspace."""
    (probe / "src").mkdir()
    (probe / "src" / "lib.rs").touch()
    (probe / "Cargo.toml").write_text(PROBE_MANIFEST, encoding="utf-8")


def ask_for_compiler_info(cargo, probe, seconds):
    """Run `cargo metadata` on the probe crate, killing Cargo at the limit.

    Returns Cargo's exit status (None when it was killed) and its stderr bytes.
    """
    command = [
        cargo, "metadata", "--format-version=1", "--offline",
        "--manifest-path", str(probe / "Cargo.toml"),
    ]
    # PWD as bash's `cd "$ROOT"` set it; the target dir for this one command.
    environment = {**os.environ, "PWD": str(ROOT), "CARGO_TARGET_DIR": str(probe / "target")}
    stderr_path = probe / "stderr"
    # A file, not a pipe: a stray grandchild of the wrapper cannot hold it open.
    with stderr_path.open("wb") as stderr:
        try:
            status = subprocess.run(
                command, executable=shell_path.resolve(cargo, ROOT), cwd=ROOT, env=environment,
                stdout=subprocess.DEVNULL, stderr=stderr, timeout=seconds, check=False,
            ).returncode
        except subprocess.TimeoutExpired:
            status = None
    return status, stderr_path.read_bytes()


def wrapper_description():
    """The compiler wrapper the environment selects, for the error message."""
    for name in ("RUSTC_WRAPPER", "CARGO_BUILD_RUSTC_WRAPPER"):
        if os.environ.get(name):
            return f"{name}={os.environ[name]}"
    return "none set in the environment; see Cargo's config files"


def report_failure(limit, timed_out, cargo_stderr):
    """Explain on stderr why builds would hang or fail, with Cargo's last words."""
    wrapper = wrapper_description()
    if timed_out:
        print(f"error: Cargo got no answer to `rustc -vV` within {limit}s (compiler wrapper: {wrapper}).", file=sys.stderr)
        print("Every build would hang the same way; a stalled sccache server is the usual cause.", file=sys.stderr)
    else:
        print(f"error: Cargo could not query the Rust compiler (compiler wrapper: {wrapper}):", file=sys.stderr)
    # Byte for byte as `tail -n 20` showed it: lines end at \n only.
    sys.stderr.flush()
    sys.stderr.buffer.write(b"".join(io.BytesIO(cargo_stderr).readlines()[-SHOWN_STDERR_LINES:]))
    print("This repository builds without a wrapper. If you enabled one, retry with RUSTC_WRAPPER= (empty),", file=sys.stderr)
    print("or restart the cache server (sccache --stop-server).", file=sys.stderr)


def main(argv=None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    # bash ignored its arguments; only -h or --help first shows this text.
    if argv[:1] and argv[0] in HELP_FLAGS:
        print(__doc__.strip())
        return 0
    limit = limit_text()
    cargo = os.environ.get("CARGO") or "cargo"
    # Cargo and rustup read .cargo/config.toml and rust-toolchain.toml from the
    # working directory, not from the manifest's, so probe from the checkout root.
    # Untimed and wrapper-free: lets rustup install the pinned toolchain first.
    # A failure ends the preflight with its status, as `set -e` did.
    lint.run([cargo, "-V"], stdout=subprocess.DEVNULL)
    seconds = limit_seconds(limit)
    # bash's EXIT trap removed the probe crate when SIGTERM ended the script too.
    signal.signal(signal.SIGTERM, raise_terminated)
    probe_directory = make_probe_directory()
    if probe_directory is None:
        return 1
    with probe_directory as directory:
        probe = Path(directory)
        write_probe_crate(probe)
        status, cargo_stderr = ask_for_compiler_info(cargo, probe, seconds)
    if status == 0:
        return 0
    if LOCK_WAIT in cargo_stderr:
        print("note: the compiler wrapper answered; another Cargo process holds the package cache", file=sys.stderr)
        return 0
    report_failure(limit, status is None, cargo_stderr)
    return 1


if __name__ == "__main__":
    try:
        lint.exit_with(main)
    except Terminated:
        die_of_sigterm()
