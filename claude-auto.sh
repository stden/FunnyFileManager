#!/bin/bash
# Purpose: Starts the Claude automation loop on Unix shells.
# Run Claude Code in fully autonomous mode (skip all permission prompts)
cd "$(dirname "$0")"
claude --dangerously-skip-permissions "$@"
