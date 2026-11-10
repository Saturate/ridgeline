#!/usr/bin/env bash
# Packages a release build of Ridgeline into dist/.
#
#   scripts/package.sh <rust-target> <version>
#
# Asset names match what the Tauri build produced, because the Homebrew cask
# and winget manifests look the files up by name.
set -euo pipefail

TARGET="$1"
VERSION="$2"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="$ROOT/target/$TARGET/release"
RES="$ROOT/crates/ridgeline-app/resources"
DIST="$ROOT/dist"

rm -rf "$DIST"
mkdir -p "$DIST"

case "$TARGET" in
  *-apple-darwin)
    case "$TARGET" in
      aarch64-*) ARCH=aarch64 ;;
      x86_64-*) ARCH=x64 ;;
    esac
    APP="$DIST/Ridgeline.app"
    mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
    cp "$BIN_DIR/ridgeline" "$APP/Contents/MacOS/ridgeline"
    cp "$RES/icon.icns" "$APP/Contents/Resources/icon.icns"
    sed "s/@VERSION@/$VERSION/g" "$ROOT/packaging/macos/Info.plist" > "$APP/Contents/Info.plist"
    # No Developer ID yet; an ad-hoc signature keeps the bundle consistent so
    # Apple Silicon will launch it after the user clears quarantine.
    codesign --force --deep --sign - "$APP"

    STAGE="$DIST/dmg"
    mkdir -p "$STAGE"
    cp -R "$APP" "$STAGE/"
    ln -s /Applications "$STAGE/Applications"
    hdiutil create -volname "Ridgeline" -srcfolder "$STAGE" -ov -format UDZO \
      "$DIST/Ridgeline_${VERSION}_${ARCH}.dmg"
    rm -rf "$STAGE"
    ;;

  *-pc-windows-msvc)
    # ISCC ships with the GitHub Windows runner images.
    ISCC="${ISCC:-iscc}"
    "$ISCC" //Q \
      "//DAppVersion=$VERSION" \
      "//DSourceExe=$(cygpath -w "$BIN_DIR/ridgeline.exe")" \
      "//DIconFile=$(cygpath -w "$RES/icon.ico")" \
      "//O$(cygpath -w "$DIST")" \
      "//FRidgeline_${VERSION}_x64-setup" \
      "$(cygpath -w "$ROOT/packaging/windows/ridgeline.iss")"
    ;;

  *-unknown-linux-gnu)
    NAME="Ridgeline_${VERSION}_x86_64-linux"
    STAGE="$DIST/$NAME"
    mkdir -p "$STAGE"
    cp "$BIN_DIR/ridgeline" "$STAGE/"
    cp "$RES/icon.png" "$STAGE/ridgeline.png"
    cp "$ROOT/packaging/linux/ridgeline.desktop" "$STAGE/"
    cp "$ROOT/LICENSE" "$STAGE/"
    tar -C "$DIST" -czf "$DIST/$NAME.tar.gz" "$NAME"
    rm -rf "$STAGE"
    ;;

  *)
    echo "unsupported target: $TARGET" >&2
    exit 1
    ;;
esac

ls -la "$DIST"
