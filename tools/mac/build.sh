#!/bin/sh
# Build step (run-004, R4-mac-kit): prerequisites, offline verify, release build of app and devkit.
#   sh tools/mac/build.sh [--skip-verify]
# Run it from anywhere; it works on the checkout that contains this script.
set -u
. "$(dirname "$0")/lib.sh"
need_macos

skip_verify=0
[ "${1:-}" = "--skip-verify" ] && skip_verify=1

note "checkout: $REPO"
note "1/4 prerequisites"
xcode-select -p >/dev/null 2>&1 || die "Xcode Command Line Tools missing. Run: xcode-select --install   then run this script again."
command -v clang >/dev/null 2>&1 || die "clang not found; reinstall the Command Line Tools (xcode-select --install)."
command -v rustup >/dev/null 2>&1 || die "rustup missing. Install from https://rustup.rs , open a new Terminal, run this script again."
PATH="$HOME/.cargo/bin:$PATH"
export PATH
command -v cargo >/dev/null 2>&1 || die "cargo not found on PATH after rustup."

# rust-toolchain.toml pins the version; cargo fetches it on first use (needs the internet once).
want=$(sed -n 's/^channel *= *"\(.*\)"/\1/p' "$REPO/rust-toolchain.toml" | head -n 1)
cd "$REPO" || die "cannot enter $REPO"
have=$(rustc --version 2>/dev/null | awk '{ print $2 }')
note "pinned toolchain: ${want:-unknown}; active: ${have:-none}"
[ -n "$have" ] || die "rustc did not start; try: rustup show"
[ -z "$want" ] || [ "$want" = "$have" ] || die "active rustc $have is not the pinned $want"

command -v cargo-deny >/dev/null 2>&1 || note "NOTE: cargo-deny missing, so verify reports BLOCKED. Install: cargo install --locked cargo-deny"

if [ "$skip_verify" -eq 0 ]; then
    note "2/4 cargo xtask verify --tier offline"
    cargo xtask verify --tier offline || die "offline verify failed (see output above)"
else
    note "2/4 verify skipped (--skip-verify)"
fi

note "3/4 release build: app and devkit"
cargo build --release -p wow-sync-app --bins || die "release build failed"

note "4/4 check outputs"
[ -x "$APP" ] || die "missing $APP"
[ -x "$DEV" ] || die "missing $DEV"
ls -l "$APP" "$DEV"
note "BUILD OK"
note "Next: sh tools/mac/bundle.sh   (installs WoW Sync.app so notifications show the WoW Sync name)"
