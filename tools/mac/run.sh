#!/bin/sh
# Run step (run-004, R4-mac-kit): start the app against a synthetic lab under the lab root.
#   sh tools/mac/run.sh enroll     create a fresh synthetic lab and open the app so you can click through setup once
#   sh tools/mac/run.sh start      start the app again on the enrolled lab (add --hidden to start without a window)
#   sh tools/mac/run.sh clean      delete the lab
# The lab is $LAB_ROOT/main (default /Users/Shared/WoWSyncLab/main). Developer mode (--dev) is used,
# exactly as the Windows harness does: it points the app at the lab's install, provider and state folders only.
# The app is stopped with Ctrl-C in this Terminal window (this script stops only the process it started).
set -u
. "$(dirname "$0")/lib.sh"
need_macos
check_lab_root
LAB="$LAB_ROOT/main"

cmd=${1:-}
[ $# -gt 0 ] && shift

run_foreground() {
    lab_app_start "$LAB" "$@"
    pid=$APP_PID
    trap 'note "stopping the app"; app_stop "$pid"; exit 0' INT TERM
    note "app started (pid $pid). Log: $LAB/app.log"
    note "Press Ctrl-C in this window to stop it."
    wait "$pid" 2>/dev/null
    note "the app exited"
}

case "$cmd" in
    enroll)
        need_release_build
        lab_remove "$LAB"
        [ ! -e "$LAB" ] || die "$LAB exists but is not a WoW Sync lab; remove it yourself or set WOWSYNC_LAB_ROOT"
        lab_create "$LAB"
        note "synthetic lab created at $LAB"
        note "Follow the setup guide in the window (docs/runs/run-004/MAC-OWNER.md, step 'Enrol the lab')."
        run_foreground
        ;;
    start)
        need_release_build
        [ -f "$LAB/.wowsync-lab" ] || die "no lab yet; run: sh tools/mac/run.sh enroll"
        case "${1:-}" in
            --hidden) run_foreground --start-hidden ;;
            "") run_foreground ;;
            *) die "unknown option: $1" ;;
        esac
        ;;
    clean)
        lab_remove "$LAB"
        note "lab removed (if it existed)"
        ;;
    *)
        die "usage: sh tools/mac/run.sh enroll | start [--hidden] | clean"
        ;;
esac
