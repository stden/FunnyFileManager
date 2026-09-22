#!/bin/bash
# Purpose: Runs scheduled repository automation from cron.
# Cron job script for FunnyFileManager
# Runs Claude Code to review and fix code, then builds, tests, and pushes.
#
# Install to crontab:
#   crontab -e
#   # Run every hour:
#   0 * * * * /path/to/FunnyFileManager/cron-run.sh
#   # Run every day at 9am:
#   0 9 * * * /path/to/FunnyFileManager/cron-run.sh
#   # Run every 30 minutes:
#   */30 * * * * /path/to/FunnyFileManager/cron-run.sh

set -uo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_FILE="$PROJECT_DIR/cron.log"
TIMESTAMP=$(date '+%Y-%m-%d %H:%M:%S')
export PATH="/usr/local/bin:/opt/homebrew/bin:$HOME/.cargo/bin:$PATH"

cd "$PROJECT_DIR"

echo "=== Cron run: $TIMESTAMP ===" >> "$LOG_FILE"

# Pull latest changes
git pull --rebase >> "$LOG_FILE" 2>&1 || echo "WARN: git pull failed" >> "$LOG_FILE"

# Run Claude Code to review and fix all issues
echo "CLAUDE: starting review..." >> "$LOG_FILE"
claude --dangerously-skip-permissions -p "Review code and fix all issues. Run cargo test --features test-helpers and cargo clippy -- -D warnings. Fix any errors or warnings. Then git add + commit + push." >> "$LOG_FILE" 2>&1 || echo "CLAUDE: exited with error" >> "$LOG_FILE"
echo "CLAUDE: done" >> "$LOG_FILE"

# Build release
if cargo build --release >> "$LOG_FILE" 2>&1; then
    echo "BUILD: OK" >> "$LOG_FILE"
else
    echo "BUILD: FAILED" >> "$LOG_FILE"
fi

# Install binary
cp target/release/ffm /Applications/FunnyFileManager.app/Contents/MacOS/ffm 2>/dev/null || true

echo "=== Done: $(date '+%H:%M:%S') ===" >> "$LOG_FILE"
echo "" >> "$LOG_FILE"
