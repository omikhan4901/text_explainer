#!/bin/sh
# Type-checks and lints the Windows app (src-tauri, te-win) from Linux or macOS:
#   scripts/wincheck.sh clippy -p text-explainer -- -D warnings
# C dependencies are "compiled" by stand-in scripts that write empty objects; that is
# fine for `cargo check`/`clippy` (nothing is linked) and never used for real builds.
# Needs: rustup target add x86_64-pc-windows-msvc
dir="$(cd "$(dirname "$0")" && pwd)"
export CC_x86_64_pc_windows_msvc="$dir/wincheck/fakecc"
export AR_x86_64_pc_windows_msvc="$dir/wincheck/fakelib"
export CARGO_TARGET_DIR="$dir/../target/wincheck"
sub="$1"
shift
exec cargo "$sub" --target x86_64-pc-windows-msvc "$@"
