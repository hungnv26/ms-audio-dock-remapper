#!/bin/sh
# Builds the release binary and wraps it in a signed (ad-hoc) .app bundle at
# target/release/MS Audio Dock Remapper.app. Pass a signing identity as the
# first argument to sign with a Developer ID instead.
set -eu
cd "$(dirname "$0")"

IDENTITY="${1:--}"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
APP="target/release/MS Audio Dock Remapper.app"

cargo build --release

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/ms-audio-dock-remapper "$APP/Contents/MacOS/"
sed "s/__VERSION__/$VERSION/g" packaging/macos/Info.plist > "$APP/Contents/Info.plist"
cp public/app-icon.icns "$APP/Contents/Resources/"
printf 'APPL????' > "$APP/Contents/PkgInfo"

codesign --force --sign "$IDENTITY" "$APP"
echo "Built $APP (version $VERSION, signed with '$IDENTITY')"
