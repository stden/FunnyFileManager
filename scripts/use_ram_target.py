#!/usr/bin/env python3
"""Opt-in RAM-backed Cargo target directory (tmpfs).

  Usage:  eval "$(scripts/use_ram_target.py)"

Points CARGO_TARGET_DIR at /dev/shm/ffm-target (tmpfs), or at $FFM_RAM_TARGET
when that is set, so all build artifacts live in RAM. The target dir is 100%
regenerable, so losing it on reboot is fine.

This replaces `source scripts/use-ram-target.sh`. A program cannot export a
variable into the shell that started it, so it prints the export line for
your shell to eval, as ssh-agent does. That line is the only thing it prints
on stdout; its messages, and this help, go to stderr. If the directory above
the target is missing, not writable, or on a filesystem mounted noexec (cargo
could not run build scripts or test binaries from it), it prints nothing to
eval and the build keeps the on-disk ./target. If only the target itself
cannot be created (a file already has its name, say), mkdir says why and it
still prints the export line, as the shell script did. It always exits 0, so
the eval never fails your shell, and it ignores its arguments, as the sourced
shell script did, unless the first one is -h or --help.

Being a program, it sees FFM_RAM_TARGET only if your shell exports it: run
`export FFM_RAM_TARGET=/path` first, or put the assignment inside the $(...):
  eval "$(FFM_RAM_TARGET=/path scripts/use_ram_target.py)"

Why an env var and not [build] target-dir in .cargo/config.toml?
  - .cargo/config.toml is committed/shared; hardcoding /dev/shm/... there
    would break every teammate and CI that lacks /dev/shm.
  - CARGO_TARGET_DIR (env) overrides build.target-dir, so this composes
    cleanly and stays machine-local.

Honest expectation: with a large page cache the read-side is already RAM-fast,
so incremental rebuilds barely change. The real wins are write-side metadata
(thousands of tiny .o/.rmeta files) and a 2-5x faster `cargo clean`.

Make it permanent for just your machine (no shared-file changes):
  echo 'export CARGO_TARGET_DIR=/dev/shm/ffm-target' >> ~/.bashrc
"""

import os
import posixpath
import shlex
import subprocess
import sys


DEFAULT_RAM_DIR = "/dev/shm/ffm-target"
HELP_FLAGS = ("-h", "--help")


def dirname(path):
    """The directory part of `path`, exactly as dirname(1) prints it."""
    head = posixpath.dirname(path.rstrip("/")).rstrip("/")
    return head or ("/" if path.startswith("/") else ".")


def mounted_noexec(directory):
    """True if `directory` is on a filesystem mounted noexec.

    statvfs asks about the filesystem that really holds `directory`. Unlike the
    shell script's `mount | grep "on <dir> .*noexec"`, it follows symlinks,
    resolves a relative path from the working directory, sees a directory
    below a mount point, and sees only the top one of stacked mounts.
    """
    if not hasattr(os, "ST_NOEXEC"):  # the flag is Linux-only
        return False
    try:
        return bool(os.statvfs(directory).f_flag & os.ST_NOEXEC)
    except OSError:  # missing: reported as not writable instead
        return False


def free_space(path):
    """The free space where `path` lives as `df -h` shows it, such as 63G; empty if df cannot tell."""
    df = subprocess.run(
        ["df", "-h", "--output=avail", path], stdout=subprocess.PIPE, text=True, errors="replace", check=False,
    )
    # `tail -1 | tr -d ' '`: the last line, without its padding.
    return "".join(df.stdout.splitlines()[-1:]).replace(" ", "")


def main(argv=None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if argv[:1] and argv[0] in HELP_FLAGS:
        # stderr, like everything but the export line: an eval must not run it.
        print(__doc__, end="", file=sys.stderr)
        return 0
    ram_dir = os.environ.get("FFM_RAM_TARGET") or DEFAULT_RAM_DIR
    parent = dirname(ram_dir)
    if mounted_noexec(parent):
        print(f"ffm: {parent} is mounted noexec; cargo can't exec from it.", file=sys.stderr)
        print("ffm: keeping on-disk ./target.", file=sys.stderr)
        return 0
    if not (os.path.isdir(parent) and os.access(parent, os.W_OK)):
        print(f"ffm: no writable {parent}; using on-disk ./target.", file=sys.stderr)
        return 0
    # mkdir(1), not os.makedirs: it words a failure as the shell script showed it.
    subprocess.run(["mkdir", "-p", ram_dir], check=False)
    print(f"export CARGO_TARGET_DIR={shlex.quote(ram_dir)}")
    print(f"ffm: cargo target -> {ram_dir} (tmpfs, {free_space(ram_dir)} free)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    # Print FFM_RAM_TARGET byte for byte, even where it is not valid UTF-8.
    for stream in (sys.stdout, sys.stderr):
        stream.reconfigure(errors="surrogateescape")
    sys.exit(main())
