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
| `hdfury-ps3-eu.iso` | `6702a1b064c966e30ad878f7dded9a787bbab459ed4a7232007f6f1e7870f987` | PS3 | unknown | 2009-08-28 10:02:56 |

Reproduce with `just hash-images`.

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

The primary reverse-engineering target. `PSP_GAME/SYSDIR/BOOT.BIN` is an
unencrypted ELF.

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

Not the reverse-engineering target of record - `pulse-psp-usa.chd` keeps that
role, and this project's whole Ghidra database (`docs/ghidra/functions/psp-pulse-usa/`)
is address-keyed to it. This disc's `BOOT.BIN` is imported into Ghidra as a
**separate, second program** (`/psp-pulse-eu/BOOT.BIN`, 10,671 functions) for
corroboration and future diffing, not as a replacement.

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
imported into Ghidra at `/psp-pure-usa/BOOT.BIN` (6,927 functions), purely for
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
at `/psp-pure-eu/BOOT.BIN` (6,934 functions - close to the USA disc's 6,927,
unlike the large, unexplained gap on the Pulse EU import; see `HANDOVER.md`)
for the same opportunistic-corroboration reason as the USA disc above.

### `hdfury-ps3-eu.iso` - Wipeout HD / Fury, PS3

```
container      raw ISO, 1103104 sectors (2.1 GiB capacity)
platform       unknown (no PS3 identification yet)
volume id      PS3VOLUME
contents       22 files in 5 directories, 2.1 GiB
```

Later target per [scope](../overview/goals.md#scope). `oag-disc` has no PS3
serial/publisher identification yet, so `platform`/`serial` read `unknown` -
that's expected, not a bug. Not otherwise explored yet.

## Region asymmetry

**The PSP copy used as the reverse-engineering target is US and the PS2 copy
is EU.** Every PSP-versus-PS2 comparison is therefore also a US-versus-EU
comparison. `pulse-psp-eu.chd` (acquired 2026-08-04, see above) is a second
PSP copy, but it is EU too, so it does not by itself resolve this - a
PSP-versus-PS2 comparison still has no matching-region pair to compare from.
What it does give: a same-platform US-versus-EU pair (`pulse-psp-usa.chd` vs
`pulse-psp-eu.chd`) to isolate region-only differences from platform-only
ones, which was not previously possible at all.

Before attributing any difference to the platform, rule out the region. A
matching-region pair, or a second copy of either release, would remove the
confound and is worth acquiring before the comparison work in M2 gets serious.

This warning belongs in every document under [comparisons](../comparisons/).

## Out of scope

`~/Downloads` also contains WipEout Fusion and WipEout 3. Both predate Pure and
are [out of scope](../overview/goals.md#scope). They are deliberately not copied
into `data/images/` and are not referenced anywhere in this project.
