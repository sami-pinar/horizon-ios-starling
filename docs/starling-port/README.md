# Starling controls on iOS

This fork's goal: play Horizon (Minecraft Java in Rust) on an iPhone with the **real Starling/GoyDevv touch
controls**, not a lookalike. Everything about how that is built, tested and decided lives in this folder.

| Document | What it answers |
| --- | --- |
| [DEVLOG.md](DEVLOG.md) | What changed, when, and why (newest first). Add an entry with every change. |
| [ARCHITECTURE.md](ARCHITECTURE.md) | How touch input is wired into the game: files, data flow, who owns which finger. |
| [BUILDING.md](BUILDING.md) | The GitHub workflows, branches, and how a build gets onto a phone. |
| [TESTING.md](TESTING.md) | How a change is tested without touching the phone: simulator, control socket, screenshots. |
| [ASSETS.md](ASSETS.md) | Which artwork is used and where it comes from (and what is never committed). |
| [DECISIONS.md](DECISIONS.md) | Why things are the way they are. |
| [ROADMAP.md](ROADMAP.md) | What is done and what is next. |

## Branches

* `main`: the upstream iOS port plus the `sim` workflow. Untouched by the experiments.
* `starling-engine`: the working branch. Starling v0.7.8's touch engine wired into the game. **Start here.**
* `starling-look`: the first, superseded attempt (Starling's sprites drawn inside the old overlay). Kept for history.

## Working rules

1. Every change gets a [DEVLOG](DEVLOG.md) entry in the same commit.
2. Nothing is called done until it ran in the simulator ([TESTING.md](TESTING.md)); the phone is for the last check.
3. Mojang's artwork is never committed ([ASSETS.md](ASSETS.md)).
