#!/usr/bin/env python3
"""Point kubectl or k9s at an Amazon EKS cluster.

Usage:
  scripts/k9s_eks.py ls [region] [profile]
  scripts/k9s_eks.py use <cluster> [region] [profile] [alias]
  scripts/k9s_eks.py open <cluster> [region] [profile] [alias]
  scripts/k9s_eks.py select [region] [profile]    # requires fzf

  ls      list the region's clusters, one per line
  use     add the cluster to your kubeconfig and make it kubectl's context
  open    add the cluster to your kubeconfig and start k9s on it
  select  pick one of the region's clusters with fzf, then open it

The words after the subcommand are taken in order, exactly as typed: one that
starts with "-" is a value too, an empty one counts as missing, and extra
ones are ignored.

The kube context is named after the alias, <cluster>@<region> by default.
An AWS CLI too old for `update-kubeconfig --alias` is asked again without
it, and the context is then the one its output names.

Defaults:
  region:  EKS_REGION or AWS_REGION or AWS_DEFAULT_REGION
  profile: EKS_PROFILE or AWS_PROFILE

Example:
  scripts/k9s_eks.py open my-cluster us-west-2 myprofile

Requires: aws, plus kubectl for use, k9s for open, and k9s and fzf for select.

This replaces scripts/k9s-eks.sh, whose shell functions you sourced from your
shell rc. A Python program cannot define functions in your shell, so they
became subcommands. `use` still switches every shell, because kubectl keeps
the current context in the kubeconfig file. These aliases in your shell rc
bring the old names back:

  alias eks_ls='/path/to/repo/scripts/k9s_eks.py ls'
  alias eks_use='/path/to/repo/scripts/k9s_eks.py use'
  alias k9s_eks='/path/to/repo/scripts/k9s_eks.py open'
  alias k9s_eks_select='/path/to/repo/scripts/k9s_eks.py select'

Being a program, it sees only what your shell exports. The default variables
must be exported: a plain `EKS_REGION=eu-west-1` line in your shell is not
seen. aws, kubectl, k9s and fzf must be programs on PATH: shell functions and
aliases with those names are not used.

`ls` now fails when aws fails. The old eks_ls ended with the status of `tr`
in its `aws | tr` pipeline, so it succeeded even then.
"""

import argparse
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
from typing import NoReturn

# shell_path.py, beside this file, finds programs as bash does. Explicitly
# first on sys.path: `python3 -P` or PYTHONSAFEPATH=1 leave this file's
# directory out, and no other `shell_path` module may stand in.
sys.path.insert(0, str(Path(__file__).resolve().parent))
import shell_path  # noqa: E402


MESSAGE_PREFIX = "k9s-eks: "
MISSING_COMMAND = 127
USAGE_ERROR = 2
NO_CONTEXT = 1
REGION_REQUIRED = "region required (pass as arg or set AWS_REGION/AWS_DEFAULT_REGION/EKS_REGION)"
REGION_VARIABLES = ("EKS_REGION", "AWS_REGION", "AWS_DEFAULT_REGION")
PROFILE_VARIABLES = ("EKS_PROFILE", "AWS_PROFILE")
# Each subcommand's positional arguments, in order. Only the cluster is required.
SUBCOMMANDS = {
    "ls": ("region", "profile"),
    "use": ("cluster", "region", "profile", "alias"),
    "open": ("cluster", "region", "profile", "alias"),
    "select": ("region", "profile"),
}
# How an AWS CLI without `update-kubeconfig --alias` rejects the option.
ALIAS_REJECTED = re.compile(
    rb"(Unknown options|unknown option|unrecognized arguments).*--alias", re.DOTALL
)
# The context update-kubeconfig reports, as in "Updated context arn:... in ...".
CONTEXT_REPORT = re.compile(rb".*context ([^ ]+)")
# The shell names the signal that killed a foreground command, except these.
UNREPORTED_SIGNALS = (signal.SIGINT, signal.SIGPIPE)
# `ls` ends like the old `aws | tr` did when its reader stopped early.
KILLED_BY_SIGPIPE = 128 + signal.SIGPIPE


def complain(message):
    """Print `message` on stderr, after the script's prefix."""
    print(MESSAGE_PREFIX + message, file=sys.stderr)


def fail(message: str, status: int) -> NoReturn:
    """Print `message` on stderr and end the script with `status`."""
    complain(message)
    raise SystemExit(status)


def has_command(command):
    """True if `command` is on PATH; otherwise say that it is missing."""
    if shell_path.find(command):
        return True
    complain(f"missing required command: {command}")
    return False


def run(command, **options):
    """subprocess.run for aws, kubectl, k9s or fzf, found on PATH as bash finds them."""
    return subprocess.run(command, executable=shell_path.find(command[0]), check=False, **options)


def require(command):
    if not has_command(command):
        raise SystemExit(MISSING_COMMAND)


def require_region(region):
    if not region:
        fail(REGION_REQUIRED, USAGE_ERROR)


def require_cluster(cluster, subcommand):
    if not cluster:
        fail(f"Usage: scripts/k9s_eks.py {subcommand} <cluster> [region] [profile] [alias]",
             USAGE_ERROR)


def first_set(names):
    """The value of the first variable in `names` that is set and not empty."""
    return next((os.environ[name] for name in names if os.environ.get(name)), "")


def shell_status(returncode):
    """A child's exit status as the shell reports it: 128+N if signal N killed it."""
    return 128 - returncode if returncode < 0 else returncode


def run_tool(command):
    """Run kubectl or k9s in the foreground; return its status as the shell would.

    Like the shell, name the signal that killed it ("Killed", "Terminated").
    """
    returncode = run(command).returncode
    if returncode < 0 and -returncode not in UNREPORTED_SIGNALS:
        print(signal.strsignal(-returncode), file=sys.stderr)
    return shell_status(returncode)


def write_stdout(data):
    """Write bytes to stdout; False if the reader has gone."""
    try:
        sys.stdout.buffer.write(data)
        sys.stdout.buffer.flush()
    except BrokenPipeError:
        # Point stdout at /dev/null, so that Python's flush at exit stays quiet.
        os.dup2(os.open(os.devnull, os.O_WRONLY), sys.stdout.fileno())
        return False
    return True


def echo(output):
    """Show aws's output on stderr, byte for byte, plus a newline."""
    sys.stderr.buffer.write(output + b"\n")
    sys.stderr.buffer.flush()


def list_clusters(region, profile):
    """Run `aws eks list-clusters`; return its status and the names, one per line.

    Like the old `aws ... | tr`, what aws printed is kept even when it fails.
    """
    if not has_command("aws"):
        return MISSING_COMMAND, b""
    if not region:
        complain(REGION_REQUIRED)
        return USAGE_ERROR, b""
    command = ["aws", "eks", "list-clusters", "--region", region,
               "--output", "text", "--query", "clusters[]"]
    if profile:
        command += ["--profile", profile]
    result = run(command, stdout=subprocess.PIPE)
    return shell_status(result.returncode), result.stdout.replace(b"\t", b"\n")


def run_aws(command):
    """Run aws with stderr merged into stdout; return its status and output.

    The output is bytes without trailing newlines, as the shell's $(...) kept it.
    """
    result = run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    return shell_status(result.returncode), result.stdout.rstrip(b"\n")


def print_clusters(region, profile):
    """Print the region's clusters, one per line; return aws's status."""
    status, clusters = list_clusters(region, profile)
    return status if write_stdout(clusters) else KILLED_BY_SIGPIPE


def add_context(cluster, region, profile, alias):
    """Add the cluster to the kubeconfig and return the name of its context.

    aws's output is echoed to stderr. Any failure ends the script.
    """
    require("aws")
    require_region(region)
    command = ["aws", "eks", "update-kubeconfig", "--name", cluster, "--region", region]
    if profile:
        command += ["--profile", profile]
    if os.environ.get("KUBECONFIG"):
        command += ["--kubeconfig", os.environ["KUBECONFIG"]]
    alias = alias or f"{cluster}@{region}"
    status, output = run_aws([*command, "--alias", alias])
    if status and ALIAS_REJECTED.search(output):
        return add_context_without_alias(command)
    echo(output)
    if status:
        raise SystemExit(status)
    # The shell passed the context on through $(...), which drops trailing newlines.
    return alias.rstrip("\n")


def add_context_without_alias(command):
    """For an AWS CLI without --alias: return the context its output names."""
    status, output = run_aws(command)
    echo(output)
    if status:
        raise SystemExit(status)
    # Lines end at "\n" only, as for sed: a "\r" or form feed is part of the name.
    for line in output.split(b"\n"):
        if report := CONTEXT_REPORT.match(line):
            return os.fsdecode(report[1])
    fail(
        "unable to determine kube context from aws output; "
        "pass an alias or use kubectl to select context",
        NO_CONTEXT,
    )


def use_cluster(cluster, region, profile, alias):
    """Make the cluster kubectl's current context; return kubectl's status."""
    require_cluster(cluster, "use")
    context = add_context(cluster, region, profile, alias)
    require("kubectl")
    return run_tool(["kubectl", "config", "use-context", context])


def open_cluster(cluster, region, profile, alias=""):
    """Start k9s on the cluster; return k9s's status."""
    require_cluster(cluster, "open")
    context = add_context(cluster, region, profile, alias)
    require("k9s")
    return run_tool(["k9s", "--context", context])


def select_cluster(region, profile):
    """Pick a cluster with fzf and open it; a cancelled pick is fzf's status.

    As in the old `eks_ls | fzf` pipeline, fzf offers whatever the listing
    printed, even an empty list after a missing aws or region. Unlike the
    pipeline, fzf starts once the listing has ended, not alongside it.
    """
    require("fzf")
    _, clusters = list_clusters(region, profile)
    picked = run(["fzf", "--prompt", "EKS cluster> "], input=clusters, stdout=subprocess.PIPE)
    if picked.returncode:
        return shell_status(picked.returncode)
    return open_cluster(os.fsdecode(picked.stdout.rstrip(b"\n")), region, profile)


def parse_subcommand(argv):
    """The subcommand, which argparse checks; it also answers -h."""
    parser = argparse.ArgumentParser(
        usage="%(prog)s [-h] {ls,use,open,select} [argument ...]",
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("subcommand", choices=SUBCOMMANDS)
    # Only the first word: argparse would take the others' "-x" or "--" for syntax.
    return parser.parse_args(argv[:1]).subcommand


def positionals(subcommand, words):
    """The subcommand's arguments by name; missing ones are empty, extra ones ignored."""
    names = SUBCOMMANDS[subcommand]
    return dict(zip(names, [*words, *[""] * len(names)]))


def main(argv=None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    subcommand = parse_subcommand(argv)
    given = positionals(subcommand, argv[1:])
    # An empty argument counts as a missing one.
    region = given["region"] or first_set(REGION_VARIABLES)
    profile = given["profile"] or first_set(PROFILE_VARIABLES)
    # A dict, not match: macOS's own python3 is 3.9.
    subcommands = {
        "ls": lambda: print_clusters(region, profile),
        "use": lambda: use_cluster(given["cluster"], region, profile, given["alias"]),
        "open": lambda: open_cluster(given["cluster"], region, profile, given["alias"]),
        "select": lambda: select_cluster(region, profile),
    }
    return subcommands[subcommand]()


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        # Ctrl+C: end quietly, killed by SIGINT as the shell functions were.
        signal.signal(signal.SIGINT, signal.SIG_DFL)
        os.kill(os.getpid(), signal.SIGINT)
