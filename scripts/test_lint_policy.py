#!/usr/bin/env python3
"""Prove the lint policy is declared in, and enforced from, each real Cargo manifest.

1. Policy: the [lints] tables of the root, mcp/ and colorer-sys/ manifests must
   match the policy below exactly. Mutated copies of compliant fixture tables
   prove the checker rejects each kind of drift with a precise message.
2. Live proof: for each crate, throwaway crates under target/ copy the real
   [lints] tables verbatim and sit below the shared clippy.toml. Clippy must
   reject every negative fixture with exactly its expected lint codes and
   accept every positive fixture.
"""

from collections import defaultdict
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tomllib


ROOT = Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "scripts" / "tests" / "fixtures" / "lint_policy"
OUTPUT = ROOT / "target" / "lint-policy-selftest"
MANIFESTS = {
    "tui": ROOT / "Cargo.toml",
    "mcp": ROOT / "mcp" / "Cargo.toml",
    "colorer": ROOT / "colorer-sys" / "Cargo.toml",
}

# Every crate: the three groups, the readability lints, honest `#[expect]`s.
GROUP = {"level": "warn", "priority": -1}
READABILITY = ("cognitive_complexity", "excessive_nesting", "too_many_lines")
COMMON = {
    **{("clippy", group): GROUP for group in ("all", "pedantic", "nursery")},
    **{("clippy", lint): "warn" for lint in READABILITY},
    ("rust", "unfulfilled_lint_expectations"): "deny",
}
UNSAFE = {
    "tui": {
        ("rust", "unsafe_code"): "deny",
        ("rust", "unsafe_op_in_unsafe_fn"): "deny",
        ("clippy", "undocumented_unsafe_blocks"): "deny",
    },
    "mcp": {("rust", "unsafe_code"): "forbid"},
    "colorer": {
        ("rust", "unsafe_op_in_unsafe_fn"): "deny",
        ("clippy", "undocumented_unsafe_blocks"): "deny",
        ("clippy", "missing_safety_doc"): "deny",
    },
}
# The only crate-wide allowances; each needs a one-line reason comment.
ALLOWANCES = {
    "module_name_repetitions",
    "must_use_candidate",
    "missing_errors_doc",
    "missing_const_for_fn",
    "option_if_let_else",
}
ENFORCED = {
    "cast_possible_truncation",
    "cast_sign_loss",
    "cast_precision_loss",
    "struct_excessive_bools",
    "wildcard_imports",
    "similar_names",
    "missing_panics_doc",
    "items_after_test_module",
}
TEST_SCOPE = (
    "unnecessary_wraps",
    "needless_pass_by_value",
    "needless_pass_by_ref_mut",
    "match_wildcard_for_single_variants",
    "unused_async",
)

CLIPPY = "[lints.clippy]\n"
# (crate, text to replace, replacement, the one violation it must cause or None)
MUTATIONS = (
    (
        "tui",
        CLIPPY,
        CLIPPY + '# Too noisy.\ncast_sign_loss = "allow"\n',
        "tui: [lints.clippy] cast_sign_loss is enforced; remove it from the manifest and fix the code",
    ),
    (
        "tui",
        CLIPPY,
        CLIPPY + '# Test helpers.\nunnecessary_wraps = "allow"\n',
        "tui: [lints.clippy] unnecessary_wraps is test-scope only; allow it in test code, not here",
    ),
    (
        "mcp",
        CLIPPY,
        CLIPPY + '# Cucumber steps.\nunused_async = "allow"\n',
        "mcp: [lints.clippy] unused_async is test-scope only; allow it in test code, not here",
    ),
    (
        "mcp",
        CLIPPY,
        CLIPPY + '# Prose.\ndoc_markdown = "allow"\n',
        "mcp: [lints.clippy] doc_markdown is not part of the lint policy",
    ),
    (
        "tui",
        CLIPPY,
        CLIPPY + 'unwrap_used = "deny"\n',
        "tui: [lints.clippy] unwrap_used is not part of the lint policy",
    ),
    (
        "tui",
        "# Every pure getter would need #[must_use]; noise without catching bugs.\n",
        "",
        'tui: [lints.clippy] must_use_candidate = "allow" needs a one-line reason comment',
    ),
    (
        "tui",
        'unsafe_code = "deny"\n',
        "",
        'tui: [lints.rust] unsafe_code is missing; the policy requires "deny"',
    ),
    (
        "tui",
        'unsafe_op_in_unsafe_fn = "deny"',
        'unsafe_op_in_unsafe_fn = "warn"',
        'tui: [lints.rust] unsafe_op_in_unsafe_fn is "warn"; the policy requires "deny"',
    ),
    (
        "tui",
        'pedantic = { level = "warn", priority = -1 }',
        'pedantic = "warn"',
        'tui: [lints.clippy] pedantic is "warn"; the policy requires { level = "warn", priority = -1 }',
    ),
    (
        "tui",
        'too_many_lines = "warn"',
        'too_many_lines = "allow"',
        'tui: [lints.clippy] too_many_lines is "allow"; the policy requires "warn"',
    ),
    (
        "mcp",
        'unsafe_code = "forbid"',
        'unsafe_code = "deny"',
        'mcp: [lints.rust] unsafe_code is "deny"; the policy requires "forbid"',
    ),
    (
        "mcp",
        'unsafe_code = "forbid"\n',
        "",
        'mcp: [lints.rust] unsafe_code is missing; the policy requires "forbid"',
    ),
    (
        "colorer",
        'undocumented_unsafe_blocks = "deny"\n',
        "",
        'colorer: [lints.clippy] undocumented_unsafe_blocks is missing; the policy requires "deny"',
    ),
    (
        "colorer",
        'missing_safety_doc = "deny"',
        'missing_safety_doc = "warn"',
        'colorer: [lints.clippy] missing_safety_doc is "warn"; the policy requires "deny"',
    ),
    (
        "colorer",
        "[lints.rust]\n",
        '[lints.rust]\nunsafe_code = "forbid"\n',
        "colorer: [lints.rust] unsafe_code is not part of the lint policy",
    ),
    # Dropping an allowance only makes a crate stricter.
    (
        "tui",
        '# Nursery style lint that fights `match`/`if let` readability.\noption_if_let_else = "allow"\n',
        "",
        None,
    ),
)

# Each negative fixture must fail with exactly these codes; positives must pass.
# `unsafe_op_in_unsafe_fn` reports under E0133: without the lint, edition 2021
# accepts an unsafe operation directly in an `unsafe fn` body.
UNSAFE_OP = "E0133"
COMMON_NEGATIVES = {
    "cast_possible_truncation": {"clippy::cast_possible_truncation"},
    "cast_sign_loss": {"clippy::cast_sign_loss"},
    "cast_precision_loss": {"clippy::cast_precision_loss"},
    "struct_excessive_bools": {"clippy::struct_excessive_bools"},
    "wildcard_imports": {"clippy::wildcard_imports"},
    "missing_panics_doc": {"clippy::missing_panics_doc"},
    "unnecessary_wraps": {"clippy::unnecessary_wraps"},
    "readability": {f"clippy::{lint}" for lint in READABILITY},
}
# One crate (one Cargo run) per set. The E0453 hard error stops compilation
# before Clippy's late lints, so the forbid proof gets a crate of its own.
NEGATIVES = {
    "tui": {
        "negative": {
            **COMMON_NEGATIVES,
            "unsafe_block": {"unsafe_code"},
            "undocumented_unsafe_block": {"clippy::undocumented_unsafe_blocks"},
            "unsafe_op_in_unsafe_fn": {UNSAFE_OP},
        },
    },
    "mcp": {
        "negative": {**COMMON_NEGATIVES, "unsafe_block": {"unsafe_code"}},
        "forbid": {"allowed_unsafe": {"E0453", "unsafe_code"}},
    },
    "colorer": {
        "negative": {
            **COMMON_NEGATIVES,
            "undocumented_unsafe_block": {"clippy::undocumented_unsafe_blocks"},
            "unsafe_op_in_unsafe_fn": {UNSAFE_OP},
            "missing_safety_doc": {"clippy::missing_safety_doc"},
        },
    },
}
POSITIVES = {
    "tui": ("checked_conversions", "test_scope_allow", "allowed_unsafe"),
    "mcp": ("checked_conversions", "test_scope_allow"),
    "colorer": ("checked_conversions", "test_scope_allow", "allowed_unsafe", "unsafe_block"),
}
LIBRARY_ROOT = (
    "//! Generated by `scripts/test_lint_policy.py`; test-scope allowances live here.\n"
    f"#![cfg_attr(test, allow({', '.join(f'clippy::{lint}' for lint in TEST_SCOPE)}))]\n\n"
)


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


def check_checker():
    """Compliant fixture tables pass; each mutation causes exactly its violation."""
    compliant = {
        crate: (FIXTURES / "manifests" / f"{crate}.toml").read_text(encoding="utf-8")
        for crate in UNSAFE
    }
    cases = [(crate, text, "", "", None) for crate, text in compliant.items()]  # unchanged
    cases += [
        (crate, compliant[crate], old, new, expected) for crate, old, new, expected in MUTATIONS
    ]
    for crate, text, old, new, expected in cases:
        if old not in text:
            raise RuntimeError(f"{crate} fixture lacks the mutation anchor {old!r}")
        actual = policy_violations(crate, text.replace(old, new, 1))
        if actual != ([expected] if expected else []):
            raise RuntimeError(
                f"{crate}: {new or old!r}\n  expected {expected}\n  got      {actual}"
            )
    print(f"PASS: checker accepts the 3 compliant manifests and judges {len(MUTATIONS)} mutations")


def check_manifests(texts):
    problems = [
        problem for crate, text in texts.items() for problem in policy_violations(crate, text)
    ]
    if problems:
        raise RuntimeError("manifests drift from the lint policy:\n  " + "\n  ".join(problems))
    print("PASS: root, mcp/ and colorer-sys/ manifests match the lint policy")


def lint_tables(crate, text):
    """The manifest's [lints.*] sections, verbatim, with a parse-back check."""
    kept, inside = [], False
    for line in text.splitlines():
        header = re.match(r"\[([^\]]+)\]", line)
        if header:
            inside = header.group(1).startswith("lints")
        if inside:
            kept.append(line)
    copied = "\n".join(kept) + "\n"
    if tomllib.loads(copied).get("lints") != tomllib.loads(text).get("lints"):
        raise RuntimeError(f"{crate}: cannot copy the [lints] tables verbatim")
    return copied


def write_crate(crate, kind, fixtures, text):
    """A crate under target/ (below clippy.toml) with the real [lints] tables."""
    directory = OUTPUT / crate / kind
    (directory / "src").mkdir(parents=True)
    edition = tomllib.loads(text)["package"]["edition"]
    manifest = (
        f"# Generated by scripts/test_lint_policy.py from the {crate} manifest.\n"
        f'[package]\nname = "lint-policy-{crate}-{kind}"\nversion = "0.0.0"\nedition = "{edition}"\n\n'
        "[workspace]\n\n"
    )
    (directory / "Cargo.toml").write_text(manifest + lint_tables(crate, text), encoding="utf-8")
    modules = "".join(f"pub mod {name};\n" for name in fixtures)
    (directory / "src" / "lib.rs").write_text(LIBRARY_ROOT + modules, encoding="utf-8")
    for name in fixtures:
        shutil.copyfile(FIXTURES / f"{name}.rs", directory / "src" / f"{name}.rs")
    return directory


def child_environment():
    """Lint levels come from the manifest alone, and clippy.toml from discovery."""
    environment = os.environ.copy()
    for name in (
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_BUILD_RUSTFLAGS",
        "CLIPPY_CONF_DIR",
        "CARGO_MANIFEST_DIR",
    ):
        environment.pop(name, None)
    environment["CARGO_TARGET_DIR"] = str(OUTPUT / "cargo-target")
    return environment


def clippy(directory):
    """Run the gate's own command, keeping the JSON diagnostics for inspection."""
    result = subprocess.run(
        [
            "cargo",
            "clippy",
            "--offline",
            "--all-targets",
            "--message-format=json",
            "--",
            "-D",
            "warnings",
        ],
        cwd=directory,
        env=child_environment(),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=300,
        check=False,
    )
    (directory / "clippy.jsonl").write_text(result.stdout, encoding="utf-8")
    return result


def codes_by_fixture(stdout):
    """Diagnostic codes keyed by the fixture file of their primary span."""
    found = defaultdict(set)
    for line in stdout.splitlines():
        record = json.loads(line) if line.startswith("{") else {}
        message = record.get("message") if record.get("reason") == "compiler-message" else None
        if not message or not message.get("code"):
            continue
        primary = [span["file_name"] for span in message["spans"] if span["is_primary"]]
        found[Path(primary[0]).stem if primary else "<no span>"].add(message["code"]["code"])
    return found


def check_negatives(crate, kind, expected, text):
    directory = write_crate(crate, kind, expected, text)
    result = clippy(directory)
    found = codes_by_fixture(result.stdout)
    wrong = [
        f"{name}: expected {sorted(codes)}, got {sorted(found.get(name, ()))}"
        for name, codes in expected.items()
        if found.get(name, set()) != codes
    ]
    wrong += [
        f"{name}: unexpected {sorted(codes)}"
        for name, codes in found.items()
        if name not in expected
    ]
    if result.returncode == 0 or wrong:
        raise RuntimeError(
            f"{crate}: negative fixtures must fail with exactly their lints "
            f"(exit {result.returncode}; see {directory / 'clippy.jsonl'}):\n  "
            + "\n  ".join(wrong)
            + f"\n{result.stderr[-2000:]}"
        )
    print(
        f"PASS: {crate} manifest rejects all {len(expected)} {kind} fixture(s) with their exact codes"
    )


def check_positives(crate, text):
    directory = write_crate(crate, "positive", POSITIVES[crate], text)
    result = clippy(directory)
    if result.returncode:
        found = {name: sorted(codes) for name, codes in codes_by_fixture(result.stdout).items()}
        raise RuntimeError(
            f"{crate}: positive fixtures must pass; got {found}\n{result.stderr[-2000:]}"
        )
    print(f"PASS: {crate} manifest accepts {len(POSITIVES[crate])} positive fixtures")


def main():
    check_checker()
    texts = {crate: path.read_text(encoding="utf-8") for crate, path in MANIFESTS.items()}
    check_manifests(texts)
    shutil.rmtree(OUTPUT, ignore_errors=True)
    for crate, text in texts.items():
        for kind, expected in NEGATIVES[crate].items():
            check_negatives(crate, kind, expected, text)
        check_positives(crate, text)


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.TimeoutExpired, tomllib.TOMLDecodeError) as error:
        sys.stdout.flush()  # keep the PASS lines ahead of the failure in a shared log
        print(f"Lint policy self-test failed: {error}", file=sys.stderr)
        sys.exit(1)
