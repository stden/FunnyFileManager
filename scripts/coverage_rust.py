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


WINDOWS_WRAPPER_SOURCE = (
    "fn main() {\n"
    "    let mut args = std::env::args_os().skip(1);\n"
    '    let program = args.next().expect("compiler path");\n'
    '    let status = std::process::Command::new(program).args(args).status().expect("spawn compiler");\n'
    "    std::process::exit(status.code().unwrap_or(1));\n"
    "}\n"
)


def build_windows_wrapper(directory: Path) -> Path:
    """Compile a native pass-through binary so cmd.exe's 8191-char limit cannot truncate rustc."""
    source = directory / "rustc-wrapper.rs"
    wrapper = directory / "rustc-wrapper.exe"
    source.write_text(WINDOWS_WRAPPER_SOURCE, encoding="utf-8")
    execute(["rustc", str(source), "-o", str(wrapper)], os.environ)
    return wrapper


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
        wrapper = build_windows_wrapper(directory)
    else:
        wrapper = shell_path.find("env")
        if not wrapper:
            raise OSError("coverage: cannot find a pass-through compiler wrapper; set CARGO_BUILD_RUSTC_WRAPPER")
    environment.pop("RUSTC_WRAPPER", None)
    environment["CARGO_BUILD_RUSTC_WRAPPER"] = str(wrapper)
    return environment


def selected_configurations(environment):
    """Return requested FFM_COVERAGE_SUITES in canonical order, or all suites."""
    raw = environment.get("FFM_COVERAGE_SUITES", "")
    wanted = {name.strip() for name in raw.split(",") if name.strip()}
    if not wanted:
        return CONFIGURATIONS
    unknown = sorted(wanted - {item[0] for item in CONFIGURATIONS})
    if unknown:
        raise OSError(f"coverage: unknown FFM_COVERAGE_SUITES: {', '.join(unknown)}")
    return tuple(item for item in CONFIGURATIONS if item[0] in wanted)


CACHEDIR_TAG = (
    "Signature: 8a477f597d28d172789f06886806bc55\n"
    "# This file is a cache directory tag created by cargo.\n"
)


def prepare_target(environment: dict[str, str]) -> None:
    """Ensure CACHEDIR.TAG for Cargo 1.99+ clean and drop cross-crate bdd binaries."""
    target = lint_steps.ROOT / environment["CARGO_LLVM_COV_TARGET_DIR"]
    target.mkdir(parents=True, exist_ok=True)
    tag = target / "CACHEDIR.TAG"
    if not tag.is_file():
        tag.write_text(CACHEDIR_TAG, encoding="utf-8")
    for stale in (target / "debug" / "deps").glob("bdd-*"):
        stale.unlink()


def run(directory: Path) -> list[Path]:
    """Export each configuration with fresh profiles; failed tests stop collection."""
    directory = directory.resolve()
    directory.mkdir(parents=True, exist_ok=True)
    environment = compiler_environment(directory)
    suites = selected_configurations(environment)
    reports = []
    for name, manifest, features in suites:
        prepare_target(environment)
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
