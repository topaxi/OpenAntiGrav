# `oag-unpack`

Inspects and extracts PSP UMD, PS2 DVD and PS3 BD-ROM disc images. Handles CHD
and raw ISO, detected by magic rather than by file extension, so a CHD named
`.iso` still works.

A PS3 disc identifies from `PS3_DISC.SFB`, which is never encrypted, so `info`
and `list` work on an encrypted image as shipped. Reading a file *out* of one is
a different matter - see [PS3 disc encryption](../formats/ps3-disc.md).

```
oag-unpack info      <image>              platform, serial, volume descriptor
oag-unpack container <image>              CHD header and track metadata
oag-unpack list      <image> [pattern]    file listing
oag-unpack extract   <image> -o <dir>     extract files
oag-unpack hexdump   <image> <file>       dump bytes from a file on the disc
oag-unpack sniff     <image> [pattern]    magic and entropy triage
```

`pattern` is a glob supporting `*` and `?`, matched case-insensitively. `*`
crosses `/`, so `*.wad` means every WAD anywhere.

## `info`

Identifies the disc from its own contents: `UMD_DATA.BIN` for PSP,
`SYSTEM.CNF` for PS2. Never guesses from the filename.

```sh
$ oag-unpack info data/images/pulse-psp-usa.chd
image          data/images/pulse-psp-usa.chd
container      CHD
capacity       223120 sectors, 436 MiB

platform       PSP
serial         UCUS-98712
boot           PSP_GAME/SYSDIR/EBOOT.BIN
identified by  UCUS-98712|60E11EE30E97B973|0001|G

volume id      SCEE
system id      PSP GAME
publisher      SCEE
application    PSP GAME
created        2008010409584600
block size     2048 bytes
volume size    223120 sectors

contents       23 files in 9 directories, 354 MiB
```

## `container`

Reports what the container claims to be, without mounting a filesystem. **Use
this when `info` fails**: it usually explains the failure.

```sh
$ oag-unpack container data/images/pulse-ps2-eu.chd
chd version    5
compressed     true
hunks          237606 x 19584 bytes
units          1900848 x 2448 bytes  (CD + 96-byte subcode)
logical size   4.3 GiB (4653275904 bytes)
metadata       1 entries
  [CHT2] TRACK:1 TYPE:MODE1 SUBTYPE:NONE FRAMES:1900848 PREGAP:0 ...
```

That output is what identified the PS2 image as a DVD packed with
`chdman createcd`. See [PS2 disc layout](../ps2/pulse-disc-layout.md).

## `list`

```sh
oag-unpack list data/images/pulse-psp-usa.chd
oag-unpack list data/images/pulse-psp-usa.chd '*.wad'
oag-unpack list data/images/pulse-psp-usa.chd --dirs --by-size
```

`--dirs` includes directories, `--by-size` sorts largest first. The LBA column
is the on-disc sector, which is how the layout ordering in the disc documents
was established.

## `extract`

```sh
oag-unpack extract data/images/pulse-psp-usa.chd -o data/extracted/psp
oag-unpack extract data/images/pulse-psp-usa.chd -o data/extracted/psp '*BOOT.BIN'
```

Existing files are skipped unless `--force` is given.

Disc paths are validated before use: a record containing `..` or an absolute
component is refused rather than written outside the output directory. Disc
paths come from a file we did not author, so they are not trusted.

## `hexdump`

```sh
oag-unpack hexdump <image> PSP_GAME/USRDIR/FE.wad
oag-unpack hexdump <image> PSP_GAME/USRDIR/FE.wad --offset 400 --length 128
```

Seeks to the sector containing `--offset`, so poking at a structure 300 MB into
an archive costs one sector read rather than 300 MB of decompression.

## `sniff`

The triage command. Reports magic bytes and Shannon entropy per file, and counts
how many have no recognised signature. **Those are the reverse-engineering
targets.**

```sh
$ oag-unpack sniff data/images/pulse-psp-usa.chd
        SIZE  ENTROPY  DENSITY      SIGNATURE                     PATH
     3854564    5.738  structured   ELF executable                PSP_GAME/SYSDIR/BOOT.BIN
     3854912    7.997  packed       PSP compressed executable     PSP_GAME/SYSDIR/EBOOT.BIN
   315029056    5.023  structured   unknown [\x01\x00\x00\x00v...]  PSP_GAME/USRDIR/Data.wad
...
23 files, 8 with no recognised signature (these are the reverse-engineering targets)
```

`--summary` groups by extension, which is the fastest way to see which unknown
format accounts for most of the disc.

Entropy guide, in bits per byte:

| Range | Density | Usually |
| --- | --- | --- |
| < 4.0 | sparse | Text, padding, sparse tables |
| 4.0 - 6.5 | structured | Geometry, index tables, headers |
| 6.5 - 7.5 | mixed | A container with compressed payloads |
| > 7.5 | packed | Compressed or encrypted |

These are heuristics for deciding what to open first, not conclusions. The
entropy contrast between `BOOT.BIN` (5.74) and `EBOOT.BIN` (8.00) is what
revealed that one of them is unencrypted.

Only the first 64 KiB of each file is read, so sniffing a 3.6 GiB disc takes
seconds.

## Limitations

- Single-track data discs only. Multi-track images and non-zero pregaps are
  refused rather than mis-read.
- Track types whose user-data offset has not been verified (`MODE2`,
  `MODE2_FORM_MIX`, `MODE2_FORM2`, `AUDIO`) are refused rather than guessed at.
- Joliet and Rock Ridge extensions are not interpreted. No in-scope disc needs
  them; if one does, that is a finding to document.
