# Roadmap

## Done
* Starling v0.7.8 touch engine drives the world (movement, look, buttons, hotbar)
* Bedrock control artwork loaded from the device's own files
* Simulator test loop with a control socket
* F3 overlay kept inside the screen's rounded corners
* Menu input: tap mode and cursor mode, click zones, mode button, cursor drawing; old overlay gone from menus
* Settings button removed from the top row

## Next
* Starling's in-game layout editor and touch settings screen (`touch_ui.rs`), so buttons can be moved
* Real safe-area insets for the whole HUD (chat, toasts, hearts) instead of fixed margins
* Cursor mode extras: sensitivity setting, a visible hold indicator
* Gyro and haptics from Starling's engine
* Retire the old Pojav-style overlay code once nothing needs it
