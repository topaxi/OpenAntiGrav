# OpenAntiGrav

A clean-room reimplementation, in Rust, of the Studio Liverpool anti-gravity
racing engine - Wipeout Pure, Pulse, HD/Fury, 2048 and the Omega Collection on
one shared engine, not a single-game project that happens to tolerate others.
Pulse is the reference implementation: the most deeply reverse-engineered and
the most verified against the original. The rest are built on that same
architecture as their own milestones open, not bolted on afterward - see
[Scope](#scope) for where each title actually stands today.

This is **not** a decompilation. The original executables are treated as a
specification: behaviour is studied, documented and then reimplemented with a
modern architecture. The goal is that gameplay is indistinguishable from the
original while the code underneath is something a contributor can read.

> **Status.** Pulse plays start to finish through its own menus into a race:
> a track and ship loaded off the disc, physics simulated at a fixed 60 Hz,
> driven with a chase camera. Its physics model (thrust, steering, airbrakes,
> the air cushion, collision and wall contact) is fully implemented from
> reverse-engineered instruction-level evidence but not yet fully verified
> tick-for-tick against the original - see
> [M4](docs/overview/roadmap.md#m4---playable-core)/[M5](docs/overview/roadmap.md#m5---full-race)
> in the roadmap for exactly what's measured. On the same engine: Pure boots
> to a Time Trial; HD/Fury and 2048 already race and draw textured on their
> own assets; a race can even mix a craft from one title with a track from
> another (Race Remix). Omega Collection's front end boots and a race starts on
> its own data, but it is incomplete. Full picture, subsystem by subsystem and title by title:
> [`docs/overview/status.md`](docs/overview/status.md). Milestone narrative:
> [`docs/overview/roadmap.md`](docs/overview/roadmap.md).

## Scope

| Title | Platform | Status |
| --- | --- | --- |
| **Wipeout Pulse** | PSP, PS2 | Reference implementation - deepest RE coverage, plays menu to race |
| Wipeout Pure | PSP | Boots to a Time Trial on the same engine |
| Wipeout HD / Fury | PS3 | Races and draws textured on its own assets |
| Wipeout 2048 | Vita | Races and draws textured on its own assets |
| Omega Collection | PS4 | Front end boots and a race starts on its own data, incomplete |

Anything before Pure is out of scope.

## Legal

**No game content is distributed with this project, and none ever will be.** No
assets, no executables, no extracted data. You supply your own legally obtained
copies. See [`docs/overview/legal.md`](docs/overview/legal.md).

## How to play

**New here? Start with the step-by-step guide:
[`docs/overview/installing.md`](docs/overview/installing.md).** It covers
Windows, Mac, Linux, Steam Deck and Android: where the game files go, what
the file names can be, and what to do when it says "no disc image found".

## Play it

You need your own copy of the game. Nothing is bundled, and this project
cannot tell you where to get one.

**Install:** prebuilt downloads (`OpenAntiGrav-<version>-linux-x86_64.AppImage`,
`-steamdeck-x86_64.AppImage`, `-linux-x86_64.tar.gz`, `-windows-x86_64.zip`) are on
GitHub Releases. Arch: `openantigrav-bin` / `openantigrav-git` (AUR, once published).
Or build from source:

```sh
cargo build --release -p oag-game
mkdir -p data/images
cp /path/to/your/pulse.chd data/images/             # a disc image you dumped, any name
target/release/oag-game                             # boots what it finds there
```

| Title | What to put in the folder |
| --- | --- |
| Pulse (PSP, PS2) and Pure (PSP) | the disc image, `.chd` or `.iso` |
| Wipeout HD / Fury (PS3) | the **encrypted** disc image, `.iso`, with your disc key beside it (`.dkey`) or typed in the game. Nothing to decrypt |
| Wipeout 2048 (Vita) | a `.vpk` (a Vita `.pkg` is not read yet) |
| Omega Collection (PS4) | the base `.pkg` **and** its update `.pkg`, in the same folder |

File names do not matter. Unpacked folders still work too; see the guide.

Check what was found without opening a window:
`target/release/oag-game --dry-run --no-audio --no-video <source>`. Linux is the only OS the program was run on
natively (Windows was run under Wine). The whole walk-through, the system libraries, where settings and logs are
kept and the error messages you may meet are in
[`docs/overview/installing.md`](docs/overview/installing.md).

## Prerequisites

The rest of this page is for contributors.

### Required

- **Rust**, the version pinned in `rust-toolchain.toml` (1.99.0). Install
  [rustup](https://rustup.rs) and it fetches that version on the first build.
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
cp "/path/to/WipEout Pulse (Europe).chd" data/images/pulse-psp-eu.chd

# What is this disc?
just unpack info data/images/pulse-psp-eu.chd

# What is on it?
just unpack list data/images/pulse-psp-eu.chd

# Which files are worth reverse engineering?
just unpack sniff data/images/pulse-psp-eu.chd

# Pull the executable out for Ghidra
just unpack extract data/images/pulse-psp-eu.chd -o data/extracted/psp '*BOOT.BIN'

# Look inside an asset archive, straight from the image
just wad list data/images/pulse-psp-eu.chd:PSP_GAME/USRDIR/FE.wad
just wad tags data/images/pulse-psp-eu.chd:PSP_GAME/USRDIR/FE.wad

# Actually look at the assets
just view data/images/pulse-psp-eu.chd:PSP_GAME/USRDIR/FEData.wad

# Recover entry names, then list an archive with them
just mine-names data/images/pulse-psp-eu.chd
just wad list data/images/pulse-psp-eu.chd:PSP_GAME/USRDIR/Data.wad \
    --names data/extracted/psp/names.txt

# Render a ship
just view data/images/pulse-psp-eu.chd:PSP_GAME/USRDIR/Data.wad \
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
  formats/   asset containers: WAD, PSARC, LZSS, sound banks, magic-number triage
  assets/    runtime asset access: read a WAD by path or straight out of a disc image
  tools/     command line tools (oag-unpack, oag-wad)
  physics/   ship dynamics and collision queries (depends on core only)
  race/      race rules: modes, lap timing, track progress
  ai/        opponent driver: follows the authored racing line, emits ship controls
  gameplay/  the World struct, the InputSnapshot the simulation consumes
  render/    the wgpu renderer: mesh pipeline, track ribbon, cameras (owns no window)
  audio/     mixing and playback, hardware-free voice pool
  input/     maps real devices onto the abstract button layer
  ui/        the front end: boot movies, menus, HUD
  view/      wgpu asset viewer
  trace/     per-tick trace capture and comparison against the original
  title/     the Title type: what makes a title's data reachable, shared by all of them
  pulse/     Wipeout Pulse's own archive names, hashes and tables
  pure/      Wipeout Pure's, deliberately thinner - only what's actually measured
  hd/        Wipeout HD / Fury's, seven PSARC archives on a PS3 disc rather than two WADs
  2048/      Wipeout 2048's, the Vita package
  game/      composition root: boots the front end, then a race, for any of the above
docs/        the primary deliverable, see docs/README.md
data/        your disc images and extracted data (gitignored)
```

This is illustrative, not exhaustive - more crates arrive as their milestone
opens, and the full intended shape (with the two dependency rules every crate
above is checked against) is in
[`docs/architecture/workspace-layout.md`](docs/architecture/workspace-layout.md),
mirrored in [`CLAUDE.md`](CLAUDE.md)'s own crate table.

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
