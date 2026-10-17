"""Where ./ffm.py finds things: the checkout, the ffm that Cargo builds, and
the directories that the environment names.
"""

import os
from pathlib import Path


# The checkout root: the directory of ffm.py, just above scripts/.
ROOT = Path(__file__).resolve().parents[1]


def env_or_home(variable, under_home):
    """$variable if it is set and not empty, else $HOME/under_home.

    Plain text: no slash is tidied away, and an unset HOME counts as empty.
    """
    return os.environ.get(variable) or f"{os.environ.get('HOME', '')}/{under_home}"


def binary(mode):
    """The ffm that `cargo build` makes in `mode`, relative to the checkout root."""
    return f"target/{mode}/ffm"
