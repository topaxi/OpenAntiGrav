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

## Contributing

`cargo build`, and `just` is the gate every change must pass. Start at
[`docs/overview/developing.md`](docs/overview/developing.md): the tools, a first
session, the crate map and how work in flight is tracked.

## Documentation

Every discovery is written down with its evidence and a confidence score, so the
engine can be understood without reopening Ghidra. Start at
[`docs/README.md`](docs/README.md).

## Licence

Dual licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your
option.
