#!/usr/bin/env bash
# Usage: scripts/set-owner.sh <github-user-or-org> <repo-name>
# Replaces the OWNER/REPO placeholders in README and docs with your real repository path.
set -euo pipefail
[ $# -eq 2 ] || { echo "usage: $0 <owner> <repo>"; exit 1; }
grep -rl "OWNER/REPO" README.md docs 2>/dev/null | xargs sed -i "s#OWNER/REPO#$1/$2#g"
echo "Updated links to https://github.com/$1/$2"
