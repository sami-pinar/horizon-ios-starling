# Development log

Newest first. One entry per change: what, why, how it was checked.

## 2026-11-15: Menu input, hidden settings button, docs
* **Menu input rewritten** (`game/touch/starling_menu.rs`). Two modes with a button at the top right: *tap* (the
  finger is the pointer, no cursor) and *cursor* (trackpad: left quarter is left/right click zones, the rest moves
  the cursor, tap clicks, long press right-clicks, tap then hold drags, two fingers scroll). The old iOS overlay
  (More, Hide GUI, Sprint, click buttons) no longer appears in menus.
* The top-row **settings button** did nothing; it is hidden by default.
* This documentation folder was added.

## 2026-11-15: F3 overlay inside the corners
* The debug overlay started at the very edge and was cut off by the phone's rounded corners and notch. It is now
  inset 56 points at the sides and 10 at the top (`debug_overlay::draw`). Checked in the simulator by screenshot.

## 2026-11-15: Starling's own controls, with their artwork
* Moved to Starling v0.7.8's touch engine (`starling-engine`). It compiled unchanged after adding one helper
  (`keybinds::available_keys`).
* The engine looks for `gui/bedrock/...` textures; added a loader that packs them from the app's data folder.
  They are fetched from Mojang's pinned public repo with checksums and never committed.
* Verified in the simulator: joystick moved the player about 4.8 blocks; look drag turned the camera 23.5 degrees for
  a 110-point drag; hotbar tap selected slot 6; attack swung the arm.

## 2026-11-15: Real wiring instead of a skin
* `game/starling_touch.rs` feeds fingers to Starling's `TouchControls` and its output to the pad pipeline.
  UIKit recognizers stand down while playing. Controller glyphs are hidden for touch-only input.
* Added the `sim.yml` workflow and the simulator test loop (see [TESTING.md](TESTING.md)).
* Found why the first attempt showed nothing: the game draws its buttons with a different function than the one
  that was edited.

## 2026-11-14: First attempt (superseded)
* Installed Horizon's iOS port on an iPhone 12 with a free Apple ID, loaded game files, and tried copying
  Starling's controls as sprites inside the old overlay (`starling-look`).
