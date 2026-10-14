# Developing OpenAntiGrav

The entry point for a contributor. For playing the game, see
[installing](installing.md) instead. For why the project is shaped this way,
see [goals](goals.md).

Three rules decide most of what follows:

1. **The simulation must not know a renderer exists.** It takes an input
   snapshot in and emits a state snapshot out, bit-identically on every
   platform ([determinism](../architecture/determinism.md)).
2. **No game content is ever committed.** Disc images and everything derived
   from them live under `data/`, which is gitignored ([legal](legal.md)).
3. **A reverse-engineering claim carries its evidence and a 0-100 confidence
   score** ([confidence rubric](../reverse-engineering/confidence-rubric.md)).
   Never invent what a disc already authors; parse it and play it.

[`CLAUDE.md`](../../CLAUDE.md) at the repository root is the fullest statement
of the working rules (written for an AI agent, readable by anyone).

## Prerequisites

### Required

- **Rust**, the version pinned in `rust-toolchain.toml` (1.99.0). Install
  [rustup](https://rustup.rs) and it fetches that version on the first build.
  The oldest compiler the code is promised to build on is 1.97.1 (`ci.yml`'s
  `msrv` job).
- **[cargo-nextest](https://nexte.st)**, the test runner. `cargo test` is not
  used.
- **[just](https://github.com/casey/just)**, the task runner. `just --list`
  shows every recipe.
- **[cargo-shear](https://github.com/Boshen/cargo-shear)**, for the
  `check-unused-deps` part of the gate.
- **Python 3**, which the gate's `check-*` scripts under `scripts/` run on.
- **System libraries** for the audio, gamepad and graphics crates. The table is
  in [installing](installing.md#build-it-yourself).
- **GStreamer development libraries**, only for `just play` and its siblings: on
  Linux they build `oag-game` with the `native-video` feature. A plain
  `cargo build` does not need them.

```sh
cargo install cargo-nextest cargo-shear just --locked
```

### For working with disc images

- **Your own disc images**, in `data/images/`, named as
  [`data/README.md`](../../data/README.md) describes. `data/images/` is empty in a
  fresh clone, and a worktree needs `just link-data` to see the main checkout's.
- **`chdman`** (from `mame-tools`) is the reference implementation our CHD reader
  is checked against, and `just extract-iso` uses it. Day-to-day use does not need
  it.

### For reverse engineering

All of it is in [toolchain](../reverse-engineering/toolchain.md); this is the short
list.

- **[Ghidra](https://ghidra-sre.org)** 11 or newer with a JDK 21 or newer, and the
  GhidraMCP bridge the repository's `.mcp.json` starts
  ([setup](../reverse-engineering/toolchain.md#ghidramcp)).
- **The Allegrex processor module** for PSP code (`just build-allegrex`, needs
  JDK 21). Stock Ghidra silently mis-decodes PSP vector instructions, so this is
  required for PSP work, not optional. See
  [Allegrex and the VFPU](../psp/allegrex-vfpu.md).
- **Emulators**, which behavioural verification is measured against: PPSSPP
  (PSP; the Arch package installs `PPSSPPSDL`), PCSX2 (PS2), RPCS3 (PS3) and Vita3K
  (Vita; [capture notes](../reverse-engineering/vita3k-capture.md)). PS4 has no
  emulator here, so Omega is read statically. Recipes:
  [emulator recipes](../reverse-engineering/emulator-recipes.md).
- **[`uv`](https://docs.astral.sh/uv/)** runs the Python scripts that drive a live
  emulator (`just trace`, `just drive`, `just autopilot`, `just scripted-emu`).
  They declare their own dependencies inline.

### Optional

- **`ffmpeg`** transcodes each disc's intro movie the first time it boots, into
  `data/cache/` (gitignored). Without it the intro is skipped and movies show no
  picture; everything else works.
- **ImageMagick** (`magick`) is only for `just frame-compare`.

## A first session

```sh
git clone <this repo> && cd OpenAntiGrav
cp "/path/to/WipEout Pulse (Europe).chd" data/images/pulse-psp-eu.chd
just unpack info data/images/pulse-psp-eu.chd     # what is this disc?
just unpack list data/images/pulse-psp-eu.chd     # what is on it?
just unpack sniff data/images/pulse-psp-eu.chd    # which files are worth a look?
```

Pull the executable out for Ghidra, and look inside an asset archive straight from
the image:

```sh
just unpack extract data/images/pulse-psp-eu.chd -o data/extracted/psp '*BOOT.BIN'
just wad list data/images/pulse-psp-eu.chd:PSP_GAME/USRDIR/FE.wad
just wad tags data/images/pulse-psp-eu.chd:PSP_GAME/USRDIR/FE.wad
```

WAD entries are stored by hash, not name. `just mine-names` recovers candidate names
into `data/extracted/psp/names.txt`, which `wad list --names` then uses:

```sh
just mine-names data/images/pulse-psp-eu.chd
just wad list data/images/pulse-psp-eu.chd:PSP_GAME/USRDIR/Data.wad \
    --names data/extracted/psp/names.txt
```

Render a ship to a PNG with no window, or open a viewer (left and right browse,
Escape quits; arrow keys orbit a model when `--screenshot` is left off):

```sh
just view data/images/pulse-psp-eu.chd:PSP_GAME/USRDIR/Data.wad \
    --mesh 'Data\Ships\Feisar\Ship.vex' --screenshot /tmp/ship.png
just view data/images/pulse-psp-eu.chd:PSP_GAME/USRDIR/FEData.wad
```

Play it. `just play` alone boots the EU Pulse disc, so the image copied above is
the one it finds:

```sh
just play                  # intro, menus, then a race
just play --race           # skip straight to a race
```

A keyword (`usa`, `ps2`, `pure`) or a path (`just play data/images/<file>`) picks
another disc. `2048` and `omega` read an unpacked package under `data/extracted/`
instead of a file in `data/images/`; the recipe in the `justfile` says what it
expects and where [`data/README.md`](../../data/README.md) explains the unpacking.

Keyboard: WASD or the arrow keys steer, X or Return thrusts, Q and E are the
airbrakes, Escape backs out one level. Tool pages: [oag-unpack](../tools/oag-unpack.md),
[oag-view](../tools/oag-view.md), [oag-game](../tools/oag-game.md), and the
[index of all of them](../tools/README.md).

When you run the game headless for a screenshot or a test, pass `--no-audio`.

## The gate

`just` on its own is the gate every commit must pass: format, lint, tests, the
documentation link check, the dependency and determinism rules, the file-size
ratchets, the names and handover checks, and the schema checks. `just --list`
names each one, and `CLAUDE.md` explains what each guards.

| You changed | Run |
| --- | --- |
| Rust code | `just` (or `just gate-affected`, which runs every check but only the tests the branch can reach) |
| Code that a disc-backed test reads | also `just test-data`, which needs your images; those tests are `#[ignore]`d and never run in CI |
| Only `docs/`, thread files, `HANDOVER.md` or doc comments | `just check-docs`, plus `just check-handover` if `HANDOVER.md` moved, `just check-names` if `names.tsv` moved and `just check-status` for the status table; if a `.rs` file changed at all, also `just fmt-check` and `just lint` |
| A recovered function name | a row in `docs/ghidra/functions/psp-pulse-usa/names.tsv` and its evidence page, in the same change; `just check-names` enforces it |

`docs/overview/status.md`'s RE-coverage table is generated: `just gen-status`
rewrites it, and `just check-status` fails when it is stale.

## Where things live

```
crates/       about 50 Rust crates, see Cargo.toml
docs/         the primary deliverable, see docs/README.md
assets/       this project's own UI layouts, strings, shaders and fallback art
schemas/      the TOML Schemas every shipped TOML file names
scripts/      the gate's checks and the emulator drivers
data/         your disc images and everything derived from them (gitignored)
HANDOVER.md   work in flight: an index, with one file per thread beside it
```

The crates worth knowing first:

| Crate | What it is |
| --- | --- |
| `oag-core` | deterministic math, the fixed-timestep clock, the seeded random generator, state hashing |
| `oag-disc`, `oag-formats` | disc images, then WAD, PSARC and the other containers |
| `oag-physics`, `oag-race`, `oag-ai`, `oag-weapons`, `oag-gameplay` | the simulation: it depends on nothing that draws |
| `oag-raceplay` | a race from load to finish line, above the renderer and below the game |
| `oag-render` | the wgpu renderer; it owns no window |
| `oag-title`, `oag-pulse`, `oag-pure`, `oag-hd`, `oag-2048`, `oag-omega` | what each title ships, as data |
| `oag-game` | the composition root: boots the front end, then a race |

The full list, each crate's reason to exist and the dependency rules that
`just check-deps` enforces are in [workspace layout](../architecture/workspace-layout.md)
and the crate table in [`CLAUDE.md`](../../CLAUDE.md).

## Work in flight

[`HANDOVER.md`](../../HANDOVER.md) records what the repository does not: work in
flight, what was deliberately deferred and why, which findings were independently
verified, and the traps that cost time. Read it before starting anything
non-trivial. Each open thread is one file in the thread directory `HANDOVER.md`
indexes, with `## Open` and `## Next Steps` sections. Delete a thread's file and
its index line together when its work lands. Because a thread file is deleted
that way, nothing outside the thread directory and `HANDOVER.md` links to one;
cite the `docs/` page, test or address instead.

## Where to read next

- [Documentation reading order](../README.md), the tree map and the "I want to..."
  table.
- [Roadmap](roadmap.md) for what is planned and [status](status.md) for what is true
  now.
- [Methodology](../reverse-engineering/methodology.md): observe, hypothesise,
  verify, document, implement.
- [ADR index](../architecture/adr/README.md): every decision that shaped the
  engine. ADRs are immutable; a changed decision gets a new one that supersedes
  the old.
- [Glossary](glossary.md) for the Wipeout-specific terms.
