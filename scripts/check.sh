#!/usr/bin/env bash
# Runs every check the Linux CI job runs. Push to main only when this passes.
# The Tauri app (src-tauri) is linted here for the Windows target (scripts/wincheck.sh);
# the Windows CI job builds, tests and packages it for real.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

step() { printf '\n\033[1;36m== %s ==\033[0m\n' "$1"; }

step "web: lint"
npm run --silent lint

step "web: types"
npm run --silent typecheck

step "web: unit tests"
npm run --silent test

step "web: build (the app embeds it)"
npx vite build --logLevel warn

if [ "${E2E:-0}" = "1" ]; then
  step "web: journeys and WCAG 2.2 AA checks (Playwright, light and dark)"
  npx playwright test
fi

if [ ! -f src-tauri/resources/dictionary.sqlite ]; then
  step "dictionary: build (bundled with the app)"
  python3 scripts/build-dictionary.py
fi

step "rust: format"
cargo fmt --all --check

step "rust: lint (core, win)"
cargo clippy -q -p te-core -p te-win --all-targets -- -D warnings

if rustup target list --installed 2>/dev/null | grep -q x86_64-pc-windows-msvc; then
  step "rust: lint the Windows app for the Windows target"
  scripts/wincheck.sh clippy -q -p te-win -p text-explainer --all-targets -- -D warnings
fi

step "rust: tests"
cargo test -q -p te-core -p te-win


printf '\n\033[1;32mAll checks passed.\033[0m\n'
