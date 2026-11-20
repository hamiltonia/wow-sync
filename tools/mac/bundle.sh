#!/bin/sh
# Builds a minimal "WoW Sync.app" around the release binary and installs it (run-004, R4-mac-kit).
#   sh tools/mac/bundle.sh [--dock] [--dir <folder>]     build and install
#   sh tools/mac/bundle.sh --uninstall                   remove the installed bundle
# Why: macOS attributes a notification to a bundle identifier. The product sets the sender to
# com.wowsync.app (crates/wow-sync-adapters/src/notify.rs, MACOS_BUNDLE_ID); the name "WoW Sync" appears
# only if a bundle with that identifier and name exists.
# Result: ~/Applications/WoW Sync.app (or --dir / WOWSYNC_BUNDLE_DIR), unsigned apart from an ad-hoc
# signature. By default the app has no Dock icon (LSUIElement, a menu-bar app); --dock shows it.
# Run tools/mac/build.sh first. Nothing outside the bundle folder and the lab root is written.
set -u
. "$(dirname "$0")/lib.sh"
need_macos
check_lab_root

dock=0
uninstall=0
while [ $# -gt 0 ]; do
    case "$1" in
        --dock) dock=1; shift ;;
        --uninstall) uninstall=1; shift ;;
        --dir) BUNDLE_DIR=${2:-}; [ -n "$BUNDLE_DIR" ] || die "--dir needs a folder"
               BUNDLE_APP="$BUNDLE_DIR/WoW Sync.app"; shift 2 ;;
        *) die "usage: sh tools/mac/bundle.sh [--dock] [--dir <folder>] | --uninstall" ;;
    esac
done

PLIST="$BUNDLE_APP/Contents/Info.plist"
ID=com.wowsync.app

if [ "$uninstall" -eq 1 ]; then
    [ -d "$BUNDLE_APP" ] || { note "no bundle at $BUNDLE_APP"; exit 0; }
    grep -q "<string>$ID</string>" "$PLIST" 2>/dev/null || die "$BUNDLE_APP is not the WoW Sync bundle; not removing it"
    rm -rf "$BUNDLE_APP"
    note "removed $BUNDLE_APP"
    exit 0
fi

[ -x "$RAW_APP" ] || die "release build missing ($RAW_APP); run tools/mac/build.sh first"
case "$BUNDLE_APP" in */"WoW Sync.app") ;; *) die "unexpected bundle path: $BUNDLE_APP" ;; esac
# Replace only a bundle that is ours.
if [ -e "$BUNDLE_APP" ]; then
    grep -q "<string>$ID</string>" "$PLIST" 2>/dev/null || die "$BUNDLE_APP exists and is not the WoW Sync bundle; move it away first"
    rm -rf "$BUNDLE_APP"
fi

version=$(sed -n 's/^version *= *"\(.*\)"/\1/p' "$REPO/Cargo.toml" | head -n 1)
version=${version:-0.0.0}
if [ "$dock" -eq 1 ]; then uielement=false; else uielement=true; fi

note "building $BUNDLE_APP (version $version, Dock icon: $([ "$dock" -eq 1 ] && echo yes || echo no))"
mkdir -p "$BUNDLE_APP/Contents/MacOS" "$BUNDLE_APP/Contents/Resources" || die "cannot create the bundle folders"
cp "$RAW_APP" "$BUNDLE_APP/Contents/MacOS/wow-sync" || die "cannot copy the binary"
chmod 755 "$BUNDLE_APP/Contents/MacOS/wow-sync"

# Icon from the brand PNG (sips and iconutil ship with macOS). A missing icon is not fatal.
SRC="$REPO/assets/brand/wow_addon_sync.png"
SET="$LAB_ROOT/bundle-build/AppIcon.iconset"
rm -rf "$LAB_ROOT/bundle-build"
mkdir -p "$SET"
ok=1
for s in 16 32 128 256 512; do
    d=$((s * 2))
    sips -z "$s" "$s" "$SRC" --out "$SET/icon_${s}x${s}.png" >/dev/null 2>&1 || ok=0
    sips -z "$d" "$d" "$SRC" --out "$SET/icon_${s}x${s}@2x.png" >/dev/null 2>&1 || ok=0
done
icon_key=""
if [ "$ok" -eq 1 ] && iconutil -c icns "$SET" -o "$BUNDLE_APP/Contents/Resources/AppIcon.icns" >/dev/null 2>&1; then
    icon_key="    <key>CFBundleIconFile</key><string>AppIcon</string>"
else
    note "WARNING: icon could not be made; the bundle has no icon"
fi
rm -rf "$LAB_ROOT/bundle-build"

cat >"$PLIST" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key><string>$ID</string>
    <key>CFBundleName</key><string>WoW Sync</string>
    <key>CFBundleDisplayName</key><string>WoW Sync</string>
    <key>CFBundleExecutable</key><string>wow-sync</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
$icon_key
    <key>NSHighResolutionCapable</key><true/>
    <key>LSUIElement</key><$uielement/>
</dict>
</plist>
EOF

plutil -lint "$PLIST" || die "Info.plist is not valid"
# Ad-hoc signature (no certificate needed) and no download quarantine; both best effort.
codesign --force --deep --sign - "$BUNDLE_APP" >/dev/null 2>&1 || note "WARNING: ad-hoc signing failed; the app stays unsigned"
xattr -cr "$BUNDLE_APP" 2>/dev/null
LSREG=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
[ -x "$LSREG" ] && "$LSREG" -f "$BUNDLE_APP" >/dev/null 2>&1

plutil -p "$PLIST"
note "installed: $BUNDLE_APP"
note "If macOS says the app cannot be opened because the developer is unverified, right-click the app in Finder and choose Open once."
