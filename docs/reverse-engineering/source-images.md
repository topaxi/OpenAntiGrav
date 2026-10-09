# Source images

The disc images used during development, identified from their own contents
rather than from filenames. Hashes are recorded so findings can be reproduced
against exactly the same data.

**No image content is committed.** See [legal](../overview/legal.md).

## In use

| File | SHA-256 | Platform | Serial | Volume date |
| --- | --- | --- | --- | --- |
| `pulse-psp-usa.chd` | `65d8c19edaa4a65025a18346a2ccfe212a88053a5737a95d78c406efb69d02a9` | PSP | `UCUS-98712` | 2008-01-04 09:58:46 |
| `pulse-psp-eu.chd` | `314e01f6100cd0cd76686ec4f29674f855d7be2e604db112276b121204f0e18a` | PSP | `UCES-00465` | 2007-11-06 14:42:47 |
| `pulse-ps2-eu.chd` | `9b352295d4e35e3a4275ad2c1167aa3c2ab8bb7eb4b3abc877fd4b936b39e2fe` | PS2 | `SCES-54748` | 2009-05-15 17:11:10 |
| `pure-psp-usa.chd` | `851075e2a894524bd89703f2f930098c99302a488424328cf77ec33da325a3f1` | PSP | `UCUS-98612` | 2005-06-02 14:29:23 |
| `pure-psp-eu.chd` | `9390d1ff457be45c518bff4aa66974b3c4cf4c4f0d0c2388d770600b41ec264f` | PSP | `UCES-00001` | 2005-06-29 14:18:53 |
| `hdfury-ps3-eu.iso` | `6702a1b064c966e30ad878f7dded9a787bbab459ed4a7232007f6f1e7870f987` | PS3 | `BCES-00664` | 2009-08-28 10:02:56 |
| `omega-ps4-eu.pkg` | `c8cbd2c2063efb41f6c881b601d4643f20236608ef5a347554f278035456a022` | PS4 | `CUSA05670` | n/a (PKG, not a disc) |
| `omega-ps4-eu-patch.pkg` | `16140a67fdfd8c6c367e56568218adb2d370ca7cd16cf82221b9352abd3f357d` | PS4 | `CUSA05670` | n/a (PKG, not a disc) |

Reproduce with `just hash-images`. It hashes everything in `data/images/`, so it
also reports any *derived* file kept there - a decrypted copy, a disc key - which
the table above deliberately does not list. Only source images belong in it.

## Details

### `pulse-psp-usa.chd` - Wipeout Pulse, PSP

```
container      CHD, 223120 sectors (436 MiB capacity)
platform       PSP
serial         UCUS-98712
boot           PSP_GAME/SYSDIR/EBOOT.BIN
identified by  UCUS-98712|60E11EE30E97B973|0001|G
volume id      SCEE
publisher      SCEE
contents       23 files in 9 directories, 354 MiB
```

**Corroboration source, not the target of record** - see `pulse-psp-eu.chd`
below and [ADR-0048](../architecture/adr/0048-eu-is-the-psp-pulse-re-target-of-record.md).
It held that role until 2026-08-05; `PSP_GAME/SYSDIR/BOOT.BIN` is an
unencrypted ELF either way.

Note the publisher: **SCEE**, Sony Computer Entertainment *Europe*, on a US
disc. Combined with the [`UCES00465` directory](../psp/pulse-disc-layout.md)
also present, this suggests the US build was derived from a European master -
now directly checkable, see `pulse-psp-eu.chd` below.

### `pulse-psp-eu.chd` - Wipeout Pulse, PSP, EU/Australia

```
container      CHD, 223072 sectors (436 MiB capacity)
platform       PSP
serial         UCES-00465
boot           PSP_GAME/SYSDIR/EBOOT.BIN
identified by  UCES-00465|91769C25134BAD9B|0001|G
volume id      SCEE
publisher      SCEE
contents       23 files in 9 directories, 354 MiB
```

Acquired 2026-08-04 specifically to close the "region asymmetry" gap noted
below - the PSP disc this project had was US, the PS2 disc EU, so every
PSP-versus-PS2 comparison was also confounded with a US-versus-EU one. This
does not remove that confound (it is still a PSP-versus-PS2 pair with no
common region), but it does let the earlier "the US disc's executable looks
like it was derived from a European master" observation be checked directly
against a real EU build rather than only inferred from strings.

**`PSP_GAME/SYSDIR/BOOT.BIN` is a genuinely different binary from
`pulse-psp-usa.chd`'s**, not the same executable under a different disc
wrapper: 3,844,732 bytes here versus 3,854,564 on the USA disc, different
SHA-256. So the USA disc's boot strings claiming an EU build does not mean
byte-identical code - it is a distinct build, not yet diffed against the
primary target. One hypothesis, unconfirmed: the difference is mostly in
networking code, since PSP Wipeout Pulse's multiplayer used region-specific
services. Not yet checked against the binary.

**The reverse-engineering target of record as of 2026-08-05** - see
[ADR-0048](../architecture/adr/0048-eu-is-the-psp-pulse-re-target-of-record.md).
`pulse-psp-usa.chd` is now the corroboration source; the bulk of this
project's Ghidra database (`docs/ghidra/functions/psp-pulse-usa/`) still
carries far more named functions than `psp-pulse-eu/` does (656 rows against
301, measured 2026-09-07), which is the cost the ADR is explicit about, not a
sign the roles are reversed from what this paragraph now says. This disc's
`BOOT.BIN` is imported into Ghidra as its own program (`/pulse/BOOT-psp-pulse-eu.BIN`,
10,671 functions).

### `pulse-ps2-eu.chd` - Wipeout Pulse, PS2

```
container      CHD, 1900848 sectors (3.6 GiB capacity)
platform       PS2
serial         SCES-54748
boot           cdrom0:\SCES_547.48;1
volume id      1
publisher      SCEE
contents       20 files in 4 directories, 3.6 GiB
```

The PS2 release was Europe-only. **The container is unusual**: it is a DVD image
packed with `chdman createcd`, so the header reports 2448-byte CD units with a
single `MODE1` track of 1,900,848 frames. The sectors inside are ordinary
2048-byte user data. See [PS2 disc layout](../ps2/pulse-disc-layout.md) for why
this matters.

2.75 GiB of the 3.6 GiB is a single padding file, `PADZ2`. Actual content is
around 1 GiB.

### `pure-psp-usa.chd` - Wipeout Pure, PSP

```
container      CHD, 123520 sectors (241 MiB capacity)
platform       PSP
serial         UCUS-98612
volume id      SCEE
publisher      SCEA
contents       38 files in 4 directories, 241 MiB
```

Not a target in its own right yet. It is here as the **format ancestor**: it
uses the same `Data.wad` / `FE.wad` / `FEData.wad` structure as Pulse, with the
same container header shape. Where a Pulse format is ambiguous, Pure's simpler
version of it is often the faster way to understand it. `BOOT.BIN` is
imported into Ghidra at `/pure/BOOT-psp-pure-usa.BIN` (6,927 functions), purely for
opportunistic corroboration alongside the format work above - not a claim
that Pure's simulation is a target, which ADR-0009 still defers.

### `pure-psp-eu.chd` - Wipeout Pure, PSP, EU

```
container      CHD, 172224 sectors (336 MiB capacity)
platform       PSP
serial         UCES-00001
boot           PSP_GAME/SYSDIR/EBOOT.BIN
identified by  UCES-00001|381EA7A814902283|0001|G
volume id      SCEE
publisher      SCEE
contents       82 files in 18 directories, 308 MiB
```

Acquired 2026-08-04, the same pass that acquired `pulse-psp-eu.chd`, for the
same reason: `pure-psp-usa.chd` above is the only Pure disc this project had,
and a second region gives a same-title comparison pair the way it did for
Pulse. `UCES-00001` is a very low serial - Pure was one of the PSP's EU
launch titles. Not a reverse-engineering target in the ADR-0009 sense (no
simulation work is planned for Pure), but `BOOT.BIN` is imported into Ghidra
at `/pure/BOOT-psp-pure-eu.BIN` (6,934 functions - close to the USA disc's 6,927,
unlike the large, unexplained gap on the Pulse EU import; see `HANDOVER.md`)
for the same opportunistic-corroboration reason as the USA disc above.

### `hdfury-ps3-eu.iso` - Wipeout HD / Fury, PS3

```
container      raw ISO, 1103104 sectors (2.1 GiB capacity)
platform       PS3
serial         BCES-00664
boot           PS3_GAME/USRDIR/EBOOT.BIN
identified by  BCES-00664
volume id      PS3VOLUME
contents       22 files in 5 directories, 2.1 GiB
```

Later target per [scope](../overview/goals.md#scope). `oag-disc` identifies PS3
discs as of 2026-08-17, from `PS3_DISC.SFB`'s `TITLE_ID` field. That file sits in
a plain region, so the disc identifies while still encrypted - identification
never needs a key, and the ground-truth test asserts exactly that. The title is
`WipEout(R) HD Fury`, version 01.03, from `PS3_GAME/PARAM.SFO`, which no tool
here parses yet.

**The image is encrypted, and 1.85 GiB of it - every `.psarc`, `DFEngine.sprx`
and `EBOOT.BIN` - is unreadable without the disc's own 16-byte key.** The key is
never inside a PC dump; it comes from BD-drive authentication, which is why
redump publishes a `.dkey` per disc separately. Layout, cipher, where the key
comes from and how a candidate key is proved correct are all in
[PS3 disc encryption](../formats/ps3-disc.md).

Decrypted 2026-08-17 and verified: three files this disc ships twice, once in a
plain region and once inside an encrypted one, are byte-identical afterwards.

```sh
python3 scripts/ps3iso.py decrypt data/images/hdfury-ps3-eu.iso <key> out.iso
```

Neither the key nor the decrypted image is committed, and neither belongs in the
table above - `data/images/hdfury-ps3-eu-dec.iso` is derived, regenerable in
about five seconds, and hashes to
`a1e2aef2beb9ea489f29b3d5379f05881164f1cceefcd518de5107244d795c16`.

**Nothing has been read out of the assets.** `.psarc` is a documented Sony
container and the archives parse, but no `oag-formats` reader exists for it, so
no texture, model or table has come off this disc.

**The executable is a different story.** `rpcs3 --decrypt` turns `EBOOT.BIN`
into a PPC64 ELF with no firmware install needed, and that ELF is imported and
analysed - 26,100 functions, with a first pass of names under
[`docs/ghidra/functions/ps3-hdfury-eu/`](../ghidra/functions/ps3-hdfury-eu/).
Reproduce the whole import with `scripts/import-ps3-eboot.sh`; the procedure and
its traps are in [toolchain.md](toolchain.md#ps3). Neither the decrypted ELF nor
the decrypted image is committed.

### `omega-ps4-eu.pkg` / `omega-ps4-eu-patch.pkg` - WipEout: Omega Collection, PS4

```
container      PS4 PKG
content id     CUSA05670 (EP9000-CUSA05670_00-WIPEOUTOMEGA00EU)
region         EU, from the content ID's own suffix - the release's bundled
               metadata.json claims "USA", which does not match and is not
               trusted over the header
platform       PS4
```

Acquired 2026-09-15 as a scene "fake PKG" (fPKG) rip meant for installation on
a jailbroken PS4 via HEN, per the release's own NFO - not a PSN retail
download. **It decrypts on a PC with no console involved, confirmed by
actually running the extraction, not inferred from the fPKG label:**

```sh
git clone --depth 1 https://github.com/maxton/LibOrbisPkg.git
cd LibOrbisPkg/PkgTool.Core && dotnet build -c Release
dotnet bin/Release/netcoreapp3.0/PkgTool.Core.dll pkg_extract \
  data/images/omega-ps4-eu.pkg data/extracted/ps4/omega-eu
```

[LibOrbisPkg](https://github.com/maxton/LibOrbisPkg)'s `PkgTool.Core` (LGPL-3.0, not MIT as this page said until 2026-10-07,
built against `netcoreapp3.0`, needs `<RollForward>LatestMajor</RollForward>`
added to its `.csproj` to run on a newer-only-installed SDK - reconfirmed
2026-09-15 against a fresh clone and dotnet 10) produced a full `uroot/`
tree: `eboot.bin`, `sce_sys/`, `sce_module/*.prx`, and five `dataNN.psarc`
archives (`data00`-`data04`). **Measured precisely 2026-09-15** (the "~25 GiB"
this page recorded earlier was a rough estimate, not a repro'd figure):
13 GiB, 11 GiB, 9.1 GiB, 2.5 GiB and 6.0 GiB respectively, ~41.6 GiB total
with `eboot.bin` and the rest. Each `.psarc` starts with a plain `PSAR`
header (`zlib` compression, matches the container
[`hdfury-ps3-eu.iso`](#hdfury-ps3-euiso---wipeout-hd--fury-ps3) already
ships) - the asset archives are genuinely plaintext, no further key needed.
`eboot.bin` is not immediately readable either: its header is `4F153D1D`,
the PS4 SELF magic, the same wrapper Vita's `eboot.bin` needed
`vita-self-decrypt.py` to get past. Here that turned out to be a non-issue -
[toolchain.md#ps4](toolchain.md#ps4) parses the SELF header directly and
finds all 10 segments already unencrypted for this build, and
[GhidraOrbis](https://github.com/astrelsky/GhidraOrbis) (`just
build-ghidra-orbis`) is the loader that reads it from there, no separate
decrypt tool needed the way Vita's is.

**Correction, 2026-09-27: `pkg_extract` as built above reproducibly drops
content, and both extraction commands on this page need a one-line patch to
this tool before they are re-run.** `LibOrbisPkg`'s `PFS/PFSCReader.cs::ReadSector`
decompresses each 64 KiB PFSC sector with a single, non-looped
`DeflateStream.Read(output, 0, hdr.BlockSz)` call and discards how many
bytes it actually returned - `Stream.Read` is never guaranteed to fill the
requested count in one call, and here it routinely does not, leaving the
untouched remainder of the sector as zero. This is the root cause of the
"garbage"/"all-zero" split
[`psarc.md`](../formats/psarc.md#block-data-location-and-the-short-read-extraction)
documents on every archive extracted this way, confirmed by patching
`ReadSector` to loop the `Read` call to completion, rebuilding, and
re-reading the same bytes straight out of `omega-ps4-eu.pkg` - full detail
and the exact patch in
[`gnf.md`'s "Root cause"](../formats/gnf.md#root-cause-confidence-90-a-short-streamread-in-the-extraction-tool-not-this-projects-reader).
Apply that patch to the `LibOrbisPkg` checkout before the `dotnet build` step
below and both `pkg_extract` invocations on this page recover real content
that the unpatched tool silently zeroed - re-extraction with the fix landed
2026-09-27 under a scratch directory, not kept and
became `data/extracted/ps4/{omega-eu,omega-eu-patch}` 2026-09-29 (the short-read
copy is `data/extracted/ps4.bak`; the old scratch paths are symlinks now); see
`omega-psarc.md` for the before/after numbers.

**`omega-ps4-eu-patch.pkg` extracts separately and adds content, rather than
completing the base `.pkg`'s.** `pkg_extract` takes exactly one `.pkg` and
one output directory - checked directly against `PkgTool/Program.cs`'s own
source, no patch-chain or base/patch merge logic anywhere in the tool - so
running it against the patch needs its own output directory:

```sh
dotnet bin/Release/netcoreapp3.0/PkgTool.Core.dll pkg_extract \
  data/images/omega-ps4-eu-patch.pkg data/extracted/ps4/omega-eu-patch
```

Its `uroot/` holds `eboot.bin` (a newer build than the base `.pkg`'s),
`sce_discmap.plt`/`sce_discmap_patch.plt`, `sce_module/*.prx`, and **four**
`dataNN.psarc` archives with names the base `.pkg` does not have at all -
`data05` (654 MiB), `data07` (20 MiB), `data08` (5.3 GiB), `data09`
(6.9 MiB), no `data06` - and, notably, **no `data00`-`data04`**. The title's
asset namespace is at least nine named archives across the two packages, not
the five the base `.pkg` alone suggests; what `data08` (by far the largest of
the four) actually holds is not yet surveyed.
[`psarc.md`'s](../formats/psarc.md#block-data-location-and-the-short-read-extraction)
"Block data location" section has the full account of why this closes the
extraction-provenance question for the base archives' still-open real/zero
split, rather than answering it.

**Read in place since 2026-10-07**: `oag_disc::ps4_pkg` reads both packages without this extraction, and its output equals the corrected `data/extracted/ps4` byte for byte ([ps4-package.md](../formats/ps4-package.md)). The extraction stays the way to make that folder.

**The executable is an RE target as of 2026-09-15** - see
[`docs/ghidra/functions/ps4-omega-eu/README.md`](../ghidra/functions/ps4-omega-eu/README.md)
for why (native x86-64, no custom Ghidra processor module, and a proven
cross-binary corroboration technique against HD/Fury and 2048). Gameplay/
simulation work is still unplanned, per [the roadmap](../overview/roadmap.md)
and [future-2048/shared-concepts.md](../future-2048/shared-concepts.md); the
asset side (the five `.psarc` archives, same container `oag-hd` already
parses) has no crypto blocker left either way, only the format/title-crate
work itself.
The extraction above was verified and then deleted (it was written to `/tmp`,
not `data/`, and ate most of a 32 GiB tmpfs doing it - rerun straight to
`data/extracted/ps4/`, which is gitignored, instead).

## Region asymmetry

**Resolved for the two targets of record, as of** [ADR-0048](../architecture/adr/0048-eu-is-the-psp-pulse-re-target-of-record.md).
Until 2026-08-05, the PSP target was US and the PS2 target was EU, so every
PSP-versus-PS2 comparison was also a US-versus-EU one. Flipping the PSP target
to `pulse-psp-eu.chd` puts both platform targets in the same region - one of
that ADR's own stated reasons for the flip. A same-platform US-versus-EU pair
(`pulse-psp-usa.chd` vs `pulse-psp-eu.chd`) still exists and is still useful
for isolating a region-only difference from a platform-only one; it is just no
longer the *only* pairing available where PSP and PS2 are concerned.

The general warning still applies wherever a comparison mixes a corroboration
binary of one region with a target of record from another - check which
region each side of any comparison is before attributing a difference to the
platform rather than the region. This warning belongs in every document under
[comparisons](../comparisons/).

## Out of scope

`~/Downloads` also contains WipEout Fusion and WipEout 3. Both predate Pure and
are [out of scope](../overview/goals.md#scope). They are deliberately not copied
into `data/images/` and are not referenced anywhere in this project.
