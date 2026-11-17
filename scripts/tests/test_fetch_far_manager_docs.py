"""Check documentation selection and source records without network access."""

import contextlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import fetch_far_manager_docs as fetcher  # noqa: E402


class FetchDocsTests(unittest.TestCase):
    def test_selection_keeps_documentation_and_licenses(self):
        for name in ("README", "LICENSE.txt", "manual.HLF", "image.svg"):
            self.assertTrue(fetcher.documentation(Path(name)))
        self.assertFalse(fetcher.documentation(Path("source.cpp")))

    def test_fetch_preserves_layout_and_records_source(self):
        def git(*args):
            if args[0] == "clone":
                repository = args[-1]
                (repository / "docs").mkdir(parents=True)
                (repository / "docs/manual.md").write_text("manual", encoding="utf-8")
                (repository / "source.cpp").write_text("source", encoding="utf-8")
                (repository / ".git").mkdir()
                (repository / ".git/internal.txt").write_text("private metadata", encoding="utf-8")
                return ""
            return "commit" if "rev-parse" in args else "origin"

        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / "docs"
            with patch.object(fetcher, "git", side_effect=git):
                fetcher.fetch(destination, "local-fixture")
            self.assertEqual((destination / "docs/manual.md").read_text(encoding="utf-8"), "manual")
            self.assertFalse((destination / "source.cpp").exists())
            self.assertFalse((destination / ".git").exists())
            source = json.loads((destination / "SOURCE.json").read_text(encoding="utf-8"))
            self.assertEqual((source["file_count"], source["repo"], source["commit"]), (1, "origin", "commit"))
            with self.assertRaises(ValueError):
                fetcher.fetch(destination, "ignored", clean=True)

    def test_git_runs_git_with_text_arguments_and_returns_stripped_output(self):
        with patch.object(fetcher.subprocess, "run") as run:
            run.return_value.stdout = " abc123\n"
            self.assertEqual(fetcher.git("-C", Path("some checkout"), "rev-parse", "HEAD"), "abc123")
        self.assertEqual(run.call_args.args[0], ["git", "-C", "some checkout", "rev-parse", "HEAD"])
        self.assertTrue(run.call_args.kwargs["check"])

    def test_clean_replaces_the_previous_download_below_docs_upstream(self):
        def git(*args):
            if args[0] == "clone":
                (args[-1] / "README").parent.mkdir(parents=True)
                (args[-1] / "README").write_text("new", encoding="utf-8")
            return "value"

        with tempfile.TemporaryDirectory() as temporary:
            checkout = Path(temporary)
            destination = checkout / "docs/upstream/far-manager"
            destination.mkdir(parents=True)
            (destination / "stale.txt").write_text("old", encoding="utf-8")
            with patch.object(fetcher, "__file__", str(checkout / "scripts/fetch_far_manager_docs.py")), \
                 patch.object(fetcher, "git", side_effect=git), contextlib.redirect_stdout(io.StringIO()):
                fetcher.fetch(destination, "local-fixture", clean=True)
            self.assertFalse((destination / "stale.txt").exists())
            self.assertEqual((destination / "README").read_text(encoding="utf-8"), "new")

    def test_main_passes_the_options_to_fetch_and_succeeds(self):
        with patch.object(fetcher, "fetch") as fetch:
            self.assertEqual(fetcher.main([]), 0)
            self.assertEqual(fetcher.main(["--destination", "out", "--repo-url", "local", "--clean"]), 0)
            self.assertEqual(fetcher.main(["-Destination", "out", "-RepoUrl", "local", "-Clean"]), 0)
        self.assertEqual([call.args for call in fetch.call_args_list], [
            (Path("docs/upstream/far-manager"), "https://github.com/FarGroup/FarManager.git", False),
            (Path("out"), "local", True),
            (Path("out"), "local", True),
        ])

    def test_main_exits_with_status_one_and_the_message_when_fetch_fails(self):
        failures = (OSError("disk full"), ValueError("unsafe clean"), subprocess.CalledProcessError(128, "git"))
        for failure in failures:
            with self.subTest(failure=failure), patch.object(fetcher, "fetch", side_effect=failure), \
                 contextlib.redirect_stderr(io.StringIO()) as err, self.assertRaises(SystemExit) as exit_:
                fetcher.main([])
            self.assertEqual((exit_.exception.code, err.getvalue()), (1, f"{failure}\n"))

    def test_script_refuses_to_clean_a_directory_outside_docs_upstream(self):
        with tempfile.TemporaryDirectory() as temporary:
            command = [sys.executable, fetcher.__file__, "--clean", "--destination", temporary]
            result = subprocess.run(command, capture_output=True, text=True, check=False)
            self.assertTrue(Path(temporary).is_dir())
        self.assertEqual((result.returncode, result.stdout), (1, ""))
        self.assertIn("--clean requires a child directory below this checkout's docs/upstream", result.stderr)


if __name__ == "__main__":
    unittest.main()
