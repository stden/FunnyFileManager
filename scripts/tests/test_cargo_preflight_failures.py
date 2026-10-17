"""Check how the preflight reports a broken wrapper, a failing Cargo and a
directory that cannot hold the probe.
"""

from pathlib import Path
import signal
import unittest

from cargo_preflight_case import ADVICE, FAIL, NO_WRAPPER, SCRIPT, PreflightTestCase
from process_signals import dies_of


class FailureTests(PreflightTestCase):
    def test_a_broken_wrapper_reports_cargos_own_reason(self):
        reason = (
            "error: could not execute process `/nonexistent/compiler-wrapper rustc -vV` (never executed)\n"
            "\nCaused by:\n  No such file or directory (os error 2)\n"
        )
        result = self.preflight(effect=FAIL.format(stderr=reason), RUSTC_WRAPPER="/nonexistent/compiler-wrapper")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(
            result.stderr,
            "error: Cargo could not query the Rust compiler "
            "(compiler wrapper: RUSTC_WRAPPER=/nonexistent/compiler-wrapper):\n" + reason + ADVICE,
        )
        self.assert_probe_removed()

    def test_a_cargo_that_exits_124_or_is_killed_before_the_limit_did_not_stall(self):
        # bash took timeout(1)'s 124 and 137 for a stall, even from Cargo itself
        # or the OOM killer; the preflight knows whether the limit ran out.
        effects = {
            "exit 124": """
                if args[0] == "metadata":
                    sys.exit(124)
            """,
            "SIGKILL": dies_of(signal.SIGKILL, 'args[0] == "metadata"'),
        }
        for case, effect in effects.items():
            with self.subTest(case=case):
                result = self.preflight(effect=effect)
                self.assertEqual((result.returncode, result.stderr), (1, (
                    f"error: Cargo could not query the Rust compiler (compiler wrapper: {NO_WRAPPER}):\n"
                    + ADVICE
                )))
                self.assert_probe_removed()

    def test_only_the_last_twenty_lines_of_cargos_stderr_are_shown(self):
        lines = "".join(f"line {number}\n" for number in range(1, 26))
        result = self.preflight(effect=FAIL.format(stderr=lines))
        shown = result.stderr.splitlines()[1:-2]
        self.assertEqual(shown, [f"line {number}" for number in range(6, 26)])

    def test_cargos_stderr_is_shown_byte_for_byte_as_tail_showed_it(self):
        # Lines end at \n only: progress redraws (\r), CRLF, form feeds, U+2028,
        # bytes that are not UTF-8 and Cyrillic all pass through unchanged, even
        # when the preflight's own stderr cannot encode them.
        progress = b"".join(b"p%d\r" % number for number in range(30)) + b"done\n"
        tail = [b"crlf\r\n", b"feed\x0c vtab\x0b sep\xe2\x80\xa8 end\n", b"latin \xe9\xff\n",
                "ошибка: не удалось\n".encode("utf-8"), b"no newline"]
        earlier = b"".join(b"early %d\n" % number for number in range(20))
        effect = f"""
            if args[0] == "metadata":
                sys.stderr.flush()
                os.write(2, {earlier + progress + b"".join(tail)!r})
                sys.exit(101)
        """
        for encoding in ("utf-8", "latin-1", "ascii"):
            with self.subTest(stderr_encoding=encoding):
                result = self.preflight_bytes(effect, PYTHONUTF8="0", PYTHONIOENCODING=encoding)
                self.assertEqual(result.returncode, 1)
                shown = result.stderr.split(b"\n", 1)[1].rsplit(b"This repository", 1)[0]
                self.assertEqual(shown, b"".join(
                    [b"early %d\n" % number for number in range(6, 20)] + [progress] + tail))

    def test_the_error_names_the_wrapper_the_environment_selects(self):
        cases = [
            ({"RUSTC_WRAPPER": "sccache", "CARGO_BUILD_RUSTC_WRAPPER": "other"}, "RUSTC_WRAPPER=sccache"),
            ({"RUSTC_WRAPPER": "", "CARGO_BUILD_RUSTC_WRAPPER": "other"}, "CARGO_BUILD_RUSTC_WRAPPER=other"),
            ({"RUSTC_WRAPPER": "", "CARGO_BUILD_RUSTC_WRAPPER": ""}, NO_WRAPPER),
            ({}, NO_WRAPPER),
        ]
        for env, wrapper in cases:
            with self.subTest(env=env):
                result = self.preflight(effect=FAIL.format(stderr="boom\n"), **env)
                self.assertEqual(
                    result.stderr.splitlines()[0],
                    f"error: Cargo could not query the Rust compiler (compiler wrapper: {wrapper}):",
                )

    def test_a_failing_cargo_version_ends_the_preflight_with_its_status(self):
        result = self.preflight(effect="""
            if args == ["-V"]:
                sys.stderr.write("error: toolchain is not installed\\n")
                sys.exit(5)
        """)
        self.assertEqual(result.returncode, 5)
        self.assertEqual(result.stderr, "error: toolchain is not installed\n")
        self.assertEqual(self.sandbox.args("cargo"), [["-V"]])
        self.assert_probe_removed()

    def test_a_missing_cargo_fails_with_status_127_naming_it_as_bash_did(self):
        for cargo in ("/nonexistent/cargo", "no-such-cargo"):
            with self.subTest(cargo=cargo):
                result = self.preflight(CARGO=cargo)
                self.assertEqual(
                    (result.returncode, result.stdout, result.stderr),
                    (127, "", f"{cargo}: No such file or directory\n"),
                )
                self.assertEqual(self.sandbox.calls(), [])
                self.assert_probe_removed()

    def test_a_cargo_that_cannot_be_executed_ends_the_preflight_with_126(self):
        self.sandbox.fake("cargo").chmod(0o644)
        # Only the sandbox's bin directory: no system cargo can stand in.
        result = self.sandbox.run(SCRIPT, env={"PATH": str(self.sandbox.bin)})
        self.assertEqual((result.returncode, result.stderr), (126, "cargo: Permission denied\n"))
        self.assert_probe_removed()

    def test_a_cargo_without_a_shebang_line_cannot_start(self):
        # bash ran such a file as a script of its own; the preflight, like lint, refuses it.
        ran = self.sandbox.root / "ran"
        plain = self.sandbox.root / "plain cargo"
        plain.write_text(f'echo "$1" >> "{ran}"\n', encoding="utf-8")
        plain.chmod(0o755)
        result = self.preflight(CARGO=str(plain))
        self.assertEqual((result.returncode, result.stderr), (126, f"{plain}: Exec format error\n"))
        self.assertFalse(ran.exists())
        self.assert_probe_removed()

    def test_a_tmpdir_that_cannot_hold_the_probe_fails_as_mktemp_did(self):
        cases = {
            "missing": (self.sandbox.root / "no such dir", "No such file or directory"),
            "a file": (self.sandbox.write("a file"), "Not a directory"),
        }
        for case, (tmpdir, reason) in cases.items():
            with self.subTest(case=case):
                self.sandbox.log.unlink(missing_ok=True)
                result = self.preflight(TMPDIR=str(tmpdir))
                self.assertEqual((result.returncode, result.stdout), (1, ""))
                self.assertEqual(
                    result.stderr,
                    f"mktemp: failed to create directory via template '{tmpdir}/ffm-wrapper-probe.XXXXXX': {reason}\n",
                )
                self.assertEqual(self.sandbox.args("cargo"), [["-V"]])

    def test_a_relative_tmpdir_is_under_the_checkout_root(self):
        (self.sandbox.repo / "scratch").mkdir()
        result = self.preflight(cwd=self.sandbox.home, TMPDIR="scratch")
        self.assertEqual(result.returncode, 0, result.stderr)
        manifest = Path(self.sandbox.args("cargo")[1][4])
        self.assertEqual(manifest.parent.parent, self.sandbox.repo / "scratch")


if __name__ == "__main__":
    unittest.main()
