"""Test the EKS helper against fake aws, kubectl, k9s and fzf; nothing reaches AWS.

ls and the command line; use, open and select have test_k9s_eks_*.py files
of their own, and k9s_eks_case.py holds what they share.
"""

import signal
import sys
import unittest

from k9s_eks_case import (
    ADDED, LIST_WEST, REGION_REQUIRED, SCRIPT, UPDATE_PROD, EksTestCase, killed_by, listing, writes,
)


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
