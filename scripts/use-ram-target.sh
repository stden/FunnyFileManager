#!/usr/bin/env bash
# Purpose: Moves Cargo target output to RAM for faster local builds.
# Opt-in RAM-backed cargo target directory (tmpfs).
#
#   Usage:  source scripts/use-ram-target.sh
#
# Points CARGO_TARGET_DIR at /dev/shm (tmpfs) so all build artifacts live in
# RAM. The target dir is 100% regenerable, so losing it on reboot is fine.
#
# Why an env var and not [build] target-dir in .cargo/config.toml?
#   - .cargo/config.toml is committed/shared; hardcoding /dev/shm/... there
#     would break every teammate and CI that lacks /dev/shm.
#   - CARGO_TARGET_DIR (env) overrides build.target-dir, so this composes
#     cleanly and stays machine-local.
#
# Honest expectation: with a large page cache the read-side is already RAM-fast,
# so incremental rebuilds barely change. The real wins are write-side metadata
# (thousands of tiny .o/.rmeta files) and a 2-5x faster `cargo clean`.
#
# Make it permanent for just your machine (no shared-file changes):
#   echo 'export CARGO_TARGET_DIR=/dev/shm/ffm-target' >> ~/.bashrc

# NOTE: no `set -u`/`set -e` here — this script is meant to be sourced, and
# those would alter the caller's interactive shell.
RAM_DIR="${FFM_RAM_TARGET:-/dev/shm/ffm-target}"

# tmpfs mounted noexec would make cargo fail to run build scripts / test bins.
if mount | grep -qE "on $(dirname "$RAM_DIR") .*noexec"; then
    echo "ffm: $(dirname "$RAM_DIR") is mounted noexec; cargo can't exec from it." >&2
    echo "ffm: keeping on-disk ./target." >&2
elif [ -d "$(dirname "$RAM_DIR")" ] && [ -w "$(dirname "$RAM_DIR")" ]; then
    mkdir -p "$RAM_DIR"
    export CARGO_TARGET_DIR="$RAM_DIR"
    echo "ffm: cargo target -> $CARGO_TARGET_DIR (tmpfs, $(df -h --output=avail "$RAM_DIR" | tail -1 | tr -d ' ') free)"
else
    echo "ffm: no writable $(dirname "$RAM_DIR"); using on-disk ./target." >&2
fi
