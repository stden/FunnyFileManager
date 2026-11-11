"""Create the Windows shortcut and batch file with Python orchestration."""

import base64
import ctypes
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

DESCRIPTION = "Funny File Manager - Terminal-based file manager inspired by Far Manager"


def desktop_directory():
    buffer = ctypes.create_unicode_buffer(32768)
    result = ctypes.windll.shell32.SHGetFolderPathW(None, 0x10, None, 0, buffer)
    if result:
        raise OSError(f"Cannot find the Windows desktop: HRESULT {result:#x}")
    return Path(buffer.value)


def properties(root, executable, terminal):
    return {"target": "wt.exe" if terminal else str(executable),
            "arguments": f'-d "{root}" -- "{executable}"' if terminal else "",
            "working_directory": "" if terminal else str(root),
            "icon": str(executable) + ",0", "description": DESCRIPTION}


def write_shortcut(path, values):
    environment = os.environ.copy()
    environment["FFM_SHORTCUT"] = json.dumps({"path": str(path), **values})
    code = ("$ErrorActionPreference='Stop'; $p=$env:FFM_SHORTCUT|ConvertFrom-Json; "
            "$s=(New-Object -ComObject WScript.Shell).CreateShortcut($p.path); "
            "$s.TargetPath=$p.target; $s.Arguments=$p.arguments; $s.WorkingDirectory=$p.working_directory; "
            "$s.IconLocation=$p.icon; $s.Description=$p.description; $s.Save()")
    host = shutil.which("pwsh.exe") or shutil.which("powershell.exe")
    if not host:
        raise OSError("A PowerShell host is required for the WScript shortcut adapter.")
    encoded = base64.b64encode(code.encode("utf-16-le")).decode("ascii")
    subprocess.run([host, "-NoProfile", "-NonInteractive", "-EncodedCommand", encoded],
                   env=environment, check=True)


def main(root):
    executable = root / "target/release/ffm.exe"
    try:
        if not executable.is_file():
            print(f"FFM executable not found at {executable}")
            print("Building FFM in release mode...")
            subprocess.run(["cargo", "build", "--release"], cwd=root, check=False)
            if not executable.is_file():
                print("Failed to build FFM. Please run 'cargo build --release' manually.", file=sys.stderr)
                return 1
        desktop = desktop_directory()
        shortcut = desktop / "FunnyFileManager.lnk"
        write_shortcut(shortcut, properties(root, executable, bool(shutil.which("wt.exe"))))
        batch = desktop / "FunnyFileManager.bat"
        batch.write_text(f'@echo off\ncd /d "{root}"\nstart "" "{executable}"\n', encoding="utf-8")
        print(f"Desktop shortcut created successfully at: {shortcut}")
        print(f"Batch file created at: {batch}")
        return 0
    except (OSError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        return 1
