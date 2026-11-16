#!/bin/bash
# Build the game for a real iPhone on this Mac, sign it with your own certificate and install it: the local twin of
# .github/workflows/ios.yml plus the signing step. Needs rustup with aarch64-apple-ios, Xcode, xcodegen, and a file
# with your signing details (default ~/.erkam/signing.env; never commit it):
#   SIGN_IDENTITY  the certificate's SHA-1 (security find-identity -v -p codesigning)
#   PROFILE        a .mobileprovision that allows the bundle id and the phone
#   ENTITLEMENTS   a plist with the app's entitlements
#   DEVICE         the phone's id (xcrun devicectl list devices)
#   BUNDLE_ID      the bundle id the profile allows
# Like the simulator script it is incremental, with 16 codegen units and no LTO. Usage: tools/ci/local-ios.sh
set -euo pipefail
cd "$(dirname "$0")/../.."
export PATH="$HOME/.cargo/bin:$PATH"
set -a
# shellcheck disable=SC1090
source "${SIGNING_ENV:-$HOME/.erkam/signing.env}"
set +a
PROFILE=$(eval echo "$PROFILE"); ENTITLEMENTS=$(eval echo "$ENTITLEMENTS")
export HORIZON_TELEMETRY_URL=off
export IOS_TARGET=aarch64-apple-ios
export HORIZON_MSA_CLIENT_ID=331e08f8-172d-43b9-835a-3108c2fcdbf3
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
export CARGO_PROFILE_RELEASE_LTO=false
export CARGO_INCREMENTAL=1
export IPHONEOS_DEPLOYMENT_TARGET=16.0
cargo build --release --locked --target "$IOS_TARGET" -p horizon-official --no-default-features --features audio,horizon-extras
( cd packaging/ios && xcodegen generate >/dev/null )
rm -rf build-ios
HORIZON_GAME_PREBUILT=1 xcodebuild -project packaging/ios/HorizonIOS.xcodeproj \
  -target HorizonGame -configuration Release -sdk iphoneos -arch arm64 \
  IOS_TARGET="$IOS_TARGET" CODE_SIGNING_ALLOWED=NO CODE_SIGN_IDENTITY= CODE_SIGNING_REQUIRED=NO \
  DEBUG_INFORMATION_FORMAT=dwarf CONFIGURATION_BUILD_DIR="$PWD/build-ios" \
  build > build-ios.log 2>&1 || { tail -60 build-ios.log; exit 1; }
APP=build-ios/HorizonGame.app
# Each local build gets the next build number, so the phone sees an update.
COUNTER="$HOME/.erkam/run-number"
RUN=$(( $(cat "$COUNTER" 2>/dev/null || echo 100) + 1 )); echo "$RUN" > "$COUNTER"
python3 tools/ci/ios_release.py --stamp-app "$APP" --run-number "$RUN" --run-attempt 1 \
  --expect-commit "$(git rev-parse HEAD)" --client-id "$HORIZON_MSA_CLIENT_ID"
cp "$PROFILE" "$APP/embedded.mobileprovision"
/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier $BUNDLE_ID" "$APP/Info.plist"
for framework in "$APP"/Frameworks/*.framework; do codesign -f -s "$SIGN_IDENTITY" "$framework"; done
codesign -f -s "$SIGN_IDENTITY" --entitlements "$ENTITLEMENTS" "$APP"
xcrun devicectl device install app --device "$DEVICE" "$APP"
xcrun devicectl device process launch --device "$DEVICE" --terminate-existing "$BUNDLE_ID"
