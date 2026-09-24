"""Run developer scripts against a throwaway HOME, checkout and PATH.

Every test that runs a repository script, or a command such a script calls,
goes through a Sandbox, so no test can reach the real machine:

- HOME, TMPDIR and the checkout the script runs from are fresh directories.
- The environment is built from scratch, so nothing leaks in from the caller:
  no CARGO_TARGET_DIR, RUSTC_WRAPPER, AWS_PROFILE, KUBECONFIG or SKIP_CLIPPY.
- PATH is the sandbox's own bin directory, then /usr/bin and /bin only.
- Every command in GUARDED starts as a refusing stub in that bin directory,
  ahead of any real one. A test replaces the commands it expects with fakes.
- Git may use local repositories only (GIT_ALLOW_PROTOCOL=file).
- A real ffm in a system PATH directory skips the test, because
  `./ffm.py install` refreshes every ffm it finds on PATH.

Fakes are small Python programs. Each call appends one JSON line (name,
arguments, working directory, environment) to a log that calls() reads back.
"""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import textwrap
from typing import NamedTuple
import unittest


REPO = Path(__file__).resolve().parents[2]
SYSTEM_PATH = ("/usr/bin", "/bin")
# Exit status of a guarded command that the test did not fake.
REFUSED = 97
GUARDED = (
    # AI tools, cloud and cluster clients.
    "claude", "codex", "aws", "kubectl", "k9s", "fzf",
    # The Rust toolchain: builds are faked, never run.
    "cargo", "cargo-llvm-cov", "cargo-nextest", "rustup", "rustc", "clippy-driver", "sccache", "cmake",
    # The file manager itself: `./ffm.py install` overwrites every ffm on PATH.
    "ffm", "ffm-mcp",
    # Desktop integration, scheduling and privilege.
    "notify-send", "xdg-open", "xdg-user-dir", "xdg-mime", "xdg-desktop-menu",
    "desktop-file-validate", "crontab", "systemctl", "sudo",
    # Version control and the network.
    "git", "gh", "curl", "wget", "ssh", "scp",
)

FAKE = """\
#!{python}
import json, os, sys
args = sys.argv[1:]
call = {{"name": {name!r}, "args": args, "cwd": os.getcwd(), "env": dict(os.environ)}}
with open({log!r}, "a", encoding="utf-8") as log:
    log.write(json.dumps(call) + "\\n")
{effect}
sys.stdout.write({stdout!r})
sys.stderr.write({stderr!r})
sys.exit({status!r})
"""


class Call(NamedTuple):
    """One recorded run of a fake command."""

    name: str
    args: list[str]
    cwd: str
    env: dict[str, str]


class Sandbox:
    """Fresh directories and fake commands for one test, removed after it."""

    def __init__(self, test):
        if os.name == "nt":
            test.skipTest("fake commands are POSIX executables")
        if any(character.isspace() for character in sys.executable):
            test.skipTest("a fake's #! line cannot hold an interpreter path with spaces")
        if any(Path(directory, "ffm").exists() for directory in SYSTEM_PATH):
            test.skipTest("a real ffm on the system PATH: `./ffm.py install` would overwrite it")
        self.root = Path(tempfile.mkdtemp(prefix="ffm-script-test-")).resolve()
        test.addCleanup(shutil.rmtree, self.root, ignore_errors=True)
        self.home = self.root / "home"
        self.bin = self.root / "bin"
        self.tmp = self.root / "tmp"
        # The space catches paths that a script forgets to quote.
        self.repo = self.root / "checkout with spaces"
        for directory in (self.home, self.bin, self.tmp, self.repo):
            directory.mkdir()
        self.log = self.root / "calls.jsonl"
        for name in GUARDED:
            self.fake(name, status=REFUSED, stderr=f"sandbox: {name} is not faked by this test\n")

    def fake(self, name, *, status=0, stdout="", stderr="", effect="", path=None):
        """Install a recording fake: on PATH as `name`, or at `path` if given.

        `effect` is Python run after the call is recorded, with `args` (the
        argument list), `os` and `sys` in scope. It may exit with its own status.
        """
        target = Path(path) if path else self.bin / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(
            FAKE.format(
                python=sys.executable, name=name, log=str(self.log),
                effect=textwrap.dedent(effect).strip(),
                stdout=stdout, stderr=stderr, status=status,
            ),
            encoding="utf-8",
        )
        target.chmod(0o755)
        return target

    def tilde_path(self, *names):
        """Move the fakes `names` to ~/tools; return a PATH that reaches them
        only through a literal `~/tools` entry, which bash expands."""
        tools = self.home / "tools"
        tools.mkdir(exist_ok=True)
        for name in names:
            os.replace(self.bin / name, tools / name)
        return os.pathsep.join(["~/tools", str(self.bin)])

    def use_real(self, name):
        """Drop the refusing stub, so `name` resolves to the system's command."""
        (self.bin / name).unlink(missing_ok=True)

    def copy(self, relative):
        """Copy a repository file into the sandbox checkout, keeping its mode."""
        target = self.repo / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(REPO / relative, target)
        return target

    def write(self, relative, text=""):
        """Create a file in the sandbox checkout, with its parent directories."""
        target = self.repo / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")
        return target

    def env(self, extra=None):
        """The complete environment a sandboxed command gets, plus `extra`."""
        environment = {
            "PATH": os.pathsep.join([str(self.bin), *SYSTEM_PATH]),
            "HOME": str(self.home),
            "TMPDIR": str(self.tmp),
            "PYTHONUTF8": "1",
            "PYTHONDONTWRITEBYTECODE": "1",
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_ALLOW_PROTOCOL": "file",
            "GIT_TERMINAL_PROMPT": "0",
            "GIT_AUTHOR_NAME": "Sandbox",
            "GIT_AUTHOR_EMAIL": "sandbox@example.com",
            "GIT_COMMITTER_NAME": "Sandbox",
            "GIT_COMMITTER_EMAIL": "sandbox@example.com",
        }
        environment.update(extra or {})
        return environment

    def execute(self, command, *, cwd=None, env=None, input=None, timeout=60):
        """Run any command in the sandbox; the checkout copy is the default cwd."""
        return subprocess.run(
            [str(part) for part in command],
            cwd=cwd or self.repo,
            env=self.env(env),
            input=input,
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=timeout,
            check=False,
        )

    def run(self, relative, *args, **options):
        """Run a copied Python script with this interpreter, as its tests do."""
        return self.execute([sys.executable, self.repo / relative, *args], **options)

    def calls(self, name=None):
        """Every recorded call of a fake, oldest first; only `name`'s if given."""
        if not self.log.exists():
            return []
        lines = self.log.read_text(encoding="utf-8").splitlines()
        calls = [Call(**json.loads(line)) for line in lines]
        return [call for call in calls if name in (None, call.name)]

    def args(self, name):
        """The argument lists of every call of `name`, oldest first."""
        return [call.args for call in self.calls(name)]

    def git(self, *args):
        """Run the real git in the sandbox and return its output; fail loudly."""
        result = self.execute([shutil.which("git", path=os.pathsep.join(SYSTEM_PATH)), *args])
        if result.returncode:
            raise AssertionError(f"git {' '.join(map(str, args))} failed:\n{result.stderr}")
        return result.stdout


class SandboxTestCase(unittest.TestCase):
    """A test case whose every test gets a fresh Sandbox as self.sandbox."""

    def setUp(self):
        self.sandbox = Sandbox(self)
