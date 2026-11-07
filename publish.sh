#!/usr/bin/env bash
# One-shot publisher for https://github.com/GwenAsa/heatmap-desktop
# Run it from inside this folder:   bash publish.sh
set -euo pipefail

OWNER="GwenAsa"
REPO="heatmap-desktop"
VERSION="v0.2.0"
DESC="Local-first activity tracker with calendar heatmaps, desktop widgets and quick capture (Tauri 2, Rust, React, SQLite)"
TOPICS="tauri rust react typescript sqlite heatmap habit-tracker linux desktop-app local-first"

cd "$(dirname "$0")"
c_ok=$'\e[32m'; c_warn=$'\e[33m'; c_err=$'\e[31m'; c_dim=$'\e[2m'; c_off=$'\e[0m'
say()  { echo "${c_ok}==>${c_off} $*"; }
warn() { echo "${c_warn}!!${c_off}  $*"; }
die()  { echo "${c_err}xx${c_off}  $*" >&2; exit 1; }

command -v git >/dev/null || die "git is not installed. Run: sudo apt install git"

# 1. Links in README/docs --------------------------------------------------------------------
say "Pointing links at github.com/$OWNER/$REPO"
bash scripts/set-owner.sh "$OWNER" "$REPO"

# 2. Local repository -------------------------------------------------------------------------
[ -d .git ] || { say "Creating a local git repository"; git init -q -b main; }
git checkout -q -B main
if [ -z "$(git config user.name || true)" ];  then read -rp "Your name for commits: " n; git config user.name "$n"; fi
if [ -z "$(git config user.email || true)" ]; then read -rp "Your email for commits (a GitHub noreply address is fine): " e; git config user.email "$e"; fi

say "Committing"
git add -A
if git diff --cached --quiet; then warn "Nothing new to commit."; else git commit -q -m "Heatmap 0.2.0: local-first activity tracker with calendar heatmaps, widgets and quick capture"; fi

# 3. Remote ---------------------------------------------------------------------------------------
URL="https://github.com/$OWNER/$REPO.git"
if git remote get-url origin >/dev/null 2>&1; then git remote set-url origin "$URL"; else git remote add origin "$URL"; fi

if command -v gh >/dev/null && gh auth status >/dev/null 2>&1; then
  say "GitHub CLI is logged in: using it for authentication"
  gh auth setup-git >/dev/null 2>&1 || true
else
  warn "Not logged in with the GitHub CLI. Git will ask for your credentials:"
  echo "   username: $OWNER"
  echo "   password: a Personal Access Token you create at https://github.com/settings/tokens"
  echo "             (classic token with the 'repo' AND 'workflow' scopes; the workflow scope is required"
  echo "              because this repository contains .github/workflows files)."
  echo "   ${c_dim}Never paste this token anywhere else. Tip: 'sudo apt install gh && gh auth login' is easier.${c_off}"
fi

# 4. Push -------------------------------------------------------------------------------------------
say "Checking what is already on GitHub"
if git ls-remote --exit-code --heads origin main >/dev/null 2>&1; then
  warn "The remote already has a 'main' branch (for example a README created by GitHub)."
  read -rp "Replace it with this project? (type yes to overwrite): " ans
  [ "$ans" = "yes" ] || die "Aborted. Nothing was pushed."
  git push -u origin main --force-with-lease || git push -u origin main --force
else
  git push -u origin main
fi
say "Code is on GitHub: https://github.com/$OWNER/$REPO"

# 5. Repository details (only with an authenticated GitHub CLI) -------------------------------------
if command -v gh >/dev/null && gh auth status >/dev/null 2>&1; then
  say "Setting description and topics"
  args=(--description "$DESC")
  for t in $TOPICS; do args+=(--add-topic "$t"); done
  gh repo edit "$OWNER/$REPO" "${args[@]}" || warn "Could not update repository details (you can set them in the web UI)."
else
  warn "Skipping description/topics. Add them on the repository page (the gear next to 'About')."
fi

# 6. Release -------------------------------------------------------------------------------------------
echo
read -rp "Create tag $VERSION now so GitHub Actions builds the .deb and .AppImage release? [y/N] " go
if [[ "$go" =~ ^[Yy]$ ]]; then
  if git rev-parse "$VERSION" >/dev/null 2>&1; then warn "Tag $VERSION already exists locally."; else git tag -a "$VERSION" -m "Heatmap ${VERSION#v}"; fi
  git push origin "$VERSION"
  say "Tag pushed. Watch the build: https://github.com/$OWNER/$REPO/actions"
  echo "   The release with both packages appears at https://github.com/$OWNER/$REPO/releases in about 15-25 minutes."
else
  echo "Skipped. Later, run:  git tag -a $VERSION -m 'Heatmap ${VERSION#v}' && git push origin $VERSION"
fi

echo
say "Last manual step: Settings > General > Social preview > upload assets/social-preview.png"
