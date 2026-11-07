#!/usr/bin/env bash
# Usage: scripts/set-owner.sh <github-user-or-org> <repo-name>
# Replaces the OWNER/REPO placeholders in README and docs with your real repository path.
set -euo pipefail
[ $# -eq 2 ] || { echo "usage: $0 <owner> <repo>"; exit 1; }
files=$(grep -rl "OWNER/REPO" README.md docs 2>/dev/null || true)
if [ -n "$files" ]; then echo "$files" | xargs sed -i "s#OWNER/REPO#$1/$2#g"; fi
echo "Links point to https://github.com/$1/$2"
