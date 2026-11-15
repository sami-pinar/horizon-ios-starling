# Building

There is no local Rust toolchain on the development Mac, so everything compiles in GitHub Actions.

| Workflow | Runs on | Produces | Use |
| --- | --- | --- | --- |
| `check.yml` | Linux, on every push to `starling-look` / `starling-engine` | pass/fail in about 5 minutes | Quick compile check of the game crate plus the touch tests |
| `sim.yml` | macOS, dispatched by hand | `horizon-sim`: an iOS Simulator app | Testing on a Mac without a phone |
| `ios.yml` | macOS, dispatched by hand | `horizon-ios-game`: an unsigned device IPA | Installing on a phone |

Dispatch: `gh workflow run ios --ref starling-engine`, and for `sim` (its file must also exist on `main`):
`gh api -X POST repos/<owner>/<repo>/actions/workflows/sim.yml/dispatches -f ref=starling-engine`.

A warm build takes roughly 15 to 20 minutes; the compile check is the fast way to find errors.

## Getting a build onto an iPhone with a free Apple ID

1. Download the `horizon-ios-game` artifact and unzip the IPA.
2. Re-sign: replace `embedded.mobileprovision` with the profile Xcode made for the sideload stub project,
   set `CFBundleIdentifier` to that project's id, `codesign` the embedded framework, then the app with the stub's
   entitlements.
3. `xcrun devicectl device install app --device <id> HorizonGame.app`, then `device process launch`.
4. A free profile lasts seven days; re-sign and reinstall before it expires.

The Apple ID itself is only ever entered by the person who owns it.

## Game files

The app reads prepared game files from `Documents/resources` (marker `horizon-resources.json`). They are copied in
from a local Horizon launcher install and are never committed.
