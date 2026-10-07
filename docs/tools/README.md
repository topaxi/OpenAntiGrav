# Tools

| Tool | Status | Purpose |
| --- | --- | --- |
| [`oag-unpack`](oag-unpack.md) | working | Inspect and extract disc images |
| [`oag-view`](oag-view.md) | **working** | Display assets from a disc image; headless screenshots |
| `oag-convert` | M1 | Convert assets to standard formats |
| [`oag-trace`](oag-trace.md) | **working** | Compare a simulation run against a trace from the original |
| [`oag-game`](oag-game.md) | **working** | The engine itself: the front end, and the race its `Launch Game` starts |
| [autopilot planning](autopilot-planning.md) | **working** | Plan an input script that drives to a point on the track, offline in our simulation or live in the emulator |
| [frame comparison](frame-compare.md) | **working** | One frame of the original and one of ours from the same captured state: teleport, capture, render, diff |
| [RPCS3 harness](../reverse-engineering/rpcs3-debugger.md) | **working** | Boot, drive and observe a PS3 title under RPCS3: headless boot, `TTY.log`, the GDB stub, and the two input gates that fail silently |

[Packaging](packaging.md) covers `just appimage` and `just appimage-portable`:
the engine as one file that runs on a Steam Deck, why AppImage rather than
Flatpak, the glibc floor that makes the second recipe necessary, how a packaged
build finds the player's own disc image, and the gamepad mapping.

[Android](android.md) covers `just apk`: a NativeActivity build of `oag-game` for
arm64 phones, how to install it, where the disc images go, and what a first pass
does not do yet.

[Releases and CI](releases.md) covers the two GitHub workflows: what `ci.yml` gates, and
what `release.yml` builds (artifact names, the zero-game-content check, how a
release is cut).

All are run through `just` for convenience:

```sh
just unpack info data/images/pulse-psp-usa.chd
```

or directly:

```sh
cargo run -p oag-tools --bin oag-unpack -- info data/images/pulse-psp-usa.chd
```
