"""Exercise coverage when Cargo passes long MSVC compiler commands in a response file."""

import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import coverage_rust


@unittest.skipUnless(os.name == "nt" and shutil.which("cargo-llvm-cov"), "requires Windows Rust coverage tools")
class WindowsRustCoverageTests(unittest.TestCase):
    def test_response_file_instruments_application_and_build_script(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root / "src").mkdir()
            (root / ".cargo").mkdir()
            (root / "Cargo.toml").write_text('[package]\nname = "probe"\nversion = "0.1.0"\nedition = "2024"\n')
            (root / "build.rs").write_text('fn main() { println!("cargo:rerun-if-changed=build.rs"); }\n')
            (root / "src/main.rs").write_text('fn main() { println!("hello"); }\n#[test] fn execute() { main(); }\n')
            # Exceed the 32,767 UTF-16 code-unit limit on a Windows command line.
            flags = [f"--cfg=coverage_fixture_argument_{number}" for number in range(1200)]
            (root / ".cargo/config.toml").write_text(f"[build]\nrustflags = {json.dumps(flags)}\n")
            artifacts = root / "reports"
            environment = {"FFM_COVERAGE_SUITES": "root-all", "CARGO_LLVM_COV_TARGET_DIR": str(artifacts / "target")}
            with patch.object(coverage_rust.lint_steps, "ROOT", root), patch.dict(os.environ, environment):
                trace, = coverage_rust.run(artifacts)
            records = trace.read_text().splitlines()
            for relative in ("src/main.rs", "build.rs"):
                self.assertIn(f"SF:{root / relative}", records)
            hits = [int(record.rsplit(",", 1)[1]) for record in records if record.startswith("DA:")]
            self.assertGreaterEqual(len(hits), 3)
            self.assertGreater(min(hits), 0)
