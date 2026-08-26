# `data/` - user-supplied game data

Everything in this directory except this README is **gitignored and must never be
committed**. OpenAntiGrav ships code and documentation only. Game content comes
from your own legally obtained copies.

See [`docs/overview/legal.md`](../docs/overview/legal.md) for the policy.

## Layout

```
data/
  images/       Disc images you supply (.chd / .iso). Normalised names, see below.
  extracted/
    psp/        Files extracted from a PSP UMD image
    ps2/        Files extracted from a PS2 DVD image
  cache/        Assets converted from your originals, rebuilt on demand
  dlc/          PSN DLC packages, as downloaded - see below
  ghidra/       Ghidra project directory (shared analysis, not versioned)
  tools/        Built third-party tooling, e.g. the Allegrex Ghidra extension
  traces/       Runtime traces captured from PPSSPP / PCSX2
  saves/        Emulator save states used by the behavioural verification suite
```

`tools/` is populated by `just build-allegrex`. It holds a git checkout and a
build output, neither of which belongs in this repository.

### In a git worktree, run `just link-data` first

This whole directory is gitignored, so it does **not** travel into a
`git worktree add` the way tracked files do: a fresh worktree gets a `data/`
holding nothing but this file. `just link-data`
(`scripts/link-worktree-data.sh`) symlinks every subdirectory above from the
main checkout into the current worktree, which is enough for `just test-data`
and for every disc-backed CLI run.

Symlinks rather than copies, deliberately - `images/` alone is several
gigabytes, and derived output (`cache/`, `traces/`, `shots/`) written through a
link lands in the main checkout, where the next worktree and the next session
can see it too. The script is idempotent, does nothing in the main checkout,
and leaves any real directory it finds in place rather than replacing it.

Worth doing before concluding anything about a test run. Without it, disc-backed
tests skip silently and a run looks greener than it is; with
`OAG_REQUIRE_GAME_DATA=1` set they instead fail in a heap, naming files that
were in the main checkout all along.

`cache/` is where a **repository checkout** caches converted assets. A packaged
build has no checkout around it and uses `~/.cache/oag/` instead, rather than
writing beside wherever it was run from - see
[packaging](../docs/tools/packaging.md#where-the-disc-image-comes-from).
Deleting either is always safe.

`cache/` holds assets converted from your own originals, where converting once is
better than reproducing the original decoder: see
[ADR-0004](../docs/architecture/adr/0004-asset-pipeline.md). It is derived
content, so the same rule applies as to everything else here, and **deleting it
is always safe**. It is rebuilt from your disc image on demand.

## Image naming

Tooling looks for these names. Copy your own images here and rename:

| Name | Title | Console | Region |
| --- | --- | --- | --- |
| `pulse-psp-usa.chd` | Wipeout Pulse | PSP | USA |
| `pulse-ps2-eu.chd` | Wipeout Pulse | PS2 | Europe |
| `pulse-psp-eu.chd` | Wipeout Pulse | PSP | Europe/Australia |
| `pure-psp-usa.chd` | Wipeout Pure | PSP | USA |
| `pure-psp-eu.chd` | Wipeout Pure | PSP | Europe |
| `hdfury-ps3-eu.iso` | WipEout HD / Fury | PS3 | Europe |
| `2048-vita-usa.pkg` | WipEout 2048 | Vita | USA (`PCSA00015`) |
| `2048-vita-eu.pkg` | WipEout 2048 | Vita | Europe (`PCSF00007`) |

`.iso` works anywhere `.chd` does; the tools sniff the container.

### The PS3 and Vita images are encrypted, and nothing here decrypts them yet

Both are readable at the *directory* level and opaque at the *content* level,
in two different ways, and both are ordinary tooling problems rather than
research ones:

- **PS3.** `hdfury-ps3-eu.iso` is a standard redump encrypted image: the ISO
  9660 filesystem is plaintext (`PS3_DISC.SFB`, `PARAM.SFO` and `ICON0.PNG` all
  read normally, and `oag-unpack list` walks it fine), while everything under
  `PS3_GAME/USRDIR/` - `DATA00..06.PSARC`, `EBOOT.BIN`, `DFENGINE.SPRX` - is
  AES-encrypted under the disc key. Decrypting needs this pressing's redump
  `.dkey`, then `PS3Dec`; the SELF then needs `scetool` before Ghidra, with
  [Ps3GhidraScripts](https://github.com/clienthax/Ps3GhidraScripts) for the
  PPC64/PRX side.
- **Vita.** In a 2048 PKG, `sce_sys/param.sfo` and `sce_pfs/files.db` (magic
  `SCENGPFS`) are plaintext, but everything inside the PFS layer is encrypted -
  `eboot.bin`, `PSP2/data1.psarc`, `data2.psarc`, even the manual PNGs. A
  `pkg2zip` run that produces a readable directory tree full of high-entropy
  files has done the PKG's AES-CTR layer but not the PFS layer, i.e. it ran
  without a zRIF. Supply the zRIF (or use `psvpfsparser` with the klicensee),
  then decrypt the Vita SELF, then
  [VitaLoaderRedux](https://github.com/CreepNT/VitaLoaderRedux) for Ghidra.

Note what these two share and Pulse/Pure do not: **HD/Fury and 2048 both ship
PSARC archives rather than WADs**, and 2048 arrives as a PKG rather than a disc
filesystem at all. That is why the content-source layer is written against
"archives resolved by name from a source's own file list" rather than "a disc
image containing WADs" - see
[ADR-0022](../docs/architecture/adr/0022-title-packages.md).

`pulse-psp-eu.chd` is not one of `oag-game`'s auto-detected `IMAGE_NAMES` (it
is a reverse-engineering reference disc, not the played-from target) - point
tools at it explicitly by path. It is a genuinely different `BOOT.BIN` from
`pulse-psp-usa.chd`'s, not a re-labelled copy - see
[source-images.md](../docs/reverse-engineering/source-images.md).

**Pure and HD/Fury are read-only format targets today, not playable ones.**
`oag-view`/`oag-unpack`/`oag-wad` read them opportunistically per
[ADR-0009](../docs/architecture/adr/0009-multi-game-fanout.md). Pointing
`oag-game` at either fails fast with a named error (`Error::WrongTitle` for
Pure's PSP disc, since its archives share Pulse's own filenames; PS3
identification doesn't exist yet, so HD/Fury falls back to the archive's own
"nothing matched" error) rather than silently loading the wrong game -
playable second-title support is tracked at roadmap M8.

Confirm what you have with:

```sh
cargo run -p oag-tools --bin oag-unpack -- info data/images/pulse-psp-usa.chd
```

Record the SHA-256 of every image you use in
[`docs/reverse-engineering/source-images.md`](../docs/reverse-engineering/source-images.md).
Hashes are fine to commit; content is not.

## DLC (`data/dlc/`)

PSN DLC packages, as downloaded (`.zip`), for two titles. **The two titles'
packs are packaged differently and land on opposite sides of the encryption
question - checked by reading the actual bytes in both cases, not assumed
from the extension either way.**

**Wipeout Pulse EU** (`UCES00465/` folder, four packs: Auricom, Harimau,
Icaras, Mirage): each is `PACKn.edat`, three `PACKn_UIn.edat` files, and one
`PARAM.pbp`. **Despite the extension, `PACKn.edat` is not encrypted.** It
reads directly as an ordinary `oag-wad`-format archive (32-33 entries,
LZSS-compressed, consistent offset chain) - `cargo run -p oag-tools --bin
oag-wad -- list <extracted PACKn.edat>` lists it like any other WAD. Checked
against Ghidra before assuming otherwise: the game's only NpDrm import,
`sceNpDrmEdataSetupKey`, is called with no key argument and
`sceNpDrmSetLicenseeKey` (which would supply one) is never imported at all -
consistent with there being no per-title secret to find, though what (if
anything) supplies a key at the OS level was not chased further once the file
itself proved to be plaintext. `PARAM.pbp` is the standard, unencrypted PBP
(title/icon metadata for the XMB). No decryption work was needed or done -
and **the `PACKn_UIn.edat` files are now checked too**: same container, same
plaintext, and `PACKn_UI1.edat` is where each pack's
`Data\Ships\<Team>\handlingstats.xml` lives, so a pack needs all four archives
mounted to be raceable rather than just visible.

**The engine reads these directly.** `oag-game` finds a pack here, unpacks the
zip into `data/cache/dlc/` once, and mounts it against whichever Pulse image is
open - the European packs work against the American disc, which the original
did not allow. Format and evidence:
[`docs/formats/dlc-pack.md`](../docs/formats/dlc-pack.md); the decision to
diverge: [ADR-0021](../docs/architecture/adr/0021-region-independent-dlc.md).
One correction to the pack list above: the pack sold as **Mirage** declares its
team as `Mantis`, which is the folder its ship is under.

**Wipeout Pure EU** (per-pack folders like `UCES00001DDELTAPAK/`, seven
packs: A7, Delta, Gamma 1, GamesRadar, Oblivion, Omega, Voice of Cod): each
holds `ICON0.png`, `PARAM.sfo`, `PIC1.png`, a 16-byte `TEST.bin`, and the
actual payload, `pi.wad`. **`pi.wad` genuinely is encrypted or otherwise
unstructured** - measured entropy 8.0000 bits/byte across the whole file (a
plain WAD, by contrast, sits well below that; Pulse's `PACKn.edat` above does
too), no recognisable magic, no readable strings anywhere sampled, and
`oag-wad` refuses it outright (`unknown WAD version 2858143392`). Unlike
Pulse's DLC, this has not been reverse-engineered - nothing here established
whether Pure's NpDrm usage matches Pulse's (single-argument
`sceNpDrmEdataSetupKey`, no per-title key) or differs; Pure is a different
executable and was not checked. `PARAM.sfo` is Sony's standard, unencrypted
metadata format (same `\0PSF` magic as the disc's own `PARAM.SFO`). No
decryption attempted.
