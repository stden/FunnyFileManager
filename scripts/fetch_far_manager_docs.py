#!/usr/bin/env python3
"""Fetch Far Manager documentation and record its exact Git source."""

import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

EXTENSIONS = {".md", ".markdown", ".rst", ".txt", ".hlf", ".chm", ".html", ".htm", ".pdf",
              ".png", ".jpg", ".jpeg", ".gif", ".svg"}


def documentation(path):
    return path.suffix.lower() in EXTENSIONS or re.fullmatch(r"(?:README|LICENSE|COPYING)(?:\..*)?", path.name, re.I)


def git(*arguments):
    return subprocess.run(["git", *map(str, arguments)], check=True, capture_output=True,
                          text=True, encoding="utf-8").stdout.strip()


def fetch(destination, url, clean=False):
    destination = destination.absolute()
    if clean and destination.exists():
        resolved = destination.resolve()
        checkout = Path(__file__).resolve().parents[1]
        allowed = (checkout / "docs/upstream").resolve()
        if allowed not in resolved.parents or destination.is_symlink():
            raise ValueError("--clean requires a child directory below this checkout's docs/upstream directory.")
        shutil.rmtree(destination)
    destination.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ffm-farmanager-docs-") as temporary:
        repository = Path(temporary) / "FarManager"
        git("clone", "--depth", "1", url, repository)
        commit = git("-C", repository, "rev-parse", "HEAD")
        origin = git("-C", repository, "config", "--get", "remote.origin.url")
        count = 0
        for path in repository.rglob("*"):
            relative = path.relative_to(repository)
            if ".git" in relative.parts or not path.is_file() or path.is_symlink() or not documentation(path):
                continue
            target = destination / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, target)
            count += 1
        source = {"repo": origin, "commit": commit, "fetched_at_utc": datetime.now(timezone.utc).isoformat(),
                  "file_count": count}
        (destination / "SOURCE.json").write_text(json.dumps(source, indent=2) + "\n", encoding="utf-8")
    print(f"Downloaded {count} documentation files to '{destination}' (commit {commit}).")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--destination", "-Destination", type=Path, default=Path("docs/upstream/far-manager"))
    parser.add_argument("--repo-url", "-RepoUrl", default="https://github.com/FarGroup/FarManager.git")
    parser.add_argument("--clean", "-Clean", action="store_true")
    arguments = parser.parse_args(argv)
    try:
        fetch(arguments.destination, arguments.repo_url, arguments.clean)
        return 0
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, str(error) + "\n")


if __name__ == "__main__":
    raise SystemExit(main())
