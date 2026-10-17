#!/usr/bin/env bash
# Purpose: Start the FFM profiling launcher beside this script.
set -euo pipefail

exec "$(dirname -- "${BASH_SOURCE[0]}")/sx.sh" "$@"
