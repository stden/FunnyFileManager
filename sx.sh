#!/usr/bin/env bash
# Purpose: Profile the optimized FFM binary and save a flamegraph for each run.
set -euo pipefail

project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$project_dir"

case "${OSTYPE:-}" in
    msys*|cygwin*|win32*)
        for candidate in python python3; do
            if command -v "$candidate" >/dev/null && "$candidate" -c "" 2>/dev/null; then
                exec "$candidate" "scripts/profile_windows.py" "$@"
            fi
        done
        printf 'Required command is missing: python\n' >&2
        exit 1
        ;;
esac

for tool in cargo rustc git flamegraph perf; do
    if ! command -v "$tool" >/dev/null; then
        printf 'Required command is missing: %s\n' "$tool" >&2
        exit 1
    fi
done

# Let Cargo check freshness even when a stripped or older release already exists.
# Keep DWARF in the executable so a captured copy needs no separate debug files.
host_target="$(rustc --print host-tuple)"
CARGO_PROFILE_RELEASE_DEBUG=1 CARGO_PROFILE_RELEASE_STRIP=none \
    CARGO_PROFILE_RELEASE_SPLIT_DEBUGINFO=off \
    cargo build --release --bin ffm --target "$host_target" --target-dir "$project_dir/target"

mkdir -p target/profiles
profile_dir="$(mktemp -d "target/profiles/ffm-$(date -u +%Y%m%dT%H%M%SZ)-XXXXXX")"
# Profile the retained copy: later builds must not replace its symbols.
cp -- "target/$host_target/release/ffm" "$profile_dir/ffm"
{
    printf 'Captured at: %s\n' "$(date -u +%FT%TZ)"
    printf 'Build: release, default features, DEBUG=1, STRIP=none, SPLIT_DEBUGINFO=off\n'
    printf 'Target: %s\n' "$host_target"
    printf 'CPU sampling: cpu-clock:u, 99 Hz, DWARF stack 16384 bytes\n'
    printf 'Git revision: '
    git rev-parse HEAD
    git status --short
    rustc --version --verbose
    cargo --version
} > "$profile_dir/metadata.txt"
printf 'FFM CPU profile: %s/%s\n' "$project_dir" "$profile_dir"
printf 'Use FFM normally, then exit with F10 to generate flamegraph.svg.\n'
printf 'Action timings are also saved in your FFM configuration directory (performance/).\n'
# Resolve local symbols without slow inline expansion or debug-server downloads.
exec env DEBUGINFOD_URLS= flamegraph \
    --no-inline \
    --output "$profile_dir/flamegraph.svg" \
    --cmd "record -e cpu-clock:u -F 99 --call-graph dwarf,16384 -o $profile_dir/perf.data" \
    -- "./$profile_dir/ffm" "$@"
