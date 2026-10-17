#!/usr/bin/env bash
# Purpose: Profile the optimized FFM binary and save a flamegraph for each run.
set -euo pipefail

project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$project_dir"

for tool in flamegraph perf; do
    if ! command -v "$tool" >/dev/null; then
        printf 'Required command is missing: %s\n' "$tool" >&2
        exit 1
    fi
done

# Keep debug symbols when building the release binary used for profiling.
if [[ ! -x target/release/ffm ]]; then
    CARGO_PROFILE_RELEASE_DEBUG=1 CARGO_PROFILE_RELEASE_STRIP=none \
        cargo build --release --bin ffm
fi

mkdir -p target/profiles
profile_dir="$(mktemp -d "target/profiles/ffm-$(date -u +%Y%m%dT%H%M%SZ)-XXXXXX")"
printf 'FFM CPU profile: %s/%s\n' "$project_dir" "$profile_dir"
printf 'Use FFM normally, then exit with F10 to generate flamegraph.svg.\n'
# Resolve local symbols without slow inline expansion or debug-server downloads.
exec env DEBUGINFOD_URLS= flamegraph \
    --no-inline \
    --output "$profile_dir/flamegraph.svg" \
    --cmd "record -e cpu-clock:u -F 99 --call-graph dwarf,16384 -o $profile_dir/perf.data" \
    -- ./target/release/ffm "$@"
