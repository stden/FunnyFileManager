"""Find a program on PATH the way bash does, for the developer scripts.

shutil.which and subprocess take a PATH entry as written, but bash expands a
leading ~ in it: with `PATH=~/.cargo/bin:...` bash finds Cargo, and so do
these functions. An empty entry is the working directory.
"""

import os
import shutil


def find_all(name, mode=os.X_OK, cwd=""):
    """Every file bash could run as `name`, a bare program name, in PATH order,
    as `type -pa` lists them.

    `mode` is the access the file must grant: os.X_OK for a program, os.F_OK
    for any file. A relative entry is taken from `cwd`, the directory the
    program will start in; "" is this process's own.
    """
    directories = os.environ.get("PATH", os.defpath).split(os.pathsep)
    # One entry at a time, so that shutil.which skips directories and, on
    # Windows, tries the PATHEXT suffixes (cargo.exe).
    matches = [
        shutil.which(name, mode, path=os.path.join(cwd, os.path.expanduser(directory) or "."))
        for directory in directories
    ]
    return [match for match in matches if match]


def find(name, mode=os.X_OK, cwd=""):
    """The file bash runs as `name`, as `command -v` prints it; None if there is none."""
    matches = find_all(name, mode, cwd)
    return matches[0] if matches else None


def resolve(word, cwd=""):
    """The file to execute for a command's first word, for subprocess's `executable`.

    A word that holds a directory (./cargo, /usr/bin/cargo) is the file itself;
    a bare name is looked up with find(). None when nothing is found:
    subprocess then searches PATH on its own and fails naming the word.
    """
    return word if os.path.dirname(word) else find(word, cwd=cwd)
