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
import signal
import subprocess
import sys

# shell_path.py, beside this file, finds programs as bash does, k9s_eks_shell.py
# runs them and k9s_eks_aws.py asks aws. Explicitly first on sys.path:
# `python3 -P` or PYTHONSAFEPATH=1 leave this file's directory out, and no
# other module of those names may stand in.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from k9s_eks_aws import add_context, list_clusters  # noqa: E402
from k9s_eks_shell import (  # noqa: E402
    KILLED_BY_SIGPIPE, USAGE_ERROR, fail, require, run, run_tool, shell_status, write_stdout,
)


REGION_VARIABLES = ("EKS_REGION", "AWS_REGION", "AWS_DEFAULT_REGION")
PROFILE_VARIABLES = ("EKS_PROFILE", "AWS_PROFILE")
# Each subcommand's positional arguments, in order. Only the cluster is required.
SUBCOMMANDS = {
    "ls": ("region", "profile"),
    "use": ("cluster", "region", "profile", "alias"),
    "open": ("cluster", "region", "profile", "alias"),
    "select": ("region", "profile"),
}


def require_cluster(cluster, subcommand):
    if not cluster:
        fail(f"Usage: scripts/k9s_eks.py {subcommand} <cluster> [region] [profile] [alias]",
             USAGE_ERROR)


def first_set(names):
    """The value of the first variable in `names` that is set and not empty."""
    return next((os.environ[name] for name in names if os.environ.get(name)), "")


def print_clusters(region, profile):
    """Print the region's clusters, one per line; return aws's status."""
    status, clusters = list_clusters(region, profile)
    return status if write_stdout(clusters) else KILLED_BY_SIGPIPE


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
