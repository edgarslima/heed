#!/usr/bin/env bash
# Builds HeedProbe.app around the capture probe and signs it with Hardened Runtime, so the
# P01.6 permission tests run against a bundle with its own TCC identity, usage descriptions,
# and the audio-input entitlement.
#
# Usage: scripts/build-macos-probe-app.sh <codesign identity> [output directory]
# Run it with `open -W --stdout <file> -a <dir>/HeedProbe.app --args <probe arguments>`, so
# TCC attributes the request to the bundle instead of the terminal.
set -euo pipefail

identity=${1:?usage: build-macos-probe-app.sh <codesign identity> [output directory]}
output=${2:-target/heed-probe}
bundle_id=pro.edgarlima.heed.probe
app="$output/HeedProbe.app"

cargo build --locked -p recorder-core --example macos-capture-probe
rm -rf "$app"
mkdir -p "$app/Contents/MacOS"
cp target/debug/examples/macos-capture-probe "$app/Contents/MacOS/heed-probe"

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>$bundle_id</string>
  <key>CFBundleExecutable</key><string>heed-probe</string>
  <key>CFBundleName</key><string>HeedProbe</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>LSMinimumSystemVersion</key><string>14.2</string>
  <key>NSMicrophoneUsageDescription</key><string>Heed records your microphone when you start a recording.</string>
  <key>NSAudioCaptureUsageDescription</key><string>Heed records the audio your Mac plays when you start a recording.</string>
</dict>
</plist>
PLIST

entitlements="$output/heed-probe.entitlements"
cat > "$entitlements" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.security.device.audio-input</key><true/>
</dict>
</plist>
PLIST

codesign --force --options runtime --entitlements "$entitlements" --sign "$identity" "$app"
codesign --verify --strict --verbose=2 "$app"
echo "built $app ($bundle_id)"
