#!/usr/bin/env python3
"""Prove the lint policy is declared in, and enforced from, each real Cargo manifest.

1. Policy: the [lints] tables of the root, mcp/ and colorer-sys/ manifests must
   match the policy below exactly. Mutated copies of compliant fixture tables
   prove the checker rejects each kind of drift with a precise message.
2. Live proof: for each crate, throwaway crates under target/ copy the real
   [lints] tables verbatim and sit below the shared clippy.toml. Clippy must
   reject every negative fixture with exactly its expected lint codes and
   accept every positive fixture, including test code that uses the
   clippy.toml allowances (unwrap, expect, index, panic, print, dbg!).
"""

from pathlib import Path
import shutil
import subprocess
import sys
import tomllib

# The lint_policy* modules beside this file hold the policy, its checker, the
# mutations and the Clippy fixtures' codes. Explicitly first on sys.path:
# `python3 -P` or PYTHONSAFEPATH=1 leave this file's directory out, and no
# other module of those names may stand in.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from lint_policy import UNSAFE  # noqa: E402
from lint_policy_checker import policy_violations  # noqa: E402
from lint_policy_clippy import FIXTURES, OUTPUT, ROOT, check_negatives, check_positives  # noqa: E402
from lint_policy_fixtures import NEGATIVES  # noqa: E402
from lint_policy_mutations import MUTATIONS  # noqa: E402

MANIFESTS = {
    "tui": ROOT / "Cargo.toml",
    "mcp": ROOT / "mcp" / "Cargo.toml",
    "colorer": ROOT / "colorer-sys" / "Cargo.toml",
}


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
