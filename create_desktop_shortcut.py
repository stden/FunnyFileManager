#!/usr/bin/env python3
"""Create a Linux desktop entry that launches FunnyFileManager in a terminal.

Usage: ./create_desktop_shortcut.py

Builds target/release/ffm first when it is missing, writes
funny-file-manager.desktop to the desktop directory ($HOME/Desktop, else
"$HOME/Рабочий стол", else what `xdg-user-dir DESKTOP` answers), and copies it
into ~/.local/share/applications when that directory exists. Stops with
status 1 when the entry cannot be written.
Arguments other than -h/--help are ignored.
Windows uses create-desktop-shortcut.ps1 instead.
"""

import argparse
import errno
import os
from pathlib import Path
import signal
import stat
import subprocess
import sys


# Symlinks resolved: bash named the checkout by the path it was reached through.
ROOT = Path(__file__).resolve().parent
# scripts/shell_path.py finds programs as bash does. Explicitly first on
# sys.path: no other `shell_path` module may stand in.
sys.path.insert(0, str(ROOT / "scripts"))
import shell_path  # noqa: E402

# A fixed path: the entry points at this checkout's build, whatever
# CARGO_TARGET_DIR says.
FFM_PATH = ROOT / "target" / "release" / "ffm"
ENTRY_NAME = "funny-file-manager.desktop"
# The desktop folder's name on a Russian-language desktop. Decoded from its
# UTF-8 bytes, as bash compared them, so it is found under any locale.
RUSSIAN_DESKTOP = os.fsdecode("Рабочий стол".encode("utf-8"))
VALIDATOR = "desktop-file-validate"
ENTRY = """\
[Desktop Entry]
Version=1.0
Type=Application
Name=Funny File Manager
Comment=Terminal-based file manager inspired by Far Manager
Exec={ffm}
Path={root}
Icon=utilities-file-manager
Terminal=true
Categories=Utility;FileManager;
"""


def xdg_desktop():
    """What `xdg-user-dir DESKTOP` answers; None when it is missing, fails or is silent."""
    try:
        result = subprocess.run(
            ["xdg-user-dir", "DESKTOP"],
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    # An empty answer is ignored: bash took it and wrote /funny-file-manager.desktop.
    return os.fsdecode(result.stdout.rstrip(b"\n")) or None


def desktop_directory():
    """$HOME/Desktop, else its Russian name, else xdg-user-dir's answer, else $HOME/Desktop."""
    home = Path.home()
    for candidate in (home / "Desktop", home / RUSSIAN_DESKTOP):
        if candidate.is_dir():
            return candidate
    answer = xdg_desktop()
    return Path(answer) if answer else home / "Desktop"


def find_program(name):
    """The file the shell runs for `name`: an executable on PATH, else any file there.

    Like the shell's search, this also finds a file without execute permission.
    """
    return shell_path.find(name) or shell_path.find(name, os.F_OK)


def run_program(path, *args):
    """Run the program at `path` to the end; its status is not checked.

    A program that cannot start is reported as the shell reports it, and the
    script goes on.
    """
    try:
        subprocess.run([path, *args], check=False)
    except OSError as error:
        # TODO(Rule of Three): install_mcp.py words the shell's report the same way.
        missing = error.errno == errno.ENOENT
        reason = "cannot execute: required file not found" if missing else error.strerror
        print(f"{path}: {reason}", file=sys.stderr)


def ensure_built():
    """Build target/release/ffm when it is missing; True once the file exists."""
    if FFM_PATH.is_file():
        return True
    print(f"FFM executable not found at {FFM_PATH}")
    print("Building FFM in release mode...")
    # bash's `cd` into the checkout, which outlasts the build: a relative
    # desktop directory now resolves there, and later commands start there.
    os.chdir(ROOT)
    os.environ["PWD"] = str(ROOT)
    cargo = find_program("cargo")
    if cargo:
        # Whether the binary now exists is what counts, not Cargo's status.
        run_program(cargo, "build", "--release")
    else:
        print("cargo: command not found", file=sys.stderr)
    return FFM_PATH.is_file()


def write_entry(desktop_file):
    """Write the desktop entry; False, after saying why, when it cannot be written.

    Deliberately stricter than bash, which went on and reported the entry as created.
    """
    # The entry is made of paths, so it is encoded as paths are: into the bytes
    # they have on disk, whatever the locale.
    text = ENTRY.format(ffm=FFM_PATH, root=ROOT)
    try:
        desktop_file.write_bytes(os.fsencode(text))
    except OSError as error:
        print(f"{desktop_file}: {error.strerror}", file=sys.stderr)
        return False
    return True


def add_execute_permission(path):
    """`chmod +x`: add the execute bits the umask allows.

    Like chmod's, a failure is reported and the script goes on: an ffm that
    another user owns keeps its mode.
    """
    umask = os.umask(0)
    os.umask(umask)
    try:
        path.chmod(stat.S_IMODE(path.stat().st_mode) | (0o111 & ~umask))
    except OSError as error:
        print(f"chmod: changing permissions of '{path}': {error.strerror}", file=sys.stderr)


def validate(desktop_file):
    """Run desktop-file-validate where `command -v` finds it; its verdict is advisory."""
    validator = find_program(VALIDATOR)
    if validator:
        run_program(validator, desktop_file)


def copy_into(directory, source):
    """`cp source directory/`; returns the copy's path.

    As with cp, an existing copy keeps its mode, a new one gets the source's
    less the umask, and a failure is reported while the script goes on.
    """
    copy = directory / source.name
    try:
        mode = source.stat().st_mode & 0o777
        with open(os.open(copy, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, mode), "wb") as target:
            target.write(source.read_bytes())
    except IsADirectoryError:
        print(f"cp: cannot overwrite directory '{copy}' with non-directory", file=sys.stderr)
    except OSError as error:
        print(f"cp: cannot create regular file '{copy}': {error.strerror}", file=sys.stderr)
    return copy


def main(argv=None):
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
        allow_abbrev=False,
    )
    # Like the bash script, anything but -h/--help is ignored.
    parser.parse_known_args(argv)
    # A path that is not UTF-8 goes out as its own bytes, as echo wrote it.
    sys.stdout.reconfigure(errors="surrogateescape", line_buffering=True)
    sys.stderr.reconfigure(errors="surrogateescape")

    desktop_file = desktop_directory() / ENTRY_NAME
    if not ensure_built():
        print("Failed to build FFM. Please run 'cargo build --release' manually.")
        return 1

    if not write_entry(desktop_file):
        return 1
    add_execute_permission(desktop_file)
    add_execute_permission(FFM_PATH)
    print(f"Desktop entry created at: {desktop_file}")

    # For systems that require desktop file validation.
    validate(desktop_file)

    # Optional: install to the applications menu, never creating its directory.
    # As in bash, the line is printed even when cp reported a failure.
    applications = Path.home() / ".local" / "share" / "applications"
    if applications.is_dir():
        copy = copy_into(applications, desktop_file)
        print(f"Application menu entry created at: {copy}")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        # Ctrl+C: end quietly, killed by SIGINT as the bash script was.
        signal.signal(signal.SIGINT, signal.SIG_DFL)
        os.kill(os.getpid(), signal.SIGINT)
