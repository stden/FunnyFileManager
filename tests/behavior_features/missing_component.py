# Purpose: A local rustup-style LSP failure, with launch counts and no downloads.
from pathlib import Path
import sys

with (Path.cwd() / "spawns").open("a", encoding="utf-8") as launches:
    launches.write("start\n")
sys.stderr.write(
    "error: 'rust-analyzer.exe' is not installed for the toolchain 'fixture'\n"
)
raise SystemExit(1)
