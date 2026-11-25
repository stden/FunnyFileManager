"""chmod +x and cp as the bash original of create_desktop_shortcut.py ran them:
a failure is reported as those commands report it, and the script goes on.
"""

import os
import stat
import sys


def add_execute_permission(path):
    """`chmod +x`: add the execute bits the umask allows.

    Like chmod's, a failure is reported and the script goes on: an ffm that
    another user owns keeps its mode.
    """
    umask = os.umask(0)
    os.umask(umask)
    try:
        path.chmod(stat.S_IMODE(path.stat().st_mode) | (0o111 & ~umask))
    except OSError as error:
        print(f"chmod: changing permissions of '{path}': {error.strerror}", file=sys.stderr)


def copy_into(directory, source):
    """`cp source directory/`; returns the copy's path.

    As with cp, an existing copy keeps its mode, a new one gets the source's
    less the umask, and a failure is reported while the script goes on.
    """
    copy = directory / source.name
    try:
        mode = source.stat().st_mode & 0o777
        with open(os.open(copy, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, mode), "wb") as target:
            target.write(source.read_bytes())
    except IsADirectoryError:
        print(f"cp: cannot overwrite directory '{copy}' with non-directory", file=sys.stderr)
    except OSError as error:
        print(f"cp: cannot create regular file '{copy}': {error.strerror}", file=sys.stderr)
    return copy
