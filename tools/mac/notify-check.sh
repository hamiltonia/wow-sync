#!/bin/sh
# T-27-11 on macOS (run-004, R4-mac-kit): show ONE native notification through the product's
# notification adapter and let the owner confirm the app name shown.
#   sh tools/mac/notify-check.sh
# It builds a tiny scratch program (outside the repository, under the lab root) that calls the same
# OsNotifier the runtime uses (`OsNotifier::with_app_id(APP_ID)`, as in
# crates/wow-sync-runtime/src/service.rs `attach_os_services`) once, with synthetic text. OsNotifier asks
# macOS to send as com.wowsync.app, so run `sh tools/mac/bundle.sh` first: the name "WoW Sync" shows
# only if that app bundle is installed. Nothing in the repository is changed. A failure to show the
# notification, or a name other than "WoW Sync", is a macOS A-27 FAIL (Q-63), not a fallback pass.
set -u
. "$(dirname "$0")/lib.sh"
need_macos
check_lab_root
PATH="$HOME/.cargo/bin:$PATH"
export PATH
command -v cargo >/dev/null 2>&1 || die "cargo not found; run tools/mac/build.sh first"

D="$LAB_ROOT/notify-check"
case "$D" in "$LAB_ROOT"/notify-check) ;; *) die "unexpected scratch path" ;; esac
[ ! -e "$D" ] || [ -f "$D/.wowsync-lab" ] || die "$D exists and is not ours; remove it first"
rm -rf "$D"
mkdir -p "$D/src"
printf 'wowsync-lab/1\n' >"$D/.wowsync-lab"

cat >"$D/Cargo.toml" <<EOF
[package]
name = "wowsync-notify-check"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
wow-sync-adapters = { path = "$REPO/crates/wow-sync-adapters" }

[workspace]
EOF
cat >"$D/src/main.rs" <<'EOF'
use wow_sync_adapters::notify::{Notifier, OsNotifier, RenderedNotice, APP_ID};

fn main() {
    let r = OsNotifier::with_app_id(APP_ID).deliver(&RenderedNotice {
        title: "WoW Sync".into(),
        body: "Manual notification check (synthetic).".into(),
        urgent: false,
    });
    match r {
        Ok(()) => println!("DELIVERED: the adapter reported success"),
        Err(e) => {
            println!("DELIVERY FAILED: {e}");
            std::process::exit(1);
        }
    }
    std::thread::sleep(std::time::Duration::from_secs(5));
}
EOF
# The repository's lock file pins the dependency versions the product was built with.
cp "$REPO/Cargo.lock" "$D/Cargo.lock"

note "building the scratch program (first build takes a few minutes)"
(cd "$D" && cargo build --release) || die "scratch build failed (copy the messages above into your report)"

OUTFILE="$LAB_ROOT/notify-check-result.txt"
pick_app
if [ -f "$BUNDLE_APP/Contents/Info.plist" ] && grep -q '<string>com.wowsync.app</string>' "$BUNDLE_APP/Contents/Info.plist"; then
    BUNDLE_STATE="installed at $BUNDLE_APP"
else
    BUNDLE_STATE="NOT installed (run: sh tools/mac/bundle.sh); macOS will show a default sender, not WoW Sync"
    note "WARNING: bundle $BUNDLE_STATE"
fi
{
    echo "T-27-11 macOS notification check"
    echo "bundle: $BUNDLE_STATE"
    env_report
} >"$OUTFILE"

printf '\nA notification should appear in the top-right corner now. Watch for it.\n'
note "showing the notification"
(cd "$D" && ./target/release/wowsync-notify-check) 2>&1 | tee -a "$OUTFILE"

printf '\n'
ask_name=""
printf 'What application name did the notification show (type it exactly; type none if nothing appeared): '
read -r ask_name </dev/tty || ask_name=""
printf 'owner-reported app name: %s\n' "$ask_name" >>"$OUTFILE"
if [ "$ask_name" = "WoW Sync" ]; then
    printf 'verdict: PASS (name "WoW Sync")\n' >>"$OUTFILE"
else
    printf 'verdict: FAIL (expected "WoW Sync")\n' >>"$OUTFILE"
fi
note "result file: $OUTFILE"
cat "$OUTFILE"
rm -rf "$D"
