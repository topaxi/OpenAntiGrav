# Packaging: the AppImage, and the Steam Deck

`just appimage` produces one file, `data/appimage/OpenAntiGrav-x86_64.AppImage`,
that runs the engine on any x86_64 Linux with a Vulkan driver - a Steam Deck
included. It contains the engine and nothing else: **no game content is ever
packaged** (see [legal](../overview/legal.md)), so the player supplies their own
disc image and the game looks for it beside the AppImage.

"A Vulkan driver" need not be a hardware one: a machine with Mesa's lavapipe
installed (`vulkan-swrast` on Arch, `mesa-vulkan-drivers` on Debian) presents a
CPU adapter that the OPTIONS -> GRAPHICS -> RENDERER row offers like any other,
marked `(cpu)`. That is a diagnostic path rather than a way to play - pair it
with RENDER SCALE 50 - but it is the difference between the AppImage starting and
not on a machine with no working GPU driver. See
[menus](../architecture/menus.md).

```sh
just appimage                       # release build, then pack
just appimage-portable              # the same, built against an older glibc
just appimage-deck                  # portable, compiled for the Steam Deck's CPU
just appimage --skip-build          # pack whatever is in target/release
just appimage --out /tmp/oag.AppImage
```

**For a Steam Deck, use `just appimage-deck`.** A native build links against
whatever glibc this machine has, and refuses to start on anything older; the
portable one is built in a container and loads on glibc 2.34 or newer, and the
Deck's is that same container build compiled for the Deck's Zen 2 core
(`-C target-cpu=znver2`, see [A CPU-tier build](#a-cpu-tier-build)), written as
`OpenAntiGrav-x86_64-steamdeck.AppImage`. `just appimage-portable` stays the
baseline x86-64 build, for an older Linux machine that may not have AVX2. That is
[the glibc floor](#glibc), the one real portability constraint here, and the only
reason there are two recipes rather than one.

[Releases and CI](releases.md) is what builds these in GitHub Actions and names the downloads.

The work happens in [`scripts/build-appimage.sh`](../../scripts/build-appimage.sh),
which assembles an AppDir from [`packaging/appimage/`](../../packaging/appimage/)
and hands it to `appimagetool`. `appimagetool` is downloaded into `data/tools/`
on first use, next to the Ghidra extensions - gitignored, like everything else
under `data/`. Nothing binary is committed: even the icon is generated, by the
freshly built binary's own `--write-icon`, which rasterises
[`assets/icons/64x64.svg`](../../assets/icons/64x64.svg) through `oag_game::icon`
- the same code path `main/window.rs` uses for the live window/taskbar icon, so
the two can never draw two different pictures.

## A CPU-tier build

`just build-cpu [cpu]` builds a release `oag-game` with `-C target-cpu=<cpu>`
into `target/cpu-<cpu>/`, leaving the baseline build alone. The default,
`x86-64-v3`, assumes AVX2, FMA and BMI2 - every Intel since Haswell and every
Zen - and so every Steam Deck; `znver2` is the Deck's own core and adds only
its scheduling model. `just appimage-deck` ships the `znver2` one, through
`scripts/build-appimage.sh --target-cpu`; `just appimage` and
`just appimage-portable` stay baseline, which starts on anything, where a tier
binary refuses to start on a CPU without the instructions. (Checked on the
Deck AppImage's binary: 128,224 instructions touch a `ymm` register against
1,223 in the baseline build, and BMI2's `shlx`/`sarx`/`shrx` appear 3,392 times
against 13.)

Measured 2026-10-03 on a Ryzen 9 7900, against the baseline build of the same
commit:

| | baseline | `x86-64-v3` | `znver2` |
| --- | --- | --- | --- |
| `Scene::render` CPU, Pulse, rocket volley (`OAG_RENDER_BENCH`), median of three | 604 us | 585 us | 566 us |
| load plus 900 ticks, HD | 2.96 s | 2.97 s | - |

**The simulation is unaffected, and that was checked rather than assumed.**
Rust never contracts `a * b + c` into a fused multiply-add on its own and
never reassociates float arithmetic, so the instructions a tier adds change
how wide integer and copy loops run, not what a float sum comes to. All four
determinism suites (`oag-core`, `oag-physics`, `oag-gameplay`, `oag-ai`) pass
under both tiers against the committed references, and ten race captures
across Pulse PSP, Pulse PS2 and HD are byte-identical to the baseline build's.
A future `mul_add` or algebraic float op in a gameplay crate would break that,
which [determinism](../architecture/determinism.md) already forbids there.

The gain is small because the CPU is rarely the limit: on an integrated GPU
the frame is GPU-bound twenty times over.

## Why AppImage, and not Flatpak

Decided 2026-07. **AppImage now, Flatpak later if the project ever wants to be
installed rather than tried.**

The Deck's SteamOS has an immutable root filesystem: `pacman` is off the table
without unlocking it, and anything installed that way is wiped by the next OS
update. Both AppImage and Flatpak survive that. What separates them is the
feedback cycle, which at this stage of the project is the whole point:

| | AppImage | Flatpak |
| --- | --- | --- |
| Getting a new build onto the Deck | copy one file | build, sign, serve or `flatpak-builder` on the device |
| Prerequisites on the Deck | none | a runtime (`org.freedesktop.Platform`) per SDK version |
| Iterating twenty times an afternoon | copy, run | rebuild the bundle each time |
| Uninstalling | delete the file | `flatpak uninstall` |
| Store-friendly, sandboxed, updatable | no | yes |

The last row is the whole case for Flatpak, and it is a case about
*distribution*, which this project is nowhere near. Until then, a single
executable file that needs nothing installed wins.

What would flip the decision: wanting sandboxing, wanting Flathub, or wanting
delta updates. None of that changes anything in the engine - the AppDir already
separates the binary from its launcher - so it stays a packaging decision rather
than an architectural one, and no ADR is warranted yet.

## What is bundled

Nothing. That is not laziness; it is the AppImage convention followed
deliberately, and it is possible because the engine's dependency surface is
almost entirely static Rust:

```
$ ldd target/release/oag-game
    linux-vdso.so.1
    libudev.so.1 => /usr/lib/libudev.so.1
    libpipewire-0.3.so.0 => /usr/lib/libpipewire-0.3.so.0
    libasound.so.2 => /usr/lib/libasound.so.2
    libgcc_s.so.1 => /usr/lib/libgcc_s.so.1
    libm.so.6 => /usr/lib/libm.so.6
    libc.so.6 => /usr/lib/libc.so.6
```

- **`libc`, `libm`, `libgcc_s`** are the base system. Bundling a C library and
  then loading the host's driver stack against it is the classic way to break an
  AppImage.
- **`libudev.so.1`** is the host's own device manager, which is what
  `gilrs` reads gamepads through. It talks to the running `systemd-udevd` and
  must be the host's copy; SteamOS has it, as does every distribution that boots
  with systemd. It is on the AppImage excludelist for exactly this reason.
- **`libasound.so.2`** is what `cpal` (`oag-audio`'s output device) plays sound
  through. Host-owned for the same reason as `libudev`: it is where
  PulseAudio/PipeWire's own ALSA plugin lives, so a bundled copy would bypass
  whatever the host's audio server actually routes through, and it is on the
  AppImage excludelist alongside it. Needs `libasound2-dev` at *build* time
  only, for its headers and `alsa.pc` - `packaging/appimage/Containerfile`
  installs it for the `--container` build the same way it installs
  `libudev-dev`.
- **`libpipewire-0.3.so.0`** is the PipeWire client library `cpal`'s PipeWire
  host talks to the running daemon through (see `crates/audio/Cargo.toml` for
  why that host is enabled). Host-owned for the same reason again: the client
  library has to match the daemon it connects to. Unlike the two above, **this
  one is a hard link-time dependency rather than a `dlopen`**, so the AppImage
  does not start at all on a machine with no `libpipewire-0.3.so.0` - the
  fallback to ALSA happens inside `cpal` at *device* selection, well after the
  loader has already needed the library. SteamOS has PipeWire, as does every
  current desktop distribution. Build time needs `libpipewire-0.3-dev`, and -
  uniquely in this workspace - `libclang-dev` too, because `libspa-sys`
  generates its bindings with bindgen. Bookworm's PipeWire is 0.3.65 against
  the `0.3` the sys crates ask for, so the container's frozen version is new
  enough.
- **Vulkan and the windowing libraries are not linked at all.** `wgpu` loads
  `libvulkan.so.1` and `winit` loads `libwayland-client`/`libX11` with `dlopen`
  at runtime, so they come from the host - which is required, not merely
  convenient: the Vulkan loader has to find the host's ICD (RADV on the Deck)
  and a bundled loader would not.
- **`ffmpeg` is a process, not a library.** The first run transcodes the intro
  through it (see [ADR-0008](../architecture/adr/0008-av1-movie-cache.md)); a
  machine without `ffmpeg` still boots, plays and races, just without a picture
  during the intro. `--no-video` skips the transcode outright.

The AppImage runtime itself is statically linked - `ldd` on the produced file
reports "not a dynamic executable" - so **`libfuse2` is not needed**, which used
to be the standard reason an AppImage would not start on SteamOS. It does need
`/dev/fuse`, which SteamOS provides. If a machine ever refuses to mount it,
`APPIMAGE_EXTRACT_AND_RUN=1 ./OpenAntiGrav-x86_64.AppImage` unpacks to a
temporary directory instead and needs no FUSE at all.

## <a id="glibc"></a>The glibc floor

**The one real portability constraint.** A dynamically linked binary records the
symbol *versions* it was linked against, and the linker always binds to the
newest the build host offers. Building on Arch with glibc 2.44 produced
references to `GLIBC_2.44` (`cosh`, `sinh`) and `GLIBC_2.43` (`acosf`, `asinf`,
`atan2f`, ...) - all libm, none of them anything the code asks for by name. On a
machine with an older glibc the loader refuses the binary outright:

```
./OpenAntiGrav-x86_64.AppImage: /usr/lib/libm.so.6: version `GLIBC_2.44' not found
```

`just appimage` therefore prints the floor it produced, computed from the binary
itself, and says so loudly when it is above the portable baseline:

```sh
objdump -T target/release/oag-game | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1
```

Check the target's, on the Deck itself, and compare:

```sh
ldd --version | head -1
```

### The fix: build against an older glibc

`just appimage-portable` does exactly that, and it is **verified, not
suggested**: `scripts/build-appimage.sh --container` builds the release binary
inside Debian bookworm - glibc 2.36 - from
[`packaging/appimage/Containerfile`](../../packaging/appimage/Containerfile),
then packages it the same way. The AppImage convention is to build on the oldest
base you intend to support, and this is that.

```sh
just appimage-portable              # podman or docker; the image is built once
CONTAINER_ENGINE=docker just appimage-portable
```

Measured on 2026-07, building the same commit both ways:

| Build | Newest GLIBC symbol referenced | Runs on |
| --- | --- | --- |
| native (Arch, glibc 2.44) | `GLIBC_2.44` | glibc 2.44 or newer only |
| `--container` (bookworm, glibc 2.36) | **`GLIBC_2.34`** | glibc 2.34 or newer |

2.34, not 2.36: bookworm's libm does not *have* the newer symbol versions, so the
linker binds to the oldest ones that satisfy each reference, and 2.34 is where
glibc last versioned the maths functions this code reaches. glibc 2.34 is
August 2021, which is older than the Steam Deck itself.

How that was checked, because a different glibc can mean a different *libm
implementation* and every symbol that forced the 2.44 floor was a transcendental
(`cosh`, `sinh`, `acosf`, `asinf`, `atan2f`) - which the simulation does reach:

- the floor itself, by both `objdump -T` and `readelf -V` on the produced binary;
- **the determinism test, run inside the container against the bookworm build**:
  `determinism_matches_the_committed_reference` passes, so the state hashes are
  the ones committed in `crates/core/src/hash.rs` and the two builds are
  interchangeable for the simulation, not merely both runnable. This is the check
  that matters; a matching telemetry line would only have agreed to two decimals.

  ```sh
  podman run --rm -v "$PWD:/src" -w /src \
      -e CARGO_HOME=/src/data/appimage/container/cargo \
      -e CARGO_TARGET_DIR=/src/data/appimage/container/target \
      oag-appimage-build:bookworm \
      cargo test --release -p oag-core --test determinism
  ```

  (`cargo test`, not `cargo nextest`: the image has no nextest, and this is the
  one place in the project that runs the plain harness.)
- the packaged file itself, run out-of-repo through the same boot-to-race smoke
  test as the native one: it reached `Launch Game`, loaded track and ship, and
  wrote a rendered frame.

The container build is named `OpenAntiGrav-x86_64-portable.AppImage`, the native
one `OpenAntiGrav-x86_64.AppImage`, so a directory holding both says which is
which - the failure this whole section exists to prevent is a cryptic
`GLIBC_2.44' not found` on the Deck from having copied the wrong file. Both
recipes also print the floor next to the filename they produced.

The image is about 1 GB and takes a couple of minutes to build, once. The
container's registry and target directory are bind-mounted into
`data/appimage/container/`, deliberately separate from the host's `target/`: the
two builds use different libcs and would otherwise invalidate each other's
incremental state on every switch. Both are gitignored, and deleting them is
always safe.

Two alternatives, neither needed now: `cargo-zigbuild`
(`--target x86_64-unknown-linux-gnu.2.34`) does the same job without a
container, and pinning old symbol versions with `.symver` directives works but is
fragile and needs revisiting whenever glibc adds a version.

**The Deck itself is now established from Valve's own package mirror, though
still not confirmed on physical hardware.** SteamOS is built from
`steamdeck-packages.steamos.cloud`, a publicly browsable Arch mirror with one
repo set per SteamOS branch (`core-3.5`, `core-3.7`, `core-3.8`, ...). Its
`core-<branch>/os/x86_64/` listings give glibc's version directly:

| SteamOS branch | glibc |
| --- | --- |
| 3.5 | 2.37 |
| 3.7 | 2.40 |
| 3.8 (current stable, 2026-08) | 2.41 |
| 3.9 (preview) | 2.43 |

Every branch back to 3.5 clears the 2.34 floor with rising margin - this
supersedes the only figure found before (2.33, from a 2022 forum post, likely
SteamOS 3.0-3.3 era). The remaining gap is that this is the repo Decks update
*from*, not a read of an actual Deck's installed `libc.so.6` - a Deck that has
gone a long time without updating could in principle sit on an older branch
than `core-rel` serves today. That is still confirmed, if it ever happens, by
step 4 of [running it on a Steam Deck](#running-it-on-a-steam-deck): run it
from a terminal, and either it starts or it names the version it wanted.

## The window/taskbar icon

[`assets/icons/64x64.svg`](../../assets/icons/64x64.svg) is the one source for
every place this project's icon appears; `oag_game::icon` rasterises it and
nothing hand-redraws it a second time. Two consumers, one function:

- `main/window.rs`'s `window_icon` hands winit an `Icon` at window creation
  (`main/gpu.rs`), which reaches the titlebar, alt-tab and, **on X11 only**,
  the taskbar.
- `oag-game --write-icon FILE --icon-size N` rasterises the same SVG to a
  PNG and exits, touching no disc and no window. `scripts/build-appimage.sh`
  and `scripts/install-desktop-file.sh` both call it.

**Wayland has no window-icon protocol**, so `winit`'s Wayland backend
implements `set_window_icon` as a no-op (checked directly against its
source) - a taskbar under niri, or any other Wayland compositor, never sees
the call above at all. What it reads instead is the window's app_id
(`main/gpu.rs` sets it to `oag-game` via `with_name`, the same field X11's
`WM_CLASS` uses) looked up against an **installed** `.desktop` entry, whose
`Icon=` key is then resolved through the user's icon theme. A packaged
AppImage installs that entry as part of assembling the AppDir (see
[what is bundled](#what-is-bundled)); `cargo run`/`just play` out of a
checkout installs nothing, so a Wayland taskbar has nothing to resolve the
running game's app_id against until you run:

```sh
just install-desktop-file
```

which builds a release binary if one is not already there, writes
`oag-game.png` at 64px and 256px into
`~/.local/share/icons/hicolor/*/apps/`, and writes
`~/.local/share/applications/oag-game.desktop` with `Exec=` pointing at the
absolute path of the binary it just built (there is no `oag-game` on `PATH`
outside an AppImage's own mount). Safe to re-run after every rebuild; safe to
remove by deleting the files it prints at the end.

## Where the disc image comes from

The AppImage ships no image, so finding the player's own is the one thing
packaging has to answer. [`crates/source/src/source.rs`](../../crates/source/src/source.rs)
is the answer, and the first hit wins:

| Order | Where | For |
| --- | --- | --- |
| 1 | the command-line argument, verbatim | `oag-game ~/roms/pulse.chd`, and everything scripted |
| 2 | `$OAG_IMAGE` (a file or a directory) | a fixed location, set once in a launcher script |
| 3 | `settings.toml`'s `[source] image` | a player who always plays off one disc, saved between runs |
| 4 | `data/images/` under the current directory | the repository checkout, which is what `just play` uses |
| 5 | the AppImage's own directory, then `images/` in it | **portable mode**: copy the AppImage and the image into one folder |
| 6 | `~/.local/share/oag/images/` | a stable per-user location |

The first three are a player *stating* which source they want and the last
three are the engine guessing, and that line matters: **a guess with more than
one answer is shown rather than resolved.** Steps 4 to 6 holding several images
opens the [disc chooser](oag-game.md#finding-the-disc-image) instead of taking
the first; a stated source never does.

Within a directory, the five normalised names from
[`data/README.md`](../../data/README.md) are tried first, in this order:
`pulse-psp-usa.chd`, `pulse-ps2-eu.chd`, `pure-psp-eu.chd`, `pure-psp-usa.chd`,
then `hdfury-ps3-eu.iso`. Pulse's own two names come first because that is the
platform the implementation follows; the others are recognised so a directory
holding only one of them is found by name rather than by alphabetical luck.
Pure and HD/Fury both boot their own front ends - HD's off a chain its XML
declares rather than one anyone has watched, which every boot of it says out
loud ([ADR-0025](../architecture/adr/0025-a-boot-chain-carries-its-provenance.md))
- though neither is played past the menus yet. After the known names, any
`.chd` or `.iso` in alphabetical order is tried - so an image under whatever
name the player's own dump has still works, and two runs in the same directory
always list them in the same order.

Portable mode reads **`$APPIMAGE`'s directory**, the AppImage file's own
location. `$APPDIR`, the mounted read-only package, is deliberately never
searched: looking inside the package would invite someone to bundle an image
into it, which is the one thing that must never happen. A unit test asserts the
search path contains neither.

When nothing is found, the error lists every directory that was tried. For a
"copy two files into a folder" workflow, that message is the entire interface.

Wipeout 2048 ships as an extracted Vita package (a directory, not a disc
image - see [`data/README.md`](../../data/README.md)), so it is not one of the
five names above and the [disc chooser](oag-game.md#finding-the-disc-image)
finds it through a parallel search, `package_search_path()`, over the same
guessed locations as steps 4-6 above with `extracted/vita` standing in for
`images`: `data/extracted/vita` under the current directory; beside the
AppImage, both the directory itself (a package dropped straight in) and its
own `extracted/vita` subdirectory; and `~/.local/share/oag/extracted/vita`.
[`scripts/push-game-data.sh`](../../scripts/push-game-data.sh) (`just push-data deck`)
copies a checkout's `data/extracted/vita` packages to the last of those on the
remote, so a Deck finds them the same way it finds `data/images`.

Wipeout Pure's PSN DLC packs need a key table to decrypt (see
[ADR-0033](../architecture/adr/0033-external-key-material-for-decryption.md)),
which is never in this repository and follows its own three-candidate search,
`default_pure_dlc_keys_path()` in
[`crates/source/src/dlc.rs`](../../crates/source/src/dlc.rs): a `keys/`
directory beside whichever of `dlc_search_path()`'s locations holds the
candidate - `data/keys/pure-dlc-keys.txt` under the current directory, beside
the AppImage, or `~/.local/share/oag/keys/pure-dlc-keys.txt`. A missing file
is not an error; Pure's packs are found and simply fail to decrypt, reported
the same way an unrelated zip is. `scripts/push-game-data.sh` offers the one
file (not the whole `data/keys/` directory, which also holds the unrelated
Vita zRIF table) and copies it to the last of those three on the remote.

The movie cache follows the same reasoning: `data/cache/movies` in a checkout
(recognised by a `data/` directory existing), and `~/.cache/oag/movies`
otherwise, rather than scattering a cache through whatever folder the AppImage
was run from. Deleting either is always safe.

## Gamepad

Hardcoded, on purpose, and living next to the keyboard mapping in
[`crates/input/src/pad.rs`](../../crates/input/src/pad.rs): same abstract button
layer, no remapping UI, no Steam Input integration (that stays planning, see
[modern features](../overview/modern-features.md#steam-input)). Steam Input needs
no integration to work anyway - it presents the user's configuration as an
ordinary gamepad, which is what this reads.

| Pad (Deck / Xbox layout) | Abstract button or axis | Notes |
| --- | --- | --- |
| Left stick | `stick_x`, `stick_y` | Analog, 15% deadzone rescaled so full travel still reaches 1 |
| D-pad | `up` / `down` / `left` / `right` | Also drives the axes, digitally, the way the keyboard does |
| A (South) | `cross` | Thrust and "activate", the original's own convention |
| B (East) | `circle` | "Cancel" |
| X (West) | `square` | |
| Y (North) | `triangle` | |
| L1 | `l`, `airbrake_left` = 1 | The left airbrake |
| R1 | `r`, `airbrake_right` = 1 | The right airbrake |
| **R2** | `cross`, past 25% travel | **Thrust.** Interpretation, not a binding: the snapshot has no analog throttle, because the original's thrust is the cross *button* |
| **L2** | `airbrake_left` **and** `airbrake_right`, analog | **"Brake".** Interpretation: the original's action set has no brake, and both airbrakes is the closest an anti-gravity ship gets. Deliberately sets no button bit, which would quantise the analog value to 0 or 1 |
| Start | `start` | Skips the intro |
| Select / Back | `select` | |

Right stick, L3/R3, the Mode button and the Deck's back paddles are unbound.

Two structural points, because both are easy to get wrong later:

- **All devices merge into one button state.** `oag_input::Controls` owns the
  single `Input`, and the keyboard's and the pad's held masks are OR'd together
  *before* the edges are computed. A press seen on two devices is one press, and
  `Input::consume_press` - which the front end needs, so one START does not both
  skip the intro and pick a language - has exactly one place to clear it.
- **The startup line lists what `gilrs` calls a gamepad**, which is not always
  one: a keyboard's HID system-control interface shows up as
  `Keychron K2 Pro System Control` on the development machine. Harmless - an
  unmapped device reports none of the buttons this polls, so it contributes
  nothing - but it is why that line is not proof a real pad was found.
- **A machine with no pad is not an error.** `gilrs` failing to open prints one
  line and the session carries on keyboard-only, which is what every headless
  capture and every CI run is. The mapping itself is free functions over plain
  values (`pad::map_button`, `pad::resolve`), so `just test` covers every case
  with no device attached.

## Running it on a Steam Deck

`just deploy-deck` (`scripts/deploy-to-deck.sh`) builds
`just appimage-portable` and rsyncs (falling back to `scp` if the remote has no
rsync) the AppImage onto the Deck's `~/Desktop`. It copies no game data.

`just push-data deck` (`scripts/push-game-data.sh`) does the data, one row per
thing in an fzf multiselect: each disc image under `data/images/`, each DLC zip
under `data/dlc/`, each unpacked 2048 package under `data/extracted/vita/`, each
Omega package under `data/extracted/ps4/` (its `.psarc` archives only - the nine
`oag_omega` mounts, not `eboot.bin`, `sce_sys/`, `sce_module/` or the disc maps;
the base extract alone is about 41 GB), and `data/keys/pure-dlc-keys.txt`. Each
row is marked `on device`, `partial` (some files missing or a different size) or
`-`, from one `find`/`stat` of the remote. Selected rows go to
`<XDG_DATA_HOME>/oag/{images,dlc,extracted/vita,extracted/ps4,keys}` there - the
same places [`crates/source/src/source.rs`](../../crates/source/src/source.rs)
and [`crates/source/src/dlc.rs`](../../crates/source/src/dlc.rs) already search,
so nothing needs setting on the Deck side - and only missing or changed files are
copied. Nothing on the device is deleted. `just push-data android` is the same
picker for a phone ([android.md](android.md)).

Only what `oag-game` reads is offered, because a Deck's disk is small. Raw
packages (`*.pkg`, and their `*.sha256`) are never offered, since the game reads
the extracts and never a package; nor are the encrypted `hdfury-ps3-eu.iso` and
its `.dkey`. A Deck that has none of Omega's archives does not list Omega in the
launcher: `ps4_search_path()` in `crates/source/src/source.rs` offers it only
when `extracted/ps4/omega-eu-patch/uroot/data09.psarc` exists.

```sh
just deploy-deck                    # deck@steamdeck, or $OAG_DECK_HOST
just deploy-deck user@host          # a different target
just deploy-deck --dry-run          # show the plan, change nothing
just deploy-deck --skip-build       # sync an AppImage already built
just push-data deck                 # pick game data, see what is there
just push-data deck user@host 'images/pulse-*' 'dlc/*'   # no picker
just push-data deck --dry-run       # pick, then only list what would go
```

Deployment itself is not yet verified on real hardware - `--dry-run` first is
worth it the first time against a new host. If something goes wrong, these are
the same steps by hand, in Desktop Mode:

1. Copy two files into one folder, e.g. `~/Games/OpenAntiGrav/`:
   `OpenAntiGrav-x86_64-portable.AppImage`, and your own disc image (named
   `pulse-psp-usa.chd`, or anything ending `.chd`/`.iso`).
2. `chmod +x OpenAntiGrav-x86_64-portable.AppImage`, which plain `scp` does not
   preserve (`just deploy-deck` does this explicitly either way).
3. Run it from a terminal the first time, so its output is visible:
   `./OpenAntiGrav-x86_64-portable.AppImage`. Expect the archive report, then
   the intro.
4. If it exits with a `GLIBC_...' not found` message, the AppImage was built
   natively: rebuild with `just appimage-portable` and copy that one over. See
   [the glibc floor](#glibc). Whatever version the message names is the Deck's
   own glibc, so please record it - it is the number that section is missing.
5. If the intro has no picture, SteamOS has no `ffmpeg` on `$PATH`; everything
   else still works, and `--no-video` skips the transcode attempt.
6. `--race` goes straight to a ship on a track, which is the fastest way to test
   that the pad is being read at all.

To launch it from Game Mode, add it through Steam's "Add a Non-Steam Game". That
route has not been tried either, and Steam Input will present its own
configuration as an ordinary gamepad rather than needing anything from the
engine.

## AUR packages

Two PKGBUILDs live in [`packaging/aur/`](../../packaging/aur/), each with its
generated `.SRCINFO`. Both install only the engine (`/usr/bin/oag-game`, the
desktop entry, the generated icon, both licences and `licences/`); no game
content, and both `provide`/`conflict` with `openantigrav`.

| Package | Builds | Notes |
| --- | --- | --- |
| `openantigrav-bin` | nothing: repackages `OpenAntiGrav-<pkgver>-linux-x86_64.tar.gz` from the GitHub Release ([releases](releases.md)) | `pkgver` is the tag without `v`. `sha256sums` is `SKIP` until a release exists: copy the tarball's line from the release's `SHA256SUMS` or run `updpkgsums`. |
| `openantigrav-git` | `cargo build --frozen --release -p oag-game` from `main` | `pkgver()` is `git describe --long --tags --match 'v*'`, falling back to `r<count>.<hash>` while no tag exists. `.SRCINFO` carries a placeholder `pkgver`; regenerate it. |

**Why distro `cargo`, not rustup.** `rust-toolchain.toml` pins 1.99.0 for CI
reproducibility, but a package builds with the packaged toolchain and Arch's
`rust` is newer than the 1.97.1 MSRV (`Cargo.toml` `rust-version`). `rustup`
conflicts with `rust` in a chroot, so `makedepends` names `cargo`;
`RUSTUP_TOOLCHAIN=stable` keeps a build on a machine that has rustup working.
`clang` is for `libspa-sys`' bindgen. `options=(!lto)` is required: with makepkg's default `-flto` the link fails with `undefined symbol: spa_format_parse_libspa_rs`, because `libspa-sys`'s C shim becomes LTO bitcode that `rust-lld` cannot link (observed 2026-10-07). `libpipewire` is a hard dependency (see [what is bundled](#what-is-bundled)); the
Vulkan driver is optional, since lavapipe gives a CPU adapter.

**Verified 2026-10-07** (local, nothing published): `openantigrav-git` built with
`makepkg` in a clean directory (`devtools` is not installed here, so no chroot
build; the build ran against this machine's rustup 1.99.0 `stable`, not Arch's
`rust`). `openantigrav-bin` built against a tarball cut with the same
`scripts/build-appimage.sh --container` binary and staging commands `release.yml`
uses. `namcap` on both PKGBUILDs and packages is clean apart from advisory
warnings (see the lane report). Both packages unpacked into a scratch root and
ran `oag-game --dry-run` against a Pulse image. `pacman -U` was not run.

### Publishing (the maintainer's steps)

Nothing here is automated, and nobody but the maintainer pushes to the AUR.

1. After the draft Release is published, in `openantigrav-bin/`: set `pkgver`,
   `updpkgsums`, `makepkg --printsrcinfo > .SRCINFO`, test with
   `makepkg -f` (or `extra-x86_64-build` from `devtools`).
2. `git clone ssh://aur@aur.archlinux.org/openantigrav-bin.git aur-bin`, copy
   `PKGBUILD` and `.SRCINFO` in, `git add`, `git commit`, `git push`.
3. For `openantigrav-git`, the same with `ssh://aur@aur.archlinux.org/openantigrav-git.git`;
   run `makepkg --printsrcinfo > .SRCINFO` after a build so `pkgver` is current.
   It needs no update per release.
