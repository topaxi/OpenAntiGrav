---
categories: [rendering, tooling]
---

# `.gtf` reads, so HD's textures are pixels

2026-08-17. `oag_formats::gtf`, [gtf.md](../../docs/formats/gtf.md), `crates/formats/tests/gtf_ground_truth.rs`. The PS3's own texture container - the single largest thing on the HD disc (7,333 files, 2.4 GiB). All 7,333 close on two independent arithmetic invariants. Full trap list on the page (mip-chain pitch, endianness, all-white alpha-only HUD textures, the unexplained 360 cubemap bytes). All 1,029 HUD sprites' source rectangles check against the textures they name, one authored exception aside. HUD textures reach a shader as of 2026-08-25, through `oag_game::sprite::Sheet` rather than an `oag_render` upload path.

**2026-09-02: all 7,333 decode.** The 44 swizzled `A8R8G8B8`/`A8B8G8R8` landed first (chasing `corner2.gtf` for [hds-strip-tabs-have-no-shape-and-no-scene-behind-them.md](../frontend/hds-strip-tabs-have-no-shape-and-no-scene-behind-them.md)); the last 9, swizzled `B8`, landed the same day. `decode::morton_index` is the RSX's own documented `cellGcm` tiling, confidence 88. Full writeup on [gtf.md](../../docs/formats/gtf.md).

**The `B8` files answered themselves by name.** All 9 are `/data/ships/<team>/textures/ambient_shadow.gtf`, 128x64, one per team. A one-channel texture called `ambient_shadow` is a craft's contact shadow, so there was a right answer to look for: read in Morton order each is that team's craft in soft silhouette (Feisar's delta and tailfin, Qirex's blunt oval), read in raster order each is horizontal banding. They are also the only textures on the disc that exercise `morton_index`'s non-square branch - every other swizzled file is square.

**`remap` is now decomposed rather than matched, and that corrected a real bug.** `gtf::Remap` reads `+0x10` as the RSX's own two packed tables (low byte: four 2-bit source selectors; high byte: four 2-bit controls; both in A, R, G, B order). The packing predicts all three words the disc carries, and `0xa9ff` cross-checks against an unrelated field - its low byte selects the *blue* source four times, and the format carrying it (`B8`) stores one byte in blue. Confidence 88, up from 75 - see the executable corroboration below. The bug: `0xa9e4` was read as forcing **blue** to one when it forces **alpha**, which rendered all 7 `A8B8G8R8` files solid blue - `fealphaluminancetexture.gtf` among them, whose texels are 100% grey.

**And the measurement that appeared to support forcing blue was an artefact worth remembering.** The roughness sweep applied the descriptor's remap to the native reading and *not* to the deliberately-wrong one, so forcing any channel to a constant lowered one side of the comparison only. Both sides get it now. Separately, roughness was measured along rows only - and a raster misread of a tiled surface comes out as row-uniform *stripes*, which a within-row metric scores as the smooth one. That is why three blocky test charts were carried as named exceptions. With both fixed (`roughness_2d`), the sweep is **43 of 43 judgeable files smoother under Morton, no exceptions**, up from 33 of 34 with one named.

**2026-09-02, later the same day: the `remap` half is now corroborated against the executable.** `Texture_BuildGcmRegisters` (`0x005a9998`, confidence 75, [renderer.md](../../docs/ghidra/functions/ps3-hdfury-eu/renderer.md)) takes one packed `(format << 16) | remap` word and writes its low 17 bits to a register slot of their own. The engine *constructs* these words rather than only copying them from files: `0xa9ff` appears at 6 `ori` sites, four of them in `FUN_00175798` building single-channel runtime surfaces with the packed word `0x0100a9ff` - format nibble `0x01`, `B8`. And a fourth word the disc does not carry, `0x09e4`, decomposes sensibly under this pair order (alpha one, red read, green and blue zero - a single-channel-in-red texture) and nonsensically under the opposite one. `Remap`'s confidence is 88, up from 85.

## Open

- **The Morton address function is still uncorroborated against the executable** - no `swizzle` string in `EBOOT.elf`, and no texture-*upload* routine located (`Texture_BuildGcmRegisters` builds the descriptor, not the tiling). That is the ceiling on its 88.
- The remap's remaining gap is narrower: the command-buffer write that consumes `+0x30` is not traced, so the slot is identified by taking exactly the remap word rather than against a decompiled `cellGcmSetTexture`.
- HUD textures reach the shader through `oag_game::sprite::Sheet` rather than a proper `oag_render` upload path
- The 360 unexplained cubemap bytes (per the trap list on gtf.md)
- A cubemap's mip levels: `face_range` addresses one and nothing decodes one

## Next Steps

- **Draw the ambient shadows.** The 9 `B8` textures decode; nothing places a quad under a craft. What size, what blend mode and what follows the ship's pitch/roll are renderer questions this format layer says nothing about - and the answers are on the disc (the ship's own `.rcsmodel`/material set) rather than inventable, per the project's never-invent rule. That is a new thread's worth of work, not this one's.
- **Locate the texture-upload path in `EBOOT.elf`.** `Texture_BuildGcmRegisters`'s seven callers are the place to start - one of them has to be where a `.gtf`'s texels get tiled, and that is the last thing holding `morton_index` at 88.
