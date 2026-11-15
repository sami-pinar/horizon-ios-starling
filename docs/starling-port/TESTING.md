# Testing without the phone

A change is tested in the iOS Simulator first. The game exposes a control socket, which makes this scriptable.

## Setup (once per build)

```
ditto -x -k HorizonGame-sim.zip app && codesign -f -s - --deep app/HorizonGame.app
xcrun simctl install <sim> app/HorizonGame.app
cp -c -R <prepared game files> <app data container>/Documents/resources     # APFS clone: no extra disk
xcrun simctl launch <sim> dev.horizon.game --control-socket /tmp/hz.sock
```

Tap Play in the launcher, wait for `/tmp/hz.sock`, then use `tools/ci/ctl.py --socket /tmp/hz.sock ...`.

## Useful commands

| Command | Use |
| --- | --- |
| `world-create [--seed S] NAME` / `open NAME` | get into a world |
| `state` | player position, yaw, pitch, selected slot, screen: the numbers a touch test asserts on |
| `key f3 tap`, `pad ...`, `click-button LABEL` | drive the game without touch |
| `screenshot PATH` | frame from the game |

## Real touches

The simulator tool injects real touches. **Quirk:** with the device in landscape, its coordinates are the
*portrait* frame (390 x 844 points). A landscape point `(lx, ly)` is tool point `(390 - ly, lx)`.
`simctl io screenshot` returns the portrait frame too; `sips -r 270` makes it upright.

## What a touch test looks like

1. Read `state` (position, yaw).
2. Send a touch path on the joystick, a look drag, a tap on a button.
3. Read `state` again: the position moved, the yaw changed by `drag / width * 180` degrees, `selected` changed.
4. Take a screenshot and look at it.

Checks already done this way: joystick movement, look drag, hotbar taps, attack swings, jump.

## Limits

The simulator is not a phone: no notch or rounded corners are drawn, GPU behavior differs, and the real
gesture feel (latency, palm touches) needs the device.
