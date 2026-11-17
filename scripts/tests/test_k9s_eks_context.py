"""Test use and open: a kubeconfig context from aws, then kubectl or k9s on it."""

import os
import signal
import subprocess
import sys
import time
import unittest

from k9s_eks_case import (
    ADDED, ALIAS_REJECTIONS, CONTEXT_TOOLS, NO_CONTEXT, REGION_REQUIRED, SCRIPT, UPDATE_PROD, EksTestCase,
    killed_by, old_aws_cli, writes,
)


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


if __name__ == "__main__":
    unittest.main()
