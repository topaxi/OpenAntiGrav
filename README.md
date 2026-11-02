# OpenAntiGrav

A clean-room reimplementation, in Rust, of the Studio Liverpool anti-gravity
racing engine: Wipeout Pure, Pulse, HD/Fury, 2048 and the Omega Collection on
one shared engine. Pulse is the reference title, the most reverse-engineered
and the most checked against the original. The others run on the same
architecture.

This is **not** a decompilation. The original executables are a specification:
behaviour is observed, written down with evidence and a confidence score, then
rebuilt from scratch. The aim is a game that plays like the original, on code a
contributor can read.

## Where each game stands

As of 2026-10-14. Per-subsystem detail and the evidence behind every cell:
[`docs/overview/status.md`](docs/overview/status.md).

| Title | Platform | What works today |
| --- | --- | --- |
| **Wipeout Pulse** | PSP, PS2 | Reference title. Front end to race, every race mode, campaign with medals and unlocks, records and ghosts. Physics and several weapons are checked against the original; Zone is partial and full-race tick parity is not claimed yet |
| Wipeout Pure | PSP | Boots through its own front end into Time Trial, Single Race and Zone on Pure's own data and HUD; campaign partial |
| Wipeout HD / Fury | PS3 | Front end, then races to the flag on HD's own circuits with its own end-of-race screens; campaign walks, partial |
| Wipeout 2048 | Vita | Races and draws textured; campaign events launch (Single Race, Speed Lap, Eliminator) |
| Omega Collection | PS4 | Front end boots and 142 of 167 campaign cells launch; a race runs on Omega's own circuits but is incomplete |

## What this offers over emulation

- **No emulator in the way.** No shader-compile stutter from an emulated GPU, which helps most on phones and handhelds, and no emulator settings to set up.
- **More than one game.** Pulse is the most complete. Pure and HD/Fury run too. 2048 and Omega run, but much less is worked out ([status](docs/overview/status.md)).
- **Local multiplayer.** Split screen or several windows, up to eight seats: [split screen](docs/rendering/split-screen.md).
- **Smooth above 30 fps.** The frame rate is not capped to the original's. The simulation runs at a fixed 60 Hz and each frame is drawn between ticks. The default limit is 240 fps and can be lifted: [between ticks](docs/rendering/between-ticks.md).
- **Optional effects on top of the original**, such as mapped shadows and motion blur. Where the original has the effect, you can keep the original's look: [modern features](docs/overview/modern-features.md).
- **Custom campaigns and tracks.** Campaigns are mod files, and tracks are authored in Blender: [mods](docs/tools/mods.md), [Blender add-on](docs/tools/blender.md).
- **Mouse, keyboard and touch** as well as pads, with button hints that follow the device you use.
- **Analogue L2 and R2 as the two airbrakes** on a pad that has analogue triggers, on every title. Thrust and brake on the triggers is the alternative.
- **Easy to translate.** Text is in plain files; Brazilian Portuguese is already there on every title: [languages](docs/ui/project-languages.md).
- **Network play, early.** Race on a dedicated server or a LAN host with every title; it has run on one machine so far, so expect rough edges: [networking](docs/networking/README.md).

Downsides:

- It is a rebuild. Some effects, graphics and behaviour will not match the original exactly. [Status](docs/overview/status.md) lists what is checked and what is not.
- It is under heavy change. Expect bugs and missing features.

## When an emulator is the better choice

For the authentic original experience, use an emulator: PPSSPP, PCSX2, RPCS3, Vita3K or shadPS4. They run the real game code, which this project does not. For online play on the original games, the community revival servers at <https://agracingfoundation.org/> are the place to go.

## Legal

**No game content is distributed with this project, and none ever will be.** You
supply your own legally obtained copies. See
[`docs/overview/legal.md`](docs/overview/legal.md).

## Play

**Step by step, for Windows, Mac, Linux, Steam Deck and Android:
[`docs/overview/installing.md`](docs/overview/installing.md).** It covers where
the game files go on each system and what to do when it says "no disc image
found".

Downloads are on GitHub Releases. Only a daily "nightly" pre-release exists so
far, and its files carry the date and commit instead of a version:
`OpenAntiGrav-nightly-<date>-<commit>-linux-x86_64.AppImage`,
`-steamdeck-x86_64.AppImage`, `-linux-x86_64.tar.gz`, `-windows-x86_64.zip`,
`-android-arm64.apk` and `-macos-arm64.zip`. Arch: `openantigrav-bin` or
`openantigrav-git` on the AUR, once published. From source:
`cargo build --release -p oag-game`. A browser build runs at
<https://oag.topaxi.com/>.

Put your files in an `images` folder (`data/images/` in a source checkout). File
names do not matter; the game reads the ID out of the file.

| Title | What to put in the folder |
| --- | --- |
| Pulse (PSP, PS2) and Pure (PSP) | the disc image, `.chd` or `.iso` |
| Wipeout HD / Fury (PS3) | the **encrypted** disc image, `.iso`, with your disc key beside it (`.dkey`) or typed in the game. Nothing to decrypt |
| Wipeout 2048 (Vita) | a `.vpk` (a Vita `.pkg` is not read yet) |
| Omega Collection (PS4) | the base `.pkg` **and** its update `.pkg`, in the same folder |

Check what was found without opening a window:
`oag-game --dry-run --no-audio --no-video`. Linux is the only system the program
has run on natively. Windows was run under Wine, and Mac and iOS have never run on
real hardware; Android's state is in [installing](docs/overview/installing.md).

## Features

- All five titles on one engine, each on its own assets: [status map](docs/overview/status.md).
- Race modes (which ones depend on the title): time trial, speed lap, Zone, single race, Eliminator, Head to Head, Tournament and the disc's campaign: [race modes](docs/gameplay/race-modes.md).
- Race Remix, a craft from one title on a track from another: [roadmap](docs/overview/roadmap.md#what-race-remix-does-today).
- Records and ghost laps, kept automatically on your machine: [persistence](docs/architecture/persistence.md).
- Local multiplayer: split screen or several windows, one seat per pad or keyboard: [split screen](docs/rendering/split-screen.md).
- Your own tracks, AI pilots and menu rows: [modding](docs/tools/modding.md).
- Presentation options (upscalers, screen filters, dynamic resolution, motion blur): [modern features](docs/overview/modern-features.md).
- The disc's languages, plus Brazilian Portuguese on every title: [languages](docs/ui/project-languages.md).
- Linux, Steam Deck, Windows, Android, macOS, iOS and the browser: [releases](docs/tools/releases.md).

## Mods

A mod is a folder, `.zip` or `.oagmod`. The [modding guide](docs/tools/modding.md)
has one short task each: a track from Blender or the Workshop, an AI pilot, a menu
row, packaging. The TOML schemas are published at
<https://oag.topaxi.com/mods/schemas/>.

## How this is built

Most of the code is written by AI coding agents (Claude), working under a human maintainer who reviews and understands every merge. The method is clean-room: the original executables are a specification. Behaviour is observed, written down with evidence and a 0-100 confidence score, then reimplemented. Nothing is transliterated from a decompiler. The structure is enforced by the `just` gate, which every change must pass:

- File length is capped (1,500 lines), and so are inline test modules.
- Dependency rules keep the simulation free of rendering, audio, windowing and network code.
- Determinism checks keep the simulation bit-identical across platforms.

## Contributing

`cargo build`, and `just` is the gate every change must pass. Start at
[`docs/overview/developing.md`](docs/overview/developing.md): the tools, a first
session, the crate map and how work in flight is tracked.

## Documentation

Every discovery is written down with its evidence and a confidence score, so the
engine can be understood without reopening Ghidra. Start at
[`docs/README.md`](docs/README.md).

## Credits

The measurements here depend on other people's tools. Thank you to:

- [PPSSPP](https://www.ppsspp.org/), [PCSX2](https://pcsx2.net/), [RPCS3](https://rpcs3.net/), [Vita3K](https://vita3k.org/) and [shadPS4](https://shadps4.net/), plus the native Omega port [wipeout_pc](https://github.com/AbdelmoumenDeghbouche/wipeout_pc): the emulators and ports we compare the original's behaviour against.
- [Ghidra](https://ghidra-sre.org/): disassembly and decompilation, with [ghidra-allegrex](https://github.com/kotcrab/ghidra-allegrex) (PSP), [ghidra-emotionengine-reloaded](https://github.com/chaoticgd/ghidra-emotionengine-reloaded) (PS2), [Ps3GhidraScripts](https://github.com/clienthax/Ps3GhidraScripts) (PS3), [VitaLoaderRedux](https://github.com/CreepNT/VitaLoaderRedux) (Vita), [GhidraOrbis](https://github.com/astrelsky/GhidraOrbis) (PS4) and [GhidraMCP](https://github.com/LaurieWired/GhidraMCP) (the bridge our agents drive it through).
- [psvpfstools](https://github.com/motoharu-gosuto/psvpfstools), [pkg2zip](https://github.com/mmozeiko/pkg2zip) and [LibOrbisPkg](https://github.com/maxton/LibOrbisPkg): opening the Vita and PS4 packages.
- [Blender](https://www.blender.org/): the track add-on is built on it.
- [wgpu](https://wgpu.rs/) and [winit](https://github.com/rust-windowing/winit): rendering and windowing.
- Everything else we use, read or reimplemented from, with links and licences, is in [CREDITS.md](CREDITS.md).

## Licence

Dual licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your
option.
