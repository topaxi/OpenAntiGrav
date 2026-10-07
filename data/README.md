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
    vita/       Decrypted Vita PKG content - see below
  cache/        Assets converted from your originals, rebuilt on demand
  dlc/          PSN DLC packages, as downloaded - see below
  keys/         zRIF strings and F00D cache for Vita PFS decryption - see below
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
| `pulse-psp-eu.chd` | Wipeout Pulse | PSP | Europe/Australia |
| `pulse-psp-usa.chd` | Wipeout Pulse | PSP | USA |
| `pulse-ps2-eu.chd` | Wipeout Pulse | PS2 | Europe |
| `pure-psp-eu.chd` | Wipeout Pure | PSP | Europe |
| `pure-psp-usa.chd` | Wipeout Pure | PSP | USA |
| `hdfury-ps3-eu.iso` | WipEout HD / Fury | PS3 | Europe |
| `data/extracted/ps3/hd-psn-eu/` | WipEout HD (no Fury) | PS3 | Europe (`NPEA00057`, v3.00), the **installed** PSN download folder (not an image) |
| `2048-vita-eu.pkg` | WipEout 2048 | Vita | Europe (`PCSF00007`) |
| `2048-vita-usa.pkg` | WipEout 2048 | Vita | USA (`PCSA00015`) |
| `omega-ps4-eu.pkg` | WipEout: Omega Collection | PS4 | Europe (`CUSA05670`), base v1.00 |
| `omega-ps4-eu-patch.pkg` | WipEout: Omega Collection | PS4 | Europe (`CUSA05670`), v1.07 patch |

`.iso` works anywhere `.chd` does; the tools sniff the container. **When a title has both a Europe and a USA image, `oag-game` opens the Europe one**; name the USA one to play it.

### The PS3 image is encrypted, and is read that way

`hdfury-ps3-eu.iso` is a standard redump encrypted image: the ISO 9660
filesystem is plaintext (`PS3_DISC.SFB`, `PARAM.SFO` and `ICON0.PNG` all read
normally, and `oag-unpack list` walks it fine), while everything under
`PS3_GAME/USRDIR/` - `DATA00..06.PSARC`, `EBOOT.BIN`, `DFENGINE.SPRX` - is
AES-encrypted under the disc key. `oag-disc` decrypts sector by sector as it
reads, with the key from `hdfury-ps3-eu.dkey` beside the image (this pressing's
redump `.dkey`, git-ignored and never committed), from `~/.config/oag/keys/`, or
entered in the chooser. `hdfury-ps3-eu-dec.iso` is a decrypted copy kept for
tests that compare the two. For Ghidra the SELF still needs `scetool` (or
`rpcs3 --decrypt`) and [Ps3GhidraScripts](https://github.com/clienthax/Ps3GhidraScripts)
for the PPC64/PRX side; that is layer 2 and is not touched here.

### The PSN Wipeout HD download is read from its installed folder

`data/extracted/ps3/hd-psn-eu/` holds what RPCS3's package installer wrote for
`NPEA00057` (v3.00): `PARAM.SFO` and `USRDIR/data01.psarc`..`data04.psarc`, plain
archives. Steps, and what that copy lacks against the Fury disc, are in
[`docs/overview/installing.md`](../docs/overview/installing.md#wipeout-hd-from-the-psn-download)
and [`docs/formats/hd-psn.md`](../docs/formats/hd-psn.md). The licence file the
download comes with is yours and is never committed.

### Vita PKGs decrypt in four steps; `data/extracted/vita/` holds the result

**The engine itself no longer needs steps 1 and 2 for a game it reads**: a
NoNpDrm `.vpk` (a ZIP of the installed folder, plain files) is read in place, see
[`docs/formats/vita-package.md`](../docs/formats/vita-package.md) and
`scripts/make-test-vpk.sh`, which builds test `.vpk` files into `data/test-vpk/`
from the extracted folders below. A `.pkg` is refused by name: step 2's PFS key is
derived by the console's F00D (which `psvpfsparser` reaches over a key service),
and three offline derivations tried against a known file all missed.

This is the checkout's own location, and the one every RE script below names
directly. It is not the only place the launcher looks: on a portable or Steam
Deck build with no `data/` beside the running AppImage,
[`docs/tools/packaging.md#where-the-disc-image-comes-from`](../docs/tools/packaging.md#where-the-disc-image-comes-from)
documents the two other places a 2048 extract is found, and
[`scripts/deploy-to-deck.sh`](../scripts/deploy-to-deck.sh) is what puts it
there.

In a 2048 PKG, `sce_sys/param.sfo` and `sce_pfs/files.db` (magic `SCENGPFS`)
are plaintext, but everything inside the PFS layer is encrypted - `eboot.bin`,
`PSP2/data1.psarc`, `data2.psarc`, even the manual PNGs. On top of that,
`eboot.bin` itself is an NpDrm-encrypted SELF, a second, independent layer
PFS decryption does not touch. Four tools, each solving one layer:

1. `pkg2zip -x <pkg>` strips the PKG's outer AES-CTR layer. This alone
   produces a readable directory tree still full of high-entropy files - it
   ran without a zRIF, so the PFS layer underneath is still opaque. Not
   wrapped in a project script; it is a single well-known command run
   directly against `data/images/2048-vita-{usa,eu}{,-patch}.pkg` or a
   `data/dlc/*.pkg`.
2. `just build-psvpfstools` builds `psvpfsparser`, which decrypts that PFS
   layer given the title's zRIF (or raw klicensee) - see
   `scripts/build-psvpfstools.sh` for the four upstream build fixes it needed.
   A patch pkg reuses its base app's zRIF (same content ID, same license) -
   checked directly, not assumed. This gets you a *readable* `eboot.bin` -
   `readelf` parses its ELF header and phdrs cleanly - but not yet a *plain*
   one; see step 4.
3. `just zrif-to-klicensee <zrif>` (`scripts/zrif-to-klicensee.py`) decodes
   the same zRIF back to its 16-byte klicensee - pure zlib with a preset
   dictionary, no AES, no console-derived secret. zRIFs and the derived
   klicensee live in `data/keys/vita-zrif.tsv`, see `data/keys/README.md` for
   how they were recovered and why this is a *different, lighter* layer than
   `psvpfsparser`'s own `-f00d_url`/`-f00d_cache` (which derives the PFS key
   and does need the F00D service).
4. `eboot.bin` past step 2 is a plain ELF header and phdrs wrapped around
   still-NpDrm-encrypted code/data segments - confirmed directly (not zlib,
   ~8 bits/byte entropy), not something Ghidra's Vita loader can open.
   `just vita-self-decrypt <path> -k <klicensee>` (`scripts/vita-self-decrypt.py`)
   decrypts and decompresses every segment and places it at its real ELF
   offset. [VitaLoaderRedux](https://github.com/CreepNT/VitaLoaderRedux)
   (`just build-vita-loader-redux`) is what actually reads the result in
   Ghidra; install is manual (File > Install Extensions). An earlier
   `strip-vita-self.py` only did the *header* half of this step and left the
   segments encrypted - removed once `vita-self-decrypt.py` did the whole job.

All seven of WipEout 2048's PKGs (USA + EUR base, patch v1.04, DLC1/DLC2) have
been run through steps 1-2; `data/extracted/vita/{PCSA00015,PCSF00007}/{base,
patch-v104,dlc1,dlc2}/` holds the result. Base and patch `eboot.elf` for both
regions have been through steps 3-4 and verified three ways, not assumed:
`readelf` reports clean `PT_LOAD`/`PT_SCE_VERSION` phdrs, disassembly at
`0x81000000` is real ARM Thumb-2, and all four import into VitaLoaderRedux
(language `ARM:LE:32:v7`, base `0x81000000`) - see
[toolchain.md#vita](../docs/reverse-engineering/toolchain.md#vita) for the
full trail. DLC PKGs carry no binaries at all, only `dlcN.psarc` +
`sce_sys` metadata - checked directly against the decrypted tree, not
inferred from the PKG type.

The four programs are imported (`/vita-2048-{eu,usa}-{v104,base}/eboot.elf`),
default target `/vita-2048-eu-v104/eboot.elf`. Naming still has no
`docs/ghidra/functions/vita-2048-*/` or `names.tsv` - those wait on a first
recovered name, per [ADR-0005](../docs/architecture/adr/0005-ghidra-conventions.md)'s
own workflow. See `HANDOVER.md`'s open threads for the current state of that
work.

Note what these two share and Pulse/Pure do not: **HD/Fury and 2048 both ship
PSARC archives rather than WADs**, and 2048 arrives as a PKG rather than a disc
filesystem at all. That is why the content-source layer is written against
"archives resolved by name from a source's own file list" rather than "a disc
image containing WADs" - see
[ADR-0022](../docs/architecture/adr/0022-title-packages.md).

**`pulse-psp-eu.chd` is one of `oag-game`'s auto-detected `IMAGE_NAMES`, as of
2026-09-07.** It used to be excluded on the reasoning that it was a
reverse-engineering reference disc rather than a played-from one; that
reasoning no longer holds now that
[ADR-0048](../docs/architecture/adr/0048-eu-is-the-psp-pulse-re-target-of-record.md)
makes it the Ghidra target of record too, and a player whose only copy is the
EU disc was always able to play it (it still matched the generic
any-`.chd`/`.iso` fallback `oag-game`'s directory scan also does), just without
the "known name" priority its USA sibling and Pure's own EU image already get.
A directory holding both Pulse discs still resolves to the USA one by default,
unchanged - see `IMAGE_NAMES`'s own doc comment in
`crates/game/src/source.rs` for why the order was left alone. It is a
genuinely different `BOOT.BIN` from `pulse-psp-usa.chd`'s, not a re-labelled
copy - see [source-images.md](../docs/reverse-engineering/source-images.md).

**Pure and HD/Fury boot too**: each has a front end of its own and a race
loads on its own data, though neither is played past the menus as far as
Pulse is (roadmap M8, [status](../docs/overview/status.md)). `oag-game` finds
the title from the image itself, so a Pure disc is never mistaken for Pulse,
and an encrypted HD `.iso` is reported as encrypted rather than failing with a
message about another game
([installing](../docs/overview/installing.md)).

### The Omega Collection PKG decrypts on a PC; the archives are plain, the executable isn't yet

`omega-ps4-eu.pkg` and its `-patch.pkg` are a scene "fake PKG" (fPKG) rip
meant for installation on a jailbroken PS4 via HEN, not a retail PSN download
- confirmed directly from the header (`content_id
EP9000-CUSA05670_00-WIPEOUTOMEGA00EU`) and the release's own NFO. Unlike a
retail PKG, that turns out not to require console-derived key material at
all: [LibOrbisPkg](https://github.com/maxton/LibOrbisPkg)'s `PkgTool.Core
pkg_extract` decrypts it fully on a PC, producing the five `dataNN.psarc`
asset archives (the same plaintext container `hdfury-ps3-eu.iso` ships) and
`eboot.bin` - which is itself still a wrapped PS4 SELF and needs a further
decrypt step no tool here has attempted yet. Full writeup, the exact command,
and the SELF caveat are in
[source-images.md](../docs/reverse-engineering/source-images.md#omega-ps4-eupkg--omega-ps4-eu-patchpkg---wipeout-omega-collection-ps4).
Omega Collection stays listed as "if feasible" in the roadmap because no
format/title crate reads PS4 content yet, not because of a remaining crypto
blocker on the asset side - see
[`docs/overview/roadmap.md`](../docs/overview/roadmap.md) and
[`docs/future-2048/shared-concepts.md`](../docs/future-2048/shared-concepts.md).

Confirm what you have with:

```sh
cargo run -p oag-tools --bin oag-unpack -- info data/images/pulse-psp-eu.chd
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
actual payload, `pi.wad`. `pi.wad` measures 8.0000 bits/byte across its whole
length (a plain WAD sits well below that; Pulse's `PACKn.edat` above does
too), and `oag-wad` refuses the raw file outright - but **it decrypts**.
Neither `BOOT.BIN` nor Pure's bundled PRXs import any NpDrm or hashing
function in either region (checked directly, unlike Pulse's single-argument
`sceNpDrmEdataSetupKey`, no per-title key), because the game hand-rolls its
own 8-round XTEA in application code; the per-pack keys are public, from an
external tool, and kept at
[`data/keys/pure-dlc-keys.txt`](keys/README.md#pure-dlc-keystxt). Decrypting
each of the seven packs with its own key turns `pi.wad` into a file
`oag-wad` parses unmodified, and the Gamma pack's manifest confirms
`Van_Uber` is Pure DLC, shipped as team id `Vanuber` (no underscore) - see
[dlc-pack.md](../docs/formats/dlc-pack.md#pures-packs-decrypt-with-an-external-key-table)
for the algorithm and the verification. `PARAM.sfo` is Sony's standard,
unencrypted metadata format (same `\0PSF` magic as the disc's own
`PARAM.SFO`) - the Gamma pack names itself `TITLE=Gamma Pack`,
`SAVEDATA_TITLE=Wipeout Pure`. Not yet wired into `oag_assets::dlc` - see the
open handover thread.
