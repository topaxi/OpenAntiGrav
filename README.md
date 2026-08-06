# OpenAntiGrav

A clean-room reimplementation of **Wipeout Pulse** in Rust, built as a
foundation for the wider Studio Liverpool anti-gravity racing lineage.

This is **not** a decompilation. The original executables are treated as a
specification: behaviour is studied, documented and then reimplemented with a
modern architecture. The goal is that gameplay is indistinguishable from the
original while the code underneath is something a contributor can read.

> **Status: M4, playable core.** `just play` boots the disc's own intro and
> menus into a real single-ship time trial: a track and ship loaded off the
> disc, physics simulated at a fixed 60 Hz, driven with a chase camera. The
> physics model (thrust, steering, airbrakes, the air cushion, collision and
> wall contact) is fully implemented from reverse-engineered instruction-level
> evidence but not yet fully verified tick-for-tick against the original - see
> [M4 in the roadmap](docs/overview/roadmap.md#m4---playable-core) for exactly
> what's measured and what's still open. Earlier milestones (disc/archive
> tooling, asset decoding, binary analysis, the trace-comparison harness) are
> done or substantially done. See
> [`docs/overview/roadmap.md`](docs/overview/roadmap.md) for the full milestone
> breakdown and [`docs/psp/pulse-disc-layout.md`](docs/psp/pulse-disc-layout.md)
> for what has been found on disc so far.

## Scope

In scope, in the order they will be tackled:

| Title | Platform | Role |
| --- | --- | --- |
| **Wipeout Pulse** | PSP, PS2 | Primary target |
| Wipeout Pure | PSP | Direct format ancestor, cross-reference |
| Wipeout HD / Fury | PS3 | Later |
| Wipeout 2048 | Vita | Long-term goal |
| Omega Collection | PS4 | If feasible |

Anything before Pure is out of scope.

## Legal

**No game content is distributed with this project, and none ever will be.** No
assets, no executables, no extracted data. You supply your own legally obtained
copies. See [`docs/overview/legal.md`](docs/overview/legal.md).

## Prerequisites

### Required

- **Rust** stable (1.88+, edition 2024). Install via [rustup](https://rustup.rs).
- **[cargo-nextest](https://nexte.st)** for the test runner.
- **[just](https://github.com/casey/just)** for the task runner.

```sh
cargo install cargo-nextest --locked
cargo install just --locked          # or: pacman -S just
```

### For working with disc images

- **`chdman`** (from `mame-tools`) is the reference implementation used to
  validate our own CHD reader. Not needed for day-to-day use; the Rust reader
  handles both `createdvd` and single-track `createcd` images natively.

```sh
sudo pacman -S mame-tools
```

### For reverse engineering

- **[Ghidra](https://ghidra-sre.org)** 11 or newer, with a JDK 21+.
- **[GhidraMCP](https://github.com/LaurieWired/GhidraMCP)**, which lets an agent
  drive Ghidra directly.
- **JDK 21 specifically**, to build the Allegrex processor module. Stock Ghidra
  silently mis-decodes PSP vector instructions, so this is required rather than
  optional for PSP work:
  ```sh
  sudo pacman -S jdk21-openjdk
  just build-allegrex     # produces an installable zip in data/tools/
  ```
  See [Allegrex and the VFPU](docs/psp/allegrex-vfpu.md).
- **[PPSSPP](https://ppsspp.org)** for PSP runtime tracing, memory inspection
  and save states. This is what behavioural verification is measured against.
  The Arch package installs the binary as `PPSSPPSDL`, not `ppsspp`.
- **[PCSX2](https://pcsx2.net)** for the PS2 side.
- **`binwalk`** (optional) for faster triage of unknown containers.
- **[`uv`](https://docs.astral.sh/uv/)** to run the Python scripts that drive
  a live PPSSPP over its websocket debugger (`just trace`, `just drive`,
  `just autopilot`, `just scripted-emu`) - they declare their own
  dependencies inline, so `uv run` is all that's needed.

```sh
sudo pacman -S ghidra ppsspp pcsx2 binwalk uv
```

Setting up the Ghidra bridge is described in
[`docs/reverse-engineering/toolchain.md`](docs/reverse-engineering/toolchain.md).

### Optional, for the game itself

- **`ffmpeg`** transcodes the disc's intro FMV the first time `just play` runs
  (cached under `data/cache/`, gitignored). Without it the boot sequence still
  plays, just without a picture.
- **ImageMagick** (`magick`) is only needed for `just frame-compare`, which
  diffs one of our renders against a PSP capture.

## Getting started

```sh
git clone <this repo> && cd OpenAntiGrav

# Put your own disc images here, named as described in data/README.md
cp "/path/to/WipEout Pulse (USA).chd" data/images/pulse-psp-usa.chd

# What is this disc?
just unpack info data/images/pulse-psp-usa.chd

# What is on it?
just unpack list data/images/pulse-psp-usa.chd

# Which files are worth reverse engineering?
just unpack sniff data/images/pulse-psp-usa.chd

# Pull the executable out for Ghidra
just unpack extract data/images/pulse-psp-usa.chd -o data/extracted/psp '*BOOT.BIN'

# Look inside an asset archive, straight from the image
just wad list data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad
just wad tags data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad

# Actually look at the assets
just view data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FEData.wad

# Recover entry names, then list an archive with them
just mine-names data/images/pulse-psp-usa.chd
just wad list data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad \
    --names data/extracted/psp/names.txt

# Render a ship
just view data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad \
    --mesh 'Data\Ships\Feisar\Ship.vex' --screenshot /tmp/ship.png

# Play it: the disc's own intro and menus into a real single-ship time trial.
# `just play` alone boots the EU disc by default; `usa` picks the one just copied above.
just play usa
# or skip straight to the race
just play usa --race
```

`oag-view` opens a window; left and right browse, escape quits. Drop
`--screenshot` from a `--mesh` or `--track` invocation and arrow keys orbit the
model instead. Add `--screenshot out.png` to render headlessly, with no window
at all.

`oag-game` (what `just play` runs) also opens a window: WASD or arrow keys
steer, X/Return thrusts, Q/E are the airbrakes. Escape backs out one level -
race to menu, menu page to the one behind it - and quits only when there's
nothing left behind. See [`docs/tools/oag-game.md`](docs/tools/oag-game.md).

`just` on its own runs the full check: format, lint and test.

## Layout

```
crates/
  core/      deterministic math, fixed-timestep clock, seeded PRNG, state hashing
  disc/      CHD and raw ISO readers, ISO 9660 filesystem walker
  formats/   WAD archives, LZSS, textures, track data, front-end XML, PMF, fonts
  assets/    runtime asset access: read a WAD by path or straight out of a disc image
  tools/     command line tools (oag-unpack, oag-wad)
  physics/   ship dynamics and collision queries (depends on core only)
  race/      race rules: modes, lap timing, track progress
  gameplay/  the World struct, the InputSnapshot the simulation consumes
  render/    the wgpu renderer: mesh pipeline, track ribbon, cameras (owns no window)
  input/     maps real devices onto the abstract button layer
  view/      wgpu asset viewer
  trace/     per-tick trace capture and comparison against the original
  game/      composition root: boots the front end, then a race
docs/        the primary deliverable, see docs/README.md
data/        your disc images and extracted data (gitignored)
```

More crates arrive as their milestone opens; the full intended shape is in
[`docs/architecture/workspace-layout.md`](docs/architecture/workspace-layout.md).

## Handover

[`HANDOVER.md`](HANDOVER.md) records what the repository does not: work in
flight, what was deliberately deferred and why, which findings are
independently verified versus single-source, and the traps that cost time.

## Documentation

Documentation is a first-class deliverable, not an afterthought. Every discovery
gets written down with its evidence and a confidence score, so a future
contributor can understand the engine without reopening Ghidra.

Start at [`docs/README.md`](docs/README.md).

## Licence

Dual licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your
option.
