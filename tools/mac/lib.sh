#!/bin/sh
# Shared helpers for the macOS kit (run-004, R4-mac-kit). Sourced by the other scripts, not run.
# POSIX sh (macOS /bin/sh and zsh both run it). Synthetic data only, under a neutral lab root.
# Never reads or writes real WoW, Battle.net or Dropbox data.

# Repository root = two levels above the script that sourced this file.
MAC_DIR=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$MAC_DIR/../.." && pwd)
APP="$REPO/target/release/wow-sync"
DEV="$REPO/target/release/devkit"

# Neutral lab root: no user name in the path. Override with WOWSYNC_LAB_ROOT.
LAB_ROOT=${WOWSYNC_LAB_ROOT:-/Users/Shared/WoWSyncLab}

note() { printf '[%s] %s\n' "$(date +%H:%M:%S)" "$*"; }
die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

# Seconds since the epoch with milliseconds (perl ships with macOS).
now() { perl -MTime::HiRes=time -e 'printf "%.3f\n", time'; }

need_macos() {
    [ "$(uname -s)" = "Darwin" ] || die "this kit runs on macOS only (found $(uname -s))"
}

# The lab root must be absolute and must not point at real game, Battle.net or Dropbox data.
check_lab_root() {
    case "$LAB_ROOT" in
        /*) ;;
        *) die "lab root must be an absolute path: $LAB_ROOT" ;;
    esac
    lower=$(printf '%s' "$LAB_ROOT" | tr '[:upper:]' '[:lower:]')
    case "$lower" in
        *dropbox* | *"world of warcraft"* | *battle.net* | *blizzard* | */library/*)
            die "lab root looks like real game, Battle.net, Dropbox or Library data: $LAB_ROOT" ;;
    esac
    [ "$LAB_ROOT" != "/" ] || die "lab root must not be /"
    mkdir -p "$LAB_ROOT" 2>/dev/null || die "cannot create $LAB_ROOT (set WOWSYNC_LAB_ROOT=<dir> to use another neutral folder)"
}

# The app bundle that tools/mac/bundle.sh installs. macOS shows "WoW Sync" as the notification
# sender only when this bundle (identifier com.wowsync.app) exists.
BUNDLE_DIR=${WOWSYNC_BUNDLE_DIR:-$HOME/Applications}
BUNDLE_APP="$BUNDLE_DIR/WoW Sync.app"
BUNDLE_EXE="$BUNDLE_APP/Contents/MacOS/wow-sync"
RAW_APP="$APP"
APP_KIND=raw

# Uses the installed bundle's executable when present (same binary, shipped layout), unless
# WOWSYNC_NO_BUNDLE=1. Sets APP and APP_KIND (bundle or raw).
pick_app() {
    if [ "${WOWSYNC_NO_BUNDLE:-0}" != 1 ] && [ -x "$BUNDLE_EXE" ]; then
        APP="$BUNDLE_EXE"
        APP_KIND=bundle
        if [ -x "$RAW_APP" ] && [ "$RAW_APP" -nt "$BUNDLE_EXE" ]; then
            note "WARNING: the release build is newer than the installed bundle; run: sh tools/mac/bundle.sh"
        fi
    else
        APP="$RAW_APP"
        APP_KIND=raw
    fi
}

need_release_build() {
    pick_app
    [ -x "$APP" ] && [ -x "$DEV" ] || die "release build missing; run tools/mac/build.sh first"
    note "app: $APP_KIND ($APP)"
}

# Creates a synthetic lab (install, provider, state) and marks it as ours.
# $1 = lab directory, directly below the lab root.
lab_create() {
    d=$1
    case "$d" in "$LAB_ROOT"/?*) ;; *) die "refusing lab outside the lab root: $d" ;; esac
    mkdir -p "$d/install" "$d/provider" "$d/state"
    "$DEV" make-install "$d/install" --folder _forever_ --product wowf >/dev/null || die "devkit make-install failed"
    for n in SynAlpha SynBeta SynGamma; do
        "$DEV" add-addon "$d/install" _forever_ "$n" "tag-$n" 1.0.0 >/dev/null || die "devkit add-addon failed"
    done
    printf 'wowsync-lab/1\n' >"$d/.wowsync-lab"
}

# Deletes a lab only when it carries our marker and sits below the lab root.
lab_remove() {
    d=$1
    case "$d" in "$LAB_ROOT"/?*) ;; *) return 0 ;; esac
    [ -f "$d/.wowsync-lab" ] && rm -rf "$d"
    return 0
}

# $1 = lab dir, remaining = extra app arguments. Starts the app in the background; sets APP_PID.
lab_app_start() {
    d=$1
    shift
    WOWSYNC_LOG="$d/app.log" "$APP" --dev --install "$d/install" --provider "$d/provider" \
        --state "$d/state" --name Mac-Check --interval-ms 5000 --recent-ms 2000 "$@" \
        >/dev/null 2>&1 &
    APP_PID=$!
}

# Stops only the process this script started, by its own PID. Returns 1 if it had to be forced.
app_stop() {
    pid=$1
    kill "$pid" 2>/dev/null || return 0
    i=0
    while kill -0 "$pid" 2>/dev/null && [ "$i" -lt 50 ]; do
        sleep 0.2
        i=$((i + 1))
    done
    if kill -0 "$pid" 2>/dev/null; then
        kill -9 "$pid" 2>/dev/null
        return 1
    fi
    return 0
}

# PIDs of $1 and every live descendant (read-only measurement; nothing is stopped through this).
tree_pids() {
    ps -axo pid=,ppid= | awk -v root="$1" '
        { kids[$2] = kids[$2] " " $1; seen[$1] = 1 }
        function walk(p,   n, a, i) {
            if (!(p in seen)) return
            print p
            n = split(kids[p], a, " ")
            for (i = 1; i <= n; i++) walk(a[i])
        }
        END { walk(root) }'
}

# Physical footprint of one process in bytes. Documented method: `vmmap --summary`, the
# "Physical footprint:" line (Activity Monitor's "Memory" column: dirty, compressed and swapped
# private pages). Falls back to `footprint -p`. Prints nothing when neither works.
footprint_bytes() {
    pid=$1
    v=$(vmmap --summary "$pid" 2>/dev/null | awk '/^Physical footprint:/ { print $3; exit }')
    if [ -z "$v" ]; then
        v=$(footprint -p "$pid" 2>/dev/null | awk '{ for (i = 1; i < NF; i++) if ($i == "Footprint:") { print $(i + 1) $(i + 2); exit } }')
    fi
    [ -n "$v" ] || return 0
    printf '%s\n' "$v" | awk '{
        s = $1; sub(/[Bb]$/, "", s)
        u = substr(s, length(s)); m = 1
        if (u ~ /[KkMmGg]/) {
            s = substr(s, 1, length(s) - 1); u = toupper(u)
            if (u == "K") m = 1024; else if (u == "M") m = 1048576; else m = 1073741824
        }
        printf "%.0f\n", s * m }'
}

# Resident set size (KiB converted to bytes) of one process, for context beside the footprint.
rss_bytes() {
    ps -o rss= -p "$1" 2>/dev/null | awk '{ printf "%.0f\n", $1 * 1024 }'
}

# Environment block for result files (no user name, no home path).
env_report() {
    printf 'date: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf 'macOS: %s (%s)\n' "$(sw_vers -productVersion 2>/dev/null)" "$(sw_vers -buildVersion 2>/dev/null)"
    printf 'arch: %s\n' "$(uname -m)"
    printf 'model: %s\n' "$(sysctl -n hw.model 2>/dev/null)"
    printf 'cpu: %s\n' "$(sysctl -n machdep.cpu.brand_string 2>/dev/null)"
    printf 'ram_gib: %s\n' "$(sysctl -n hw.memsize 2>/dev/null | awk '{ printf "%.0f", $1 / 1073741824 }')"
    printf 'rustc: %s\n' "$(rustc --version 2>/dev/null)"
    printf 'app: %s\n' "$APP_KIND"
    printf 'commit: %s\n' "$(git -C "$REPO" rev-parse --short HEAD 2>/dev/null)"
    printf 'branch: %s\n' "$(git -C "$REPO" rev-parse --abbrev-ref HEAD 2>/dev/null)"
}
