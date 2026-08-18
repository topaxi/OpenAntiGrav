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

**The `Titles` column says which releases the parser is validated as *working*
on** - one definition, because the first draft carried two and the rows split
along the seam between them. `Pulse, Pure` means Pure's own data goes through it;
`Pulse` covers both "nobody has pointed it at another title" and "somebody did
and it broke", and [pure-status](pure-status.md) is where that difference is
recorded. It is not a portability claim either way: a `Pulse`-only row may well
be portable and simply untried.

Before this column existed every row read as `Pulse` implicitly, which is the
ambiguity [ADR-0022](../architecture/adr/0022-title-packages.md) exists to
remove. Pure is PSP-only, so a PS2-only row can only ever say `Pulse`.

| Format | Extension | Platforms | Titles | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| [WAD container](wad.md) | `.wad` | PSP, PS2 | Pulse, Pure | **understood** | Implemented and validated against all nine archives, and the header/entry layout is [confirmed in a second binary](wad.md#and-now-a-second-binary-which-reads-the-layout-back-in-one-shot). Entry names are stored as a CRC-32 variant, [recovered and reimplemented](wad.md#the-name-hash); most names still have to be mined rather than read. |
| [PSP indexed texture](psp-texture.md) | `.mip` | PSP | Pulse, Pure | **understood** | Implemented; renders correctly. **Sometimes swizzled**, and `+0x07` bit 0 says which - this row read "Not swizzled" until 2026-08-09, which [psp-texture](psp-texture.md#is-the-pixel-data-swizzled-sometimes-and-0x07-says-which) had already corrected and `texture_swizzle_flag_ground_truth` counts at 88 of 5,375 Pulse PSP `Texture` nodes. |
| [PS2 texture](ps2-texture.md) | - | PS2 | Pulse | **understood** | A GS upload packet, not a texture file: `PSMT8`- or `PSMT4`-swizzled texels and a `CSM1` palette. Implemented and validated over 5,348 blobs; all 5,348 decode, the five `PSMT4` ones being the [PS2 font atlases](fnt.md). The permutation is checked separately from the framing, by a corpus-wide smoothness comparison against the opposite reading. Which set belongs to which model is solved for ships and for each circuit's own `track.vex` (directory position, not a name); a model built from several small shared-atlas pieces is [still open](ps2-texture.md#how-a-model-finds-its-texture-set-directory-position-not-a-name). A **standalone** texture is found by its declared name with the extension rewritten `.mip`/`.tga` -> `.pct`, which is what the loader itself does; that closes the front-end and HUD images, which were previously reachable only through a three-entry table recovered by correlating pictures. |
| [LZSS](lzss.md) | - | PS2 | Pulse | **understood** | Implemented and verified against all 6,053 compressed entries. |
| [`.vex` scene](vex.md) | `.vex` | PSP, PS2 | Pulse, Pure | **partial** | **The** 3D format. Node tree, scene hierarchy, geometry and embedded textures implemented on both platforms - PS2 batches are VIF packets rather than vertex arrays, validated over 11.8M vertices. PS2 textures are not in the file; they live in [separate archive entries](ps2-texture.md). A `Texture` node's **runtime asset path at payload `+0x38`** is now read: a track's node header carries no name at all, so it is the only name a track texture has, and reading it is what made trackside animated textures findable. Material entries are fully parsed; the unused `+0x0c..0x14` is proven zero on every material of all 12 circuits, which rules out an authored UV scroll rate. |
| [Sky and fog](skycube.md) | - | PSP | Pulse | **partial** | `Skycube` `0x3c6` and `fogCube` `0x3d3`, the two environment classes every circuit authors. A `Skycube` payload **is a `Mesh` payload**, so the existing decoders read it unchanged; validated against all 40 sky nodes on the disc. `fogCube` is a 64-byte 4x4 box volume and two `{rgb, near, far}` sets that its runtime interpolates across the box - decoded, and drawn. Neither handler is recovered from the executable. |
| [Lighting](lighting.md) | - | PSP | Pulse | **partial** | `AmbientLight` `0x12c`, `DirectionalLight` `0x131` (16-byte `{r, g, b, intensity}`) and `PointLight` `0x132` (32-byte `{r, g, b, range}` plus an undecoded `{1, 0, 0, 0}` trailer), the track's light rig. Placement is the node's own transform chain, not the payload. Validated against every entry of `Data.wad` (1142 files): 74/86/10 on the 40 track/zone files, 106/114/10 disc-wide - the excess is mostly front-end `ship_FE.vex` carousel lighting. `Dynamic Point Light` `0x3c2` is authored nowhere on the disc. Neither handler is recovered from the executable, and colour/intensity are not clamped - real values reach 5.7854. |
| [Pads](pads.md) | - | PSP | Pulse | **done** | `Speedup Pad` `0x3bd` and `Weapon Pad` `0x3be`, the plates on the track surface. A pad's bind handler calls the `Mesh` bind first, so its payload **is a `Mesh` payload** and a pad carries its own geometry - which is why the `Data\Pads\` models on the disc are never loaded. Its trigger volume is the mesh's own bounding box at `+0x10`/`+0x20`, expanded `min.y -= 2.0` / `max.y += 8.0` at load. Validated against all 762 pad nodes on the disc; drawn. The boost the trigger applies is not implemented yet. |
| [Track data](track.md) | - | PSP | Pulse, Pure | **understood** | The `WO Track` spline graph, racing line and AI corridor. Implemented, and validated against all 40 track files with nothing left over. The `section` PVS payload is documented but not implemented. |
| [Collision geometry](collision.md) | - | PSP, PS2 | Pulse, Pure | **understood** | An indexed triangle soup in `.vex` nodes under five collision class IDs, separate from the render mesh. Implemented and validated on both discs: 319 of 319 nodes close exactly over 602,086 vertices. **Now `Pulse, Pure`**: Pure's renumbered floor, wall and reset IDs are all recovered - two by a facing statistic calibrated on Pulse, all three by the [class-name table index](pure-status.md#the-renumbering-is-a-table-index-and-both-executables-carry-the-table) - and a craft flown into Pure's reset surface respawns. |
| [Front-end XML](fexml.md) | `.xml` | PSP, PS2 | Pulse, Pure | **understood** | Name-shortened XML. Screens, widgets, handling stats. |
| [Menu definitions](fe-menu-definitions.md) | `.xml` | PSP | Pulse, Pure | **read, not implemented** | The 22 screens `Skin.xml` pulls in, 17 of which resolve in `Data.wad`. Read to settle the menus' layout, colours and `transition` durations; the tree this project draws stays [its own](../architecture/menus.md). Pure's set differs in kind, not only in value. |
| [DLC pack](dlc-pack.md) | `.edat` | PSP | Pulse | **understood** | Not a new container and **not encrypted**, despite the extension: a `PACKn.edat` is an ordinary [WAD](wad.md), and all four Pulse packs parse with the unmodified reader, offset chains closing. Entry 0 is a fragment of the disc's own `Data\Plugins\PI001\Definition.xml`, so a pack's `PI_Team`/`PI_Track` additions need no reader of their own. The ship is in `PACKn.edat` and its `handlingstats.xml` in `PACKn_UI1.edat` - both, or the ship loads and cannot race. Mounted [independently of region](../architecture/adr/0021-region-independent-dlc.md). This is also what retired the "PS2-only" reading of `Auricom`/`Harimau`/`Icaras`, and identified the fourth team as `Mantis` - the folder behind the name Mirage. Pure's packs are a separate, genuinely encrypted problem. |
| [Handling stats](handling-stats.md) | `.xml` | PSP, PS2 | Pulse | **understood** | Ship tuning is data, not code. Implemented and validated on both discs: 8 teams x 4 speed classes, every attribute required. Four fields are [pre-scaled at load](../ghidra/functions/psp-pulse-usa/engine.md). **`Pulse` and not `Pulse, Pure`**: Pure's whole element set is enumerated and every difference but one is handled, but `<pitch>` still makes the parser refuse all ten of its laddered files - see [pure-status](pure-status.md#handling-stats-the-schema-holds-the-parser-does-not). |
| [Weapon stats](weapon-stats.md) | `.xml` | PSP | Pulse | **schema read, nothing implemented** | The weapons are data too: `Data\XML\WeaponStats_Race.xml` and `_Elimination.xml` carry every weapon's tunables, the seven disturber effects and the **pickup distribution** - weighted per speed class, per weapon, and separately for AI, human, front and back of the grid. Fourteen weapon types, one parser each. Turbo, Shield and Autopilot share one two-attribute schema and damage nobody, which is what makes them the cheap three to build. |
| [PSP movie](pmf.md) | `.PMF` | PSP | Pulse, Pure | **understood** | PSMF header and MPEG program stream. Header and demuxer implemented and validated against all 17 movies; the H.264 inside is [transcoded out of process](../architecture/adr/0004-asset-pipeline.md) by default, or decoded through a system GStreamer install on Linux with `--features native-video` ([ADR-0017](../architecture/adr/0017-gstreamer-native-video.md)) - either way, no H.264 decoder ships in this repository. |
| [Bitmap font](fnt.md) | `.fnt` | PSP, PS2, PS3 | Pulse, Pure, HD Fury | **understood** | Metrics and atlas both decoded and validated across all five fonts and 863 glyphs. On the PSP the atlas header is a 64-byte [`.vex`](vex.md) Texture node, not a `.mip` header, and its texels are the one thing on the disc stored **already swizzled**. The PS2 ships the same five fonts **metrics-only** and keeps each glyph sheet in the following archive entry as a `PSMT4` [PS2 texture](ps2-texture.md), on the GS's 0-128 alpha scale. **Wipeout HD writes the same layout big-endian** - the magic is one word, so `\x01FNT` becomes `TNF\x01` - and its atlases ship *linear* where both PSP titles' ship swizzled, read from the flag rather than from the console. Every HD font was refused as "not a .fnt" until 2026-08-18 and its whole front end drew in the built-in 5x7 face. |
| [PSP sound bank](psp-audio.md) | `.bnk` | PSP | Pulse | **understood** | An `SBlk` descriptor block plus PS-ADPCM waveforms. Container, header and codec implemented and validated across all 39 banks and 546,681 audio blocks. **Per-sound boundaries and per-sound names are both resolved**, from the SCREAM engine's own arithmetic: 595 waveform spans [tile all 39 sections exactly](psp-audio.md#where-each-sound-starts), number each bank's declared `waveform_count`, and each carries its PS-ADPCM terminator in its final two blocks - the codec agreeing with the command table. 607 [sound names](psp-audio.md#every-sound-has-a-name), one per cue in every bank, including strings the disassembler found independently. The 43 unread command opcodes and the per-sound sample rate remain [open](psp-audio.md#not-determined). |
| [Particle system](pob.md) | `.pob` | PSP, PS2, PS3 | Pulse, HD/Fury | **partial** | `SYSP` container, the slot table's pointer-fixup mechanism, the name field and **the emitter tree** are decoded and validated on three discs (35 PSP files / 76 emitters, 41 PS2 files / 90 emitters, 88 HD files / 249 emitters, one parser with a byte order passed in - HD writes the same layout big-endian, magic `PSYS`). An emitter record carries its schedule, shape, speeds, lifetimes, 256-entry colour table, four channel blocks and its child/sibling pointers, all at fixed offsets with no fixup. What a *slot-resolved* record's fields mean is still not decoded - 43-80% of those targets are readable developer strings (texture paths, layer names). |
| Ship data | `.dat` | PSP | Pulse | unknown | Paired with ships; likely handling or collision. |
| [HD HUD layouts](hd-hud.md) | `.xml` | PS3 | HD Fury | **understood** | HD's in-race HUD is Pulse's HUD in the same [dialect](fexml.md), plain rather than shortened, **composed out of fragments**: 18 roots across 8 modes and 3 skins, each pulling in up to 16 more by `<LoadXML SrcRel=>`. All 18 compose with **nothing missing and nothing skipped**, into 2,320 widgets, cross-checked against a second walker. Two rules the lineage forced and Pulse could not have shown: `OffsetX`/`OffsetY` **compose** down the tree where `x`/`y` do not (a strict no-op on Pulse's 5 layouts, which nest nothing), and a sprite's `src` names **source art** - `hdHUD.mip` and `detonator_hud2.tga` ship as `.gtf`, so 12 of 12 references resolve rather than 10. `oag_hd::hud`, `oag_game::hud::compose`. **No pixels**: `.gtf` is unimplemented and no stand-in atlas exists. |
| [Env settings](envsettings.md) | `.envsettings` | PS3 | HD Fury | **understood** | **The light rig a circuit authors**, and the thing `mesh.wgsl`'s two invented light directions were a stand-in for. Plain UTF-8 `key=value`, no sections or escaping; 33 files, every line of all of them parsing, and the 32 circuit files sharing **34 keys** across `Lighting`, `Fog`, `Water` and `HDR and Bloom`. **The sun direction, the sun's hue and the constant ambient are drawn** - `just play hd --race` lights Talon's Junction off its own file. Two traps the corpus forced: `Lighting.Sky colour` is a **byte** quadruple where every other colour is normalised, and the only thing that says so is the missing decimal point; and `Lighting.Sun direction` is **not a unit vector** - eight distinct lengths over 32 circuits, four of them degenerate - so a reader that assumes one is wrong on most of the corpus. **The magnitudes are read and not applied**: sun colour reaches 4.0 and ambient 3.0 beside a `Tone maximum brightness` of 4.0, which only a linear pipeline with a tonemapper can hold, and this one is [gamma-authoritative](../architecture/adr/0020-gamma-authoritative-colour-space.md) with neither. The hue survives, the scale does not, and the load report says so per race. |
| [RCS model](rcsmodel.md) | `.rcsmodel` | PS3 | HD Fury | **partly understood** | **All of HD's render geometry**, 643 files and 686 MiB. On the PSP a [`Mesh`](vex.md) node's payload *is* its geometry; on the PS3 that payload is a bounding-box pair (`min <= max` on 1,638 of 1,638 nodes) and a 32-bit word addressing a chunk here. **Positions and triangles read**: `i16` triples through a per-mesh bias and `1/128` scale, big-endian `u16` triangle lists - **50,873 of 50,873 submeshes across all 643 files** have an in-range index buffer whose count divides by three, in **two chunk layouts** told apart by byte `+0x06`. Implemented in `oag_formats::rcsmodel`; `just view --mesh` draws an HD craft and an HD circuit. **The vertex stride is in no field of the file** and is recovered four independent ways - the authored bounding box, whether the vertex normals decode to unit vectors, the file's own buffer packing, and the geometry's compactness - which agree on every chunk where more than one of them answers. **Vertex normals read too**: a packed 11:11:10 signed triple at `+6`, unit on 99.5 % of every model's vertices and agreeing with the geometry on 82-93 %. **The material table is read too**: a chunk's `+0x20` indexes it (resolved by name on Assegai - `WindscreenShape` to `glass_texture_n`), and the low two bits of a material's `+0x10` say whether the surface is see-through - structural evidence, since 13,183 of 13,188 opaque materials hold one default blend-factor pair where the see-through ones range over eight. **The texture coordinate is confirmed too** - the last four bytes of a vertex, two big-endian halves, on every stride; sampling a craft's own `.gtf` through them renders the words "ASSEGAI DEVELOPMENTS" legibly along the hull, which no wrong reading produces. A material also names the [`.gtf`](hd-status.md#what-is-genuinely-new) it paints with at `+0x58`, so an HD craft and an HD circuit draw **textured and blended**. Still undecoded: the four bytes at `+0x0a` on stride 18, and which of a material's *two* textures a shader samples for what - that second slot is a normal map, an emissive map, a coverage mask and a lightmap on four different materials, and [the material file](rcsmaterial.md) says why one file cannot answer it. |
| [RCS material](rcsmaterial.md) | `.rcsmaterial` | PS3 | HD Fury | **partly understood** | 1,632 shader containers, wall-to-wall 32-bit hashes. A circuit authors ~50 by name (`track_surface`, `glass_reflect`, `emissive_bloom`) and a second identical set for its reversed direction. **Not one shader**: a variant count and a 0x40-byte variant table, each record naming a vertex and a fragment `SHO` block inside the same file - `etched_glass_tech` declares 70 variants and carries 20 fragment programs. That is why "which of a material's two textures is what" could not be read off one sampler table: the twenty disagree about which unit a sampler sits at, and **what selects a variant is unread**. **38 sampler names recovered by `~crc32` preimage**, `lightmap`, `EmissiveTexture`, `NormalTexture`, `AlphaMask` and `Texture1`/`2`/`3` among them. The lightmap is the one identified beyond doubt and the only one wired. |
| [PS2 music archive](ps2-audio.md) | `.wad` | PS2 | Pulse | **understood** | `PS2MUSIC.WAD`, **not** the WAD container: a 12-byte-entry directory over 48 kHz 16-bit stereo PCM, uncompressed. Implemented; the chain closes to the byte and the sample rate is pinned against the PSP's ATRAC3plus copies of the same sixteen tracks. |
| [PS2 prerace voice archive](ps2-voice.md) | `.wad` | PS2 | Pulse | **understood** | `PRERACE.WAD`. The same count-first container as `PS2MUSIC.WAD` - the `20 00 00 00` header is an entry count of 32, not a version - over 32 clips of dual-mono 16-bit PCM speech. `ps2_music` parses it unchanged; the chain closes to the byte and all 32 clips measure L==R at 1.0000. **44,100 Hz**: every clip carries a pad of exactly 44,100 silent frames at each end, and the 32 clips cross-correlate 32-for-32 against the PSP disc's ATRAC3plus set at +0.9993, an alignment that does not exist at 48 kHz. The low entropy was a near-silent voice-over lead-in, not emptiness. |
| [Bink video](bik.md) | `.bik` | PS3 | HD Fury | **container understood** | HD's video, 37 files and 101.2 MiB: the two studio logo reels, 16 circuit previews and 19 front-end mode icons. The container header is implemented and validated on all 37 - three arithmetic invariants each, the load-bearing one walking past the variable-length audio arrays - and it is **little-endian on a big-endian console**, the one HD payload that is not Pulse's format byte-swapped, because it is RAD's container written by a PC authoring tool. The video is [transcoded out of process](../architecture/adr/0008-av1-movie-cache.md) like `.PMF` and `.IPF`, and [ADR-0024](../architecture/adr/0024-in-process-codecs-and-ffmpeg-as-a-last-resort.md)'s category check was made rather than inherited: the one pure-Rust Bink decoder on crates.io is GPL-3.0-or-later, which the licence bar excludes. The logo reel is decoded and **is** the Studio Liverpool ident. Six files carry audio; it is measured and not played, because nothing plays an HD movie yet. |
| [PS2 IPU video](ipf.md) | `.IPF` | PS2 | Pulse | **understood** | Magic `IPUF`: a 32-byte header over fixed-size slots, one intra-only MPEG-2 picture each for the PS2's Image Processing Unit. The looping menu backdrop. Container implemented and validated on both files - the slot arithmetic closes to the byte, and `ffmpeg`'s own IPU demuxer independently cuts the payload at the 495 boundaries the slot headers declare. The video is [transcoded out of process](../architecture/adr/0008-av1-movie-cache.md), like `.PMF`. |

## Platform formats

Documented elsewhere; we only need to read them.

The `Titles` column means the same thing here, and the reason a platform row
often carries more than one title is worth stating: these are the console's
formats rather than a game's, so where another title on the same hardware ships
one it goes through unchanged. That is the null result the format layer wants,
and recording it is what stops the next person re-running the probe.

Two rows now cross a console generation rather than only a second title.
`ISO 9660` and `SFO` say `Pulse, Pure, HD Fury`: `oag-disc`'s walker reads the
PS3 disc's 22 files with no change at all, and a PS3 `PARAM.SFO` is the same
`\0PSF` key/value file the PSP writes. Rows that say only `Pulse` do so because
Pure does not ship that thing - no firmware update payload, no
`GSHARE/SHARE.BIN`, no `~SCE` modules - not because anything failed. The four
`HD Fury` rows are PS3-only formats, from the one PS3 image this project holds.

| Format | Extension | Platform | Titles | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| ISO 9660 | - | all three | Pulse, Pure, HD Fury | **understood** | [`oag-disc`](../../crates/disc/src/iso9660.rs). Reads the PS3 disc unchanged - 22 files in 5 directories - and [identifies it](../reverse-engineering/source-images.md#hdfury-ps3-euiso---wipeout-hd--fury-ps3) from `PS3_DISC.SFB` without needing its key. |
| CHD | `.chd` | - | Pulse, Pure | **understood** | Via the `chd` crate; layout detection in [`chd_source.rs`](../../crates/disc/src/chd_source.rs) |
| ELF | `.BIN`, `.IRX` | both | Pulse, Pure | identified | Both main executables are unencrypted ELF |
| PSP `~PSP` | `.prx`, `.BIN` | PSP | Pulse, Pure | identified | Compressed/encrypted executable. Not needed: `BOOT.BIN` is plaintext. `sniff` finds 2 on Pulse's UMD and **9 on Pure's**, all `.prx` - Pure ships its network stack loose where Pulse packs most of it. |
| PSP `~SCE` | `.prx` | PSP | Pulse | identified | Relocatable library. **Pulse only**: `sniff` finds 6 on Pulse's UMD and **zero on Pure's**, whose modules are all `~PSP` or plain ELF. |
| PBP | `.BIN` | PSP | Pulse | identified | `GSHARE/SHARE.BIN` |
| SFO | `.SFO` | all three | Pulse, Pure, HD Fury | identified | Magic `\0PSF`. Key/value metadata. Unchanged on the PS3, where `PS3_GAME/PARAM.SFO` is what names the disc: `BCES-00664`, `WipEout(R) HD Fury`. It sits in a plain region, so it reads without the disc key. |
| PSAR | `.BIN` | PSP | Pulse | identified | Firmware update archive. Not relevant. **Not** the PS3's `.psarc` below, despite both starting `PSAR` - different container, same four bytes. |
| ATRAC3 | `.AT3` | PSP | Pulse, Pure | identified | RIFF wrapped. `ffmpeg` decodes it - the codec has no Rust decoder, which is what [ADR-0019](../architecture/adr/0019-atrac3plus-out-of-process.md) is about. |
| MPEG-1 Layer III | `.mp3` | PS3 | HD Fury | **understood** | HD's music, in no console container at all: 36 files under `/data/music/`, 48 kHz stereo, plain MP3. Read and decoded **in process** by `symphonia`, so this path needs no `ffmpeg`. Named by the executable's own templates and declared as `PI_Music` - see [hd-status.md](hd-status.md#music-plain-mp3-declared-the-way-the-psp-titles-declare-theirs). |
| MPEG-2 PS | `.PSS` | PS2 | Pulse | identified | `ffmpeg` decodes it. |
| IOP module archive | `.IMG` | PS2 | Pulse | unknown | `IOPRP310.IMG`, magic `RESET`. Not relevant to gameplay. |
| PNG | `.PNG` | PSP | Pulse | **understood** | Standard |
| [PS3 disc encryption](ps3-disc.md) | - | PS3 | HD Fury | **understood** | Per-sector AES-128-CBC over the spans sector 0's region table declares encrypted, IV from the absolute LBA. Implemented in [`scripts/ps3iso.py`](../../scripts/ps3iso.py) rather than `oag-formats` - it is a disc layer, not an asset - and verified by decrypting three files the disc also ships in the clear and getting byte-identical results. Needs the disc's own `.dkey`, which is never in the image. |
| SFB | `.SFB` | PS3 | HD Fury | **understood** | Magic `.SFB`. A 32-byte-entry keyed table - 16-byte key, big-endian offset and length - over values that follow it. `PS3_DISC.SFB` is what identifies a PS3 disc, and its `TITLE_ID` field holds the serial already hyphenated. Read in [`oag-disc`](../../crates/disc/src/platform.rs) rather than `oag-formats`, which depends on it; only the one field identification needs. |
| [PSARC](psarc.md) | `.psarc` | PS3 | HD Fury | **understood** | The PS3 asset archive - `PSAR` 1.3, zlib, 64 KiB blocks, big-endian, paths stored in full rather than hashed. Implemented in `oag_formats::psarc`, and still in [`scripts/psarc.py`](../../scripts/psarc.py), which `just psarc` and `just hd-survey` drive; all seven of Wipeout HD / Fury's read, and **11,664 of 11,664 entries carry a digest this project predicts from the entry's own path**, which ties the manifest, the entry ordering and the entry stride together in one check. |
| [GTF](gtf.md) | `.gtf` | PS3 | HD Fury | **understood** | The PS3's own texture container, and **the single largest thing on the HD disc** - 7,333 files, 2.4 GiB. Sony's `CellGcmTexture` written out behind a twelve-byte header. Implemented in `oag_formats::gtf` and swept against the whole disc: **7,333 of 7,333** close on two independent arithmetic invariants, over a corpus that runs 3x1 to 2048x2048, one to twelve mip levels, ten format bytes and 131 non-power-of-two textures. **7,280 decode** - BC1/BC2/BC3 and linear `A8R8G8B8`; the 53 that do not are in the RSX's Morton order and are refused by name rather than read linearly. Two rules the length check forced: **`pitch` does not halve down the mip chain**, and a pitch on a compressed texture is bytes of a *block* row. The texel payload is **little-endian inside a big-endian file** - measured by roughness over 2,040 textures (1,969 to 69, ratio 6.58x) and confirmed by the picture. **Drawn since 2026-08-18 in two places**: an `.rcsmodel` material samples one, and `oag_game::sprite` decodes the front end's three - which used to be fed to the PSP `.mip` parser and reported as `zero-sized texture 1281x0`, a complaint about a format the file is not. |
| SELF | `.BIN`, `.sprx` | PS3 | HD Fury | identified | Magic `SCE\0`. `EBOOT.BIN` and `DFEngine.sprx`. Encrypted with console keys, a layer entirely separate from the disc above; nothing here reads PS3 code yet. |
| PUP | `.PUP` | PS3 | HD Fury | identified | Magic `SCEUF`. The 256 MiB firmware update payload every PS3 disc carries. In the clear, and not relevant. |

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

**[HD status](hd-status.md)** is the same probe run one title further and one
console generation across, and it carries the premise further than the WAD
header does. *Wipeout HD / Fury* ships **the same `.vex`, version 6, with
Pulse's own class IDs** - byte-swapped for the PS3, with the render geometry
lifted out into a container of its own. The `WO Track` spline, the collision
soup, the pad volumes and the `section` visibility mask all read, corpus-wide;
and **ten of HD's sixteen environments are a Pulse or Pure circuit's spline in
the same world coordinates**, Talon's Junction among them.

**[HD front end](hd-frontend.md)** is the same reading applied to
`/data/plugins/frontend/gui/skin.xml` - HD ships six copies of it and they agree
on every layout global - plus a read-only sweep of the PS3 executable for the
boot entry point. It is what `oag_hd::frontend` was filled in from, and it is
also where the two things that are *not* recovered are named: which of the six
skins the runtime loads, and therefore which logo reel plays.

**[HD HUD](hd-hud.md)** is the third page of that kind, and the first where a
*Pulse* reader was pointed at HD's data and came back changed. All eighteen of
HD's in-race layouts compose, into 2,320 widgets - and getting there corrected
two rules `oag_game::hud` had held since it was written, both of which Pulse's
own five layouts are silent about: an `<Item>`'s offset **composes** with the
one it inherits rather than replacing it, and a widget with no `name` is drawn
rather than dropped. Composing is a strict no-op on Pulse, which is the check
that it is a correction and not a regression.

Note what that does **not** do to the `Titles` column above: none of it went
through `oag-formats`, which has no big-endian path, so no row gained an
`HD Fury`. The measurement is a deliberately separate reading of the same
layouts, which is why agreement between the two means something about the
format rather than about one implementation.
