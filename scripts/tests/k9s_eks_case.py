"""The sandbox checkout, the fake aws effects and the expected messages that
the tests of scripts/k9s_eks.py share; nothing reaches AWS.
"""

import sys
import unittest

from sandbox import Sandbox
from script_files import K9S_EKS


SCRIPT = "scripts/k9s_eks.py"
REGION_REQUIRED = (
    "k9s-eks: region required (pass as arg or set AWS_REGION/AWS_DEFAULT_REGION/EKS_REGION)\n"
)
NO_CONTEXT = (
    "k9s-eks: unable to determine kube context from aws output; "
    "pass an alias or use kubectl to select context\n"
)
ADDED = "Added new context prod@us-east-1 to /sandbox/.kube/config\n"
UPDATE_PROD = ["eks", "update-kubeconfig", "--name", "prod", "--region", "us-east-1"]


def listing(region, *profile):
    """aws's arguments to list the clusters of `region`."""
    return ["eks", "list-clusters", "--region", region,
            "--output", "text", "--query", "clusters[]", *profile]


LIST_WEST = listing("us-west-2")
# The tool each context subcommand runs, and its arguments before the context.
CONTEXT_TOOLS = {"use": ["kubectl", "config", "use-context"], "open": ["k9s", "--context"]}
# What AWS CLI versions without `update-kubeconfig --alias` print for it.
ALIAS_REJECTIONS = (
    "\nUnknown options: --alias, prod@us-east-1\n",
    "aws: error: unknown option --alias\n",
    "usage: aws [options]\naws: error: unrecognized arguments:\n  --alias prod@us-east-1\n",
)
# Run the script with its output in files, to compare the exact bytes.
TO_FILES = '"$0" "$@" >"$OUT" 2>"$ERR" </dev/null'


def old_aws_cli(rejection, retry_output, retry_status=0):
    """An aws effect that rejects --alias and answers the retry without it.

    `retry_output` may be bytes, for output that is not valid UTF-8.
    """
    if isinstance(retry_output, str):
        retry_output = retry_output.encode()
    return f"""
        if "--alias" in args:
            sys.stdout.write({rejection!r})
            sys.exit(252)
        sys.stdout.buffer.write({retry_output!r})
        sys.exit({retry_status})
    """


def killed_by(signal_number):
    """A fake's effect: die of `signal_number`: SIGKILL, SIGTERM or SIGPIPE.

    Python ignores SIGPIPE, so the fake first restores its default action.
    """
    return ("import signal\nsignal.signal(signal.SIGPIPE, signal.SIG_DFL)\n"
            f"os.kill(os.getpid(), {int(signal_number)})")


def writes(data):
    """A fake's effect: write `data`, bytes that need not be UTF-8, to stdout."""
    return f"sys.stdout.buffer.write({data!r})"


class EksTestCase(unittest.TestCase):
    def setUp(self):
        self.sandbox = Sandbox(self)
        # The script, its modules, and shell_path.py, through which it finds
        # aws, kubectl, k9s and fzf.
        for relative in K9S_EKS:
            self.sandbox.copy(relative)

    def run_script(self, *args, env=None):
        return self.sandbox.run(SCRIPT, *args, env=env, input="")

    def run_for_bytes(self, *args):
        """Run the script; return its status, stdout and stderr, as bytes."""
        stdout, stderr = self.sandbox.root / "stdout", self.sandbox.root / "stderr"
        result = self.sandbox.execute(
            ["sh", "-c", TO_FILES, sys.executable, self.sandbox.repo / SCRIPT, *args],
            env={"OUT": str(stdout), "ERR": str(stderr)},
        )
        return result.returncode, stdout.read_bytes(), stderr.read_bytes()

    def without(self, name):
        """An environment in which `name` is no command at all."""
        (self.sandbox.bin / name).unlink(missing_ok=True)
        return {"PATH": str(self.sandbox.bin)}

    def assert_result(self, result, status, stdout="", stderr=""):
        self.assertEqual((result.returncode, result.stdout, result.stderr), (status, stdout, stderr))

    def forget_calls(self):
        """Start a subtest with an empty call log."""
        self.sandbox.log.unlink(missing_ok=True)
