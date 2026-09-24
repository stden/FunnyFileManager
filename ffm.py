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

import os
from pathlib import Path
import signal
import sys


ROOT = Path(__file__).resolve().parent
# scripts/ holds shell_path.py, which finds programs as bash does, and the
# launcher_* modules below. Explicitly first on sys.path: no other module of
# those names may stand in.
sys.path.insert(0, str(ROOT / "scripts"))
from shell_path import find  # noqa: E402
from launcher_build import build, launch  # noqa: E402
from launcher_colorer import colorer_command  # noqa: E402
from launcher_install import refresh_stale_copies  # noqa: E402
from launcher_output import fail, say, status, warning  # noqa: E402
from launcher_paths import binary, env_or_home  # noqa: E402
from launcher_process import die_by, run, run_or_exit  # noqa: E402

PREFLIGHT = ROOT / "scripts/cargo_preflight.py"
LINT = ROOT / "scripts/lint.py"
COVERAGE = ROOT / "scripts/coverage.py"
HELP_WORDS = ("help", "-h", "--help")


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
