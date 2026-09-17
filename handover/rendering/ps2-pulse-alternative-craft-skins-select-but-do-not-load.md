# PS2 Pulse's alternative craft skins are selectable and do not load

2026-09-17. Reported from play by the user: "on PS2 pulse, the craft
alternative textures are selectable, but do not work/load (works on PSP
pulse)."

The mechanism (`crates/game/src/livery/ship_skin.rs`, `oag_texture::ship_skin`,
`oag_render::mesh::ship_skin::apply`) is a texture swap on the same hull: four
of the built model's texture slots are replaced from the `.dat` the
`PI_ModelSkin` definition names. Two things about the PS2 path make that
likely to miss without an error:

- **A PS2 hull's textures are external.** `livery.rs` rebuilds the hull with
  `ps2_texture_set` (the archive entry directly before the model) because the
  embedded texture block is empty; the skin is applied *after* that rebuild
  (deliberately - the comment says so). Whether the four slot indices the
  skin applier targets still mean the same slots on the externally-textured
  rebuild is unchecked.
- **The PS2 `.dat` may not be the PSP's format.** `oag_texture::ship_skin`
  decodes the PSP layout; the PS2 port's skin files may be GS-packet
  textures (as its other textures are - `docs/formats/ps2-texture.md`), so
  the decode would fail or produce nothing, and the report line is where to
  look first.

## Open

- What the loader report says on `just play data/images/pulse-ps2-eu.chd`
  with an alternative skin selected: a "did not decode" line, a "declares no
  skin" line, or a clean apply that draws nothing.
- Whether the PS2 disc's `PI_ModelSkin` `location` entries even resolve in
  `WADS2.WAD` (`oag-wad list` and hash them).

## Next Steps

1. Reproduce with the report on, and classify which of the three it is.
2. If the `.dat` is PS2-formatted, teach `oag_texture::ship_skin` the PS2
   layout with a ground-truth test on the disc; if it is a slot mismatch,
   map slots by texture name rather than index on the external-set path.
3. Screenshot the skinned craft on both sources at player size.
