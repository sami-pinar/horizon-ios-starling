# Architecture

## Two input systems, one game

The iOS port had its own Pojav-style touch layer (`crates/horizon-client/src/touch.rs`, `game/touch.rs`, plus
UIKit gesture recognizers in the Swift launcher). This fork adds Starling's engine next to it and gives it the
screens it is good at, leaving the old code as a fallback (`HORIZON_AMETHYST_TOUCH=1` brings it back).

| Situation | Handled by |
| --- | --- |
| Playing in a world | `starling_touch.rs`, driving Starling's `TouchControls` |
| Any menu, inventory, pause, title screen | `touch/starling_menu.rs` |

`starling_on()` (iOS only) decides. While either module owns the screen, the UIKit recognizers are told to stand
down (`touch_gestures(0, 2, ..)`) so they cannot steal a finger and cancel it.

## In the world: `game/starling_touch.rs`

```
winit Touch --> starling_event --> TouchControls::update --> camera turn --> look_turn
                                                         \-> fingers' roles
every frame: TouchControls::poll_state --> PadState --> Pads::feed_touch --> poll_pads --> game actions
```

* `TouchControls` (crate `horizon-client-render`, `src/touch.rs` and `src/touch/*`) is Starling v0.7.8 unchanged.
  It turns fingers into a virtual controller (`PadState`) and camera turns; each finger gets a role when it lands
  and keeps it, so moving, looking and pressing never take each other's fingers.
* The pad pipeline (`game/pad.rs`) already turned controller buttons into actions, so jump, attack, use,
  sprint and sneak need no extra wiring. `feed_touch` sets the pad's virtual state.
* Touch-only input must not look like a controller: `poll_pads` does not set `in_use` while only touch feeds
  the pad, so no button glyphs or guide appear.
* The hotbar is a row of GUI pixels the engine knows nothing of; a finger landing there selects the slot.
* Shortcut buttons (chat, debug, camera) press their keys through `touch_press_key`.

## In menus: `game/touch/starling_menu.rs`

Two modes, switched by the button at the top right and remembered in `Documents/starling-ui-mode.txt`:

* **Tap**: the finger is the pointer. This reuses the port's direct gestures (`touch_menu`), so sliders, slots
  and text fields keep working. No cursor is drawn.
* **Cursor**: a trackpad.
  * Left quarter of the screen: two mouse-button zones (upper left click, lower right click), held while touched,
    taking two thirds of its width; a thin full-height strip takes the last third and holds Shift, so a click on
    a slot while it is held is a shift-click. The launcher's Settings has a toggle to hide all three (they still work).
  * Remaining three quarters: dragging moves the cursor (speed-dependent gain); a tap clicks at the cursor;
    a long press right-clicks; tap, then press and hold (or drag) holds the left button; two fingers scroll.
  * Button edges are queued two frames ahead, one frame apart, so widgets find the pointer (they are located
    while drawing) before a press arrives.

## The look

The engine draws with Bedrock control textures when they are present at
`<Documents>/resources/assets/minecraft/textures/gui/bedrock` and with a plain fallback when they are not
(see [ASSETS.md](ASSETS.md)). `hud.rs: bedrock_control_sprites` packs them into the HUD atlas. The menu buttons
reuse the same plate.

## Safe areas

The game draws edge to edge, and a phone's rounded corners and notch hide anything at the edge. The F3 overlay
is inset (`debug_overlay::draw`'s `inset`), the menu buttons keep clear of the corner, and Starling's own controls
sit inside their layout margins. Other HUD pieces (chat, toasts) are not inset yet; see the [ROADMAP](ROADMAP.md).
