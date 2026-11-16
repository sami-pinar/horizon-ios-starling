#!/bin/bash
# Build the game for the iOS Simulator on this Mac and install it, the local twin of .github/workflows/sim.yml.
# Needs rustup with the aarch64-apple-ios-sim target, Xcode and xcodegen. The first run compiles everything; later
# runs only what changed (incremental, 16 codegen units, no LTO). Usage: tools/ci/local-sim.sh [simulator UDID]
set -euo pipefail
cd "$(dirname "$0")/../.."
export PATH="$HOME/.cargo/bin:$PATH"
export HORIZON_TELEMETRY_URL=off
export IOS_TARGET=aarch64-apple-ios-sim
export HORIZON_MSA_CLIENT_ID=331e08f8-172d-43b9-835a-3108c2fcdbf3
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
export CARGO_PROFILE_RELEASE_LTO=false
export CARGO_INCREMENTAL=1
export IPHONEOS_DEPLOYMENT_TARGET=16.0
SIM="${1:-789A73D4-DC0D-4715-9380-1F27F5061A1A}"
cargo build --release --locked --target "$IOS_TARGET" -p horizon-official --no-default-features --features audio,horizon-extras
( cd packaging/ios && xcodegen generate >/dev/null )
HORIZON_GAME_PREBUILT=1 xcodebuild -project packaging/ios/HorizonIOS.xcodeproj \
  -target HorizonGame -configuration Release -sdk iphonesimulator -arch arm64 \
  IOS_TARGET="$IOS_TARGET" CODE_SIGNING_ALLOWED=NO CODE_SIGN_IDENTITY= CODE_SIGNING_REQUIRED=NO \
  DEBUG_INFORMATION_FORMAT=dwarf CONFIGURATION_BUILD_DIR="$PWD/build-sim" \
  build > build-sim.log 2>&1 || { tail -60 build-sim.log; exit 1; }
codesign -f -s - --deep build-sim/HorizonGame.app
xcrun simctl terminate "$SIM" dev.horizon.game 2>/dev/null || true
xcrun simctl install "$SIM" build-sim/HorizonGame.app
echo "installed on $SIM"
