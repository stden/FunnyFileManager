"""Measure every production Python file, including subprocesses in test sandboxes.

Only canonical sources and copies made by scripts/tests/sandbox.py are traced.
Other temporary files cannot safely be credited after tests delete them.
Explicit report filenames retain production files that tests never execute.
"""

import configparser
from pathlib import Path
import re
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
CONFIGURATION = Path(__file__).with_suffix(".ini")
TEST_DIRECTORIES = {"tests", "test_support", "fixtures", "benches", "examples"}
SANDBOX_ROOT = "*/ffm-script-test-*/checkout with spaces"
SUITES = (
    ("-m", "unittest", "discover", "-s", "scripts/tests"),
    ("scripts/test_format_markdown.py",),
    ("scripts/test_recording.py",),
    ("scripts/test_lint_policy.py",),
    ("scripts/test_readability.py",),
)


def require_coverage():
    """Require subprocess-aware coverage.py without importing scripts/coverage.py."""
    result = subprocess.run(
        [sys.executable, "-m", "coverage", "--version"],
        cwd=ROOT, capture_output=True, text=True, check=False,
    )
    version = re.search(r"version (\d+)\.(\d+)", result.stdout)
    if result.returncode or not version or tuple(map(int, version.groups())) < (7, 10):
        sys.exit("coverage: Python coverage needs coverage>=7.10; run: python -m pip install coverage==7.17.0")


def production_files():
    """List tracked and new production Python sources; test directories alone are exempt."""
    result = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z", "--", "*.py"],
        cwd=ROOT, capture_output=True, text=True, check=True,
    )
    return sorted({
        name for name in result.stdout.split("\0")
        if name and not TEST_DIRECTORIES.intersection(Path(name).parts[:-1])
    })


def configuration(directory, sources):
    """Build exact tracing filters and aliases for known test-checkout copies."""
    config = configparser.ConfigParser(interpolation=None)
    with CONFIGURATION.open(encoding="utf-8") as template:
        config.read_file(template)
    config["run"]["data_file"] = str(directory / ".coverage")
    config["run"]["include"] = "\n".join(
        pattern for source in sources
        for pattern in (str(ROOT / source), f"{SANDBOX_ROOT}/{source}")
    )
    config["paths"] = {"project": f".\n{ROOT}\n{SANDBOX_ROOT}"}
    return config


def run(directory: Path) -> Path:
    """Run the Python suites and return a native JSON report with every source."""
    require_coverage()
    sources = production_files()
    if not sources:
        sys.exit("coverage: no production Python sources found")
    directory = directory.resolve()
    directory.mkdir(parents=True, exist_ok=True)
    config_path = directory / "python.ini"
    with config_path.open("w", encoding="utf-8") as output:
        configuration(directory, sources).write(output)
    command = [sys.executable, "-m", "coverage"]
    subprocess.run([*command, "erase", "--rcfile", str(config_path)], cwd=ROOT, check=True)
    for suite in SUITES:
        subprocess.run([*command, "run", "--rcfile", str(config_path), *suite], cwd=ROOT, check=True)
    subprocess.run([*command, "combine", "--rcfile", str(config_path)], cwd=ROOT, check=True)
    report = directory / "python.json"
    subprocess.run(
        [*command, "json", "--rcfile", str(config_path), "-o", str(report), *sources],
        cwd=ROOT, check=True,
    )
    return report
