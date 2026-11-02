"""Test ./ffm.py colorer: clone or update Colorer and its schemes, then build it."""

import unittest

from ffm_case import LauncherTest, error, status


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


if __name__ == "__main__":
    unittest.main()
