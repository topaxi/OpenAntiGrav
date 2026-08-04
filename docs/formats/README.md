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
| [WAD container](wad.md) | `.wad` | PSP, PS2 | **understood** | Implemented and validated against all nine archives, and the header/entry layout is [confirmed in a second binary](wad.md#and-now-a-second-binary-which-reads-the-layout-back-in-one-shot). Entry names are stored as a CRC-32 variant, [recovered and reimplemented](wad.md#the-name-hash); most names still have to be mined rather than read. |
| [PSP indexed texture](psp-texture.md) | `.mip` | PSP | **understood** | Implemented; renders correctly. Not swizzled. |
| [PS2 texture](ps2-texture.md) | - | PS2 | **understood** | A GS upload packet, not a texture file: `PSMT8`-swizzled texels and a `CSM1` palette. Implemented and validated over 5,348 blobs; 5,343 decode, the five `PSMT4` ones are refused. The permutation is checked separately from the framing, by a corpus-wide smoothness comparison against the opposite reading. Which set belongs to which model is solved for ships and for each circuit's own `track.vex` (directory position, not a name); a model built from several small shared-atlas pieces is [still open](ps2-texture.md#how-a-model-finds-its-texture-set-directory-position-not-a-name). |
| [LZSS](lzss.md) | - | PS2 | **understood** | Implemented and verified against all 6,053 compressed entries. |
| [`.vex` scene](vex.md) | `.vex` | PSP, PS2 | **partial** | **The** 3D format. Node tree, scene hierarchy, geometry and embedded textures implemented on both platforms - PS2 batches are VIF packets rather than vertex arrays, validated over 11.8M vertices. PS2 textures are not in the file; they live in [separate archive entries](ps2-texture.md). A `Texture` node's **runtime asset path at payload `+0x38`** is now read: a track's node header carries no name at all, so it is the only name a track texture has, and reading it is what made trackside animated textures findable. Material entries are fully parsed; the unused `+0x0c..0x14` is proven zero on every material of all 12 circuits, which rules out an authored UV scroll rate. |
| [Sky and fog](skycube.md) | - | PSP | **partial** | `Skycube` `0x3c6` and `fogCube` `0x3d3`, the two environment classes every circuit authors. A `Skycube` payload **is a `Mesh` payload**, so the existing decoders read it unchanged; validated against all 40 sky nodes on the disc. `fogCube` is a 64-byte 4x4 box volume and two `{rgb, near, far}` sets that its runtime interpolates across the box - decoded, and drawn. Neither handler is recovered from the executable. |
| [Pads](pads.md) | - | PSP | **done** | `Speedup Pad` `0x3bd` and `Weapon Pad` `0x3be`, the plates on the track surface. A pad's bind handler calls the `Mesh` bind first, so its payload **is a `Mesh` payload** and a pad carries its own geometry - which is why the `Data\Pads\` models on the disc are never loaded. Its trigger volume is the mesh's own bounding box at `+0x10`/`+0x20`, expanded `min.y -= 2.0` / `max.y += 8.0` at load. Validated against all 762 pad nodes on the disc; drawn. The boost the trigger applies is not implemented yet. |
| [Track data](track.md) | - | PSP | **understood** | The `WO Track` spline graph, racing line and AI corridor. Implemented, and validated against all 40 track files with nothing left over. The `section` PVS payload is documented but not implemented. |
| [Collision geometry](collision.md) | - | PSP, PS2 | **understood** | An indexed triangle soup in `.vex` nodes under five collision class IDs, separate from the render mesh. Implemented and validated on both discs: 319 of 319 nodes close exactly over 602,086 vertices. |
| [Front-end XML](fexml.md) | `.xml` | PSP, PS2 | **understood** | Name-shortened XML. Screens, widgets, handling stats. |
| [Handling stats](handling-stats.md) | `.xml` | PSP, PS2 | **understood** | Ship tuning is data, not code. Implemented and validated on both discs: 8 teams x 4 speed classes, every attribute required. Four fields are [pre-scaled at load](../ghidra/functions/psp-pulse-usa/engine.md). |
| [PSP movie](pmf.md) | `.PMF` | PSP | **understood** | PSMF header and MPEG program stream. Header and demuxer implemented and validated against all 17 movies; the H.264 inside is [transcoded out of process](../architecture/adr/0004-asset-pipeline.md), not decoded here. |
| [Bitmap font](fnt.md) | `.fnt` | PSP | **understood** | Metrics and atlas both decoded and validated across all five fonts and 863 glyphs. The atlas header is a 64-byte [`.vex`](vex.md) Texture node, not a `.mip` header, and its texels are the one thing on the disc stored **already swizzled**. |
| [PSP sound bank](psp-audio.md) | `.bnk` | PSP | **partial** | An `SBlk` descriptor block plus PS-ADPCM waveforms. Container, header and codec implemented and validated across all 39 banks and 546,681 audio blocks; banks also carry their own name. Where each individual sound starts inside a bank is [not resolved](psp-audio.md#not-determined). |
| [Particle system](pob.md) | `.pob` | PSP, PS2 | **partial** | `SYSP` container, the slot table's pointer-fixup mechanism, and the name field are decoded and validated on both discs (35 PSP files, 41 PS2 files, unmodified parser). What a resolved record's fields mean is not - 43-80% of resolved targets are readable developer strings (texture paths, layer names). |
| Ship data | `.dat` | PSP | unknown | Paired with ships; likely handling or collision. |
| [PS2 music archive](ps2-audio.md) | `.wad` | PS2 | **understood** | `PS2MUSIC.WAD`, **not** the WAD container: a 12-byte-entry directory over 48 kHz 16-bit stereo PCM, uncompressed. Implemented; the chain closes to the byte and the sample rate is pinned against the PSP's ATRAC3plus copies of the same sixteen tracks. |
| PS2 prerace archive | `.wad` | PS2 | unknown | `PRERACE.WAD`. Header `20 00 00 00`, entropy 0.08. Mostly empty. |
| [PS2 IPU video](ipf.md) | `.IPF` | PS2 | **understood** | Magic `IPUF`: a 32-byte header over fixed-size slots, one intra-only MPEG-2 picture each for the PS2's Image Processing Unit. The looping menu backdrop. Container implemented and validated on both files - the slot arithmetic closes to the byte, and `ffmpeg`'s own IPU demuxer independently cuts the payload at the 495 boundaries the slot headers declare. The video is [transcoded out of process](../architecture/adr/0008-av1-movie-cache.md), like `.PMF`. |

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

**[Pure status](pure-status.md)** measures how far that goes: what the existing
tooling reads off *Wipeout Pure*'s UMD unchanged, and every place a parser turns
out to have overfit to Pulse. It is the
[ADR-0009](../architecture/adr/0009-multi-game-fanout.md) probe, and it is the
reason the pages above now say **which discs** a claim is validated against
rather than just "real data".
