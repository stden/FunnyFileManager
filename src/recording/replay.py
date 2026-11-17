#!/usr/bin/env python3
"""Replay this FFM session. Usage: python session.py [path/to/ffm]."""
import json
import pathlib
import subprocess
import sys
import tempfile


def main():
    rows = [line.removeprefix("# FFM ") for line in
            pathlib.Path(__file__).read_text(encoding="utf-8").splitlines()
            if line.startswith("# FFM ")]
    header = json.loads(rows[0])
    executable = sys.argv[1] if len(sys.argv) > 1 else header["executable"]
    with tempfile.TemporaryDirectory(prefix="ffm-replay-") as directory:
        recording = pathlib.Path(directory) / "events.jsonl"
        recording.write_text("\n".join(rows) + "\n", encoding="utf-8")
        subprocess.run([executable, "--replay", str(recording)],
                       cwd=header["directory"], check=True)


if __name__ == "__main__":
    main()
