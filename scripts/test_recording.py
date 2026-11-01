"""Exercise the generated Python launcher without running filesystem operations."""
import json
import pathlib
import runpy
import tempfile
import unittest
from unittest.mock import patch

ROOT = pathlib.Path(__file__).resolve().parents[1]


class RecordingLauncherTests(unittest.TestCase):
    def test_launcher_preserves_unicode_payload_cwd_and_executable_override(self):
        with tempfile.TemporaryDirectory() as directory:
            script = pathlib.Path(directory) / "session with spaces.py"
            rows = [
                {"version": 1, "executable": "original ffm", "directory": directory, "size": [80, 24]},
                {"delay_ms": 0, "event": {"Paste": "Ж🙂\n'\"\\"}},
                {"clipboard": {"Ok": "recorded clipboard"}},
            ]
            script.write_text(
                (ROOT / "src/recording/replay.py").read_text(encoding="utf-8")
                + "\n" + "\n".join("# FFM " + json.dumps(row) for row in rows), encoding="utf-8"
            )
            with patch("sys.argv", [str(script), "replacement ffm"]):
                with patch("subprocess.run") as execute:
                    execute.side_effect = lambda args, **kwargs: self.check_call(args, kwargs, rows)
                    runpy.run_path(str(script), run_name="__main__")
                    execute.assert_called_once()

    def check_call(self, args, kwargs, expected):
        self.assertEqual(args[:2], ["replacement ffm", "--replay"])
        actual = [json.loads(line) for line in pathlib.Path(args[2]).read_text(encoding="utf-8").splitlines()]
        self.assertEqual(actual, expected)
        self.assertEqual(kwargs, {"cwd": expected[0]["directory"], "check": True})


if __name__ == "__main__":
    unittest.main()
