#!/bin/sh
# A-29 memory measurement of the WoW Sync app on macOS (run-004, R4-mac-kit). Mirrors tools/mem/run.ps1.
#   sh tools/mac/mem.sh [--scenarios a,b,c] [--runs 3] [--scale 1.0] [--out <dir>]
# Needs: tools/mac/build.sh done, and the lab enrolled once: sh tools/mac/run.sh enroll.
# --scale shortens idle/UI durations (smoke tests only; gated numbers use 1.0).
#
# Scenarios (same as the Windows harness, with the macOS differences marked):
#   a  start hidden, 120 s idle.
#   b  start with a window, 60 s shown while YOU scroll and press "Sync now", then close the window
#      (script asks), then 120 s with the window closed.
#   c  start hidden, 60 s while YOU open and close the window 20 times from the menu-bar icon,
#      then 300 s idle. (Windows drives this with a second launch; macOS has no such hook, see
#      the evidence handover.)
#
# Memory method: private resident memory = macOS "physical footprint" (vmmap --summary, the same
# number Activity Monitor shows as "Memory"), summed over EVERY process in the app's process tree,
# sampled about once per second. Verdict as in A-29: whole-run maximum of the aggregate <= 100 MiB.
# RSS (ps) is recorded for context only. Sync start = launch to the first change in the lab's
# state or provider files, measured with a fresh synthetic change made just before each launch.
# Synthetic data only, under the lab root. Never touches WoW, Battle.net or Dropbox.
# Runs the executable inside the installed WoW Sync.app when tools/mac/bundle.sh has been run, else
# target/release/wow-sync (WOWSYNC_NO_BUNDLE=1 forces the latter); the results file records which.
set -u
. "$(dirname "$0")/lib.sh"
need_macos
check_lab_root
need_release_build

SCENARIOS=a,b,c
RUNS=3
SCALE=1.0
OUT=""
while [ $# -gt 0 ]; do
    case "$1" in
        --scenarios) SCENARIOS=${2:-}; shift 2 ;;
        --runs) RUNS=${2:-}; shift 2 ;;
        --scale) SCALE=${2:-}; shift 2 ;;
        --out) OUT=${2:-}; shift 2 ;;
        *) die "unknown option: $1 (see the header of tools/mac/mem.sh)" ;;
    esac
done
case "$RUNS" in '' | *[!0-9]* | 0) die "--runs must be a positive whole number" ;; esac
[ -n "$OUT" ] || OUT="$LAB_ROOT/results/mem-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$OUT" || die "cannot create $OUT"
LAB="$LAB_ROOT/main"
[ -f "$LAB/.wowsync-lab" ] || die "no enrolled lab; run: sh tools/mac/run.sh enroll"
[ -n "$(ls -A "$LAB/state" 2>/dev/null)" ] || die "the lab has no saved setup; finish the setup guide: sh tools/mac/run.sh enroll"

LIMIT=104857600
FINDINGS="$OUT/findings.txt"
RUNLINES="$OUT/runs.tsv"
SYNCLINES="$OUT/syncstart.tsv"
: >"$FINDINGS"
: >"$RUNLINES"
: >"$SYNCLINES"

finding() { printf '%s\n' "$*" >>"$FINDINGS"; note "FINDING: $*"; }
secs() { awk -v n="$1" -v s="$SCALE" 'BEGIN { v = int(n * s + 0.5); if (v < 1) v = 1; print v }'; }
prompt() { printf '\n>>> %s\n    Press Return when done: ' "$*"; read -r _ans </dev/tty || true; }
ask() { printf '\n>>> %s ' "$*"; read -r ANSWER </dev/tty || ANSWER=""; }

# ---- sampling ----
# sample <csv> <phase> <t0>: one row "elapsed,phase,procs,footprint_bytes,rss_bytes" for the whole process tree.
sample() {
    csv=$1
    phase=$2
    t0=$3
    procs=0
    total=0
    rss=0
    missing=0
    for p in $(tree_pids "$ROOT_PID"); do
        printf '%s\n' "$p" >>"$SEEN"
        procs=$((procs + 1))
        fp=$(footprint_bytes "$p")
        if [ -z "$fp" ]; then
            missing=$((missing + 1))
        else
            total=$((total + fp))
        fi
        r=$(rss_bytes "$p")
        rss=$((rss + ${r:-0}))
    done
    [ "$missing" -eq 0 ] || finding "$TAG: footprint unreadable for $missing process(es) in a sample (vmmap/footprint denied?); aggregate is low"
    printf '%s,%s,%s,%s,%s\n' "$(awk -v a="$(now)" -v b="$t0" 'BEGIN { printf "%.1f", a - b }')" "$phase" "$procs" "$total" "$rss" >>"$csv"
}

# phase <name> <seconds>: sample about once per second until the time is up or the root exits.
phase() {
    name=$1
    dur=$2
    note "$TAG phase '$name' for ${dur}s"
    t0=$(now)
    end=$(awk -v a="$t0" -v d="$dur" 'BEGIN { printf "%.3f", a + d }')
    while [ "$(awk -v n="$(now)" -v e="$end" 'BEGIN { print (n < e) ? 1 : 0 }')" = 1 ]; do
        if ! kill -0 "$ROOT_PID" 2>/dev/null; then
            finding "$TAG: the app exited during phase '$name'"
            return 1
        fi
        sample "$CSV" "$name" "$T_RUN"
        sleep 1
    done
    return 0
}

# ---- sync start ----
snapshot() {
    find "$LAB/state" "$LAB/provider" -type f -exec stat -f '%N|%Fm|%z' {} + 2>/dev/null | sort
}

measure_sync_start() {
    before="$OUT/snap-$TAG.before"
    snapshot >"$before"
    t0=$(now)
    lab_app_start "$LAB" "$@"
    ROOT_PID=$APP_PID
    while :; do
        el=$(awk -v a="$(now)" -v b="$t0" 'BEGIN { printf "%.2f", a - b }')
        if [ "$(awk -v e="$el" 'BEGIN { print (e >= 30) ? 1 : 0 }')" = 1 ]; then
            printf '%s\t\ttimeout\n' "$TAG" >>"$SYNCLINES"
            note "$TAG sync start: timeout (no state/provider change in 30 s)"
            break
        fi
        if ! kill -0 "$ROOT_PID" 2>/dev/null; then
            printf '%s\t\tapp-exited\n' "$TAG" >>"$SYNCLINES"
            finding "$TAG: the app exited before the first sync"
            break
        fi
        snapshot >"$OUT/snap-$TAG.now"
        changed=$(comm -13 "$before" "$OUT/snap-$TAG.now" | head -n 1 | cut -d'|' -f1 | sed "s|^$LAB/||")
        if [ -n "$changed" ]; then
            printf '%s\t%s\t%s\n' "$TAG" "$el" "$changed" >>"$SYNCLINES"
            note "$TAG sync start: $el s ($changed)"
            break
        fi
        sleep 0.2
    done
    rm -f "$before" "$OUT/snap-$TAG.now"
    T_RUN=$t0
}

# ---- statistics ----
median_stdin() { sort -n | awk '{ a[NR] = $1 } END { if (NR) { if (NR % 2) print a[(NR + 1) / 2]; else print (a[NR / 2] + a[NR / 2 + 1]) / 2 } }'; }
# phase_values <csv> <phase> <column> <lastN>: the last N values of a column for a phase.
phase_values() { awk -F, -v p="$2" -v c="$3" '$2 == p { print $c }' "$1" | tail -n "$4"; }
mib() { awk -v b="$1" 'BEGIN { printf "%.2f", b / 1048576 }'; }

finish_run() {
    if ! app_stop "$ROOT_PID"; then finding "$TAG: app root did not exit on SIGTERM; forced by its own PID"; fi
    sleep 0.5
    for p in $(sort -u "$SEEN"); do
        [ "$p" = "$ROOT_PID" ] && continue
        if kill -0 "$p" 2>/dev/null; then
            # Only report: a surviving PID may have been recycled, so nothing is stopped through it.
            finding "$TAG: app-owned child pid $p still alive after the root exited (not stopped by the kit)"
        fi
    done
    maxprocs=$(awk -F, '{ if ($3 > m) m = $3 } END { print m + 0 }' "$CSV")
    [ "$maxprocs" -le 1 ] || finding "$TAG: app-owned process count reached $maxprocs"
}

record_run() {
    sc=$1
    run=$2
    steadyphase=idle
    [ "$sc" = b ] && steadyphase=hidden
    steady=$(phase_values "$CSV" "$steadyphase" 4 60 | median_stdin)
    steadyrss=$(phase_values "$CSV" "$steadyphase" 5 60 | median_stdin)
    peak=$(awk -F, '{ if ($4 > m) m = $4 } END { print m + 0 }' "$CSV")
    printf '%s\t%s\t%s\t%s\t%s\n' "$sc" "$run" "${steady:-0}" "$peak" "${steadyrss:-0}" >>"$RUNLINES"
    note "$TAG: steady median $(mib "${steady:-0}") MiB, whole-run max $(mib "$peak") MiB"
}

OLD_IFS=$IFS
IFS=,
set -- $SCENARIOS
IFS=$OLD_IFS
for sc in "$@"; do
    case "$sc" in a | b | c) ;; *) die "unknown scenario: $sc (use a, b or c)" ;; esac
done

for sc in "$@"; do
    i=1
    while [ "$i" -le "$RUNS" ]; do
        TAG="$sc-run$i"
        CSV="$OUT/$TAG.csv"
        SEEN="$OUT/$TAG.pids"
        printf 'elapsed_s,phase,procs,footprint_bytes,rss_bytes\n' >"$CSV"
        : >"$SEEN"
        note "=== $TAG ==="
        # A small synthetic change so the first sync has something to publish.
        "$DEV" add-addon "$LAB/install" _forever_ SynAlpha "tag-$TAG" 1.0.0 >/dev/null
        ROOT_PID=0
        T_RUN=$(now)
        case "$sc" in
            a)
                measure_sync_start --start-hidden
                phase idle "$(secs 120)"
                ;;
            b)
                measure_sync_start
                prompt "A window is open. For the next $(secs 60) s, scroll the page and press 'Sync now' now and then. Measuring starts as soon as you press Return."
                phase shown "$(secs 60)"
                prompt "Close the window with its red button. The app should keep running (menu-bar icon). If it quit, say so in your report."
                phase hidden "$(secs 120)"
                ;;
            c)
                measure_sync_start --start-hidden
                prompt "Measuring starts after you press Return and lasts $(secs 60) s. In that time open the window from the menu-bar icon and close it again, 20 times. Count your cycles."
                phase cycles "$(secs 60)"
                ask "How many open-and-close cycles did you complete (number)?"
                case "$ANSWER" in 20) ;; *) finding "$TAG: owner reports ${ANSWER:-no answer} of 20 show cycles" ;; esac
                phase idle "$(secs 300)"
                ;;
        esac
        kill -0 "$ROOT_PID" 2>/dev/null && sample "$CSV" end "$T_RUN"
        finish_run
        record_run "$sc" "$i"
        i=$((i + 1))
    done
done

# ---- summary ----
SUMMARY="$OUT/summary-$(printf '%s' "$SCENARIOS" | tr -d ',')-$(date +%H%M%S).txt"
{
    echo "A-29 macOS summary, scale=$SCALE; physical footprint aggregated over all app processes, MiB; limit 100"
    env_report
    for sc in "$@"; do
        rows=$(awk -F'\t' -v s="$sc" '$1 == s' "$RUNLINES")
        [ -n "$rows" ] || continue
        n=$(printf '%s\n' "$rows" | wc -l | tr -d ' ')
        meds=$(printf '%s\n' "$rows" | awk -F'\t' '{ print $3 }')
        peaks=$(printf '%s\n' "$rows" | awk -F'\t' '{ print $4 }')
        rsss=$(printf '%s\n' "$rows" | awk -F'\t' '{ print $5 }')
        peakmax=$(printf '%s\n' "$peaks" | sort -n | tail -n 1)
        verdict=FAIL
        [ "$(awk -v p="$peakmax" -v l="$LIMIT" 'BEGIN { print (p <= l) ? 1 : 0 }')" = 1 ] && verdict=PASS
        list=$(printf '%s\n' "$meds" | awk '{ printf "%s%.2f", (NR > 1 ? ", " : ""), $1 / 1048576 }')
        echo "scenario $sc: runs=$n steady-median per run [$list] min/median/max = $(mib "$(printf '%s\n' "$meds" | sort -n | head -n 1)")/$(mib "$(printf '%s\n' "$meds" | median_stdin)")/$(mib "$(printf '%s\n' "$meds" | sort -n | tail -n 1)"); whole-run max $(mib "$peakmax"); verdict $verdict"
        echo "   context (median of runs): resident set size (ps) $(mib "$(printf '%s\n' "$rsss" | median_stdin)")"
    done
    ssv=$(awk -F'\t' '$2 != "" { print $2 }' "$SYNCLINES")
    if [ -n "$ssv" ]; then
        echo "sync start (launch -> first state/provider change), s: n=$(printf '%s\n' "$ssv" | wc -l | tr -d ' ') min/median/max = $(printf '%s\n' "$ssv" | sort -n | head -n 1)/$(printf '%s\n' "$ssv" | median_stdin)/$(printf '%s\n' "$ssv" | sort -n | tail -n 1); timeouts=$(awk -F'\t' '$2 == ""' "$SYNCLINES" | wc -l | tr -d ' ')"
    else
        echo "sync start: no measurement; timeouts=$(awk -F'\t' '$2 == ""' "$SYNCLINES" | wc -l | tr -d ' ')"
    fi
    awk -F'\t' '{ printf "   %s: %s s [%s]\n", $1, ($2 == "" ? "none" : $2), $3 }' "$SYNCLINES"
    echo "findings: $(wc -l <"$FINDINGS" | tr -d ' ')"
    sed 's/^/   /' "$FINDINGS"
} >"$SUMMARY"
cat "$SUMMARY"
note "results written to $OUT"
