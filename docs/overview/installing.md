# Installing and running OpenAntiGrav with your own game files

This page is for someone who wants to play, not to contribute. It was written
by following it literally on 2026-10-07 on Linux, with a release build of the
commit it ships in. Every command and every message quoted here was run.

**What is verified and what is not.**

- Linux is the only OS this was tried on. Windows and macOS are goals
  ([goals](goals.md)) but nothing here was run on them.
- The build was done on a machine that already had the system libraries. The
  package names below come from the project's own AppImage build recipe
  (`packaging/appimage/Containerfile`) and were not installed on a clean
  machine.
- Nothing here was listened to. Every run used `--no-audio` and read the
  picture and the log only.

**No game content ships with this project, and none ever will.** You supply
files from discs and downloads you own. This page does not say where to get
them and does not cover keys. See [legal](legal.md).

## 1. Build it

You need a Rust toolchain. The repository pins one in `rust-toolchain.toml`
(1.99.0 on 2026-10-07), and `rustup` fetches it on the first build by itself.

```sh
git clone <this repo> && cd OpenAntiGrav
cargo build --release -p oag-game
```

The binary is `target/release/oag-game`. A cold build takes several minutes.

System libraries the build and the program need:

| What | Debian / Ubuntu | Arch |
| --- | --- | --- |
| C toolchain, `pkg-config` | `build-essential pkg-config` | `base-devel` |
| Gamepads | `libudev-dev` | `systemd-libs` |
| Sound | `libasound2-dev` | `alsa-lib` |
| Sound server client | `libpipewire-0.3-dev`, `libclang-dev` | `pipewire`, `clang` |
| Graphics | a Vulkan driver, for example `mesa-vulkan-drivers` | `vulkan-radeon`, `vulkan-intel` or `nvidia-utils` |

`libpipewire-0.3.so` is a hard link-time dependency: the program does not
start at all without it ([packaging](../tools/packaging.md#what-is-bundled)).
Vulkan, Wayland and X11 are loaded at run time, so a missing Vulkan driver
shows up when the window opens, not at build time.

Optional:

- **`ffmpeg`** converts each game's intro movie the first time you boot a
  title. Without it the intro plays as a black screen with the text
  `INTRO FRAME n / N (NO PICTURE)` and everything else works. With it the
  first Pulse boot spends about 80 seconds converting, once, and caches the
  result.
- `just` runs the repository's recipes (`just play ...`). Not needed to play.
  If you use it, `just play` also turns on the `native-video` feature, which
  needs GStreamer development libraries. The plain `cargo` line above does
  not.

You do not need `cargo-nextest`, Ghidra or any emulator to play.

## 2. What each title needs

Put your files in `data/images/` next to where you run the program. The names
below are the ones the program recognises first. Any other `.chd` or `.iso` in
that folder is also found, in alphabetical order (checked with a Pulse image
named `wipeout.chd`).

| Title | Platform | What to supply | File or folder name |
| --- | --- | --- | --- |
| Wipeout Pulse | PSP | disc image, `.chd` or `.iso`, USA or Europe | `pulse-psp-usa.chd`, `pulse-psp-eu.chd` |
| Wipeout Pulse | PS2 | disc image, Europe | `pulse-ps2-eu.chd` |
| Wipeout Pure | PSP | disc image, USA or Europe | `pure-psp-usa.chd`, `pure-psp-eu.chd` |
| Wipeout HD / Fury | PS3 | **decrypted** disc image, Europe | `hdfury-ps3-eu-dec.iso` |
| Wipeout 2048 | Vita | **unpacked** package folder | `data/extracted/vita/PCSF00007` (Europe) or `PCSA00015` (USA) |
| Omega Collection | PS4 | **unpacked** base and patch folders | `data/extracted/ps4/omega-eu` and `omega-eu-patch` |

The first three rows are the easy ones: a normal dump of your own disc is
read as it is, CHD (single-track, either `createdvd` or `createcd`) or ISO.

### HD / Fury: the disc must be decrypted first

A PS3 disc dump is encrypted. The program opens it and fails with a message
that names Pulse's files instead of saying so:

```
Error: opening the archives in data/images/hdfury-ps3-eu.iso

Caused by:
    data/images/hdfury-ps3-eu.iso (PS3) holds none of: PSP_GAME/USRDIR/Data.wad, WADS2.WAD, PSP_GAME/USRDIR/FE.wad, WADSP.WAD
```

That is the "this disc is still encrypted" message. The project's own script
decrypts a dump given the disc key your own dump tool recorded for that disc:

```sh
python3 scripts/ps3iso.py decrypt data/images/hdfury-ps3-eu.iso \
    "$(cat data/images/hdfury-ps3-eu.dkey)" data/images/hdfury-ps3-eu-dec.iso
```

The script needs the Python package `cryptography` (`pip install
cryptography`), and it prints a Python traceback instead of usage text if you
give it too few arguments. Run on 2026-10-07 it took 7 seconds and wrote a
file byte-identical to the project's existing decrypted image, which the
program then opened as `Wipeout HD`. The format is described in
[PS3 disc encryption](../formats/ps3-disc.md). See
[troubleshooting](#5-when-it-goes-wrong) for what happens with both files in
one folder.

### 2048 and Omega: unpack the package into a folder

These two ship as console download packages, not discs. The program does not
read a `.pkg` file. It reads a folder you unpacked and decrypted from a
package you own. This project supplies no key and does not say where one
comes from; the license key for a package is part of your own copy.

The tools the project used and the exact steps are in
[`data/README.md`](../../data/README.md) (the Vita steps) and
[source-images](../reverse-engineering/source-images.md) (Omega). They were
**not re-run** for this page. The folders the program reads are:

```
data/extracted/vita/PCSF00007/        2048, Europe (PCSA00015 for USA)
    base/PSP2/data.psarc              required: this file is what marks a usable copy
    dlc1/PSP2/dlc1.psarc              DLC pack 1, mounted behind base/ when present
    dlc2/PSP2/dlc2.psarc              DLC pack 2, the same
    patch-v104/                       the 1.04 patch: NOT read, see below

data/extracted/ps4/
    omega-eu/uroot/data00..04.psarc   base package
    omega-eu-patch/uroot/data09.psarc the day-one patch: required
```

2048 starts with `base/` alone (checked: a folder holding only
`base/PSP2/data.psarc` opens as `Wipeout 2048`). The DLC packs are mounted
when their folders are there. The 1.04 patch's archives are deliberately not
mounted yet, because they would be searched behind the entries they replace
(`crates/2048/src/lib.rs`, `EXTRA_CANDIDATES`), so unpacking it changes
nothing today. A `base/` folder that is itself a symlink was not found, so use
real folders. Omega needs the patch: only the patch's
`data09.psarc` carries the front end, so a base-only folder is not offered.
Omega's layout was checked from one package form only, the one the project's
maintainer holds, so a differently packaged copy may unpack to a different
shape.

### Pulse downloadable packs

Pulse's Europe DLC zips can sit in `data/dlc/` and are mounted against
whichever Pulse disc is open, region does not matter
([DLC packs](../formats/dlc-pack.md)). The dry run above did not print a line
that proves a pack was mounted, so treat this as not re-verified here.

## 3. Run it and check it was found

From the folder that holds `data/`:

```sh
target/release/oag-game                    # a disc image found in data/images
target/release/oag-game data/images/pulse-psp-eu.chd
target/release/oag-game data/extracted/vita/PCSF00007
target/release/oag-game data/extracted/ps4
```

Name the source for 2048 and Omega. `--dry-run`, `--screenshot` and `--race`
with nothing named look only for disc images, so on a folder-only setup they
end in the "no disc image found" error. A plain windowed `oag-game` with
nothing named, and `oag-game --launcher`, open a chooser that lists every
image and every unpacked folder found under `data/`, with the platform shown
as `unknown` for a folder; pick with the arrow keys and Enter. (Seen with
several folders present. A lone folder in a window was not re-run cleanly, so
name it to be sure.)

To check a setup without opening a window:

```sh
target/release/oag-game --dry-run --no-audio --no-video data/images/pulse-psp-eu.chd
```

Leave `--no-video` out and the first run also converts the intro movie, which
takes about 80 seconds once.

A good result is one line naming the title and the source:

```
[INFO ] Wipeout Pulse: data/images/pulse-psp-eu.chd
```

The other titles print `Wipeout Pure`, `Wipeout HD`, `Wipeout 2048` and
`Wipeout: Omega Collection` in the same place. To see the picture without a
window, write one frame to a file:

```sh
# the first menu the disc shows (Language Selection); the default 300 ticks only reaches the intro
target/release/oag-game --no-audio --no-video --screenshot /tmp/menu.png --until "Language Selection" --hold start data/images/pulse-psp-eu.chd
target/release/oag-game --no-audio --race --no-intro --screenshot /tmp/race.png --ticks 120 data/images/pulse-psp-eu.chd
```

Controls in a window: arrow keys or WASD steer, X or Return thrusts, Q and E
are the airbrakes, Escape goes back one level. A gamepad works with no setup
([packaging](../tools/packaging.md#gamepad)). Full list of flags:
`oag-game --help`; the page is [oag-game](../tools/oag-game.md). Note that
`--help` still opens with "Run Wipeout Pulse from a disc image" whatever title
you give it.

### Where things are kept

| What | Where on Linux |
| --- | --- |
| Settings | `$XDG_CONFIG_HOME/oag/settings.toml`, usually `~/.config/oag/settings.toml` |
| Records, progress, ghosts | `~/.config/oag/` (`records.toml`, `ghosts/`). Saved by themselves, there are no save slots |
| Log file | `$XDG_STATE_HOME/oag/logs/oag-game.log`, usually `~/.local/state/oag/logs/oag-game.log`. Kept seven days |
| Converted movies and sound | `data/cache/` if a `data/` folder is next to where you run it, otherwise `~/.cache/oag/` |
| Images, if not in `data/images/` | `~/.local/share/oag/images/` |
| 2048 and Omega folders, if not in `data/extracted/` | `~/.local/share/oag/extracted/vita/` and `.../extracted/ps4/` |

Deleting the cache is always safe. Other ways to name a source, in the order
the program tries them: the command line, `$OAG_IMAGE`, then `[source] image`
in `settings.toml`, then the folders above
([packaging](../tools/packaging.md#where-the-disc-image-comes-from)).
`$OAG_IMAGE` takes a disc image or a folder of images; it **rejects** an
unpacked 2048 or Omega folder. Use the command line or `settings.toml` for
those.

`RUST_LOG=debug` raises the terminal's detail, and `--log-file FILE` moves the
log file (an empty value writes none).

## 4. What you can do today

One line each; the full per-subsystem picture is [status](status.md).

- **Pulse (PSP, PS2):** intro, menus and a race on the disc's own data, with
  an eight-craft grid and AI built (see status). The reference title. Physics is implemented from the
  original's code but not yet checked tick for tick.
- **Pure:** boots to its menus and a race loads on its own data.
- **HD / Fury:** boots, and a race loads on the disc's own circuits with
  textures; behaviour uses Pulse's rules where HD's were not measured.
- **2048:** boots, and a race loads on its own circuits and ships.
- **Omega:** its front end boots and a race loads on its own circuits; the
  zone mode, speed classes and boost are not measured, and sound is not read.

"A race loads" means the screenshot run above shows a ship on the circuit
with its HUD. How far each title plays beyond that is in [status](status.md);
do not expect a finished game outside Pulse.

## 5. When it goes wrong

| What you see | Cause and fix |
| --- | --- |
| `Error: no disc image found. OpenAntiGrav ships no game content ...` then a `Searched:` list | Nothing was in `data/images/` relative to where you ran it, or you ran it from another folder. Run from the folder that holds `data/`, or name the file. 2048 and Omega folders are never found this way: name them. The hint's advice to "put a pulse-psp-usa.chd beside the executable" only works for an AppImage; use `data/images/` |
| `Error: OAG_IMAGE is set to ..., which is not a disc image and holds none` | `$OAG_IMAGE` cannot name an unpacked 2048 or Omega folder, or points at nothing. Unset it and name the folder on the command line |
| `... (PS3) holds none of: PSP_GAME/USRDIR/Data.wad, WADS2.WAD, ...` | The PS3 image is still encrypted. Decrypt it as in section 2. With both `hdfury-ps3-eu.iso` and `hdfury-ps3-eu-dec.iso` in `data/images/`, the encrypted one is tried first and fails: name the decrypted one on the command line, or keep only that one |
| Black screen with `INTRO FRAME n / N (NO PICTURE)` | `ffmpeg` is missing or `--no-video` was given. Install `ffmpeg` and run once without the flag. The game is otherwise fine |
| Window opens black, or no window | Vulkan driver missing or broken. The terminal prints a `renderer: vulkan: ...` line naming the adapter. A CPU adapter such as `llvmpipe` works but is slow |
| Program does not start and the loader says it cannot open `libpipewire-0.3.so.0` (not run: needs a machine without it) | Install the PipeWire client library from the table in section 1 |
| `frame: 50 ms` warnings fill the terminal | You are on a CPU renderer. Lower RENDER SCALE in the options. The log file is quieter |

If something else fails, the log file named above has the detail. Include it
when you report a problem.
