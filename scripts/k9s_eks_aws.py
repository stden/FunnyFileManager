"""What scripts/k9s_eks.py asks aws: the region's clusters, and a kubeconfig
context for one of them.
"""

import os
import re
import subprocess

from k9s_eks_shell import (
    MISSING_COMMAND, USAGE_ERROR, complain, echo, fail, has_command, require, run, shell_status,
)


NO_CONTEXT = 1
REGION_REQUIRED = "region required (pass as arg or set AWS_REGION/AWS_DEFAULT_REGION/EKS_REGION)"
# How an AWS CLI without `update-kubeconfig --alias` rejects the option.
ALIAS_REJECTED = re.compile(
    rb"(Unknown options|unknown option|unrecognized arguments).*--alias", re.DOTALL
)
# The context update-kubeconfig reports, as in "Updated context arn:... in ...".
CONTEXT_REPORT = re.compile(rb".*context ([^ ]+)")


def require_region(region):
    if not region:
        fail(REGION_REQUIRED, USAGE_ERROR)


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
