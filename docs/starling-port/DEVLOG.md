# Development log

Newest first. One entry per change: what, why, how it was checked.

## 2026-11-15: Erkam branding, real shift-click, touch controls editor
* **Shift strip is a shift-click now**: tapping it clicks left at the cursor with Shift down (the click is queued
  behind the key; Shift comes up two frames after the last queued edge). Holding the finger on it keeps Shift down.
* **Options > Controls > Touch Controls...** (iOS only) opens Starling's own editor (`TouchControls::open_editor`,
  `edit_touch`) over the current screen, dimmed: drag buttons, Opacity/Size sliders, add/remove, Grid, Reset,
  Done/Cancel. Done writes `horizon-touch.json`, loaded again at start (`starling_touch.rs`). The fancier editor of
  Starling's `touch_ui.rs` (8k lines) is not ported.
* **Erkam**: the title banner reads ERKAM STYLE (`title_edition.rs`, letters in a 5x7 font drawn with the vanilla
  texture's shades), the version line reads "Minecraft Erkam Style 26.3", the app and launcher say Erkam
  (`CFBundleDisplayName`, launcher texts, English strings), and the app icon and launcher mark are the Erkam logo.
  Bundle ids, the executable name and file names keep their old spelling so installs and saves stay.

## 2026-11-15: Clean cursor
* The cursor's black outline is gone; it is a plain white plus. Its middle pixel is now centred on the pointer
  (it used to sit half a pixel off, with the pointer on the pixel's corner). The mode button's icons lost their
  shadow too.

## 2026-11-15: Chat typing bar, wider zones, eye toggle, new icons
* **Chat**: the phone's keyboard covers the chat box, so typed text is now shown again in a bar near the top, with a
  blinking caret and a placeholder (`chat.rs: draw_typing_bar`).
* The click zones are 1.5 quarters of the screen wide (the buttons were too thin): mouse buttons two thirds of it,
  the shift strip a third.
* An **eye button** under the mode button (cursor mode) hides or shows the zones; hidden, they still work and
  click as usual. Remembered in `Documents/starling-ui-mode.txt` (`cursor hidden`); the launcher's setting also hides.
* Mode icons redrawn: a reticle (cursor) and a touch ripple (tap), centred on the plate's face (the Bedrock plate
  sits off its box), with a soft shadow; eye icon struck through in red while hidden.

## 2026-11-15: Cursor mode refinements, shift strip
* The cursor is a Minecraft-style crosshair (white plus, dark edge); the mode button shows the same crosshair.
* Cursor speed is a constant 1.5 times the finger's travel (it was speed-dependent, up to 2.6).
* Two-finger scroll collects finger travel and sends whole lines (the game's lists ignore fractions).
* New **shift strip** in the left quarter: the mouse buttons take two thirds of its width, a full-height strip
  half their width takes the rest and holds Shift (shift-click). Icon: shift arrow, plus, mouse.
* Launcher Settings gained "Hide click zones in cursor mode" (bit 64 of the touch flags); the zones still work.
* The top row (debug, chat, pause, camera) is re-centred with no gap where the settings button was.

## 2026-11-15: Menu input, hidden settings button, docs
* **Menu input rewritten** (`game/touch/starling_menu.rs`). Two modes with a button at the top right: *tap* (the
  finger is the pointer, no cursor) and *cursor* (trackpad: left quarter is left/right click zones, the rest moves
  the cursor, tap clicks, long press right-clicks, tap then hold drags, two fingers scroll). The old iOS overlay
  (More, Hide GUI, Sprint, click buttons) no longer appears in menus.
* The top-row **settings button** did nothing; it is hidden by default.
* This documentation folder was added.
* Checked in the simulator (menus): tap mode opened Options with a plain tap; mode button flips and remembers the
  mode; in cursor mode a drag moved the cursor and highlighted the button under it, a quick tap clicked World
  Options, a tap on the upper-left zone clicked Cancel with the other hand, a long press right-clicked an
  inventory stack (64 stone became 32 carried). Not testable with the simulator tool: tap-then-hold (needs two
  touches within 340 ms) and two-finger scroll; both are implemented and need a check on the phone.

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

## 2026-11-15 (earlier): First attempt (superseded)
* Installed Horizon's iOS port on an iPhone 12 with a free Apple ID, loaded game files, and tried copying
  Starling's controls as sprites inside the old overlay (`starling-look`).
