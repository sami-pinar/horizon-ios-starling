# Artwork

## Starling's controls need Bedrock textures

Starling's touch engine draws its buttons, D-pad and joystick with Minecraft Bedrock control textures
(`gui/bedrock/ui/jump`, `gui/bedrock/controls/up`, ...). Starling does not ship them: its Android app downloads
them onto the device from Mojang's public `bedrock-samples` repository at a pinned revision, checking a SHA-256 for
each file. Without them the engine draws a plain procedural fallback.

This fork does the same thing, by hand: the files listed in Starling's `packaging/android/bedrock-controls.json`
are fetched at the pinned revision, verified, and copied into the app's data folder at
`Documents/resources/assets/minecraft/textures/gui/bedrock`.

**They are never committed to this repository or put in a workflow artifact.** They belong to Mojang and are
only for the owner's own device.

## Menu buttons

The menu mode button, the mouse zones and the cursor use the same plate (the Bedrock `joystick_knob` when it is
installed, drawn bevels otherwise) and pixel icons drawn in code, so they need no extra art.

## Earlier attempt

`crates/horizon-client/assets/touch/*.png` holds Starling's own MIT-licensed touch art, used by the first,
superseded attempt on `starling-look`.
