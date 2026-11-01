"""Test ./ffm.py install: cargo install --force, then every stale ffm on PATH."""

import os
import unittest

from ffm_case import LauncherTest, error, status, warning


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


if __name__ == "__main__":
    unittest.main()
