# Wipeout Pulse (PSP) disc layout

Source: `pulse-psp-usa.chd`, serial **UCUS-98712**, volume date 2008-01-04.
See [source images](../reverse-engineering/source-images.md).

Reproduce with:

```sh
just unpack list  data/images/pulse-psp-usa.chd --dirs
just unpack sniff data/images/pulse-psp-usa.chd
```

## Contents

23 files in 9 directories, 354 MiB of a 436 MiB UMD.

| Size | LBA | Path | Signature | Entropy |
| ---: | ---: | --- | --- | ---: |
| 48 | 32 | `UMD_DATA.BIN` | disc ID text | 3.56 |
| 11,784 | 55200 | `PSP_GAME/ICON0.PNG` | PNG | 7.97 |
| 264,192 | 55296 | `PSP_GAME/ICON1.PMF` | PSMF video | 7.89 |
| 472 | 219696 | `PSP_GAME/PARAM.SFO` | `\0PSF` metadata | 3.24 |
| 148,577 | 55216 | `PSP_GAME/PIC1.PNG` | PNG | 7.99 |
| 25,924 | 55440 | `PSP_GAME/SND0.AT3` | RIFF / ATRAC3 | 7.98 |
| **3,854,564** | 55456 | **`PSP_GAME/SYSDIR/BOOT.BIN`** | **ELF** | **5.74** |
| 3,854,912 | 48 | `PSP_GAME/SYSDIR/EBOOT.BIN` | `~PSP` encrypted | 8.00 |
| 18,374,528 | 6048 | `PSP_GAME/SYSDIR/UPDATE/DATA.BIN` | `PSAR` firmware archive | 8.00 |
| 5,246,816 | 1952 | `PSP_GAME/SYSDIR/UPDATE/EBOOT.BIN` | `~PSP` encrypted | 8.00 |
| 2,080 | 1936 | `PSP_GAME/SYSDIR/UPDATE/PARAM.SFO` | `\0PSF` metadata | 2.86 |
| 958,720 | 61216 | `PSP_GAME/USRDIR/BEData.wad` | [WAD](../formats/wad.md) | 5.56 |
| **315,029,056** | 61696 | **`PSP_GAME/USRDIR/Data.wad`** | [WAD](../formats/wad.md) | 5.02 |
| 817,216 | 57392 | `PSP_GAME/USRDIR/FE.wad` | [WAD](../formats/wad.md) | 3.12 |
| 7,007,104 | 57792 | `PSP_GAME/USRDIR/FEData.wad` | [WAD](../formats/wad.md) | 1.14 |
| 21,584 | 219648 | `PSP_GAME/USRDIR/PRX/libfont.prx` | `~SCE` | 7.98 |
| 3,936 | 219664 | `PSP_GAME/USRDIR/PRX/libmp3.prx` | `~SCE` | 7.80 |
| 1,904 | 219680 | `PSP_GAME/USRDIR/PRX/pspnet_ap_dialog_dummy.prx` | `~SCE` | 7.50 |
| 6,956,112 | 219712 | `PSP_GAME/USRDIR/UCES00465/GSHARE/SHARE.BIN` | `\0PBP` | 7.99 |
| 21,584 | 57344 | `PSP_GAME/USRDIR/UCES00465/PRX/libfont.prx` | `~SCE` | 7.98 |
| 3,936 | 57360 | `PSP_GAME/USRDIR/UCES00465/PRX/libmp3.prx` | `~SCE` | 7.80 |
| 1,904 | 57376 | `PSP_GAME/USRDIR/UCES00465/PRX/pspnet_ap_dialog_dummy.prx` | `~SCE` | 7.50 |
| **8,429,869** | 215520 | **`PSP_GAME/USRDIR/UCES00465/SYSDIR/BOOT.BIN`** | **ELF** | **6.02** |

## Findings

### `BOOT.BIN` is an unencrypted ELF

**The most consequential finding so far.** `PSP_GAME/SYSDIR/BOOT.BIN` is a plain
ELF with entropy 5.74, which is what unencrypted MIPS code looks like.
`EBOOT.BIN`, alongside it and almost exactly the same size, is the `~PSP`
encrypted variant with entropy 7.997.

The reverse-engineering target therefore needs no decryption step: extract it
and load it straight into Ghidra.

```sh
just unpack extract data/images/pulse-psp-usa.chd \
    -o data/extracted/psp 'PSP_GAME/SYSDIR/BOOT.BIN'
```

Confirmed with `readelf`: `ELF32`, `MIPS R3000`, type `0xffa0` (PSP), flags
`0x10a23001` (`noreorder`, `eabi32`, `mips2`), 96 sections, 2.45 MiB of `.text`.
Stripped, so there are no symbols to help.

Confidence: **99**.

### Four WAD archives

`Data.wad` at 315 MiB is 89% of the disc's content and holds essentially
everything. The other three split out by area:

| File | Size | Likely role |
| --- | --- | --- |
| `Data.wad` | 315 MiB | Main asset archive: tracks, ships, textures, audio |
| `FEData.wad` | 6.7 MiB | Front end (menu) assets |
| `FE.wad` | 798 KiB | Front end structure or layout |
| `BEData.wad` | 936 KiB | "Back end", presumably in-race. **Unconfirmed.** |

The `FE`/`BE` split is inferred from the abbreviations and from Pure using the
same `FE` naming. `BE` as "back end" is a guess; confidence **55**.

`FEData.wad`'s entropy of 1.14 is remarkable, far below anything else on the
disc. That means long runs of repeated bytes, which suggests either heavy
padding or uncompressed sparse data such as bitmap fonts or UI masks.

The container format is documented in [formats/wad.md](../formats/wad.md).

### A European directory on a US disc

`PSP_GAME/USRDIR/UCES00465/` uses the serial of the **European** release of
Pulse, on a disc whose own `UMD_DATA.BIN` says `UCUS-98712`. The disc's ISO 9660
publisher field also reads **SCEE**, Sony Computer Entertainment Europe.

It contains:

- `GSHARE/SHARE.BIN`, 6.6 MiB, a `\0PBP` container. `GSHARE` almost certainly
  means *game sharing*, the PSP feature that transmits a playable subset to
  nearby consoles over ad-hoc wireless.
- Duplicates of the three `PRX` modules from `USRDIR/PRX/`, byte-for-byte
  identical in size.
- `SYSDIR/BOOT.BIN`, **8.4 MiB** and an unencrypted ELF.

That last one looked like a puzzle: it is more than twice the size of the main
`BOOT.BIN` (3.85 MiB), where a game-sharing payload should be a *subset*.

Comparing the two ELFs resolves it. Both are `ELF32`, `MIPS R3000`, type
`0xffa0` (PSP), flags `mips2 eabi32`, and **both are stripped** (`.symtab`
contains only the null entry). The section sizes are the interesting part:

| Section | `SYSDIR/BOOT.BIN` | `UCES00465/SYSDIR/BOOT.BIN` |
| --- | ---: | ---: |
| `.text` | 0x272a3c (2.45 MiB) | 0x15a2b4 (1.35 MiB) |
| `.data` | 0x29118 (164 KiB) | **0x6176c9 (6.09 MiB)** |
| `.rodata` | 0x385e4 (222 KiB) | 0x18220 (97 KiB) |
| `.bss` | 0xba7c0 (745 KiB) | 0x920b0 (585 KiB) |

The game-share build has **45% less code and 38 times more data**. That is
exactly the shape of a self-contained executable: the receiving PSP has no UMD,
so content the main build streams from `Data.wad` has to be linked in, while the
code for everything the shared subset cannot do is dropped.

So it is the game-share host executable, and not a leftover European master or
an unstripped build. The unstripped possibility would have been a gift for M2;
it is ruled out.

Confidence: **88**. The section profile is decisive about what kind of binary it
is; it has not been run or disassembled to confirm behaviour.

Reproduce with:

```sh
just unpack extract data/images/pulse-psp-usa.chd -o data/extracted/psp '*BOOT.BIN'
readelf -S -W data/extracted/psp/PSP_GAME/SYSDIR/BOOT.BIN
readelf -S -W data/extracted/psp/PSP_GAME/USRDIR/UCES00465/SYSDIR/BOOT.BIN
```

**Still unexplained:** why the directory carries the *European* serial on a US
disc, and why the disc's ISO 9660 publisher field reads SCEE. The
build-derived-from-a-European-master reading remains the best guess, at
confidence **50**, and nothing currently depends on it.

### Firmware update payload

`PSP_GAME/SYSDIR/UPDATE/` holds a PSP firmware updater: a `PSAR` archive and its
own encrypted `EBOOT.BIN`. Standard on retail UMDs and irrelevant to us.

### No network modules

Unlike Pure, which ships 26 `PRX` modules including the full `pspnet` stack and
`libhttp`, Pulse ships three: `libfont`, `libmp3` and a dummy network dialog.

Pure had downloadable content delivered over HTTP. Pulse's absent HTTP stack
suggests it either does not use online downloads or uses a different mechanism.
Worth knowing before any of M8's networking work.

## Layout observations

The on-disc ordering is not alphabetical and not directory-grouped. `EBOOT.BIN`
sits at LBA 48, near the start; `Data.wad` occupies LBA 61696 onward, the bulk
of the disc; `PARAM.SFO` and some `PRX` modules are at LBA 219648, near the end.

UMD seek times are significant, so this ordering is likely deliberate: files
needed at boot placed early, the streaming archive placed contiguously. If load
ordering ever becomes relevant, the LBA column above is the evidence.
