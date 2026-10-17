"""Compare a manifest's [lints] tables with the lint policy, and say how they differ."""

import json
import re
import tomllib

from lint_policy import ALLOWANCES, COMMON, ENFORCED, TEST_SCOPE, UNSAFE


def render(value):
    """A lint level as the manifest spells it."""
    if isinstance(value, dict):
        return "{ " + ", ".join(f"{key} = {render(item)}" for key, item in value.items()) + " }"
    return json.dumps(value)


def has_reason(text, lint):
    """True when the lint's line carries a comment or follows a comment line."""
    lines = text.splitlines()
    for index, line in enumerate(lines):
        if re.match(rf"\s*{lint}\s*=", line):
            return "#" in line or (index > 0 and lines[index - 1].lstrip().startswith("#"))
    return False


def mismatch(crate, key, required, actual):
    table, lint = key
    found = "missing" if actual is None else render(actual)
    return f"{crate}: [lints.{table}] {lint} is {found}; the policy requires {render(required)}"


def extra(crate, key, value, text):
    """Why a lint the policy does not require breaks it, or None when it may stay."""
    table, lint = key
    where = f"{crate}: [lints.{table}] {lint}"
    if key == ("rust", "unexpected_cfgs"):
        return None
    if lint in ALLOWANCES and value == "allow":
        return (
            None if has_reason(text, lint) else f'{where} = "allow" needs a one-line reason comment'
        )
    if lint in ENFORCED:
        return f"{where} is enforced; remove it from the manifest and fix the code"
    if lint in TEST_SCOPE:
        return f"{where} is test-scope only; allow it in test code, not here"
    return f"{where} is not part of the lint policy"


def policy_violations(crate, text):
    """Every difference between a manifest's [lints] tables and the crate's policy."""
    lints = tomllib.loads(text).get("lints", {})
    declared = {
        (table, lint): value
        for table in ("rust", "clippy")
        for lint, value in lints.get(table, {}).items()
    }
    required = {**COMMON, **UNSAFE[crate]}
    problems = [
        mismatch(crate, key, value, declared.get(key))
        for key, value in required.items()
        if declared.get(key) != value
    ]
    problems += [
        extra(crate, key, value, text) for key, value in declared.items() if key not in required
    ]
    return [problem for problem in problems if problem]
