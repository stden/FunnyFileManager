"""Test ffm.py, the build-and-run launcher, against fake Cargo, git, cmake and ffm.

scripts/cargo_preflight.py and scripts/lint.py are recording fakes without the
executable bit, so only `<interpreter> <file>` can run them, as ffm.py must.
"""

import os
import shlex
import signal
import subprocess
import sys
import unittest

from sandbox import Sandbox


def status(message):
    return f"\033[0;32m[FFM]\033[0m {message}\n"


def error(message):
    return f"\033[0;31m[ERROR]\033[0m {message}\n"


def warning(message):
    return f"\033[1;33m[WARNING]\033[0m {message}\n"


RUST_MISSING = (
    error("Rust is not installed!")
    + "Please install Rust from https://rustup.rs/\n"
    + "Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh\n"
)
# Every command line that runs the Rust check first; [] is no command at all.
RUST_CHECKED = (
    ["build"], ["release"], ["test"], ["lint"], ["coverage"],
    ["install"], ["reinstall"], [], ["somewhere"], ["--version"],
)
BUILD_STARTED = status("Building FunnyFileManager...")
BUILDING = BUILD_STARTED + status("Build completed successfully!")
STARTING = status("Starting FunnyFileManager...")
NO_BINARY = warning("FFM binary not found, building...")
HELP = """\
FunnyFileManager Build & Run Script

Usage: ./ffm.py [command] [options]

Commands:
  (no command)     Build (if needed) and run FFM in debug mode
  build [mode]     Build FFM (mode: debug|release, default: debug)
  release          Build in release mode and run
  run              Run FFM (assumes already built)
  test             Run all tests
  lint [mode]      Rust linter gate (fmt + clippy; see scripts/lint.py)
  coverage         Run test coverage analysis
  clean            Clean build artifacts
  colorer          Install Colorer + Far's syntax schemes, then rebuild
  install          Rebuild and install FFM to ~/.cargo/bin (--force)
  reinstall        Same as install
  help             Show this help message

Examples:
  ./ffm.py                  # Build and run in debug mode
  ./ffm.py release          # Build and run in release mode
  ./ffm.py build release    # Only build in release mode
  ./ffm.py test             # Run tests
  ./ffm.py lint             # fmt --check + clippy feature matrix
  ./ffm.py lint clippy-fast # all targets, default features
"""
CARGO_MISSING = "cargo: No such file or directory\n"


class LauncherTest(unittest.TestCase):
    """A sandbox checkout holding ffm.py, with its helper scripts and Cargo faked."""

    def setUp(self):
        self.sandbox = Sandbox(self)
        self.repo = self.sandbox.repo
        self.sandbox.copy("ffm.py")
        # ffm.py finds its programs through it.
        self.sandbox.copy("scripts/shell_path.py")
        for name in ("cargo_preflight", "lint"):
            helper = self.sandbox.fake(name, path=self.repo / f"scripts/{name}.py")
            helper.chmod(0o644)
        self.sandbox.fake("cargo")

    def ffm(self, *args, env=None, cwd=None):
        return self.sandbox.run("ffm.py", *args, env=env, cwd=cwd)

    def ffm_bytes(self, *args, env=None, unset=(), stdout=subprocess.PIPE):
        """Run ffm.py in the sandbox with raw bytes out; `unset` drops variables."""
        environment = self.sandbox.env(env)
        for name in unset:
            del environment[name]
        return subprocess.run(
            [sys.executable, self.repo / "ffm.py", *args], cwd=self.repo, env=environment,
            stdout=stdout, stderr=subprocess.PIPE, timeout=60, check=False,
        )

    def ffm_with_stdout(self, redirection, *args):
        """Run ffm.py with its stdout redirected by sh, e.g. ">&-" closes it."""
        script = f'exec "$@" {redirection}'
        return self.sandbox.execute(["sh", "-c", script, "sh", sys.executable, self.repo / "ffm.py", *args])

    def names(self):
        return [call.name for call in self.sandbox.calls()]

    def forget_calls(self):
        self.sandbox.log.unlink(missing_ok=True)

    def hide(self, name):
        """Remove `name`'s stub; the returned env keeps the system's copy off PATH too."""
        (self.sandbox.bin / name).unlink()
        return {"PATH": str(self.sandbox.bin)}

    def place_binary(self, mode, *, status=0, stdout=""):
        """A recording ffm at target/<mode>/ffm, as if Cargo had built it."""
        path = self.repo / "target" / mode / "ffm"
        return self.sandbox.fake("ffm-binary", path=path, status=status, stdout=stdout)

    def fake_cargo_that_builds(self, *, status=0):
        """Fake Cargo whose `build` leaves a recording ffm at target/<mode>/ffm."""
        built = self.sandbox.fake("ffm-binary", path=self.sandbox.root / "built-ffm", status=status)
        self.sandbox.fake("cargo", effect=f"""
            if args[:1] == ["build"]:
                import shutil
                mode = "release" if "--release" in args else "debug"
                target = os.path.join({str(self.repo)!r}, "target", mode)
                os.makedirs(target, exist_ok=True)
                shutil.copy2({str(built)!r}, os.path.join(target, "ffm"))
        """)



class HelpTests(LauncherTest):
    def test_help_lists_every_command_and_example(self):
        for args in (["help"], ["-h"], ["--help"], ["help", "extra"]):
            with self.subTest(args=args):
                result = self.ffm(*args)
                self.assertEqual((result.returncode, result.stdout, result.stderr), (0, HELP, ""))
        self.assertEqual(self.sandbox.calls(), [])


class RustCheckTests(LauncherTest):
    def test_a_missing_cargo_stops_every_rust_command(self):
        env = self.hide("cargo")
        for args in RUST_CHECKED:
            with self.subTest(args=args):
                result = self.ffm(*args, env=env)
                self.assertEqual((result.returncode, result.stdout, result.stderr), (1, RUST_MISSING, ""))
        self.assertEqual(self.sandbox.calls(), [])

    def test_a_failing_preflight_stops_every_rust_command_before_cargo(self):
        self.sandbox.fake("cargo_preflight", path=self.repo / "scripts/cargo_preflight.py", status=3)
        self.place_binary("debug")
        for args in RUST_CHECKED:
            with self.subTest(args=args):
                self.forget_calls()
                result = self.ffm(*args)
                self.assertEqual((result.returncode, result.stdout), (1, ""))
                self.assertEqual(self.names(), ["cargo_preflight"])

    def test_the_preflight_runs_first_from_the_checkout_root(self):
        result = self.ffm("build", cwd=self.sandbox.home)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.names(), ["cargo_preflight", "cargo"])
        self.assertEqual({call.cwd for call in self.sandbox.calls()}, {str(self.repo)})
        self.assertEqual(self.sandbox.args("cargo_preflight"), [[]])


class BuildTests(LauncherTest):
    def test_any_mode_but_release_builds_debug(self):
        for args in (["build"], ["build", "debug"], ["build", "anything"], ["build", ""]):
            with self.subTest(args=args):
                self.forget_calls()
                result = self.ffm(*args)
                self.assertEqual((result.returncode, result.stdout), (0, BUILDING))
                self.assertEqual(self.sandbox.args("cargo"), [["build"]])

    def test_release_mode_builds_release_and_ignores_extra_words(self):
        result = self.ffm("build", "release", "extra")
        self.assertEqual((result.returncode, result.stdout), (0, BUILDING))
        self.assertEqual(self.sandbox.args("cargo"), [["build", "--release"]])

    def test_status_lines_stay_in_order_with_cargo_output(self):
        self.sandbox.fake("cargo", stdout="Compiling ffm\n")
        result = self.ffm("build")
        expected = BUILD_STARTED + "Compiling ffm\n" + status("Build completed successfully!")
        self.assertEqual(result.stdout, expected)

    def test_a_failed_build_exits_1(self):
        self.sandbox.fake("cargo", status=101)
        result = self.ffm("build")
        self.assertEqual(result.returncode, 1)
        self.assertEqual(result.stdout, BUILD_STARTED + error("Build failed!"))


class LaunchTests(LauncherTest):
    def test_release_builds_then_runs_the_release_binary(self):
        self.fake_cargo_that_builds(status=7)
        result = self.ffm("release", "--", "-x", "two words", cwd=self.sandbox.home)
        self.assertEqual((result.returncode, result.stdout), (7, BUILDING + STARTING))
        self.assertEqual(self.sandbox.args("cargo"), [["build", "--release"]])
        self.assertEqual(self.sandbox.args("ffm-binary"), [["--", "-x", "two words"]])
        self.assertEqual(self.sandbox.calls("ffm-binary")[0].cwd, str(self.repo))

    def test_a_failed_release_build_runs_nothing(self):
        self.place_binary("release")
        self.sandbox.fake("cargo", status=101)
        result = self.ffm("release")
        self.assertEqual(result.returncode, 1)
        self.assertTrue(result.stdout.endswith(error("Build failed!")))
        self.assertEqual(self.sandbox.calls("ffm-binary"), [])

    def test_a_release_build_that_leaves_no_binary_falls_back_to_debug(self):
        # The release build left nothing in target/release (CARGO_TARGET_DIR
        # elsewhere): a debug build follows, and the debug binary runs.
        self.place_binary("debug", status=4)
        result = self.ffm("release")
        self.assertEqual(self.sandbox.args("cargo"), [["build", "--release"], ["build"]])
        self.assertEqual((result.returncode, result.stdout), (4, BUILDING + NO_BINARY + BUILDING + STARTING))
        self.assertEqual(self.sandbox.args("ffm-binary"), [[]])

    def test_a_binary_still_missing_after_the_build_exits_127(self):
        expected = f"{self.repo / 'target/debug/ffm'}: No such file or directory\n"
        for args in (["release"], ["run"], []):
            with self.subTest(args=args):
                result = self.ffm(*args)
                self.assertEqual((result.returncode, result.stderr), (127, expected))
                self.assertTrue(result.stdout.endswith(NO_BINARY + BUILDING + STARTING))

    def test_a_binary_that_cannot_be_executed_exits_126(self):
        binary = self.place_binary("debug")
        binary.chmod(0o644)
        plain_file = self.ffm("run")
        binary.unlink()
        binary.mkdir()
        directory = self.ffm("run")
        for result in (plain_file, directory):
            self.assertEqual(result.returncode, 126)
            self.assertEqual(result.stderr, f"{binary}: Permission denied\n")

    def test_ffm_is_executed_by_its_full_path(self):
        # A #! program sees the path exec was given as $0.
        binary = self.sandbox.write("target/debug/ffm", '#!/bin/sh\necho "$0"\n')
        binary.chmod(0o755)
        result = self.ffm("run", cwd=self.sandbox.home)
        self.assertEqual((result.returncode, result.stdout), (0, STARTING + f"{binary}\n"))

    def test_every_program_sees_the_checkout_root_as_pwd(self):
        self.place_binary("debug")
        for pwd in (str(self.sandbox.home), "/", None):
            with self.subTest(PWD=pwd):
                self.forget_calls()
                self.ffm(env={"PWD": pwd} if pwd else None, cwd=self.sandbox.home)
                self.assertEqual({call.env["PWD"] for call in self.sandbox.calls()}, {str(self.repo)})
                self.assertEqual(self.names(), ["cargo_preflight", "ffm-binary"])

    def test_ffm_starts_with_sigpipe_and_sigxfsz_at_their_defaults(self):
        if not os.path.exists("/proc/self/status"):
            self.skipTest("reads the signal masks from Linux /proc")
        binary = self.sandbox.write("target/debug/ffm", "#!/bin/sh\nexec grep '^SigIgn' /proc/self/status\n")
        binary.chmod(0o755)
        result = self.ffm("run")
        ignored = int(result.stdout.removeprefix(STARTING).split()[1], 16)
        for number in (signal.SIGPIPE, signal.SIGXFSZ):
            self.assertFalse(ignored & 1 << (number - 1), number.name)

    def test_run_starts_a_built_binary_without_cargo(self):
        self.place_binary("debug", status=5, stdout="ffm output\n")
        result = self.ffm("run", "-h", "--", "x")
        self.assertEqual((result.returncode, result.stdout), (5, STARTING + "ffm output\n"))
        self.assertEqual(self.names(), ["ffm-binary"])
        self.assertEqual(self.sandbox.args("ffm-binary"), [["-h", "--", "x"]])

    def test_run_builds_a_missing_binary_first(self):
        self.fake_cargo_that_builds()
        result = self.ffm("run")
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout, NO_BINARY + BUILDING + STARTING)
        self.assertEqual(self.names(), ["cargo", "ffm-binary"])

    def test_run_without_a_binary_or_cargo_fails(self):
        result = self.ffm("run", env=self.hide("cargo"))
        self.assertEqual((result.returncode, result.stderr), (1, CARGO_MISSING))
        self.assertEqual(result.stdout, NO_BINARY + BUILD_STARTED + error("Build failed!"))

    def test_a_failed_build_runs_no_ffm(self):
        self.sandbox.fake("cargo", status=101)
        for args in (["run"], [], ["somewhere"]):
            with self.subTest(args=args):
                self.forget_calls()
                result = self.ffm(*args)
                self.assertEqual((result.returncode, result.stderr), (1, ""))
                self.assertTrue(result.stdout.endswith(BUILD_STARTED + error("Build failed!")), result.stdout)
                self.assertEqual(self.sandbox.args("cargo"), [["build"]])

    def test_no_command_runs_the_built_debug_binary(self):
        self.place_binary("debug")
        result = self.ffm(cwd=self.sandbox.home)
        self.assertEqual((result.returncode, result.stdout), (0, STARTING))
        self.assertEqual(self.names(), ["cargo_preflight", "ffm-binary"])
        call = self.sandbox.calls("ffm-binary")[0]
        self.assertEqual((call.args, call.cwd), ([], str(self.repo)))

    def test_no_command_builds_a_missing_binary_without_a_warning(self):
        self.fake_cargo_that_builds()
        result = self.ffm()
        self.assertEqual((result.returncode, result.stdout), (0, BUILDING + STARTING))
        self.assertEqual(self.sandbox.args("cargo"), [["build"]])

    def test_unknown_words_and_leading_options_all_go_to_ffm(self):
        self.place_binary("debug")
        for args in (
            ["somewhere", "-h", "--x"], ["--version"], ["-x", "-h"], ["--"],
            ["--", "run"], [""], ["-"], ["--help=x"],
        ):
            with self.subTest(args=args):
                self.forget_calls()
                result = self.ffm(*args)
                self.assertEqual((result.returncode, result.stdout), (0, STARTING))
                self.assertEqual(self.sandbox.args("ffm-binary"), [args])


class TestCommandTests(LauncherTest):
    SUITES = (
        ["test", "--features", "test-helpers"],
        ["test", "--features", "test-helpers,s3", "--test", "bdd", "--test", "bdd_ru"],
        ["test", "--features", "keepass", "--lib", "keepass"],
    )

    def test_every_feature_set_runs_in_order(self):
        result = self.ffm("test", "extra")
        self.assertEqual(result.returncode, 0)
        self.assertEqual(
            result.stdout,
            status("Running tests...")
            + status("Running BDD suites...")
            + status("Running keepass feature tests..."),
        )
        self.assertEqual(self.sandbox.args("cargo"), list(self.SUITES))

    def test_the_first_failing_suite_exits_1(self):
        for index, failing in enumerate(self.SUITES):
            with self.subTest(failing=failing):
                self.forget_calls()
                self.sandbox.fake("cargo", effect=f"if args == {failing!r}: sys.exit(101)")
                self.assertEqual(self.ffm("test").returncode, 1)
                self.assertEqual(self.sandbox.args("cargo"), list(self.SUITES[: index + 1]))

    def test_a_suite_killed_by_a_signal_exits_1(self):
        self.sandbox.fake("cargo", effect="import signal; os.kill(os.getpid(), signal.SIGTERM)")
        result = self.ffm("test")
        self.assertEqual((result.returncode, result.stderr), (1, "Terminated  cargo test --features test-helpers\n"))
        self.assertEqual(self.sandbox.args("cargo"), [self.SUITES[0]])

    def test_ctrl_c_that_the_suite_handles_lets_the_run_go_on(self):
        # Ctrl+C reaches the whole foreground group: this script and the suite.
        # The script waits for the suite, which cleans up and exits 0, and goes on.
        marker = self.sandbox.root / "cleaned up"
        self.sandbox.fake("cargo", effect=f"""
            import signal, time
            def clean_up(*_):
                time.sleep(0.5)
                open({str(marker)!r}, "w").close()
                sys.exit(0)
            if args == {self.SUITES[0]!r}:
                signal.signal(signal.SIGINT, clean_up)
                os.kill(os.getppid(), signal.SIGINT)
                os.kill(os.getpid(), signal.SIGINT)
                time.sleep(10)
        """)
        result = self.ffm("test")
        self.assertEqual((result.returncode, result.stderr), (0, ""))
        self.assertTrue(marker.exists())
        self.assertEqual(self.sandbox.args("cargo"), list(self.SUITES))

    def test_ctrl_c_that_kills_the_suite_kills_the_run_without_a_traceback(self):
        self.sandbox.fake("cargo", effect=f"""
            import signal, time
            if args == {self.SUITES[0]!r}:
                signal.signal(signal.SIGINT, signal.SIG_DFL)
                os.kill(os.getppid(), signal.SIGINT)
                time.sleep(0.3)
                os.kill(os.getpid(), signal.SIGINT)
        """)
        result = self.ffm("test")
        self.assertEqual((result.returncode, result.stderr), (-signal.SIGINT, ""))
        self.assertEqual(result.stdout, status("Running tests..."))
        self.assertEqual(self.sandbox.args("cargo"), [self.SUITES[0]])


class SignalStatusTests(LauncherTest):
    def test_a_command_killed_by_signal_n_exits_128_plus_n(self):
        lint = self.repo / "scripts/lint.py"
        commands = {
            "lint": ("lint", lint, shlex.join([sys.executable, str(lint), "all"])),
            "coverage": ("cargo", None, "cargo tarpaulin"),
            "clean": ("cargo", None, "cargo clean"),
        }
        for command, (fake, path, shown) in commands.items():
            # Named on stderr with the command, except for Ctrl-C.
            reports = {signal.SIGKILL: f"Killed  {shown}\n", signal.SIGTERM: f"Terminated  {shown}\n", signal.SIGINT: ""}
            for number, report in reports.items():
                with self.subTest(command=command, signal=number.name):
                    # Only the child gets the signal, so this script goes on.
                    self.sandbox.fake(fake, path=path, effect=f"""
                        import signal
                        signal.signal(signal.SIGINT, signal.SIG_DFL)
                        os.kill(os.getpid(), {int(number)})
                    """)
                    result = self.ffm(command)
                    self.assertEqual((result.returncode, result.stderr), (128 + number, report))


class LintTests(LauncherTest):
    def test_lint_runs_lint_py_in_the_given_mode(self):
        # "--" goes on to lint.py as the mode, which lint.py refuses.
        modes = (
            ([], "all"), ([""], "all"), (["fmt"], "fmt"), (["clippy-fast", "extra"], "clippy-fast"),
            (["--"], "--"), (["--", "fmt"], "--"),
        )
        for args, mode in modes:
            with self.subTest(args=args):
                self.forget_calls()
                result = self.ffm("lint", *args, cwd=self.sandbox.home)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout, status("Running Rust linter gate (fmt + spelling + clippy)..."))
                self.assertEqual(self.names(), ["cargo_preflight", "lint"])
                call = self.sandbox.calls("lint")[0]
                self.assertEqual((call.args, call.cwd), ([mode], str(self.repo)))

    def test_lint_exits_with_the_lint_status(self):
        self.sandbox.fake("lint", path=self.repo / "scripts/lint.py", status=3)
        self.assertEqual(self.ffm("lint").returncode, 3)


class CoverageTests(LauncherTest):
    def test_an_installed_tarpaulin_runs_directly(self):
        self.sandbox.fake("cargo", status=4)
        result = self.ffm("coverage")
        self.assertEqual((result.returncode, result.stdout), (4, status("Running test coverage...")))
        self.assertEqual(self.sandbox.args("cargo"), [["tarpaulin"]])

    def test_a_missing_tarpaulin_is_installed_first(self):
        env = self.hide("cargo-tarpaulin")
        # A failed install is ignored; tarpaulin's own status is the result.
        self.sandbox.fake("cargo", effect="sys.exit(1 if args[0] == 'install' else 6)")
        result = self.ffm("coverage", env=env)
        self.assertEqual(result.returncode, 6)
        self.assertEqual(
            result.stdout,
            status("Running test coverage...") + warning("cargo-tarpaulin not installed. Installing..."),
        )
        self.assertEqual(self.sandbox.args("cargo"), [["install", "cargo-tarpaulin"], ["tarpaulin"]])


class CleanTests(LauncherTest):
    def test_clean_runs_cargo_clean_without_the_rust_check(self):
        self.sandbox.fake("cargo", status=4)
        result = self.ffm("clean", "extra", cwd=self.sandbox.home)
        self.assertEqual((result.returncode, result.stdout), (4, status("Cleaning build artifacts...")))
        self.assertEqual(self.names(), ["cargo"])
        call = self.sandbox.calls("cargo")[0]
        self.assertEqual((call.args, call.cwd), (["clean"], str(self.repo)))

    def test_clean_without_cargo_exits_127(self):
        result = self.ffm("clean", env=self.hide("cargo"))
        self.assertEqual((result.returncode, result.stderr), (127, CARGO_MISSING))

    def test_a_cargo_that_cannot_be_executed_exits_126(self):
        (self.sandbox.bin / "cargo").chmod(0o644)
        result = self.ffm("clean", env={"PATH": str(self.sandbox.bin)})
        self.assertEqual((result.returncode, result.stderr), (126, "cargo: Permission denied\n"))


class InstallTests(LauncherTest):
    INSTALLING = status("Rebuilding and installing FunnyFileManager to ~/.cargo/bin...")
    REFRESHED = warning("Run 'hash -r' in shells opened before now — bash caches the old path.")

    def setUp(self):
        super().setUp()
        self.sandbox.use_real("ffm")  # No ffm on PATH until a test puts one there.
        self.cargo_bin = self.sandbox.home / ".cargo/bin/ffm"

    def fake_cargo_that_installs(self):
        self.sandbox.fake("cargo", effect=f"""
            path = {str(self.cargo_bin)!r}
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, "w") as binary:
                binary.write("fresh build\\n")
            os.chmod(path, 0o755)
        """)

    def put_ffm(self, directory, text, mode=0o755):
        path = self.sandbox.root / directory / "ffm"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        path.chmod(mode)
        return path

    def path_env(self, *directories):
        """PATH with `directories` first, then the sandbox's usual PATH."""
        return {"PATH": os.pathsep.join([*map(str, directories), self.sandbox.env()["PATH"]])}

    def test_a_failed_install_exits_1(self):
        self.sandbox.fake("cargo", status=101)
        result = self.ffm("install", cwd=self.sandbox.home)
        self.assertEqual((result.returncode, result.stdout), (1, self.INSTALLING + error("Install failed!")))
        call = self.sandbox.calls("cargo")[0]
        self.assertEqual((call.args, call.cwd), (["install", "--path", ".", "--force"], str(self.repo)))

    def test_without_ffm_on_path_it_prints_the_path_hint(self):
        for command in ("install", "reinstall"):
            with self.subTest(command=command):
                result = self.ffm(command)
                self.assertEqual(result.returncode, 0)
                self.assertEqual(
                    result.stdout,
                    self.INSTALLING
                    + status(f"Installed: {self.cargo_bin}")
                    + warning("~/.cargo/bin is not on your PATH — add it to use 'ffm' directly:")
                    + '  export PATH="$HOME/.cargo/bin:$PATH"\n',
                )

    def test_cargo_home_decides_where_ffm_is_installed(self):
        cargo_home = self.sandbox.root / "cargo home"
        for value, expected in ((str(cargo_home), cargo_home / "bin/ffm"), ("", self.cargo_bin)):
            with self.subTest(CARGO_HOME=value):
                result = self.ffm("install", env={"CARGO_HOME": value})
                self.assertIn(status(f"Installed: {expected}"), result.stdout)

    def test_every_stale_executable_copy_on_path_is_refreshed(self):
        self.fake_cargo_that_installs()
        stale = self.put_ffm("stale", "old\n")
        current = self.put_ffm("current", "fresh build\n")
        plain = self.put_ffm("plain", "old\n", mode=0o644)
        (self.sandbox.root / "folder/ffm").mkdir(parents=True)
        env = self.path_env(
            stale.parent, current.parent, plain.parent, self.sandbox.root / "folder",
            stale.parent, self.cargo_bin.parent,
        )
        result = self.ffm("install", env=env)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            result.stdout,
            self.INSTALLING
            + status(f"Installed: {self.cargo_bin}")
            + status(f"Refreshed stale copy: {stale}")
            + status(f"'ffm' resolves to: {stale}")
            + self.REFRESHED,
        )
        self.assertEqual(stale.read_text(encoding="utf-8"), "fresh build\n")
        self.assertEqual(stale.stat().st_mode & 0o777, 0o755)
        self.assertEqual(plain.read_text(encoding="utf-8"), "old\n")

    def test_a_copy_that_cannot_be_replaced_is_reported(self):
        # Cargo installed somewhere else, so there is nothing to copy from.
        stale = self.put_ffm("stale", "old\n")
        result = self.ffm("install", env=self.path_env(stale.parent))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            result.stdout,
            self.INSTALLING
            + status(f"Installed: {self.cargo_bin}")
            + warning(f"Stale copy shadowing yours (not writable): {stale}")
            + warning(f"  sudo cp {self.cargo_bin} {stale}")
            + status(f"'ffm' resolves to: {stale}")
            + self.REFRESHED,
        )
        self.assertEqual(stale.read_text(encoding="utf-8"), "old\n")

    def test_a_read_only_copy_is_reported(self):
        if os.geteuid() == 0:
            self.skipTest("root may write a read-only file")
        self.fake_cargo_that_installs()
        stale = self.put_ffm("stale", "old\n", mode=0o555)
        result = self.ffm("install", env=self.path_env(stale.parent))
        self.assertIn(warning(f"Stale copy shadowing yours (not writable): {stale}"), result.stdout)
        self.assertEqual(stale.read_text(encoding="utf-8"), "old\n")

    def test_the_installed_path_is_printed_as_the_environment_spells_it(self):
        root, home = self.sandbox.root, self.sandbox.home
        cases = (
            ({"CARGO_HOME": f"{root}/ch/"}, f"{root}/ch//bin/ffm"),
            ({"CARGO_HOME": f"{root}//ch"}, f"{root}//ch/bin/ffm"),
            ({"CARGO_HOME": "./ch"}, "./ch/bin/ffm"),
            ({"HOME": f"{home}/"}, f"{home}//.cargo/bin/ffm"),
            # No backslash escape is interpreted: this \t stays two characters.
            ({"CARGO_HOME": f"{root}/c\\tb"}, f"{root}/c\\tb/bin/ffm"),
        )
        for env, expected in cases:
            with self.subTest(env=env):
                self.assertIn(status(f"Installed: {expected}"), self.ffm("install", env=env).stdout)

    def test_an_unset_home_counts_as_empty(self):
        result = self.ffm_bytes("install", unset=("HOME",))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(b"Installed: /.cargo/bin/ffm\n", result.stdout)

    def test_a_tilde_in_path_finds_ffm_and_its_stale_copies(self):
        self.fake_cargo_that_installs()
        stale = self.put_ffm("home/stale", "old\n")
        env = {"PATH": os.pathsep.join(["~/stale", "~/.cargo/bin", self.sandbox.env()["PATH"]])}
        result = self.ffm("install", env=env)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            result.stdout,
            self.INSTALLING
            + status(f"Installed: {self.cargo_bin}")
            + status(f"Refreshed stale copy: {stale}")
            + status(f"'ffm' resolves to: {stale}")
            + self.REFRESHED,
        )
        self.assertEqual(stale.read_text(encoding="utf-8"), "fresh build\n")

    def test_copies_are_refreshed_in_exactly_the_path_entries(self):
        # A blank splits no entry, and a * matches nothing: gXb is not on PATH.
        self.fake_cargo_that_installs()
        spaced = self.put_ffm("with space", "old\n")
        pattern = self.put_ffm("g*b", "old\n")
        outside = self.put_ffm("gXb", "other\n")
        result = self.ffm("install", env=self.path_env(spaced.parent, pattern.parent))
        self.assertEqual(result.returncode, 0, result.stderr)
        for copy, text in ((spaced, "fresh build\n"), (pattern, "fresh build\n"), (outside, "other\n")):
            self.assertEqual(copy.read_text(encoding="utf-8"), text, copy)

    def test_path_bytes_and_the_em_dash_come_out_whatever_the_locale(self):
        # Python's own stdout refused both: a strict UTF-8 locale, or ASCII.
        strict = {"LC_ALL": "C.UTF-8", "PYTHONUTF8": "0"}
        cargo_home = f"{self.sandbox.root}/ch\udcff"
        result = self.ffm_bytes("install", env={**strict, "CARGO_HOME": cargo_home})
        self.assertEqual((result.returncode, result.stderr), (0, b""))
        self.assertIn(b"Installed: " + os.fsencode(cargo_home) + b"/bin/ffm\n", result.stdout)
        result = self.ffm_bytes("install", env={**strict, "PYTHONIOENCODING": "ascii"})
        self.assertEqual((result.returncode, result.stderr), (0, b""))
        self.assertIn("on your PATH — add it".encode(), result.stdout)


class ColorerTests(LauncherTest):
    def setUp(self):
        super().setUp()
        self.sandbox.fake("git")
        # Only the build's output may reach the terminal; configure's is discarded.
        self.sandbox.fake("cmake", effect="print('build output' if args[0] == '--build' else 'configure noise')")
        self.colorer_home = self.sandbox.root / "data home/ffm"
        self.library = self.colorer_home / "Colorer-library"
        self.schemes = self.colorer_home / "Colorer-schemes"
        self.env = {"XDG_DATA_HOME": str(self.colorer_home.parent)}

    def test_a_missing_cmake_stops_before_fetching(self):
        env = {**self.env, **self.hide("cmake")}
        result = self.ffm("colorer", env=env)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(
            result.stdout,
            error("cmake is required to build Colorer.")
            + "  Debian/Ubuntu: sudo apt-get install cmake g++ libicu-dev libxml2-dev zlib1g-dev libminizip-dev\n",
        )
        self.assertEqual(self.sandbox.calls(), [])
        self.assertFalse(self.colorer_home.exists())

    def test_a_first_install_clones_builds_and_clones_the_schemes(self):
        result = self.ffm("colorer", env=self.env)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            result.stdout,
            status("Cloning Colorer-library...")
            + status("Building Colorer-library (this takes a few minutes)...")
            + "build output\n"
            + status("Cloning the syntax schemes...")
            + status(f"Colorer installed under {self.colorer_home}")
            + status("Rebuild to pick it up:  ./ffm.py install"),
        )
        self.assertEqual(self.names(), ["git", "cmake", "cmake", "git"])
        self.assertEqual(self.sandbox.args("git"), [
            ["clone", "--depth", "1", "https://github.com/colorer/Colorer-library", str(self.library)],
            ["clone", "--depth", "1", "https://github.com/colorer/Colorer-schemes", str(self.schemes)],
        ])
        build = str(self.library / "_build")
        self.assertEqual(self.sandbox.args("cmake"), [
            ["-S", str(self.library), "-B", build, "-DCMAKE_BUILD_TYPE=Release", "-DCOLORER_USE_VCPKG=OFF",
             "-DCOLORER_BUILD_TOOLS=OFF", "-DCOLORER_BUILD_TEST=OFF"],
            ["--build", build, "--parallel"],
        ])
        self.assertTrue(self.colorer_home.is_dir())

    def test_existing_clones_are_updated_and_a_failed_update_is_ignored(self):
        for checkout in (self.library, self.schemes):
            (checkout / ".git").mkdir(parents=True)
        self.sandbox.fake("git", status=1)
        result = self.ffm("colorer", env=self.env)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(status("Updating Colorer-library..."), result.stdout)
        self.assertIn(status("Updating the syntax schemes..."), result.stdout)
        self.assertEqual(self.sandbox.args("git"), [
            ["-C", str(self.library), "pull", "--ff-only", "--quiet"],
            ["-C", str(self.schemes), "pull", "--ff-only", "--quiet"],
        ])

    def test_colorer_home_defaults_under_home(self):
        default = self.sandbox.home / ".local/share/ffm"
        for env in ({}, {"XDG_DATA_HOME": ""}):
            with self.subTest(env=env):
                result = self.ffm("colorer", env=env)
                self.assertIn(status(f"Colorer installed under {default}"), result.stdout)

    def test_a_failed_clone_exits_1(self):
        self.sandbox.fake("git", status=128)
        result = self.ffm("colorer", env=self.env)
        self.assertEqual((result.returncode, result.stdout), (1, status("Cloning Colorer-library...")))
        self.assertEqual(self.names(), ["git"])

    def test_a_failed_schemes_clone_exits_1(self):
        self.sandbox.fake("git", effect="sys.exit(128 if 'Colorer-schemes' in ' '.join(args) else 0)")
        result = self.ffm("colorer", env=self.env)
        self.assertEqual(result.returncode, 1)
        self.assertNotIn("Colorer installed", result.stdout)

    def test_a_failed_configure_exits_1_with_a_hint(self):
        self.sandbox.fake("cmake", status=1)
        result = self.ffm("colorer", env=self.env)
        self.assertEqual(result.returncode, 1)
        hint = "CMake configure failed. Missing libicu-dev / libxml2-dev / zlib1g-dev / libminizip-dev?"
        self.assertTrue(result.stdout.endswith(error(hint)))
        self.assertEqual(self.names(), ["git", "cmake"])

    def test_a_failed_build_exits_1_before_the_schemes(self):
        self.sandbox.fake("cmake", effect="sys.exit(2 if args[0] == '--build' else 0)")
        result = self.ffm("colorer", env=self.env)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(self.names(), ["git", "cmake", "cmake"])

    def test_colorer_skips_the_rust_check(self):
        self.assertEqual(self.ffm("colorer", env=self.env).returncode, 0)
        self.assertNotIn("cargo_preflight", self.names())

    def test_colorer_paths_are_used_as_the_environment_spells_them(self):
        data_home = f"{self.colorer_home.parent}/"
        result = self.ffm("colorer", env={"XDG_DATA_HOME": data_home})
        self.assertEqual(result.returncode, 0, result.stderr)
        library = f"{data_home}/ffm/Colorer-library"
        self.assertEqual(self.sandbox.args("git")[0][-1], library)
        self.assertEqual(self.sandbox.args("cmake")[1], ["--build", f"{library}/_build", "--parallel"])
        self.assertIn(status(f"Colorer installed under {data_home}/ffm"), result.stdout)

    def test_a_colorer_home_that_cannot_be_made_is_reported_and_the_clone_runs(self):
        blocker = self.sandbox.root / "a file"
        blocker.write_text("", encoding="utf-8")
        self.sandbox.fake("git", status=128)
        result = self.ffm("colorer", env={"XDG_DATA_HOME": str(blocker)})
        self.assertEqual((result.returncode, result.stdout), (1, status("Cloning Colorer-library...")))
        self.assertEqual(result.stderr, f"mkdir: cannot create directory '{blocker}/ffm': Not a directory\n")
        self.assertEqual(self.names(), ["git"])


class PathLookupTests(LauncherTest):
    """Commands are looked up with a leading ~ in a PATH entry expanded."""

    def test_cargo_behind_a_tilde_entry_is_found_and_path_is_passed_on_as_it_was(self):
        self.hide("cargo")
        for name in ("cargo", "cargo-tarpaulin"):
            self.sandbox.fake(name, path=self.sandbox.home / ".cargo/bin" / name)
        path = os.pathsep.join(["~/.cargo/bin", str(self.sandbox.bin)])
        for args, expected in ((["build"], [["build"]]), (["clean"], [["clean"]]), (["coverage"], [["tarpaulin"]])):
            with self.subTest(args=args):
                self.forget_calls()
                result = self.ffm(*args, env={"PATH": path})
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(self.sandbox.args("cargo"), expected)
                self.assertEqual(self.sandbox.calls("cargo")[0].env["PATH"], path)

    def test_cmake_behind_a_tilde_entry_is_found(self):
        self.hide("cmake")
        self.sandbox.fake("cmake", path=self.sandbox.home / "tools/cmake")
        self.sandbox.fake("git")
        env = {"PATH": os.pathsep.join(["~/tools", str(self.sandbox.bin)]), "XDG_DATA_HOME": str(self.sandbox.root)}
        result = self.ffm("colorer", env=env)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.names(), ["git", "cmake", "cmake", "git"])


class OutputTests(LauncherTest):
    """A stdout that cannot be written is reported, and the script goes on."""

    def test_a_failed_write_is_reported_and_the_script_goes_on(self):
        # Fakes that leave stdout alone: Python's sys.stdout is None when fd 1 is closed.
        self.sandbox.fake("cargo_preflight", path=self.repo / "scripts/cargo_preflight.py", effect="sys.exit(0)")
        self.sandbox.fake("cargo", effect="sys.exit(0)")
        redirections = {">&-": "Bad file descriptor", ">/dev/full": "No space left on device"}
        for redirection, reason in redirections.items():
            if not os.path.exists("/dev/full") and "full" in redirection:
                continue
            with self.subTest(redirection=redirection):
                self.forget_calls()
                result = self.ffm_with_stdout(redirection, "build")
                report = f"{self.repo / 'ffm.py'}: write error: {reason}\n"
                # The last command was echo, and its status was the script's.
                self.assertEqual((result.returncode, result.stderr), (1, report * 2))
                self.assertEqual(self.names(), ["cargo_preflight", "cargo"])
                self.assertEqual(self.ffm_with_stdout(redirection, "help").returncode, 1)

    def test_a_closed_pipe_ends_the_script_silently_by_sigpipe(self):
        read_end, write_end = os.pipe()
        os.close(read_end)
        try:
            result = self.ffm_bytes("build", stdout=write_end)
        finally:
            os.close(write_end)
        self.assertEqual((result.returncode, result.stderr), (-signal.SIGPIPE, b""))
        self.assertEqual(self.names(), ["cargo_preflight"])


if __name__ == "__main__":
    unittest.main()
