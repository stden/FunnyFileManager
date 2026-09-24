"""Test the EKS helper against fake aws, kubectl, k9s and fzf; nothing reaches AWS."""

import os
import signal
import subprocess
import sys
import time
import unittest

from sandbox import Sandbox


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
        self.sandbox.copy(SCRIPT)
        # The script finds aws, kubectl, k9s and fzf through it.
        self.sandbox.copy("scripts/shell_path.py")

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


class ListTests(EksTestCase):
    def test_prints_one_cluster_per_line(self):
        self.sandbox.fake("aws", stdout="alpha\tbeta\tgamma\n")
        result = self.run_script("ls", "us-west-2", "ops")
        self.assert_result(result, 0, stdout="alpha\nbeta\ngamma\n")
        self.assertEqual(self.sandbox.args("aws"), [[*LIST_WEST, "--profile", "ops"]])

    def test_region_and_profile_default_to_the_first_non_empty_variable(self):
        self.sandbox.fake("aws")
        cases = [
            ({"EKS_REGION": "r1", "AWS_REGION": "r2", "AWS_DEFAULT_REGION": "r3",
              "EKS_PROFILE": "p1", "AWS_PROFILE": "p2"}, ["--region", "r1"], ["--profile", "p1"]),
            ({"EKS_REGION": "", "AWS_REGION": "r2", "AWS_DEFAULT_REGION": "r3",
              "EKS_PROFILE": "", "AWS_PROFILE": "p2"}, ["--region", "r2"], ["--profile", "p2"]),
            ({"AWS_REGION": "", "AWS_DEFAULT_REGION": "r3", "AWS_PROFILE": ""},
             ["--region", "r3"], []),
        ]
        for environment, region, profile in cases:
            # An empty argument counts as a missing one.
            for args in (["ls"], ["ls", "", ""]):
                with self.subTest(environment=environment, args=args):
                    self.forget_calls()
                    self.assert_result(self.run_script(*args, env=environment), 0)
                    expected = ["eks", "list-clusters", *region,
                                "--output", "text", "--query", "clusters[]", *profile]
                    self.assertEqual(self.sandbox.args("aws"), [expected])

    def test_arguments_win_over_the_environment(self):
        self.sandbox.fake("aws")
        self.run_script("ls", "us-west-2", "ops", env={"EKS_REGION": "r1", "EKS_PROFILE": "p1"})
        self.assertEqual(self.sandbox.args("aws"), [[*LIST_WEST, "--profile", "ops"]])

    def test_a_missing_region_stops_before_aws(self):
        self.assert_result(self.run_script("ls"), 2, stderr=REGION_REQUIRED)
        self.assertEqual(self.sandbox.calls(), [])

    def test_a_missing_aws_exits_127(self):
        result = self.run_script("ls", "us-west-2", env=self.without("aws"))
        self.assert_result(result, 127, stderr="k9s-eks: missing required command: aws\n")

    def test_an_aws_failure_is_the_exit_status_after_what_aws_printed(self):
        self.sandbox.fake("aws", status=254, stdout="alpha\tbeta\n", stderr="paging failed\n")
        result = self.run_script("ls", "us-west-2")
        self.assert_result(result, 254, stdout="alpha\nbeta\n", stderr="paging failed\n")

    def test_an_aws_killed_by_a_signal_ends_with_the_shell_status(self):
        self.sandbox.fake("aws", effect=killed_by(signal.SIGKILL))
        self.assert_result(self.run_script("ls", "us-west-2"), 128 + signal.SIGKILL)

    def test_the_output_is_passed_on_byte_for_byte(self):
        # Only tabs change: no newline translation, and no need for UTF-8.
        self.sandbox.fake("aws", effect=writes(b"al\xffpha\tbeta\r\ngam\rma\n"))
        self.assertEqual(self.run_for_bytes("ls", "us-west-2"),
                         (0, b"al\xffpha\nbeta\r\ngam\rma\n", b""))

    def test_a_reader_that_stops_early_ends_it_quietly(self):
        # More than a pipe holds, so the script is still writing when head leaves.
        self.sandbox.fake("aws", effect="sys.stdout.write('cluster\\t' * 100000)")
        stderr = self.sandbox.root / "stderr"
        result = self.sandbox.execute(
            ["bash", "-c", '"$0" "$1" ls us-west-2 2>"$ERR" | head -c1 >/dev/null; '
             'echo "${PIPESTATUS[0]}"', sys.executable, self.sandbox.repo / SCRIPT],
            env={"ERR": str(stderr)},
        )
        # The status of `aws | tr` when SIGPIPE killed tr, and nothing on stderr.
        self.assertEqual((result.stdout, stderr.read_text(encoding="utf-8")),
                         (f"{128 + signal.SIGPIPE}\n", ""))


class ContextTests(EksTestCase):
    def test_use_adds_the_alias_context_and_switches_kubectl(self):
        self.sandbox.fake("aws", stdout=ADDED)
        self.sandbox.fake("kubectl", stdout='Switched to context "prod@us-east-1".\n')
        result = self.run_script("use", "prod", "us-east-1", "ops")
        self.assert_result(result, 0, stdout='Switched to context "prod@us-east-1".\n', stderr=ADDED)
        self.assertEqual(self.sandbox.args("aws"),
                         [[*UPDATE_PROD, "--profile", "ops", "--alias", "prod@us-east-1"]])
        self.assertEqual(self.sandbox.args("kubectl"), [["config", "use-context", "prod@us-east-1"]])
        self.assertEqual(self.sandbox.calls("k9s"), [])

    def test_aws_and_kubectl_behind_a_tilde_path_entry_are_found_as_bash_finds_them(self):
        self.sandbox.fake("aws", stdout=ADDED)
        self.sandbox.fake("kubectl")
        path = self.sandbox.tilde_path("aws", "kubectl")
        result = self.run_script("use", "prod", "us-east-1", env={"PATH": path})
        self.assert_result(result, 0, stderr=ADDED)
        self.assertEqual([call.name for call in self.sandbox.calls()], ["aws", "kubectl"])

    def test_open_starts_k9s_on_the_named_alias(self):
        self.sandbox.fake("aws", stdout=ADDED)
        self.sandbox.fake("k9s")
        self.assert_result(self.run_script("open", "prod", "us-east-1", "", "admin"), 0, stderr=ADDED)
        self.assertEqual(self.sandbox.args("aws"), [[*UPDATE_PROD, "--alias", "admin"]])
        self.assertEqual(self.sandbox.args("k9s"), [["--context", "admin"]])
        self.assertEqual(self.sandbox.calls("kubectl"), [])

    def test_the_tool_status_is_the_exit_status(self):
        self.sandbox.fake("aws", stdout=ADDED)
        for subcommand, (tool, *arguments) in CONTEXT_TOOLS.items():
            with self.subTest(subcommand=subcommand):
                self.forget_calls()
                self.sandbox.fake(tool, status=3)
                self.assertEqual(self.run_script(subcommand, "prod", "us-east-1").returncode, 3)
                self.assertEqual(self.sandbox.args(tool), [[*arguments, "prod@us-east-1"]])

    def test_a_tool_killed_by_a_signal_is_reported_as_the_shell_did(self):
        self.sandbox.fake("aws", stdout=ADDED)
        # The shell names the signal, except SIGPIPE, and reports status 128+N.
        reports = {signal.SIGKILL: "Killed\n", signal.SIGTERM: "Terminated\n", signal.SIGPIPE: ""}
        for subcommand, (tool, *_) in CONTEXT_TOOLS.items():
            for signal_number, report in reports.items():
                with self.subTest(subcommand=subcommand, signal=signal_number):
                    self.sandbox.fake(tool, effect=killed_by(signal_number))
                    result = self.run_script(subcommand, "prod", "us-east-1")
                    self.assert_result(result, 128 + signal_number, stderr=ADDED + report)

    def test_an_aws_killed_by_a_signal_ends_with_the_shell_status(self):
        self.sandbox.fake("aws", effect=killed_by(signal.SIGKILL))
        for subcommand, (tool, *_) in CONTEXT_TOOLS.items():
            with self.subTest(subcommand=subcommand):
                self.sandbox.fake(tool)
                result = self.run_script(subcommand, "prod", "us-east-1")
                self.assert_result(result, 128 + signal.SIGKILL, stderr="\n")
                self.assertEqual(self.sandbox.calls(tool), [])

    def test_region_profile_and_kubeconfig_come_from_the_environment(self):
        self.sandbox.fake("aws")
        self.sandbox.fake("kubectl")
        environment = {"AWS_REGION": "eu-west-1", "AWS_PROFILE": "dev", "KUBECONFIG": "/k/config"}
        self.assert_result(self.run_script("use", "prod", env=environment), 0, stderr="\n")
        self.assertEqual(self.sandbox.args("aws"), [[
            "eks", "update-kubeconfig", "--name", "prod", "--region", "eu-west-1",
            "--profile", "dev", "--kubeconfig", "/k/config", "--alias", "prod@eu-west-1",
        ]])

    def test_the_context_drops_trailing_newlines_like_the_shell(self):
        # The shell passed the context through $(...); aws still got the alias as typed.
        self.sandbox.fake("aws")
        for subcommand, (tool, *arguments) in CONTEXT_TOOLS.items():
            with self.subTest(subcommand=subcommand):
                self.forget_calls()
                self.sandbox.fake(tool)
                self.run_script(subcommand, "prod", "us-east-1", "", "admin\n\n")
                self.assertEqual(self.sandbox.args("aws"), [[*UPDATE_PROD, "--alias", "admin\n\n"]])
                self.assertEqual(self.sandbox.args(tool), [[*arguments, "admin"]])

    def test_the_aws_output_is_echoed_byte_for_byte(self):
        self.sandbox.fake("aws", effect=writes(b"Added context \xe9t\xe9\r\n\n"))
        self.sandbox.fake("kubectl")
        self.assertEqual(self.run_for_bytes("use", "prod", "us-east-1"),
                         (0, b"", b"Added context \xe9t\xe9\r\n"))
        self.assertEqual(self.sandbox.args("kubectl"), [["config", "use-context", "prod@us-east-1"]])

    def test_an_old_aws_cli_is_asked_again_without_the_alias(self):
        self.sandbox.fake("kubectl")
        updated = "Updated kubeconfig\nUpdated context first in /a\nUpdated context second in /b\n"
        for rejection in ALIAS_REJECTIONS:
            with self.subTest(rejection=rejection):
                self.forget_calls()
                self.sandbox.fake("aws", effect=old_aws_cli(rejection, updated))
                self.assert_result(self.run_script("use", "prod", "us-east-1"), 0, stderr=updated)
                self.assertEqual(self.sandbox.args("aws"),
                                 [[*UPDATE_PROD, "--alias", "prod@us-east-1"], UPDATE_PROD])
                self.assertEqual(self.sandbox.args("kubectl"), [["config", "use-context", "first"]])

    def test_the_context_named_by_an_old_aws_cli_is_taken_byte_for_byte(self):
        # As with sed, a line ends at "\n" only, and the name is any bytes up to a space.
        self.sandbox.fake("kubectl")
        for output, context in (
            (b"Updated context abc\r\n", "abc\r"),
            (b"Updated context a\x0cb in /k\n", "a\x0cb"),
            (b"Updated context a\x0bb in /k\n", "a\x0bb"),
            ("Updated context a b in /k\n".encode(), "a b"),
            (b"Updated context a\xffb in /k\n", os.fsdecode(b"a\xffb")),
        ):
            with self.subTest(output=output):
                self.forget_calls()
                self.sandbox.fake("aws", effect=old_aws_cli(ALIAS_REJECTIONS[0], output))
                self.assertEqual(self.run_for_bytes("use", "prod", "us-east-1"),
                                 (0, b"", output))
                self.assertEqual(self.sandbox.args("kubectl"), [["config", "use-context", context]])

    def test_a_failed_retry_ends_with_its_status(self):
        self.sandbox.fake("aws", effect=old_aws_cli(ALIAS_REJECTIONS[0], "denied\n", retry_status=3))
        self.assert_result(self.run_script("use", "prod", "us-east-1"), 3, stderr="denied\n")
        self.assertEqual(len(self.sandbox.calls("aws")), 2)
        self.assertEqual(self.sandbox.calls("kubectl"), [])

    def test_a_retry_that_names_no_context_fails(self):
        self.sandbox.fake("aws", effect=old_aws_cli(ALIAS_REJECTIONS[0], "Updated kubeconfig\n"))
        result = self.run_script("open", "prod", "us-east-1")
        self.assert_result(result, 1, stderr="Updated kubeconfig\n" + NO_CONTEXT)
        self.assertEqual(self.sandbox.calls("k9s"), [])

    def test_any_other_aws_failure_is_echoed_and_ends_the_script(self):
        failure = "An error occurred (ResourceNotFoundException): No cluster found\n"
        self.sandbox.fake("aws", status=255, stderr=failure)
        self.assert_result(self.run_script("use", "prod", "us-east-1"), 255, stderr=failure)
        self.assertEqual(len(self.sandbox.calls("aws")), 1)
        self.assertEqual(self.sandbox.calls("kubectl"), [])

    def test_a_missing_cluster_is_a_usage_error(self):
        for subcommand in CONTEXT_TOOLS:
            usage = f"k9s-eks: Usage: scripts/k9s_eks.py {subcommand} <cluster> [region] [profile] [alias]\n"
            with self.subTest(subcommand=subcommand):
                self.assert_result(self.run_script(subcommand), 2, stderr=usage)
                # An empty cluster is reported before a missing aws, as before.
                result = self.run_script(subcommand, "", "us-east-1", env=self.without("aws"))
                self.assert_result(result, 2, stderr=usage)
        self.assertEqual(self.sandbox.calls(), [])

    def test_a_missing_aws_is_reported_before_a_missing_region(self):
        result = self.run_script("use", "prod", env=self.without("aws"))
        self.assert_result(result, 127, stderr="k9s-eks: missing required command: aws\n")

    def test_a_missing_region_stops_before_aws(self):
        for subcommand in CONTEXT_TOOLS:
            with self.subTest(subcommand=subcommand):
                self.assert_result(self.run_script(subcommand, "prod"), 2, stderr=REGION_REQUIRED)
        self.assertEqual(self.sandbox.calls(), [])

    def test_a_missing_tool_exits_127_after_the_kubeconfig_update(self):
        for subcommand, (tool, *_) in CONTEXT_TOOLS.items():
            with self.subTest(subcommand=subcommand):
                self.forget_calls()
                self.sandbox.fake("aws", stdout=ADDED)
                result = self.run_script(subcommand, "prod", "us-east-1", env=self.without(tool))
                message = f"k9s-eks: missing required command: {tool}\n"
                self.assert_result(result, 127, stderr=ADDED + message)
                self.assertEqual(len(self.sandbox.calls("aws")), 1)

    def test_ctrl_c_ends_it_quietly_by_sigint(self):
        self.sandbox.fake("aws", effect="import time; time.sleep(30)")
        # A session of its own, so that the SIGINT reaches only the script and aws.
        process = subprocess.Popen(
            [sys.executable, self.sandbox.repo / SCRIPT, "use", "prod", "us-east-1"],
            cwd=self.sandbox.repo, env=self.sandbox.env(), start_new_session=True,
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        )
        self.addCleanup(process.kill)
        deadline = time.monotonic() + 20
        while not self.sandbox.calls("aws") and time.monotonic() < deadline:
            time.sleep(0.05)
        os.killpg(process.pid, signal.SIGINT)
        stdout, stderr = process.communicate(timeout=30)
        self.assertEqual((process.returncode, stdout, stderr), (-signal.SIGINT, "", ""))


class SelectTests(EksTestCase):
    def setUp(self):
        super().setUp()
        self.sandbox.fake("aws", effect=f"""
            if args[1] == "list-clusters":
                print("alpha\\tbeta")
            else:
                sys.stdout.write({ADDED!r})
        """)
        self.sandbox.fake("k9s")
        self.fzf_input = self.sandbox.root / "fzf-input"

    def fake_fzf(self, status=0, pick=b""):
        """An fzf that saves what it was offered, prints `pick`, and exits with `status`."""
        self.sandbox.fake("fzf", status=status, effect=f"""
            offered = sys.stdin.read()
            with open({str(self.fzf_input)!r}, "w", encoding="utf-8") as saved:
                saved.write(offered)
            sys.stdout.buffer.write({pick!r})
        """)

    def test_opens_the_picked_cluster(self):
        self.fake_fzf(pick=b"beta\n")
        self.assert_result(self.run_script("select", "us-west-2", "ops"), 0, stderr=ADDED)
        self.assertEqual(self.sandbox.args("fzf"), [["--prompt", "EKS cluster> "]])
        self.assertEqual(self.fzf_input.read_text(encoding="utf-8"), "alpha\nbeta\n")
        self.assertEqual(self.sandbox.args("aws"), [
            [*LIST_WEST, "--profile", "ops"],
            ["eks", "update-kubeconfig", "--name", "beta", "--region", "us-west-2",
             "--profile", "ops", "--alias", "beta@us-west-2"],
        ])
        self.assertEqual(self.sandbox.args("k9s"), [["--context", "beta@us-west-2"]])

    def test_every_tool_behind_a_tilde_path_entry_is_found_as_bash_finds_them(self):
        self.fake_fzf(pick=b"beta\n")
        path = self.sandbox.tilde_path("aws", "fzf", "k9s")
        self.assert_result(self.run_script("select", "us-west-2", env={"PATH": path}), 0, stderr=ADDED)
        self.assertEqual([call.name for call in self.sandbox.calls()], ["aws", "fzf", "aws", "k9s"])

    def test_a_pick_that_is_not_utf8_is_opened_byte_for_byte(self):
        self.fake_fzf(pick=b"al\xffpha\n")
        self.assert_result(self.run_script("select", "us-west-2"), 0, stderr=ADDED)
        cluster = os.fsdecode(b"al\xffpha")
        self.assertEqual(self.sandbox.args("aws")[1][:4], ["eks", "update-kubeconfig", "--name", cluster])
        self.assertEqual(self.sandbox.args("k9s"), [["--context", f"{cluster}@us-west-2"]])

    def test_a_cancelled_pick_is_the_exit_status(self):
        self.fake_fzf(status=130)
        self.assertEqual(self.run_script("select", "us-west-2").returncode, 130)
        self.assertEqual(self.sandbox.args("aws"), [LIST_WEST])
        self.assertEqual(self.sandbox.calls("k9s"), [])

    def test_an_fzf_killed_by_a_signal_ends_with_the_shell_status(self):
        self.sandbox.fake("fzf", effect="sys.stdin.read()\n" + killed_by(signal.SIGKILL))
        self.assert_result(self.run_script("select", "us-west-2"), 128 + signal.SIGKILL)
        self.assertEqual(self.sandbox.calls("k9s"), [])

    def test_an_empty_pick_is_a_usage_error(self):
        self.fake_fzf(pick=b"\n")
        usage = "k9s-eks: Usage: scripts/k9s_eks.py open <cluster> [region] [profile] [alias]\n"
        self.assert_result(self.run_script("select", "us-west-2"), 2, stderr=usage)
        self.assertEqual(self.sandbox.args("aws"), [LIST_WEST])

    def test_a_missing_fzf_exits_127_before_aws(self):
        result = self.run_script("select", "us-west-2", env=self.without("fzf"))
        self.assert_result(result, 127, stderr="k9s-eks: missing required command: fzf\n")
        self.assertEqual(self.sandbox.calls(), [])

    def test_a_missing_region_still_starts_fzf_on_an_empty_list(self):
        # As in the old `eks_ls | fzf` pipeline: the listing fails, fzf's status wins.
        self.fake_fzf(status=130)
        self.assert_result(self.run_script("select"), 130, stderr=REGION_REQUIRED)
        self.assertEqual(self.sandbox.args("fzf"), [["--prompt", "EKS cluster> "]])
        self.assertEqual(self.fzf_input.read_text(encoding="utf-8"), "")
        self.assertEqual(self.sandbox.calls("aws"), [])

    def test_a_failed_listing_still_offers_what_aws_printed(self):
        # As in the old `eks_ls | fzf` pipeline: fzf starts, and its status wins.
        self.fake_fzf(status=130)
        self.sandbox.fake("aws", status=254, stdout="alpha\n", stderr="expired token\n")
        self.assert_result(self.run_script("select", "us-west-2"), 130, stderr="expired token\n")
        self.assertEqual(self.fzf_input.read_text(encoding="utf-8"), "alpha\n")

    def test_a_missing_aws_still_starts_fzf_on_an_empty_list(self):
        self.fake_fzf(status=130)
        result = self.run_script("select", "us-west-2", env=self.without("aws"))
        self.assert_result(result, 130, stderr="k9s-eks: missing required command: aws\n")
        self.assertEqual(self.fzf_input.read_text(encoding="utf-8"), "")


class CommandLineTests(EksTestCase):
    def test_bad_subcommands_are_usage_errors(self):
        for args in ([], ["list"], ["--bogus"], ["--", "ls"]):
            with self.subTest(args=args):
                result = self.run_script(*args)
                self.assertEqual((result.returncode, result.stdout), (2, ""))
                self.assertIn("usage:", result.stderr)
        self.assertEqual(self.sandbox.calls(), [])

    def test_extra_arguments_are_ignored_as_before(self):
        self.sandbox.fake("aws", stdout=ADDED)
        self.sandbox.fake("kubectl")
        self.assert_result(self.run_script("ls", "us-west-2", "ops", "extra"), 0, stdout=ADDED)
        self.assert_result(self.run_script("use", "prod", "us-east-1", "ops", "admin", "extra"),
                           0, stderr=ADDED)
        self.assertEqual(self.sandbox.args("aws"), [
            [*LIST_WEST, "--profile", "ops"],
            [*UPDATE_PROD, "--profile", "ops", "--alias", "admin"],
        ])

    def test_words_that_look_like_options_are_values(self):
        self.sandbox.fake("aws")
        self.sandbox.fake("kubectl")
        for args, expected in (
            (["ls", "-r"], listing("-r")),
            (["ls", "-h"], listing("-h")),
            (["ls", "--", "ops"], listing("--", "--profile", "ops")),
            (["use", "-x", "us-east-1"],
             ["eks", "update-kubeconfig", "--name", "-x", "--region", "us-east-1",
              "--alias", "-x@us-east-1"]),
            (["use", "prod", "us-east-1", "-dev"],
             [*UPDATE_PROD, "--profile", "-dev", "--alias", "prod@us-east-1"]),
        ):
            with self.subTest(args=args):
                self.forget_calls()
                self.assertEqual(self.run_script(*args).returncode, 0)
                self.assertEqual(self.sandbox.args("aws"), [expected])

    def test_help_shows_the_subcommands_and_the_old_names(self):
        result = self.run_script("-h")
        self.assertEqual(result.returncode, 0)
        self.assertIn("scripts/k9s_eks.py open my-cluster us-west-2 myprofile", result.stdout)
        self.assertIn("alias k9s_eks_select='/path/to/repo/scripts/k9s_eks.py select'", result.stdout)
        # A program sees only exported variables and real commands.
        self.assertIn("The default variables\nmust be exported", result.stdout)
        self.assertIn("shell functions and\naliases with those names are not used", result.stdout)
        # The one deliberate change of behaviour is documented.
        self.assertIn("`ls` now fails when aws fails", result.stdout)


if __name__ == "__main__":
    unittest.main()
