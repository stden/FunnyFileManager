#!/bin/bash
# Purpose: Automates the repository commit flow used by local contributors.
set -e

MESSAGE="${1:-Auto commit}"

git add -A
git commit -m "$MESSAGE"
git pull --rebase
git push
