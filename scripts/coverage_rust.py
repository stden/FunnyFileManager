"""Collect independent LLVM line reports for every owned Rust package.

Every configuration runs the complete Cargo test suite, including custom
BDD harnesses. Only test, test-support, example and benchmark source files
are filtered. Build scripts and all production entrypoints stay measured.
Native/platform cfg code is measured when compiled on the current host;
the native Colorer CI environment supplies its external engine separately.
Unix runs require OpenSSH for the private loopback SFTP integration fixture.
"""

import os
from pathlib import Path
import subprocess

import lint_steps
import shell_path


# These conventions name test/benchmark ownership, never runtime modules.
# LLVM uses extended POSIX regex: capturing groups, with both path separators.
TEST_SOURCES = (
    r"(^|[\\/])(tests(_[^\\/]*)?|test_support|benches|examples)([\\/]|\.rs$)"
    r"|(^|[\\/])[^\\/]+_tests\.rs$"
    r"|(^|[\\/])bench_[^\\/]*([\\/]|\.rs$)"
)
CONFIGURATIONS = (
    ("root-all", "Cargo.toml", ("--all-features",)),
    ("root-fallback", "Cargo.toml", ("--no-default-features", "--features", "test-helpers")),
    ("mcp-all", "mcp/Cargo.toml", ("--all-features",)),
    ("colorer-sys-all", "colorer-sys/Cargo.toml", ("--all-features",)),
)


def compiler_environment(directory: Path) -> dict[str, str]:
    """Supply a working pass-through for Cargo's empty-wrapper configuration."""
    environment = {**os.environ, "PWD": str(lint_steps.ROOT)}
    environment.setdefault("CARGO_LLVM_COV_TARGET_DIR", str(directory / "target"))
    environment.setdefault("FFM_TEST_SFTP_SERVER", "1")
    application_name = "ffm.exe" if os.name == "nt" else "ffm"
    application = lint_steps.ROOT / environment["CARGO_LLVM_COV_TARGET_DIR"] / "debug" / application_name
    environment.setdefault("FFM_APPLICATION_TEST_BINARY", str(application))
    if environment.get("RUSTC_WRAPPER") or environment.get("CARGO_BUILD_RUSTC_WRAPPER"):
        return environment
    if os.name == "nt":
        wrapper = directory / "rustc-wrapper.cmd"
        wrapper.write_text("@echo off\n%*\n", encoding="utf-8")
    else:
        wrapper = shell_path.find("env")
        if not wrapper:
            raise OSError("coverage: cannot find a pass-through compiler wrapper; set CARGO_BUILD_RUSTC_WRAPPER")
    environment.pop("RUSTC_WRAPPER", None)
    environment["CARGO_BUILD_RUSTC_WRAPPER"] = str(wrapper)
    return environment


def run(directory: Path) -> list[Path]:
    """Export each configuration with fresh profiles; failed tests stop collection."""
    directory = directory.resolve()
    directory.mkdir(parents=True, exist_ok=True)
    environment = compiler_environment(directory)
    reports = []
    for name, manifest, features in CONFIGURATIONS:
        report = directory / f"rust-{name}.json"
        command = [
            "cargo", "llvm-cov", "test",
            "--manifest-path", str(lint_steps.ROOT / manifest),
            *features,
            "--no-cfg-coverage", "--no-fail-fast", "--include-build-script",
            "--ignore-filename-regex", TEST_SOURCES,
            "--json", "--skip-functions", "--output-path", str(report),
        ]
        execute(command, environment)
        if name == "mcp-all":
            # Cucumber has its own CLI; libtest flags belong to this target alone.
            command.extend(["--no-clean", "--test", "application_sessions", "--", "--ignored"])
            execute(command, environment)
        reports.append(export_lines(report, manifest, environment))
    return reports


def execute(command, environment):
    """Run one coverage command with the collector's private artifacts."""
    print("→", *command, flush=True)
    subprocess.run(
        command, executable=shell_path.resolve(command[0], lint_steps.ROOT),
        cwd=lint_steps.ROOT, env=environment, check=True,
    )


def export_lines(report, manifest, environment):
    """Export native DA records after the last test; retain JSON diagnostics."""
    trace = report.with_suffix(".lcov")
    execute([
        "cargo", "llvm-cov", "report", "--manifest-path", str(lint_steps.ROOT / manifest),
        "--lcov", "--skip-functions", "--include-build-script",
        "--ignore-filename-regex", TEST_SOURCES, "--output-path", str(trace),
    ], environment)
    return trace
