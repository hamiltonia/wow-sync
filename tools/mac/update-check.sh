#!/bin/sh
# T-28-13 on macOS (run-004, R4-mac-kit): one real anonymous GET of the public GitHub Releases
# endpoint, through the product's own fetch code (D-24 P4-1). This is the only network call in the kit.
#   sh tools/mac/update-check.sh
# Runs the repository's manual test `manual_live_release_fetch_is_one_anonymous_get`
# (crates/wow-sync-adapters/src/update_http.rs) with the pinned toolchain. Nothing is changed.
# "LIVE: ok" or "LIVE: Http(404)" (no release published yet) is a PASS; any other result is a FAIL.
set -u
. "$(dirname "$0")/lib.sh"
need_macos
check_lab_root
PATH="$HOME/.cargo/bin:$PATH"
export PATH
command -v cargo >/dev/null 2>&1 || die "cargo not found; run tools/mac/build.sh first"

OUTFILE="$LAB_ROOT/update-check-result.txt"
{
    echo "T-28-13 macOS live update check"
    env_report
} >"$OUTFILE"

cd "$REPO" || die "cannot enter $REPO"
note "running the live check (one GET to api.github.com)"
cargo test -p wow-sync-adapters --lib manual_live -- --ignored --nocapture 2>&1 | tee -a "$OUTFILE"

if grep -q 'LIVE: ok' "$OUTFILE" || grep -q 'LIVE: Http(404)' "$OUTFILE"; then
    if grep -q '^test result: ok' "$OUTFILE"; then
        echo "verdict: PASS" >>"$OUTFILE"
    else
        echo "verdict: FAIL (test did not pass)" >>"$OUTFILE"
    fi
else
    echo "verdict: FAIL (no usable LIVE line)" >>"$OUTFILE"
fi
note "result file: $OUTFILE"
tail -n 3 "$OUTFILE"
