#!/usr/bin/env bash
# Runs every check the Linux CI job runs. Push to main only when this passes.
# The Tauri app itself (src-tauri) is built, linted and tested by the Windows CI job.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

step() { printf '\n\033[1;36m== %s ==\033[0m\n' "$1"; }

step "rust: format"
cargo fmt --all --check

step "rust: lint (core, win)"
cargo clippy -q -p te-core -p te-win --all-targets -- -D warnings

if rustup target list --installed 2>/dev/null | grep -q x86_64-pc-windows-msvc; then
  step "rust: lint Windows code for the Windows target"
  cargo clippy -q -p te-win --target x86_64-pc-windows-msvc -- -D warnings
fi

step "rust: tests"
cargo test -q -p te-core -p te-win

step "web: lint"
npm run --silent lint

step "web: types"
npm run --silent typecheck

step "web: unit tests"
npm run --silent test

step "web: build"
npx vite build --logLevel warn

printf '\n\033[1;32mAll checks passed.\033[0m\n'
