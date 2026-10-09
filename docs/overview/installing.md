# Play OpenAntiGrav with your own game files

This page is for someone who wants to play, not to contribute. It has two
halves. **Steps 1 to 5 are the guide**: short steps, one thing each. **"For the
curious"** at the end keeps the technical detail: building from source, the
exact file layouts, every error message.

**You need your own copy of the game.** No game content ships with this
project, and none ever will. This page does not say where to get game files,
disc keys or licences. See [legal](legal.md).

**What was tried, and what was not** (checked 2026-10-07):

- **Linux**: every folder, file name and message below was run on this
  machine with the real binary, using `--dry-run` (it starts the game's data
  without opening a window) and a real window under a software-rendered X
  session. Nothing was listened to: every run used `--no-audio`.
- **Windows**: the folders come from the code and the Windows folder lookup
  library's source. The program itself ran under Wine
  ([Wine and Proton](../tools/wine.md)), **not on a real Windows PC**.
- **macOS**: there is **no download** for it. It builds from source on paper
  only; nobody has run it. The folder is read from the same library.
- **Steam Deck** and **Android**: the Deck steps use the Linux program; the
  Android steps are from [Android](../tools/android.md), tried on an emulator
  (Waydroid) and not on a phone yet.

## 1. Pick your computer and get the program

| You have | Download this from the project's GitHub Releases page | Then |
| --- | --- | --- |
| **Windows 10 or 11, 64-bit** | `OpenAntiGrav-<version>-windows-x86_64.zip` | Right-click the zip, "Extract All". Open the new folder. The program is `oag-game.exe`. |
| **Linux PC** | `OpenAntiGrav-<version>-linux-x86_64.AppImage` | Make it runnable: right-click, Properties, tick "Is executable" (or `chmod +x` the file). Needs a Vulkan graphics driver. No FUSE? Use the `.tar.gz` of the same name. |
| **Steam Deck** | `OpenAntiGrav-<version>-steamdeck-x86_64.AppImage` | Switch to Desktop Mode, save it in a folder, make it runnable as above. |
| **Android phone** (64-bit, Android 8 or newer) | `OpenAntiGrav-<version>-android-arm64.apk` | Copy it to the phone and open it (allow "install unknown apps" for your file manager), or `adb install -r` it. |
| **Mac** | nothing yet | There is no Mac download. A source build is possible ("Build it yourself" below) but has never been run. |
| **Arch Linux** | the AUR packages `openantigrav-bin` or `openantigrav-git` | Install like any AUR package, once they are published. |

**As of 2026-10-07 only a "nightly" pre-release is published** (a build made every
day from the latest code); a proper numbered release has not been published yet.
The file names below carry the nightly's date and commit instead of a version.
If the page has no download for your system, build from source ("Build it
yourself" below).

Optional, any desktop: install **`ffmpeg`**. Without it the game skips each
title's intro movie and goes straight to the menu. Everything else works.

## 2. Make the "images" folder and put your files in it

The game looks in a few folders for your files, and uses the first one that has
something in it. Pick **one** folder from your system's row and use it every
time. All paths are real: they are the ones in `crates/source/src/source.rs`.

### Windows

The easiest folder is **next to the program**: open the folder that holds
`oag-game.exe`, make a new folder called `images`, and put your files in it.
The path is `...\OpenAntiGrav-<version>-windows-x86_64\images\`. (Tried on
Linux with a copy of the program in its own folder.)

The other folder that always works, wherever the program is kept:

1. Press the Windows key and R together.
2. Type `%APPDATA%\oag\images` and press Enter. If Windows says the folder
   does not exist, type `%APPDATA%`, press Enter, make a folder `oag` in it,
   then a folder `images` in that, and put your files in it.

That is `C:\Users\<your name>\AppData\Roaming\oag\images`. Your **disc key**
folder, if a game needs one, is `%APPDATA%\oag\keys`.

**Three likely problems (Windows)**

1. *A "Windows protected your PC" box appears.* The program is not signed by a
   publisher, which is expected for a hobby project; this was not seen on a
   real PC. Choose "More info", then "Run anyway", only if you downloaded it
   from the project's own Releases page.
2. *A black text window opens beside the game.* Leave it open. It prints
   messages, and the project sends its log to a file as well. (Expected from
   how the program is built, a console program; not seen on a real PC.)
3. *A window says "NO DISC IMAGE FOUND".* Your files are not in a folder the
   window lists. It names the exact folder to use, written with `/` instead of
   `\` (the screen's letters have no backslash). Fix it, then start the game
   again.

### macOS

No download exists, so a Mac player must build from source ("Build it
yourself" in the curious part). If you do: the folder is
`~/Library/Application Support/oag/images`. In Finder choose Go, then "Go to
Folder...", paste that path, and make the folders if they are missing. A folder
called `images` next to the program also works. **None of this has been run on a
Mac.**

### Linux

Pick one:

- `~/.local/share/oag/images/` works everywhere. Open it in your file manager
  with Ctrl+L and paste the path. Make the folders if they are missing.
- With the **AppImage**, put the files in the **same folder as the AppImage**,
  or in a folder called `images` beside it. Nothing to configure.
- Running from a checkout of the source: `data/images/` in the folder you start
  the game from.

Your disc key folder, if a game needs one, is `~/.config/oag/keys/`.

**Three likely problems (Linux)**

1. *The AppImage does nothing.* It is not marked runnable, or the machine has
   no FUSE. Mark it runnable, or use the `.tar.gz`.
2. *A window opens black, or no window.* No working Vulkan graphics driver.
   Install your distribution's Vulkan driver (for example
   `mesa-vulkan-drivers`).
3. *"NO DISC IMAGE FOUND" window, or the terminal prints "no disc image found".*
   The files are in a folder that is not searched. The window and the message
   both list the folders. Move the files into the first one listed.

### Steam Deck

Use Desktop Mode (hold the power button, choose it). Then follow Linux above:
the simplest is to save the AppImage in a folder and put your files **in the
same folder**. A file manager (Dolphin) is the "Files" icon. A gamepad works
with no setup. To start it from Game Mode, add the AppImage as a non-Steam
game. (Not tried on a Deck by this guide.)

**Three likely problems (Steam Deck)**: the same three as Linux, plus: *the
Deck AppImage will not start on a different PC.* It is built for the Deck's own
processor, so a normal PC needs the plain `linux-x86_64` AppImage.

### Android

Android 11 and newer keep an app's files where a phone's file manager cannot
see them, so the files go in from a computer with a USB cable and `adb`
(Android's own tool):

1. On the phone, open Settings, About phone, tap "Build number" seven times.
   Then open the new Developer options and turn on USB debugging.
2. Plug the phone in. Accept "Allow USB debugging" on its screen.
3. Install the app (step 1), and start it once, so it makes its folders.
4. On the computer, run this once per game file (change the file name):

   ```sh
   adb push "Wipeout Pulse.chd" /sdcard/Android/data/org.openantigrav.game/files/data/images/
   ```

   A big file takes minutes. The folder name is `data/images` inside the app's
   own files folder; it is the same `images` folder as on a computer.
5. Start the app again.

If you have the project's source, `just push-data android` does step 4 with a
picker. See [Android](../tools/android.md).

**Three likely problems (Android)**

1. *The app shows "NO DISC IMAGE FOUND".* The screen prints the exact `adb
   push` line and folder. Push again to that folder.
2. *"INSTALL_FAILED_UPDATE_INCOMPATIBLE" when updating.* The new APK was
   signed with another key. Uninstalling the old app deletes its files,
   including your game files, so push them again afterwards.
3. *An encrypted PS3 disc needs a key.* There is no paste on Android in this
   build. Type it on the keypad, or push a `.dkey` file next to the image (see
   "Wipeout HD / Fury" below).

## 3. What to put in the folder, per game

**Do the file names matter? No.** Name the file anything you like, spaces and
brackets included. The game finds a file by its **ending** (`.chd`, `.iso`,
`.vpk`, `.pkg`) and then reads the game's own ID out of the file to know which
game it is. The ending's capital letters do not matter either (`.CHD` works).
Checked 2026-10-07: `Wipeout Pulse (Europe) (En,Fr,De).chd`, `Wipeout Pure
USA.CHD`, `Wipeout HD Fury (Europe).iso` and `Wipeout Omega Collection.pkg` all
opened by their title. The exact names listed in the curious part are only the
ones tried **first**.

Put in **as many games as you like**. With more than one, the game opens a list
(the "chooser") so you pick. With one, it opens that. If you have the same game
from Europe and the USA, it opens the Europe one first; start the USA one by
giving its name on the command line.

### Wipeout Pulse and Wipeout Pure (PSP, or PS2 for Pulse)

Put the disc image, a `.chd` or `.iso` file, in the folder. Nothing else.

### Wipeout HD / Fury (PS3 disc)

Put the disc image (`.iso`) in the folder. **You do not decrypt it.** The game
reads the encrypted file as it is and decrypts each piece in memory, so no
decrypted copy is ever written. It needs the **disc key**, a 32-digit code
(letters A to F and numbers) that your own disc dump tool recorded for your own
disc. It is often saved as a `.dkey` file. This guide does not say where to get
one. Give the game the key one of three ways:

1. **A file beside the image** with the **same name and a `.dkey` ending**:
   `Wipeout HD Fury (Europe).iso` and `Wipeout HD Fury (Europe).dkey`. If you
   rename the image, rename the key file the same way (checked: a key with
   the old name beside a renamed image is not found).
2. **A key file in the keys folder** (Windows `%APPDATA%\oag\keys`, Linux
   `~/.config/oag/keys/`, macOS `~/Library/Application Support/oag/keys`). Any
   name ending `.dkey` or `.key` works there (checked), and it is tried on every
   encrypted disc.
3. **Typed in the game.** The list shows the disc as `NEEDS DISC KEY`. Select
   it, press Enter, and a keypad opens. Type the 32 digits, or paste them with
   Ctrl+V. The game checks the key against the disc and saves it only when it
   opens the disc. Wrong key: `THAT KEY DOES NOT OPEN THIS DISC`, nothing saved.

### Wipeout HD, the PlayStation Store version (no Fury)

This one is not a disc image. It is the folder the PlayStation installed
(`PARAM.SFO` beside a `USRDIR` folder), copied into a folder called
`extracted/ps3/<any name>` next to your `images` folder: `~/.local/share/oag/extracted/ps3/`
on Linux, `%APPDATA%\oag\extracted\ps3\` on Windows. The steps use an
emulator's installer and your own licence file, so they are in the curious
part: [Wipeout HD from the PSN download](#wipeout-hd-from-the-psn-download).

### Wipeout 2048 (Vita)

Put a **`.vpk` file** in the folder. A `.vpk` is a zip of the game's installed
folder. A patch `.vpk` or add-on (DLC) `.vpk` of the same game next to it is
added automatically and is not listed as a game of its own. You can also use
the unpacked folder instead (see "For the curious").

**A raw Vita `.pkg` file does not work yet.** Its files are locked with a key
that only the Vita itself can make, and this project cannot carry that. If you
give the game one by name on the command line it says so; dropped in a folder it is simply not listed. What works today: a `.vpk` made from the
game installed on **your own** Vita (the tool for that is a Vita-side homebrew
plugin called NoNpDrm; this guide does not cover it, and no real NoNpDrm
`.vpk` was available to test, only a stand-in zip made from unpacked files).
Unpacking a `.pkg` on a PC is possible with several tools and your game's
licence code, and the steps are written down for the curious in
[data/README.md](../../data/README.md) and
[Vita packages](../formats/vita-package.md). Those steps were not re-run for
this page.

### Wipeout Omega Collection (PS4)

Put **both** `.pkg` files in the **same folder**: the base game and the update
(patch). The game pairs them by the game ID inside, **not by the file names**,
puts the update in front and reads both in place. There is nothing to unpack.
The base alone is not found at all (checked): the update holds the front end,
so a missing update looks like "NO DISC IMAGE FOUND". This works for
the kind of `.pkg` that was made for a jailbroken PS4, whose keys are public. An
ordinary Store `.pkg` is tied to a console and is refused with a message saying
so. If you already unpacked folders into `extracted/ps4`, they win over the
`.pkg` files in the list ([PS4 packages](../formats/ps4-package.md)).

### Downloadable extras (Pulse DLC)

Pulse's add-on zips go in a folder called `dlc` next to `images` (for example
`~/.local/share/oag/dlc`, or `dlc` beside the AppImage). Not re-verified for this page. See
[DLC packs](../formats/dlc-pack.md).

## 4. Start it, and what you should see

Start the program: double-click `oag-game.exe` (Windows), the AppImage (Linux,
Deck), or the app icon (Android). A window opens.

- **One game found**: it starts that game: a short intro (when `ffmpeg` is
  installed, and after about 80 seconds of converting the very first time),
  then the menu.
- **More than one found**: a list titled `SELECT A DISC IMAGE`. Each row shows
  the game, the console and the game's ID, then the file name. Move with the
  arrow keys, press Enter, or click or tap a row. A row that cannot open is
  shown dim, with a sentence at the bottom saying why (an `ENCRYPTED` disc says
  "PRESS X TO ENTER THE DISC KEY").
- **Nothing found**: a dark screen headed `NO DISC IMAGE FOUND`. It says "put
  your own game files in this folder", spells out the folder for your computer,
  and lists the file endings that work. Press Escape to quit. (Seen 2026-10-07 on Linux
  in a 1280x720 window. On Android the same screen shows the `adb push` line.)

Controls: arrow keys or WASD steer, X or Return thrust, Q and E are the air
brakes, Escape goes back. A gamepad works with no setup.

### What you can play today

Pulse plays from the menus into a race. The other games start and race on
their own data, but only Pulse is close to finished: see [status](status.md).

## 5. When something does not work

| You see | Do this |
| --- | --- |
| `NO DISC IMAGE FOUND` | Put the file in the folder the screen names (section 2). Check the ending: `.chd`, `.iso`, `.vpk`, or two `.pkg`. |
| The game list has your file dim, `NEEDS DISC KEY` | An encrypted PS3 disc. Give it the key (section 3). |
| Your Omega `.pkg` is not listed | You need **both** the base and the update in the same folder. |
| `NO DISC IMAGE FOUND`, but a `.pkg` is in the folder | Either a Vita `.pkg` (not supported yet: use a `.vpk` or unpacked folder) or an Omega base `.pkg` without its update `.pkg`. Neither is ever listed. |
| No sound / no intro | No `ffmpeg`: the intro is skipped, the game still plays. |
| Anything else | The log file has the detail: see "Where things are kept" below. Include it when you report a problem. |

## For the curious

Everything from here down is the technical detail the steps above stand on.

## Build it yourself

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
  title. Without it the program logs one line, `ffmpeg is not installed, so
  the intro is skipped and movies show no picture`, skips the conversion, and
  the intro is skipped straight to the menu; everything else works. With it the
  first Pulse boot spends about 80 seconds converting, once, and caches the
  result.
- `just` runs the repository's recipes (`just play ...`). Not needed to play.
  If you use it, `just play` also turns on the `native-video` feature, which
  needs GStreamer development libraries. The plain `cargo` line above does
  not.

You do not need `cargo-nextest`, Ghidra or any emulator to play.

## The files in detail

The names below are the ones the program looks for first, in `data/images/`. **Any
other file name works as well**: every `.chd`, `.iso` and `.vpk` in a searched
folder is found, in alphabetical order, and an Omega `.pkg` is found with its
patch by the game ID inside the files, not by name (checked 2026-10-07 with names
like `Wipeout Pulse (Europe) (En,Fr,De).chd`, `Wipeout Pure USA.CHD` and
`Wipeout Omega Collection update.pkg`, one run each). Where the folders are on
each OS is in the player's guide above.

| Title | Platform | What to supply | File or folder name |
| --- | --- | --- | --- |
| Wipeout Pulse | PSP | disc image, `.chd` or `.iso`, Europe or USA (Europe is opened when you have both) | `pulse-psp-eu.chd`, `pulse-psp-usa.chd` |
| Wipeout Pulse | PS2 | disc image, Europe | `pulse-ps2-eu.chd` |
| Wipeout Pure | PSP | disc image, Europe or USA | `pure-psp-eu.chd`, `pure-psp-usa.chd` |
| Wipeout HD / Fury | PS3 | disc image, Europe, **encrypted as dumped** with your disc key beside it (a `.dkey`), or an already decrypted image | `hdfury-ps3-eu.iso` + `hdfury-ps3-eu.dkey`, or `hdfury-ps3-eu-dec.iso` |
| Wipeout HD (no Fury) | PS3 | the **installed** PSN download, Europe | `data/extracted/ps3/hd-psn-eu` |
| Wipeout 2048 | Vita | a **`.vpk`** (a NoNpDrm dump; a patch or DLC `.vpk` beside it is mounted too), or an **unpacked** package folder | `data/images/2048-eu.vpk` (any name), or `data/extracted/vita/PCSF00007` (Europe) / `PCSA00015` (USA) |
| Omega Collection | PS4 | the base **`.pkg` and its patch `.pkg`** (a fake package, read in place), or **unpacked** base and patch folders | `data/images/omega-ps4-eu.pkg` + `omega-ps4-eu-patch.pkg` in one folder, or `data/extracted/ps4/omega-eu` and `omega-eu-patch` |

The first three rows are the easy ones: a normal dump of your own disc is
read as it is, CHD (single-track, either `createdvd` or `createcd`) or ISO.

### HD / Fury: the disc is read encrypted, in place

A PS3 disc dump is encrypted, and the program reads it that way: each sector is
decrypted as it is read, so no decrypted copy is ever written. It needs the disc
key that your own dump tool recorded for that disc (redump publishes it as a
`.dkey`). Give it the key one of three ways:

1. **A file beside the image**, same name with `.dkey` (or `.key`):
   `data/images/hdfury-ps3-eu.iso` and `data/images/hdfury-ps3-eu.dkey`. The
   file holds the key as 32 hex digits or as the 16 raw bytes redump writes.
2. **A key file in the program's own keys folder**, `oag/keys/` under your
   config directory (`~/.config/oag/keys/` on Linux). Any `.dkey` or `.key`
   there is tried against any encrypted image.
3. **Typed or pasted in the chooser.** An encrypted image with no key is listed
   as `NEEDS DISC KEY`; select it and a keypad asks for the 32 digits. On a
   desktop you can also type them, or paste with Ctrl+V. The key is checked
   against the image first and saved to the keys folder only if it opens it. On
   Android the keypad and a key file beside the image work; there is no paste
   there in this build.
4. **Dropped on the chooser's window.** Drop a `.dkey` or `.key` file (256
   bytes or fewer) on the chooser, or on its key prompt: it is read, checked
   against every locked image listed (the prompt's image, when it is open),
   saved to the keys folder for each it opens, and the rows are re-read. A key
   that opens none says `THAT KEY DOES NOT OPEN THIS DISC` in the prompt, and a
   file that is no key says so. A larger file dropped is taken as a disc image:
   listed, selected and, if it plays, booted. The window says `DROP THE FILE TO
   USE IT` while one is dragged over it. **winit's Wayland backend never
   reports a drop** (`DroppedFile` exists only in its X11 code, winit 0.30.13),
   so this works under X11, XWayland, Windows and macOS, and not on a native
   Wayland window; paste and the file beside the image work everywhere. The
   launcher's logic is covered by unit tests and a disc-backed test
   (`launcher_ground_truth`); a real drag was not driven.

   **In the browser**, the page takes the same key as a dropped or picked file
   or as pasted digits, and remembers it per disc: see
   [the web build](../tools/web.md#disc-keys).

The program never ships a key and never prints yours. A decrypted image
(`hdfury-ps3-eu-dec.iso`, made by any tool) still works and is tried first.
The format is described in [PS3 disc encryption](../formats/ps3-disc.md).

Seen working on 2026-10-07 under a software-rendered X session: with the image
alone in its folder the chooser lists `NEEDS DISC KEY`; Enter opens the keypad;
typing, and Ctrl+V of a spaced key, fill the buffer; a wrong key reports
`THAT KEY DOES NOT OPEN THIS DISC` and stores nothing; the right one is saved to
the keys folder, the row becomes `WIPEOUT HD` and boots to the front end. Wayland
paste is unverified (the clipboard crate is built with its Wayland support).

Measured on 2026-10-07 (release build, the same race, three runs each at a
load average of 13): the encrypted image reaches the first race frame in
1.87-1.96 s and the decrypted one in 1.85-1.88 s, and the two PNGs are
byte-identical.

### Wipeout HD from the PSN download

The Store version (`NPEA00057`, v3.00) is Wipeout HD **without Fury**: no Talon's
Junction, Zone circuits or Detonator, eight circuits and twelve teams. It is read
from the folder the PlayStation installs it into, copied under `data/extracted/ps3/`.
Census and what differs from the disc: [hd-psn](../formats/hd-psn.md).

1. Extract the download zip into a new empty folder. The package inside is a `.pkg`.
   Never run anything from it.
2. Install it with RPCS3's own installer into a **private** RPCS3 profile, so your
   main one is untouched, and keep that profile under `data/` (it holds Sony firmware,
   your licence file and 1 GB of game data, and `data/` is not committed). The package
   needs your own licence file for the game (a `.rap`); it is part of your copy of the
   game, and this project supplies none and does not say where one comes from. Put it
   where RPCS3 looks for licences (`dev_hdd0/home/00000001/exdata/` of that profile), then:

   ```sh
   P=$PWD/data/scratch/hd-psn-profile
   mkdir -p $P/rpcs3 && cp -r ~/.config/rpcs3/dev_flash* $P/rpcs3/   # the firmware folders the check used
   XDG_CONFIG_HOME=$P rpcs3 --headless --installpkg path/to/package.pkg
   ```

   Use `--headless`: `--no-gui` with the same arguments sat idle and installed nothing
   (checked 2026-10-07, RPCS3 0.0.42). It installed 1 GB in a few seconds. Add
   `XDG_CACHE_HOME` and `XDG_DATA_HOME` under the same folder if you want its log out of
   `~/.cache` (that log prints a package key; do not paste it anywhere).
3. Copy the install folder into place:

   ```sh
   cp -r $P/rpcs3/dev_hdd0/game/NPEA00057 data/extracted/ps3/hd-psn-eu
   ```

   The result is `PARAM.SFO` and `USRDIR/data01.psarc` to `data04.psarc` side by
   side. **Leave out any `.EDAT` file** (the licence stub): this project does not read
   it. The program recognises the folder by `PARAM.SFO` beside `USRDIR/`.
4. Run it. With nothing else on the search path the folder is found by itself
   (Europe first); or name it: `oag-game data/extracted/ps3/hd-psn-eu`.

What the PSN copy lacks is simply not offered: the race page's TRACK and TEAM rows
list the eight circuits and twelve teams the package ships, a bare `--race` opens
Vineta K, and the loader report names each absent effect. The race
campaign screens and the menu backdrop are Fury's and are not drawn (hd-psn.md,
"What draws differently"); the menu boxes draw from the file this build's own executable
names. Checked 2026-10-07: the install opened as `Wipeout HD`, raced Vineta K
tick for tick like the disc, and its menu rows drew.

### 2048: a `.vpk`, or a folder

A 2048 `.vpk` (the ZIP a NoNpDrm dump of your own console's install makes) is
read as it is: put it in `data/images/` or name it. A patch `.vpk` and DLC
`.vpk` files of the same game beside it are mounted behind it and are not listed
as games of their own. A `.vpk` stored without compression is read fastest; a
deflated archive larger than 1 GiB is refused by name (re-pack it with
`zip -0`). The program does **not** read a Vita `.pkg`: its game files are
protected with a key your console derives in hardware, which is not something
this project can carry. It says so by name if you give it one. Make a `.vpk`
from your own console with NoNpDrm, or unpack the folder as below.
Read and compared with the unpacked folders on 2026-10-07 using `.vpk` files built
from them (no real NoNpDrm dump was available): [Vita packages](../formats/vita-package.md).

### Omega, and a 2048 folder: unpack the package into a folder

**Omega: put both `.pkg` files in the same folder** (`data/images/`) and the
program reads them in place, patch mounted ahead of the base; nothing to unpack
([PS4 packages](../formats/ps4-package.md)). This works for a **fake package**
(the HEN/jailbreak rip), whose keys are public; a retail PSN package is keyed to
a console and is refused with a message saying so. If you also have an unpacked
`data/extracted/ps4` folder, that folder is the one listed and booted by default;
name a `.pkg` to use it instead.

A 2048 Vita `.pkg` is not read: the program reads a `.vpk` or a folder you unpacked.
The licence for a package is part of your own copy, and this project does not
say where one comes from. The project may ship fixed public package keys that
other open-source tools already ship, never a per-game licence
([legal](legal.md#fixed-public-keys)).

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

When a folder holds both a European (`PCSF00007`) and a USA (`PCSA00015`) 2048 extract, the European one is opened; name the other directly to play it.

2048 starts with `base/` alone (checked: a folder holding only
`base/PSP2/data.psarc` opens as `Wipeout 2048`). The DLC packs are mounted
when their folders are there. **The 1.04 patch is mounted when its folder is there**
(`patch-v104/` beside `base/`, as `oag-unpack` writes it), over the base archive,
as the shipped v1.04 executable does (`crates/2048/src/lib.rs`, `PATCH_CANDIDATES`;
census in [patches](../formats/patches.md)). Without it the base game
runs as the v1.00 original did at launch. Omega is the opposite: its patch is required. A `base/` folder that is itself a symlink was not found, so use
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

## Run it from a terminal and check what was found

From the folder that holds `data/`:

```sh
target/release/oag-game                    # a disc image found in data/images
target/release/oag-game data/images/pulse-psp-eu.chd
target/release/oag-game data/extracted/vita/PCSF00007
target/release/oag-game data/extracted/ps4
```

With nothing named, `--dry-run`, `--screenshot` and `--race` look for a disc
image first and then for an unpacked 2048 folder (`data/extracted/vita/*/`)
or Omega folder (`data/extracted/ps4/`), in that order. A plain windowed
`oag-game` with nothing named, and `oag-game --launcher`, open a chooser that
lists every image and every unpacked folder found, with the platform shown as
`unknown` for a folder; pick with the arrow keys and Enter. With several
titles installed, name the one you want.

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
| Converted movies and sound | `~/.cache/oag/`. A source checkout (a `justfile` and a `data/` folder next to where you run it), or a `data/cache/` that already exists, keeps it in `data/cache/` instead |
| Images, if not in `data/images/` | `~/.local/share/oag/images/` |
| 2048 and Omega folders, if not in `data/extracted/` | `~/.local/share/oag/extracted/vita/` and `.../extracted/ps4/` |

On Windows the settings, records, ghosts and `keys/` live under
`%APPDATA%\oag`, and on macOS under `~/Library/Application Support/oag` (read from
the folder library's source, not run there). Deleting the cache is always safe. Other ways to name a source, in the order
the program tries them: the command line, `$OAG_IMAGE`, then `[source] image`
in `settings.toml`, then the folders above
([packaging](../tools/packaging.md#where-the-disc-image-comes-from)).
`$OAG_IMAGE` takes a disc image, a folder of images, or an unpacked 2048 or
Omega folder (or the folder holding one).

`RUST_LOG=debug` raises the terminal's detail, and `--log-file FILE` moves the
log file (an empty value writes none).

## What you can do today

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

## Error messages

| What you see | Cause and fix |
| --- | --- |
| `Error: no disc image found. OpenAntiGrav ships no game content ...` then a `Searched:` list | Nothing was in the listed places. Relative paths are relative to the folder you ran it from (the message names it): run from the folder that holds `data/`, or name the source. The list shows where disc images (`.chd`, `.iso`), unpacked 2048 folders and unpacked Omega folders are looked for. A Vita `.pkg` is never read: use a `.vpk` or unpack it (see "The files in detail"); an Omega `.pkg` pair is read in place from `data/images/` |
| `Error: OAG_IMAGE is set to ..., which is not a disc image (.chd, .iso), an unpacked 2048 or Omega folder, or a folder holding one` | `$OAG_IMAGE` points at nothing usable. Unset it, or point it at a disc image or an unpacked folder |
| `... is an encrypted PS3 disc image and no disc key opens it` | No key beside the image or in the keys folder passed the check, or the one there belongs to another disc. Put the right `.dkey` beside the image, or enter it in the chooser (see "The files in detail"). With both `hdfury-ps3-eu.iso` and `hdfury-ps3-eu-dec.iso` in `data/images/`, the decrypted one is used |
| The intro is skipped, and the log line `ffmpeg is not installed, so the intro is skipped and movies show no picture` | `ffmpeg` is missing (or `--no-video` was given, which logs nothing). Install `ffmpeg` and run once without the flag. The game is otherwise fine |
| Window opens black, or no window | Vulkan driver missing or broken. The terminal prints a `renderer: vulkan: ...` line naming the adapter. A CPU adapter such as `llvmpipe` works but is slow |
| Program does not start and the loader says it cannot open `libpipewire-0.3.so.0` (not run: needs a machine without it) | Install the PipeWire client library from the table in section 1 |
| `frame: 50 ms` warnings fill the terminal | You are on a CPU renderer. Lower RENDER SCALE in the options. The log file is quieter |

If something else fails, the log file named above has the detail. Include it
when you report a problem.
