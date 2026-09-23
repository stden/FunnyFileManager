#!/usr/bin/env bash
# Purpose: Fail within seconds when Cargo's compiler wrapper stalls or is broken.
#
# A wrapper that never answers (typically a shared sccache server holding
# Cargo's output pipes open) turns every build into a wait of many minutes with
# no rustc running. This asks for `rustc -vV`, the first thing every build
# runs, through the wrapper this repository's Cargo configuration and the
# caller's environment select. It uses a throwaway crate with its own target
# dir, so Cargo's cached rustc info cannot skip the call.
#
# Usage: scripts/cargo-preflight.sh
#   FFM_WRAPPER_PROBE_SECS  seconds before the wrapper counts as stalled (20)
#   CARGO                   Cargo to probe with (default: `cargo` on PATH)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIMIT="${FFM_WRAPPER_PROBE_SECS:-20}"
CARGO_BIN="${CARGO:-cargo}"

# GNU timeout (gtimeout with Homebrew coreutils). --foreground keeps Cargo in
# the terminal's process group, so Ctrl+C stops the probe at once.
TIMEOUT="$(command -v timeout || command -v gtimeout || true)"
if [[ -z "$TIMEOUT" ]] || ! "$TIMEOUT" --foreground -k 1 5 true 2> /dev/null; then
    echo "note: no GNU timeout(1); skipping the compiler-wrapper probe" >&2
    exit 0
fi

# Cargo and rustup read .cargo/config.toml and rust-toolchain.toml from the
# working directory, not from the manifest's, so probe from the checkout root.
cd "$ROOT"
# Untimed and wrapper-free: lets rustup install the pinned toolchain first.
"$CARGO_BIN" -V > /dev/null

PROBE="$(mktemp -d "${TMPDIR:-/tmp}/ffm-wrapper-probe.XXXXXX")"
trap 'rm -rf "$PROBE"' EXIT
mkdir "$PROBE/src"
: > "$PROBE/src/lib.rs"
printf '[package]\nname = "wrapper-probe"\nversion = "0.0.0"\nedition = "2021"\n\n[workspace]\n' \
    > "$PROBE/Cargo.toml"

status=0
CARGO_TARGET_DIR="$PROBE/target" "$TIMEOUT" --foreground -k 5 "$LIMIT" \
    "$CARGO_BIN" metadata --format-version=1 --offline --manifest-path "$PROBE/Cargo.toml" \
    > /dev/null 2> "$PROBE/stderr" || status=$?
if [[ $status -eq 0 ]]; then
    exit 0
fi
# Cargo queries the compiler before it takes the package-cache lock, so a wait
# on that lock means the wrapper answered and another Cargo is busy.
if grep -q 'Blocking waiting for file lock' "$PROBE/stderr"; then
    echo "note: the compiler wrapper answered; another Cargo process holds the package cache" >&2
    exit 0
fi

if [[ -n "${RUSTC_WRAPPER-}" ]]; then
    wrapper="RUSTC_WRAPPER=$RUSTC_WRAPPER"
elif [[ -n "${CARGO_BUILD_RUSTC_WRAPPER-}" ]]; then
    wrapper="CARGO_BUILD_RUSTC_WRAPPER=$CARGO_BUILD_RUSTC_WRAPPER"
else
    wrapper="none set in the environment; see Cargo's config files"
fi
if [[ $status -eq 124 || $status -eq 137 ]]; then
    echo "error: Cargo got no answer to \`rustc -vV\` within ${LIMIT}s (compiler wrapper: ${wrapper})." >&2
    echo "Every build would hang the same way; a stalled sccache server is the usual cause." >&2
else
    echo "error: Cargo could not query the Rust compiler (compiler wrapper: ${wrapper}):" >&2
fi
tail -n 20 "$PROBE/stderr" >&2
echo "This repository builds without a wrapper. If you enabled one, retry with RUSTC_WRAPPER= (empty)," >&2
echo "or restart the cache server (sccache --stop-server)." >&2
exit 1
