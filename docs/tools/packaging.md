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
just appimage --skip-build          # pack whatever is in target/release
just appimage --out /tmp/oag.AppImage
```

**For a Steam Deck, use `just appimage-portable`.** A native build links against
whatever glibc this machine has, and refuses to start on anything older; the
portable one is built in a container and loads on glibc 2.34 or newer. That is
[the glibc floor](#glibc), the one real portability constraint here, and the only
reason there are two recipes rather than one.

The work happens in [`scripts/build-appimage.sh`](../../scripts/build-appimage.sh),
which assembles an AppDir from [`packaging/appimage/`](../../packaging/appimage/)
and hands it to `appimagetool`. `appimagetool` is downloaded into `data/tools/`
on first use, next to the Ghidra extensions - gitignored, like everything else
under `data/`. Nothing binary is committed: even the icon is generated, by
[`scripts/appimage-icon.py`](../../scripts/appimage-icon.py).

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

**What is still unverified is the Deck itself.** The SteamOS glibc version was
never established - SteamOS is a rolling Arch snapshot, the only figure findable
was 2.33 from a 2022 discussion, and no hardware was available to check. A 2.34
floor clears any SteamOS 3.x that has seen an update, but the number gets
*confirmed* by step 4 of
[running it on a Steam Deck](#running-it-on-a-steam-deck): run it from a
terminal, and either it starts or it names the version it wanted.

## Where the disc image comes from

The AppImage ships no image, so finding the player's own is the one thing
packaging has to answer. [`crates/game/src/source.rs`](../../crates/game/src/source.rs)
is the answer, and the first hit wins:

| Order | Where | For |
| --- | --- | --- |
| 1 | the command-line argument, verbatim | `oag-game ~/roms/pulse.chd`, and everything scripted |
| 2 | `$OAG_IMAGE` (a file or a directory) | a fixed location, set once in a launcher |
| 3 | `data/images/` under the current directory | the repository checkout, which is what `just play` uses |
| 4 | the AppImage's own directory, then `images/` in it | **portable mode**: copy the AppImage and the image into one folder |
| 5 | `~/.local/share/oag/images/` | a stable per-user location |

Within a directory, `pulse-psp-usa.chd` and `pulse-ps2-eu.chd` are tried by name
first (the normalised names from [`data/README.md`](../../data/README.md)), then
any `.chd` or `.iso` in alphabetical order - so an image under whatever name the
player's own dump has still works, and two runs in the same directory always
open the same one.

Portable mode reads **`$APPIMAGE`'s directory**, the AppImage file's own
location. `$APPDIR`, the mounted read-only package, is deliberately never
searched: looking inside the package would invite someone to bundle an image
into it, which is the one thing that must never happen. A unit test asserts the
search path contains neither.

When nothing is found, the error lists every directory that was tried. For a
"copy two files into a folder" workflow, that message is the entire interface.

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

Not yet verified on real hardware. These are the steps to try, in Desktop Mode:

1. Copy two files into one folder, e.g. `~/Games/OpenAntiGrav/`:
   `OpenAntiGrav-x86_64.AppImage`, and your own disc image (named
   `pulse-psp-usa.chd`, or anything ending `.chd`/`.iso`).
2. `chmod +x OpenAntiGrav-x86_64.AppImage`, which `scp` does not preserve.
3. Run it from a terminal the first time, so its output is visible:
   `./OpenAntiGrav-x86_64.AppImage`. Expect the archive report, then the intro.
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
