"""Check documentation selection and source records without network access."""

import json
from pathlib import Path
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


if __name__ == "__main__":
    unittest.main()
