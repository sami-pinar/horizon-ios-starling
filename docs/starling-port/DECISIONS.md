# Decisions

**Use Starling's engine, not a skin.** The first attempt only redrew the old overlay with Starling's sprites. It
looked similar and felt the same. The real engine (finger roles, look formula, dynamic joystick, layouts) is what
was wanted, so it is wired in through the game's existing virtual-controller input.

**Feed the pad pipeline.** Starling's engine outputs a `PadState`. The game already maps pad buttons to actions,
so touch needs no parallel action code, and the behaviors (jump, sprint, sneak, attack, use) are identical to a
controller's.

**Hide controller glyphs for touch.** Fingers are not a controller; showing "A: Jump" would be wrong.

**Fetch Bedrock art, never commit it.** See [ASSETS.md](ASSETS.md).

**Replace the UIKit gesture layer while a Starling module owns the screen.** Two systems reading the same fingers
caused buttons that sometimes did nothing (UIKit recognizers cancelling winit's touches).

**Menu: tap and cursor modes, remembered.** Direct tapping is the simplest; the trackpad mode lets one thumb
click while the other moves the cursor. The mode lives in a one-word file in `Documents`.

**Test in the simulator before the phone.** The control socket reports exact positions, so touch behavior is
checked with numbers, not by eye alone.
