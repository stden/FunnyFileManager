#!/usr/bin/env python3
"""FunnyFileManager Build & Run Script

Usage: ./ffm.py [command] [options]

Commands:
  (no command)     Build (if needed) and run FFM in debug mode
  build [mode]     Build FFM (mode: debug|release, default: debug)
  release          Build in release mode and run
  run              Run FFM (assumes already built)
  test             Run all tests
  lint [mode]      Rust linter gate (fmt + clippy; see scripts/lint.py)
  coverage         CI's line-coverage gate (llvm-cov; see scripts/coverage.py)
  clean            Clean build artifacts
  colorer          Install Colorer + Far's syntax schemes, then rebuild
  install          Rebuild and install FFM to ~/.cargo/bin (--force)
  reinstall        Same as install
  help             Show this help message

Examples:
  ./ffm.py                  # Build and run in debug mode
  ./ffm.py release          # Build and run in release mode
  ./ffm.py build release    # Only build in release mode
  ./ffm.py test             # Run tests
  ./ffm.py lint             # fmt --check + clippy feature matrix
  ./ffm.py lint clippy-fast # all targets, default features
"""

# Builds and runs FunnyFileManager on Unix-like systems (Ubuntu, macOS, etc.).

import errno
import filecmp
import os
from pathlib import Path
import shlex
import shutil
import signal
import subprocess
import sys
from typing import NoReturn


ROOT = Path(__file__).resolve().parent
# scripts/shell_path.py finds programs as bash does. Explicitly first on
# sys.path: no other `shell_path` module may stand in.
sys.path.insert(0, str(ROOT / "scripts"))
from shell_path import find, find_all, resolve  # noqa: E402

PREFLIGHT = ROOT / "scripts/cargo_preflight.py"
LINT = ROOT / "scripts/lint.py"
COVERAGE = ROOT / "scripts/coverage.py"
COLORER_REPOSITORIES = "https://github.com/colorer"
HELP_WORDS = ("help", "-h", "--help")
# The file descriptor itself: sys.stdout is None when it was closed.
STDOUT = 1
# The status of a command that cannot start: a file it needs is missing, or another reason.
NOT_FOUND = 127
NOT_EXECUTABLE = 126
# A command killed by one of these is not reported: Ctrl-C, or a reader that went away.
QUIET_SIGNALS = (signal.SIGINT, signal.SIGPIPE)

GREEN = "\033[0;32m"
RED = "\033[0;31m"
YELLOW = "\033[1;33m"
NC = "\033[0m"  # No colour


def say(line):
    """Print one line and return 0, or 1 if it could not be written.

    The line is written at once, unbuffered, so it stays in order with the
    output of the commands around it. Paths keep the exact bytes the
    environment gave them, and this file's text stays UTF-8 in any locale.
    A failed write is reported on stderr and the script goes on; a closed
    pipe ends it by SIGPIPE, without a word.
    """
    data = f"{line}\n".encode("utf-8", "surrogateescape")
    try:
        while data:
            data = data[os.write(STDOUT, data):]
    except OSError as problem:
        print(f"{sys.argv[0]}: write error: {problem.strerror}", file=sys.stderr)
        return 1
    return 0


def status(message):
    return say(f"{GREEN}[FFM]{NC} {message}")


def error(message):
    return say(f"{RED}[ERROR]{NC} {message}")


def warning(message):
    return say(f"{YELLOW}[WARNING]{NC} {message}")


def fail(message, *hints) -> NoReturn:
    """Print an error and its hint lines, then stop with status 1."""
    error(message)
    for hint in hints:
        say(hint)
    sys.exit(1)


def die_by(signal_number):
    """End this process killed by `signal_number`, so a calling script sees it."""
    signal.signal(signal_number, signal.SIG_DFL)
    os.kill(os.getpid(), signal_number)


# TODO(DRY): scripts/lint.py (for lint, the pre-commit hook and the preflight)
# and install_mcp.py carry their own copy of this command runner. Move them
# into one shared module once every script port has landed.
def run(command, **options):
    """Run a command to the end and return its exit status.

    - A bare name is looked up as find() looks it up.
    - Ctrl-C reaches the command as well, and the command decides what it
      means: this script waits for it. If the command dies of it, so does
      this script; if the command exits instead, the script goes on.
    - A command that cannot start gets 127 when a file it needs is missing,
      126 otherwise, and the reason is printed on stderr.
    """
    executable = resolve(str(command[0]))
    interrupted = False

    def note_interrupt(_signal_number, _frame):
        nonlocal interrupted
        interrupted = True

    handler = signal.getsignal(signal.SIGINT)
    # A Ctrl-C ignored on entry, as for `ffm.py &` in a script, stays ignored.
    if handler is not signal.SIG_IGN:
        signal.signal(signal.SIGINT, note_interrupt)
    try:
        returncode = subprocess.run(command, executable=executable, check=False, **options).returncode
    except OSError as problem:
        print(f"{problem.filename}: {problem.strerror}", file=sys.stderr)
        return NOT_FOUND if problem.errno == errno.ENOENT else NOT_EXECUTABLE
    finally:
        signal.signal(signal.SIGINT, handler)
    if interrupted and returncode == -signal.SIGINT:
        die_by(signal.SIGINT)
    return exit_status(returncode, command)


def exit_status(returncode, command):
    """A finished command's status: its own, or 128 + N after signal N.

    The signal is named on stderr with the command, unless it is in QUIET_SIGNALS.
    """
    if returncode >= 0:
        return returncode
    signal_number = -returncode
    if signal_number not in QUIET_SIGNALS:
        print(f"{signal.strsignal(signal_number)}  {shlex.join(map(str, command))}", file=sys.stderr)
    return 128 + signal_number


def run_or_exit(command):
    """Run a command that must pass: any failure stops the script with status 1."""
    if run(command):
        sys.exit(1)


def env_or_home(variable, under_home):
    """$variable if it is set and not empty, else $HOME/under_home.

    Plain text: no slash is tidied away, and an unset HOME counts as empty.
    """
    return os.environ.get(variable) or f"{os.environ.get('HOME', '')}/{under_home}"


def enter_root():
    """Work from the checkout root, with PWD naming it for the programs started here."""
    os.chdir(ROOT)
    os.environ["PWD"] = str(ROOT)


def check_rust():
    """Stop unless Cargo is installed and its compiler wrapper answers."""
    if not find("cargo"):
        fail(
            "Rust is not installed!",
            "Please install Rust from https://rustup.rs/",
            "Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh",
        )
    # Fail in seconds, not minutes, when the compiler wrapper has stalled.
    run_or_exit([sys.executable, PREFLIGHT])


def binary(mode):
    """The ffm that `cargo build` makes in `mode`, relative to the checkout root."""
    return f"target/{mode}/ffm"


def build(mode):
    """Build ffm: release when `mode` is "release", debug for any other word."""
    status("Building FunnyFileManager...")
    release = ["--release"] if mode == "release" else []
    if run(["cargo", "build", *release]):
        fail("Build failed!")
    return status("Build completed successfully!")


def launch(mode, args) -> NoReturn:
    """Replace this process with target/<mode>/ffm, given `args`.

    A missing binary is built first, always in debug, and then the debug
    binary runs.
    """
    path = binary(mode)
    if not os.path.isfile(path):
        warning("FFM binary not found, building...")
        build("debug")
        path = binary("debug")
    status("Starting FunnyFileManager...")
    # The full path to exec, the short one as argv[0].
    executable = f"{ROOT}/{path}"
    try:
        os.execv(executable, [path, *args])
    except OSError as problem:
        print(f"{executable}: {problem.strerror}", file=sys.stderr)
        sys.exit(NOT_FOUND if problem.errno == errno.ENOENT else NOT_EXECUTABLE)


def same_bytes(first, second):
    """Whether two files hold the same bytes, as `cmp -s` says; unreadable is different."""
    try:
        return filecmp.cmp(first, second, shallow=False)
    except OSError:
        return False


def refresh_stale_copies(cargo_bin):
    """Overwrite every other executable ffm on PATH whose bytes differ from cargo_bin.

    A copy in /usr/local/bin outlived rebuild after rebuild and kept serving a
    UI from August: `cargo install` writes only to ~/.cargo/bin, PATH order
    alone decides which binary actually runs, and nothing warns you the two
    disagree. Whichever copy wins should be the one just built.
    """
    for other in find_all("ffm"):
        if other == cargo_bin or not os.path.isfile(other) or same_bytes(cargo_bin, other):
            continue
        try:
            shutil.copyfile(cargo_bin, other)
        except OSError:
            warning(f"Stale copy shadowing yours (not writable): {other}")
            warning(f"  sudo cp {cargo_bin} {other}")
        else:
            status(f"Refreshed stale copy: {other}")


def fetch(checkout, label):
    """Fast-forward a Colorer clone, keeping it if that fails, or make a shallow one."""
    if os.path.isdir(f"{checkout}/.git"):
        status(f"Updating {label}...")
        run(["git", "-C", checkout, "pull", "--ff-only", "--quiet"])
        return
    status(f"Cloning {label}...")
    url = f"{COLORER_REPOSITORIES}/{os.path.basename(checkout)}"
    run_or_exit(["git", "clone", "--depth", "1", url, checkout])


def build_command(args):
    check_rust()
    return build("release" if args[:1] == ["release"] else "debug")


def release_command(args):
    check_rust()
    build("release")
    launch("release", args)


def run_command(args):
    launch("debug", args)


def test_command(_args):
    check_rust()
    # Every invocation CI makes, because a feature-gated target is the one
    # that breaks unnoticed: cargo *skips* a test target whose
    # `required-features` are off rather than failing, so a plain
    # `cargo test` reports success while never compiling
    # `comprehensive_test` or either BDD suite.
    status("Running tests...")
    run_or_exit(["cargo", "test", "--features", "test-helpers"])
    status("Running BDD suites...")
    run_or_exit(["cargo", "test", "--features", "test-helpers,s3", "--test", "bdd", "--test", "bdd_ru"])
    # `keepass` is enabled by default and covered above; retain this
    # focused invocation for a separate backend-test result.
    status("Running keepass feature tests...")
    run_or_exit(["cargo", "test", "--features", "keepass", "--lib", "keepass"])
    return 0


def lint_command(args):
    check_rust()
    status("Running Rust linter gate (fmt + spelling + clippy)...")
    mode = args[0] if args and args[0] else "all"
    return run([sys.executable, LINT, mode])


def coverage_command(_args):
    check_rust()
    status("Running test coverage...")
    return run([sys.executable, COVERAGE])


def clean_command(_args):
    status("Cleaning build artifacts...")
    return run(["cargo", "clean"])


def install_command(_args):
    """Rebuild from source and install over whatever is already on PATH.

    --force is not optional here: without it `cargo install --path .` refuses
    to overwrite an install of the same version, so a rebuild after a source
    change would silently leave the OLD binary on PATH.
    """
    check_rust()
    status("Rebuilding and installing FunnyFileManager to ~/.cargo/bin...")
    if run(["cargo", "install", "--path", ".", "--force"]):
        fail("Install failed!")
    cargo_bin = f"{env_or_home('CARGO_HOME', '.cargo')}/bin/ffm"
    status(f"Installed: {cargo_bin}")
    if not find("ffm"):
        warning("~/.cargo/bin is not on your PATH — add it to use 'ffm' directly:")
        return say('  export PATH="$HOME/.cargo/bin:$PATH"')
    refresh_stale_copies(cargo_bin)
    status(f"'ffm' resolves to: {find('ffm')}")
    return warning("Run 'hash -r' in shells opened before now — bash caches the old path.")


def colorer_command(_args):
    """Install Colorer, the engine of Far's FarColorer plugin, plus Far's schemes.

    Kept out of `cargo build` on purpose. Cloning and CMake-building a C++
    library would turn every build of the file manager into something that
    needs the network and several minutes; this does it once, deliberately,
    and the build script then finds the result.
    """
    if not find("cmake"):
        fail(
            "cmake is required to build Colorer.",
            "  Debian/Ubuntu: sudo apt-get install cmake g++ libicu-dev libxml2-dev zlib1g-dev libminizip-dev",
        )
    colorer_home = f"{env_or_home('XDG_DATA_HOME', '.local/share')}/ffm"
    library = f"{colorer_home}/Colorer-library"
    try:
        os.makedirs(colorer_home, exist_ok=True)
    except OSError as problem:
        # Reported, not fatal: the clone that follows fails with its own reason.
        print(f"mkdir: cannot create directory '{problem.filename}': {problem.strerror}", file=sys.stderr)
    fetch(library, "Colorer-library")
    status("Building Colorer-library (this takes a few minutes)...")
    configure = [
        "cmake", "-S", library, "-B", f"{library}/_build", "-DCMAKE_BUILD_TYPE=Release",
        "-DCOLORER_USE_VCPKG=OFF", "-DCOLORER_BUILD_TOOLS=OFF", "-DCOLORER_BUILD_TEST=OFF",
    ]
    if run(configure, stdout=subprocess.DEVNULL):
        fail("CMake configure failed. Missing libicu-dev / libxml2-dev / zlib1g-dev / libminizip-dev?")
    run_or_exit(["cmake", "--build", f"{library}/_build", "--parallel"])
    fetch(f"{colorer_home}/Colorer-schemes", "the syntax schemes")
    status(f"Colorer installed under {colorer_home}")
    return status("Rebuild to pick it up:  ./ffm.py install")


def default_command(args):
    """Build the debug binary if it is missing, then run it with every argument."""
    check_rust()
    if not os.path.isfile(binary("debug")):
        build("debug")
    launch("debug", args)


COMMANDS = {
    "build": build_command,
    "release": release_command,
    "run": run_command,
    "test": test_command,
    "lint": lint_command,
    "coverage": coverage_command,
    "clean": clean_command,
    "install": install_command,
    "reinstall": install_command,
    "colorer": colorer_command,
}


def main(argv=None) -> int:
    """Run the command named by the first argument; any other first word goes to ffm."""
    argv = sys.argv[1:] if argv is None else argv
    # CPython starts with both ignored, and exec would hand that on to ffm.
    # A shell leaves them at their defaults, so a closed pipe ends this too.
    signal.signal(signal.SIGPIPE, signal.SIG_DFL)
    signal.signal(signal.SIGXFSZ, signal.SIG_DFL)
    enter_root()
    command = argv[0] if argv else None
    if command in HELP_WORDS:
        return say(__doc__.strip())
    if command in COMMANDS:
        return COMMANDS[command](argv[1:])
    return default_command(argv)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        # Ctrl+C while no command ran: end by SIGINT, without a traceback.
        die_by(signal.SIGINT)
