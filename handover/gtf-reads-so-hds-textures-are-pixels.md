# `.gtf` reads, so HD's textures are pixels

2026-08-17. `oag_formats::gtf`, [gtf.md](../docs/formats/gtf.md), `crates/formats/tests/gtf_ground_truth.rs`. The PS3's own texture container - the single largest thing on the HD disc (7,333 files, 2.4 GiB). All 7,333 close on two independent arithmetic invariants; 7,280 decode (BC1/BC2/BC3 plus linear `A8R8G8B8`); 53 Morton-swizzled textures are refused by name rather than misread. Full trap list on the page (mip-chain pitch, endianness, all-white alpha-only HUD textures, the unexplained 360 cubemap bytes). All 1,029 HUD sprites' source rectangles check against the textures they name, one authored exception aside. HUD textures reach a shader as of 2026-08-25, through `oag_game::sprite::Sheet` rather than an `oag_render` upload path.

## Open

- 53 Morton-swizzled textures are refused by name rather than decoded
- HUD textures reach the shader through `oag_game::sprite::Sheet` rather than a proper `oag_render` upload path
- The 360 unexplained cubemap bytes (per the trap list on gtf.md)

## Next Steps

- No next step named in the original record - read the prose above and decide one.
