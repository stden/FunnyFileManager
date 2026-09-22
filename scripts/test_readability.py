#!/usr/bin/env python3
"""Prove shared readability limits with real Clippy, without invoking Cargo.

The selected rustup toolchain supplies both compilers. Negative fixtures must
produce all three configured diagnostics from each crate directory; equivalent
refactored fixtures must pass. A direct rustc test also compares their behavior.
"""

import json
import os
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "scripts" / "tests" / "fixtures" / "readability"
OUTPUT = ROOT / "target" / "readability-selftest"
CRATES = {"tui": ROOT, "mcp": ROOT / "mcp", "colorer": ROOT / "colorer-sys"}
LINTS = {
    "clippy::cognitive_complexity",
    "clippy::excessive_nesting",
    "clippy::too_many_lines",
}


def child_environment():
    """Exercise ancestor discovery, regardless of the caller's Cargo session."""
    environment = os.environ.copy()
    for name in ("CLIPPY_CONF_DIR", "CARGO_MANIFEST_DIR"):
        environment.pop(name, None)
    return environment


def run(command, cwd, environment):
    return subprocess.run(
        [str(part) for part in command],
        cwd=cwd,
        env=environment,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=60,
        check=False,
    )


def tool_path(name, environment):
    result = run(["rustup", "which", name], ROOT, environment)
    if result.returncode:
        raise RuntimeError(f"Cannot resolve {name} for rust-toolchain.toml:\n{result.stderr}")
    return Path(result.stdout.strip())


def diagnostics(stderr):
    codes = set()
    for line in stderr.splitlines():
        try:
            diagnostic = json.loads(line)
        except json.JSONDecodeError:
            continue
        code = diagnostic.get("code")
        if diagnostic.get("level") in ("error", "warning") and isinstance(code, dict):
            codes.add(code["code"])
    return codes


def check_fixture(driver, crate, directory, fixture, environment):
    result = run(
        [
            driver,
            "--edition=2021",
            "--crate-type=lib",
            "--emit=metadata",
            "--error-format=json",
            "--crate-name=readability_fixture",
            "-Dwarnings",
            *[f"-D{lint}" for lint in sorted(LINTS)],
            FIXTURES / f"{fixture}.rs",
            "-o",
            OUTPUT / f"{crate}-{fixture}.rmeta",
        ],
        directory,
        environment,
    )
    (OUTPUT / f"{crate}-{fixture}.jsonl").write_text(result.stderr, encoding="utf-8")
    if fixture == "refactored":
        if result.returncode:
            raise RuntimeError(f"{crate}: refactored fixture failed:\n{result.stderr}")
        return
    codes = diagnostics(result.stderr)
    if result.returncode == 0 or codes != LINTS:
        missing = ", ".join(sorted(LINTS - codes)) or "none"
        unexpected = ", ".join(sorted(codes - LINTS)) or "none"
        raise RuntimeError(
            f"{crate}: expected rejection by exactly the readability lints; "
            f"missing: {missing}; unexpected: {unexpected}\n{result.stderr}"
        )


def check_equivalence(compiler, environment):
    executable = OUTPUT / ("equivalence.exe" if os.name == "nt" else "equivalence")
    result = run(
        [compiler, "--edition=2021", "--test", FIXTURES / "equivalence.rs", "-o", executable],
        ROOT,
        environment,
    )
    if result.returncode:
        raise RuntimeError(f"Cannot compile behavior comparisons:\n{result.stderr}")
    result = run([executable, "--quiet"], ROOT, environment)
    if result.returncode:
        raise RuntimeError(f"Behavior comparisons failed:\n{result.stdout}\n{result.stderr}")
    print("PASS: complex and refactored behavior agrees (3 tests)")


def main():
    OUTPUT.mkdir(parents=True, exist_ok=True)
    environment = child_environment()
    driver = tool_path("clippy-driver", environment)
    compiler = tool_path("rustc", environment)
    for crate, directory in CRATES.items():
        check_fixture(driver, crate, directory, "complex", environment)
        check_fixture(driver, crate, directory, "refactored", environment)
        print(f"PASS: {crate} discovers shared limits; complex rejected, refactored accepted")
    check_equivalence(compiler, environment)


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.TimeoutExpired) as error:
        print(f"Readability self-test failed: {error}", file=sys.stderr)
        sys.exit(1)
