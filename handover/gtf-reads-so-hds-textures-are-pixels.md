# `.gtf` reads, so HD's textures are pixels

2026-08-17. `oag_formats::gtf`, [gtf.md](../docs/formats/gtf.md), `crates/formats/tests/gtf_ground_truth.rs`. The PS3's own texture container - the single largest thing on the HD disc (7,333 files, 2.4 GiB). All 7,333 close on two independent arithmetic invariants; 7,280 decode (BC1/BC2/BC3 plus linear `A8R8G8B8`); 53 Morton-swizzled textures are refused by name rather than misread. Full trap list on the page (mip-chain pitch, endianness, all-white alpha-only HUD textures, the unexplained 360 cubemap bytes). All 1,029 HUD sprites' source rectangles check against the textures they name, one authored exception aside. HUD textures reach a shader as of 2026-08-25, through `oag_game::sprite::Sheet` rather than an `oag_render` upload path.

**2026-09-02: 44 of the 53 swizzled textures decode now.** Found while chasing a different question - what `Data\FE\Images\corner2.gtf` (a corner mask for `<Bracket corner="true">` widgets) actually looks like, for [hds-strip-tabs-have-no-shape-and-no-scene-behind-them.md](hds-strip-tabs-have-no-shape-and-no-scene-behind-them.md) - and once one swizzled file needed reading, reading the format properly rather than hand-decoding one file was the right size of fix. `decode::morton_index` is the RSX's own documented `cellGcm` tiling; confirmed by an exact synthetic-fixture match plus a corpus-wide roughness comparison, 33 of 34 judgeable files smoother under the real reading (one named, inspected, understood exception). Found and fixed a second, real bug along the way: `Texture::remap`'s `0xa9e4` value (all 7 `A8B8G8R8` files) forces blue to `0xff` rather than reading it, previously undocumented as acted-on because nothing decoded those files to need it - `Texture::apply_remap` now does. Confidence 88 on the address function, 7,324 of 7,333 `.gtf` files decode. Full writeup on [gtf.md](../docs/formats/gtf.md). Only `B8` (9 files, one channel, meaning still unread) remains refused.

## Open

- `B8` (9 files, swizzled) is refused rather than decoded - one channel, and what the byte itself means is unread; a different question from the Morton address function the other 44 settled
- HUD textures reach the shader through `oag_game::sprite::Sheet` rather than a proper `oag_render` upload path
- The 360 unexplained cubemap bytes (per the trap list on gtf.md)
- The Morton address function is not corroborated against the executable - no `swizzle` string in `EBOOT.elf`, no upload routine located - so confidence caps at 88 rather than reaching for a runtime-verified score

## Next Steps

- No next step named in the original record - read the prose above and decide one.
