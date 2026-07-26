# Asset formats

Every distinct format found on an in-scope disc, with how well it is understood.

## Status meanings

| Status | Meaning |
| --- | --- |
| **understood** | Documented, implemented, and validated against real data |
| **partial** | Structure known, gaps remain |
| **identified** | Recognised as a known format, not yet implemented |
| **unknown** | Found, not yet decoded |

## Wipeout formats

These are the project's actual work.

| Format | Extension | Platforms | Status | Notes |
| --- | --- | --- | --- | --- |
| [WAD container](wad.md) | `.wad` | PSP, PS2 | **understood** | Implemented and validated against all nine archives. Entry names are stored as a CRC-32 variant, [recovered and reimplemented](wad.md#the-name-hash); most names still have to be mined rather than read. |
| [PSP indexed texture](psp-texture.md) | `.mip` | PSP | **understood** | Implemented; renders correctly. Not swizzled. |
| [LZSS](lzss.md) | - | PS2 | **understood** | Implemented and verified against all 6,053 compressed entries. |
| [`.vex` scene](vex.md) | `.vex` | PSP, PS2 | **partial** | **The** 3D format. Node tree, geometry and embedded textures implemented; `Transform` nodes and the scene hierarchy are not, so a whole track cannot be assembled yet. |
| [Track data](track.md) | - | PSP | **understood** | The `WO Track` spline graph, racing line and AI corridor. Implemented, and validated against all 40 track files with nothing left over. The `section` PVS payload is documented but not implemented. |
| [Front-end XML](fexml.md) | `.xml` | PSP, PS2 | **understood** | Name-shortened XML. Screens, widgets, handling stats. |
| [Handling stats](handling-stats.md) | `.xml` | PSP | **understood** | Ship tuning is data, not code. |
| [PSP movie](pmf.md) | `.PMF` | PSP | **understood** | PSMF header and MPEG program stream. Header and demuxer implemented and validated against all 17 movies; the H.264 inside is [transcoded out of process](../architecture/adr/0004-asset-pipeline.md), not decoded here. |
| [Bitmap font](fnt.md) | `.fnt` | PSP | **partial** | Metrics resolved and validated across all five fonts; the glyph atlas's pixel layout is the same unresolved swizzle as [`.mip`](psp-texture.md). |
| Sound bank | `.bnk` | PSP | unknown | `03000000` blobs. |
| Particle system | `.pob` | PSP | unknown | Magic `SYSP`. |
| Ship data | `.dat` | PSP | unknown | Paired with ships; likely handling or collision. |
| PS2 music archive | `.wad` | PS2 | unknown | `PS2MUSIC.WAD`. Header `10 00 00 00`, **not** the WAD container. |
| PS2 prerace archive | `.wad` | PS2 | unknown | `PRERACE.WAD`. Header `20 00 00 00`, entropy 0.08. Mostly empty. |
| IPU video | `.IPF` | PS2 | unknown | Magic `IPUF`. Targets the PS2 Image Processing Unit. |

## Platform formats

Documented elsewhere; we only need to read them.

| Format | Extension | Platform | Status | Notes |
| --- | --- | --- | --- | --- |
| ISO 9660 | - | both | **understood** | [`oag-disc`](../../crates/disc/src/iso9660.rs) |
| CHD | `.chd` | - | **understood** | Via the `chd` crate; layout detection in [`chd_source.rs`](../../crates/disc/src/chd_source.rs) |
| ELF | `.BIN`, `.IRX` | both | identified | Both main executables are unencrypted ELF |
| PSP `~PSP` | `.BIN` | PSP | identified | Encrypted executable. Not needed: `BOOT.BIN` is plaintext. |
| PSP `~SCE` | `.prx` | PSP | identified | Relocatable library |
| PBP | `.BIN` | PSP | identified | `GSHARE/SHARE.BIN` |
| SFO | `.SFO` | PSP | identified | Magic `\0PSF`. Key/value metadata. |
| PSAR | `.BIN` | PSP | identified | Firmware update archive. Not relevant. |
| ATRAC3 | `.AT3` | PSP | identified | RIFF wrapped. `ffmpeg` decodes it. |
| MPEG-2 PS | `.PSS` | PS2 | identified | `ffmpeg` decodes it. |
| IOP module archive | `.IMG` | PS2 | unknown | `IOPRP310.IMG`, magic `RESET`. Not relevant to gameplay. |
| PNG | `.PNG` | PSP | **understood** | Standard |

## How to add a format

1. Find it. `oag-unpack sniff` flags anything without a recognised signature;
   those are the targets.
2. Look at it. `oag-unpack hexdump` for the header, and across several files of
   the same type to see what varies.
3. Hypothesise a structure, then **test it against a second file**. A layout
   that only explains one file usually explains none.
4. Write the page: layout table, evidence, confidence, open questions.
5. Implement the parser in `oag-formats`, with tests built from hand-authored
   fixtures rather than game data. See
   [ADR-0006](../architecture/adr/0006-no-copyrighted-content.md).
6. Update the row above.

Step 3 is the one that catches wrong readings. The WAD directory layout was
settled by checking that each entry's offset equalled the previous entry's
offset plus size, rounded up to alignment, across the whole table; a wrong field
ordering fails that immediately.

## Cross-title reuse

The WAD container header appears in the same shape in Pure PSP, Pulse PSP and
Pulse PS2. That is the first evidence for the premise this project is built on:
one asset pipeline serving the whole lineage. Tracked in
[future-2048](../future-2048/shared-concepts.md).
