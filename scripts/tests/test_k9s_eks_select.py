"""Test select: the region's clusters offered in fzf, and the pick opened in k9s."""

import os
import signal
import unittest

from k9s_eks_case import ADDED, LIST_WEST, REGION_REQUIRED, EksTestCase, killed_by


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


if __name__ == "__main__":
    unittest.main()
