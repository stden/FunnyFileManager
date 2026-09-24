"""After `cargo install`: every other ffm on PATH gets the bytes just built."""

import filecmp
import os
import shutil

from launcher_output import status, warning
from shell_path import find_all


def same_bytes(first, second):
    """Whether two files hold the same bytes, as `cmp -s` says; unreadable is different."""
    try:
        return filecmp.cmp(first, second, shallow=False)
    except OSError:
        return False


def refresh_stale_copies(cargo_bin):
    """Overwrite every other executable ffm on PATH whose bytes differ from cargo_bin.

    A copy in /usr/local/bin outlived rebuild after rebuild and kept serving a
    UI from August: `cargo install` writes only to ~/.cargo/bin, PATH order
    alone decides which binary actually runs, and nothing warns you the two
    disagree. Whichever copy wins should be the one just built.
    """
    for other in find_all("ffm"):
        if other == cargo_bin or not os.path.isfile(other) or same_bytes(cargo_bin, other):
            continue
        try:
            shutil.copyfile(cargo_bin, other)
        except OSError:
            warning(f"Stale copy shadowing yours (not writable): {other}")
            warning(f"  sudo cp {cargo_bin} {other}")
        else:
            status(f"Refreshed stale copy: {other}")
