# Source images

The disc images used during development, identified from their own contents
rather than from filenames. Hashes are recorded so findings can be reproduced
against exactly the same data.

**No image content is committed.** See [legal](../overview/legal.md).

## In use

| File | SHA-256 | Platform | Serial | Volume date |
| --- | --- | --- | --- | --- |
| `pulse-psp-usa.chd` | `65d8c19edaa4a65025a18346a2ccfe212a88053a5737a95d78c406efb69d02a9` | PSP | `UCUS-98712` | 2008-01-04 09:58:46 |
| `pulse-ps2-eu.chd` | `9b352295d4e35e3a4275ad2c1167aa3c2ab8bb7eb4b3abc877fd4b936b39e2fe` | PS2 | `SCES-54748` | 2009-05-15 17:11:10 |
| `pure-psp-usa.chd` | `851075e2a894524bd89703f2f930098c99302a488424328cf77ec33da325a3f1` | PSP | `UCUS-98612` | 2005-06-02 14:29:23 |
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
also present, this suggests the US build was derived from a European master.
Interesting but not currently actionable.

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
version of it is often the faster way to understand it.

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

**The PSP copy is US and the PS2 copy is EU.** Every PSP-versus-PS2 comparison
is therefore also a US-versus-EU comparison.

Before attributing any difference to the platform, rule out the region. A
matching-region pair, or a second copy of either release, would remove the
confound and is worth acquiring before the comparison work in M2 gets serious.

This warning belongs in every document under [comparisons](../comparisons/).

## Out of scope

`~/Downloads` also contains WipEout Fusion and WipEout 3. Both predate Pure and
are [out of scope](../overview/goals.md#scope). They are deliberately not copied
into `data/images/` and are not referenced anywhere in this project.
