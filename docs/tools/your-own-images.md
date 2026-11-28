# Get a working image from your own copy

This page says, for each title, what file or folder OpenAntiGrav can open, and
how to get there from a copy you legally own.

**Use only your own copy, your own console and your own licence.** This project
ships no game content, no per-game licences and no disc keys, and does not say
where to get any of them. See [legal](../overview/legal.md).

**Where each route starts.** This project does not document how to run a disc
or console dumping tool. A route below starts with what your own dump gives you,
and ends with the file the game accepts.

**What was tested.** This page collects steps from other pages (2026-11-28).
None was re-run for it. The linked pages say what was tried.

| Title | You end up with | Desktop | Web | Details |
| --- | --- | --- | --- | --- |
| Pulse and Pure (PSP) | `.iso` or `.chd` | yes | yes | [PSP](#wipeout-pulse-and-pure-psp) |
| Pulse (PS2) | `.iso` or `.chd` | yes | yes | [PS2](#wipeout-pulse-ps2) |
| HD and Fury (PS3 disc) | `.iso`, plus your disc key if encrypted | yes | yes | [PS3 disc](#from-the-ps3-disc) |
| HD, no Fury (PlayStation Store) | an installed folder | yes | no | [PS3 Store](#from-the-playstation-store) |
| 2048 (Vita) | a `.vpk` or an unpacked folder | yes | not yet | [Vita](#wipeout-2048-vita) |
| Omega Collection (PS4) | the base `.pkg` and the patch `.pkg` | yes | not yet | [PS4](#wipeout-omega-collection-ps4) |

"Desktop" means the installed program. "Web" means the
[browser build](web.md), where you pick the file in the page and it is read in
your own browser tab. Which folder to put files in, on each computer and on
Android, is in [installing](../overview/installing.md#2-make-the-images-folder-and-put-your-files-in-it).
File names do not matter. The game reads the title's ID from inside the file.

## Wipeout Pulse and Pure (PSP)

1. Dump your own UMD with your own dump tool. The result is an `.iso`.
2. Optional: [compress it to `.chd`](#compress-an-iso-to-chd).

The game looks for `.iso` and `.chd` files. Other compressed forms, such as
`.cso`, are not found.

**What the game accepts**

- Desktop: `.iso` or `.chd`.
- Web: `.iso` or `.chd`.

## Wipeout Pulse (PS2)

The PS2 release is Europe only.

1. Dump your own disc with your own dump tool. The result is an `.iso`.
2. Optional: [compress it to `.chd`](#compress-an-iso-to-chd).

**What the game accepts**

- Desktop: `.iso` or `.chd`.
- Web: `.iso` or `.chd`. Not everything plays there yet: see
  [the web build](web.md).

## Compress an ISO to CHD

This is for PSP and PS2 images. It saves space and is never needed.

`chdman` comes with MAME's tools (the `mame-tools` package on Arch). The game
reads CHD files made by `chdman createdvd`, and single-track ones made by
`chdman createcd`.

```sh
chdman createdvd -i game.iso -o game.chd
```

That command was not run for this page. To check the result, use
[Check that it worked](#check-that-it-worked).

## Wipeout HD and Fury (PS3)

There are two versions. The disc has HD and Fury. The PlayStation Store download
has HD without Fury.

### From the PS3 disc

1. Dump your own disc with your own dump tool. The result is an `.iso`.
2. **Do not decrypt it.** The game reads an encrypted image and decrypts it in
   memory. An image you already decrypted also works.
3. For an encrypted image, you need the 16-byte key of your own disc, recorded
   by your own dump tool.
4. Give the game that key in one of these ways:
   - A file beside the image with the same name, ending `.dkey` or `.key`:
     `game.iso` and `game.dkey`. It holds 32 hex digits, or the 16 raw bytes.
   - A `.dkey` or `.key` file in the program's keys folder: `~/.config/oag/keys/`
     on Linux, `%APPDATA%\oag\keys` on Windows,
     `~/Library/Application Support/oag/keys` on macOS.
   - Typed into the game. The disc is listed as `NEEDS DISC KEY`. Select it and
     enter the 32 digits.

The game checks the key against the disc. A wrong key is not saved. More:
[HD and Fury (PS3 disc)](../overview/installing.md#wipeout-hd--fury-ps3-disc)
and [PS3 disc encryption](../formats/ps3-disc.md).

**What the game accepts**

- Desktop: `.iso`, decrypted, or encrypted with its key.
- Web: the same. Drop or pick the `.dkey` or `.key` file, or paste the 32
  digits: see [disc keys on the web](web.md#disc-keys).

### From the PlayStation Store

This is the Europe version (`NPEA00057`, v3.00). You need the package (`.pkg`)
of your own purchase and the licence file (`.rap`) that comes with it. The tool
is RPCS3's own package installer.

1. Put the `.pkg` in a new, empty folder. Never run anything from it.
2. Make a separate RPCS3 profile for this, so your main one is not touched. It
   needs the PS3 firmware, and your licence file in
   `dev_hdd0/home/00000001/exdata/`.
3. Install the package with that profile, using `rpcs3 --headless --installpkg`.
4. Copy the installed folder, `dev_hdd0/game/NPEA00057`, to
   `extracted/ps3/<any name>/`:
   - Linux: `~/.local/share/oag/extracted/ps3/`
   - Windows: `%APPDATA%\oag\extracted\ps3\`
   - macOS: `~/Library/Application Support/oag/extracted/ps3/`
   - Source checkout: `data/extracted/ps3/`
5. Leave out any `.EDAT` file.

The result has `PARAM.SFO` beside a `USRDIR` folder. The exact Linux commands,
and the optional game updates, are in
[Wipeout HD from the PSN download](../overview/installing.md#wipeout-hd-from-the-psn-download).
What this copy holds compared with the disc: [HD from the PSN download](../formats/hd-psn.md).

**What the game accepts**

- Desktop: the installed folder.
- Web: no. The page takes a single file, not a folder.

## Wipeout 2048 (Vita)

**The game cannot read a Vita `.pkg`.** Its files are locked with a key that is
derived by the console's own hardware: see
[Vita packages](../formats/vita-package.md). It reads one of these:

- a `.vpk`, which is a zip of the game's installed folder, or
- that folder, unpacked.

### A `.vpk` from your own console

A NoNpDrm dump of the game installed on your own Vita gives you the folder's
files in the clear. Zip it as in [Pack a `.vpk`](#pack-a-vpk). This project does
not document how to use NoNpDrm. The reader was tested with stand-in `.vpk`
files made from unpacked folders, not with a real NoNpDrm dump.

### Decrypt your own `.pkg` on a computer

You need your own package and your own licence for it, as a zRIF string. The
four steps below are the ones in
[Vita PKGs decrypt in four steps](../../data/README.md#vita-pkgs-decrypt-in-four-steps-dataextractedvita-holds-the-result).
**Only steps 1 and 2 are needed to play.**

1. Remove the outer layer: `pkg2zip -x your-game.pkg`. The files inside are
   still encrypted.
2. Decrypt them with `psvpfsparser`, from the `psvpfstools` project, using your
   zRIF. It also needs the console's key derivation, which it takes from a
   service or a cache file (its `-f00d_url` and `-f00d_cache` options). This
   project provides neither and does not say where to find one. `psvpfstools`
   needs fixes to build on a current system; the linked section above names the
   build recipe a source checkout has for it.
3. Turn the zRIF into the klicensee (`just zrif-to-klicensee` in a source
   checkout). Not needed to play.
4. Decrypt `eboot.bin` (`just vita-self-decrypt`). Not needed to play: the game
   never reads it. It is for reverse engineering.

Then arrange the decrypted files in one folder per title ID, `PCSF00007` for
Europe or `PCSA00015` for the USA:

```
PCSF00007/
    base/PSP2/data.psarc     required
    patch-v104/              the 1.04 patch, optional
    dlc1/                    DLC pack 1, optional
    dlc2/                    DLC pack 2, optional
```

Each sub-folder holds one package's decrypted files, `sce_sys/` included. Put
the title folder in `extracted/vita/`: `~/.local/share/oag/extracted/vita/` on
Linux, `%APPDATA%\oag\extracted\vita\` on Windows,
`~/Library/Application Support/oag/extracted/vita/` on macOS, or
`data/extracted/vita/` in a source checkout. Use real folders: a `base/` that is
a symbolic link is not found.

A patch or DLC package uses the same steps. A patch uses the base game's zRIF.
More:
[2048: a `.vpk`, or a folder](../overview/installing.md#2048-a-vpk-or-a-folder).

### Pack a `.vpk`

A `.vpk` is one file, which is easier to copy than a folder. Zip the **contents**
of the folder, so `sce_sys/param.sfo` is at the top of the zip. From inside
`base/`:

```sh
zip -r -0 -X ../../2048.vpk . -x eboot.elf
```

Then put the `.vpk` in your `images` folder.

`-0` stores the files without compression. A compressed file over 1 GiB inside a
`.vpk` is refused. For the patch and each DLC pack, make a separate `.vpk` the
same way and keep them all in the same folder. The game adds them by the title
ID inside.

In a source checkout, `scripts/make-test-vpk.sh PCSF00007` does this from
`data/extracted/vita/` and writes into `data/test-vpk/`.

**What the game accepts**

- Desktop: a `.vpk`, or the unpacked folder. Not a `.pkg`.
- Web: not yet. A later change is due to add `.vpk`. It does not work today.

## Wipeout Omega Collection (PS4)

**Two files are needed: the base `.pkg` and its patch `.pkg`.** The patch is
required today, because it replaces the front end: see
[Omega status](../formats/omega-status.md).

**Only one kind of PS4 package opens.** It is the "fake package" kind used on
jailbroken consoles, whose keys are public. A package from the PlayStation
Store is tied to a console and is refused with a message that says so. **This
project does not document how to make a fake package from your own copy**, and
does not say where to get one.

If you have both files from your own copy in that form:

1. Put the base `.pkg` and the patch `.pkg` in the **same folder**, your
   `images` folder.
2. Nothing else. Do not unpack them. The game pairs them by the title ID inside
   and reads them in place. The file names do not matter.

A base package without its patch is not listed. Unpacked folders also work; the
layout is in [installing](../overview/installing.md#omega-and-a-2048-folder-unpack-the-package-into-a-folder)
and the reader in [PS4 packages](../formats/ps4-package.md).

**What the game accepts**

- Desktop: the base and patch `.pkg` side by side, or the unpacked folders.
- Web: not yet. A later change is due to add the `.pkg` pair. It does not work
  today.

## Check that it worked

Start the game. Your title should open, or appear in the list of games. To check
without opening a window:

```sh
oag-game --dry-run --no-audio --no-video path/to/your-file
```

A good result is one line with the title's name and your file. If something else
appears, see [when something does not work](../overview/installing.md#5-when-something-does-not-work).
