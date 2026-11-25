"""Fake effects and runners for the signal tests of lint.py, the pre-commit hook
and the preflight: a fake that dies of a signal or waits for one, a script
signalled as a process group, and a script that writes into a closed pipe.
"""

import os
import signal
import subprocess
import sys
import time


def dies_of(signal_number, condition):
    """A fake's effect: when `condition` holds, die of `signal_number`.

    Python handles SIGINT and ignores SIGPIPE, so the fake restores their default actions first.
    """
    return f"""
        import signal
        if {condition}:
            signal.signal(signal.SIGINT, signal.SIG_DFL)
            signal.signal(signal.SIGPIPE, signal.SIG_DFL)
            os.kill(os.getpid(), {int(signal_number)})
    """


def waits_for_a_signal(started, condition):
    """A fake's effect: when `condition` holds, create `started` and sleep until a signal ends it."""
    return f"""
        import signal, time
        if {condition}:
            signal.signal(signal.SIGINT, signal.SIG_DFL)
            open({str(started)!r}, "w").close()
            time.sleep(60)
    """


def signal_group(sandbox, command, started, signal_number, env=None):
    """Run a copied script, send `signal_number` to its process group once `started` exists.

    Returns (returncode, stdout, stderr); the script and its fakes share the group,
    as a terminal's Ctrl+C or a cancelled CI step reaches them all.
    """
    script, *args = command
    process = subprocess.Popen(
        [sys.executable, sandbox.repo / script, *args],
        cwd=sandbox.repo, env=sandbox.env(env), stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, encoding="utf-8", process_group=0,
        # A test runner that ignores SIGINT would pass that on; Ctrl+C reaches a default handler.
        preexec_fn=lambda: signal.signal(signal.SIGINT, signal.SIG_DFL),
    )
    try:
        deadline = time.monotonic() + 30
        while not started.exists() and time.monotonic() < deadline:
            time.sleep(0.05)
        os.killpg(process.pid, signal_number)
        stdout, stderr = process.communicate(timeout=30)
    finally:
        process.kill()  # does nothing once the script has ended
    return process.returncode, stdout, stderr


def run_into_closed_pipe(sandbox, script, *args, env=None):
    """Run a copied script whose stdout is a pipe nobody reads any more, as after `| head -1`.

    Returns the finished process, with stderr as bytes.
    """
    read_end, write_end = os.pipe()
    os.close(read_end)
    try:
        return subprocess.run(
            [sys.executable, sandbox.repo / script, *args], cwd=sandbox.repo, env=sandbox.env(env),
            stdout=write_end, stderr=subprocess.PIPE, timeout=60, check=False,
        )
    finally:
        os.close(write_end)
