# PS2 Pulse's alternative craft skins are selectable and do not load

2026-09-17. Reported from play by the user: "on PS2 pulse, the craft
alternative textures are selectable, but do not work/load (works on PSP
pulse)."

The mechanism (`crates/game/src/livery/ship_skin.rs`, `oag_texture::ship_skin`,
`oag_render::mesh::ship_skin::apply`) is a texture swap on the same hull: four
of the built model's texture slots are replaced from the `.dat` the
`PI_ModelSkin` definition names.

## 2026-09-25: two bugs found, one fixed, one still open

Reproduced with `--dry-run --team AG_Systems --skin Alternative` against
`pulse-ps2-eu.chd`. It was a clean apply that draws nothing (the third of the
thread's own three cases), for two separate reasons stacked on top of each
other:

**Bug 1, fixed.** `oag_render::mesh::ps2_textures::resolve_texture_slots`
labelled every resolved PS2 texture slot with `Ps2TextureSet::resolve`'s own
`{name_hash:08x}` string - the WAD entry's identity, since a PS2 WAD entry
carries only a hash, never a name (`oag_formats::wad::Entry` has no name
field). `oag_game::livery::ship_skin::apply` and `oag_render::gantry` both
match a slot by its *declared* name (`\TEXTUREn.TGA`, a billboard name), so
every match against a hash string failed silently. Fixed by relabelling each
resolved slot with the `Texture` node's own declared name (the same
last-path-component convention the PSP embedded path already uses), deduped
per distinct name so a track's hundreds of material slots still cost one
clone per distinct texture rather than one per slot. See
`crates/render/src/mesh/ps2_textures.rs`'s `resolve_texture_slots` doc.

**Bug 2, still open.** Fixing the label was not enough: a PS2 `Ship.vex`
does not declare `texture1.tga`..`texture4.tga` slots at all. `AG_Systems`,
for example, names `ALL_Textures.tga` (256x256) instead, and the nested
texture set has no standalone `textureN.tga`/`.pct` entry to fall back to -
confirmed by resolving all four names against the set directly, not merely
by their absence from a node. The PS2 build's paintable surface is a single
merged atlas where the PSP build has four separate slots.

**The original does apply these `.dat` files on PS2 too - confidence 95,
evidenced rather than assumed.** `strings SCES_547.48` (extract with
`just unpack extract data/images/pulse-ps2-eu.chd SCES_547.48 --out <dir>`)
carries `ship_alt.dat`, `ship_eliminator.dat`, `PI_ModelSkin`, the
`%s\%s.dat` format string, and `ALL_TEXTURES.TGA`/`\ALL_TEXTURES.TGA`
together - four load-bearing strings for one mechanism landing side by side.
See `docs/formats/ps2-texture.md`'s "A ship's paintable surface is one atlas
on PS2, four slots on PSP" section for the full writeup.

**What is not recovered: the byte layout the PS2 build's own applier writes
into that atlas.** A straight 2x2 tiling of the four 128x128 blocks fits the
dimensions (`256 = 2 * 128` both axes), but that is a reading of arithmetic,
not a measurement - there is no standalone `textureN.tga` left on the PS2
disc to decode and compare a quadrant against, so confidence on the packing
itself is well under 50 and nothing in this project implements it on that
basis. `crates/game/src/livery/ship_skin.rs`'s `apply` now detects this case
specifically and reports it (`this hull's paintable surface is
ALL_Textures.tga, not the four texture1.tga..texture4.tga slots ...`)
instead of the generic "no slot is named `texture1.tga`" miss, which would
otherwise read as "this hull has no skin support" when the disc's own
strings say the opposite.

Verified visually too: PSP `--team AG_Systems --skin Alternative` renders the
cyan livery; PS2 with the same flags still renders the baseline red/white
paint, honestly, rather than something invented.

## Open

- The PS2 counterpart of `Skin_ApplyToModel` (name unrecovered) has not been
  found or decompiled. Its xrefs to the `ALL_TEXTURES.TGA` string, or to
  `%s\%s.dat`, are where to look - needs the Ghidra bridge against the
  `ps2-pulse-eu` program.
- Whether the packing is really a plain 2x2 raster tile, some other order
  (the PSP's own block 4 has a documented "composite" alternate use -
  `ship-skin.md` - so a PS2-only composite is not implausible either), or
  something GS-swizzle-shaped the way this title's other PS2 textures are.
- Whether every base-roster team's hull follows the same one-atlas shape as
  `AG_Systems`, and whether the twelve-team PS2 DLC roster does too.

## Next Steps

1. Decompile the PS2 executable's own skin applier via its xrefs to
   `ALL_TEXTURES.TGA`/`%s\%s.dat` (Ghidra, `ps2-pulse-eu` program - pass
   `program=` explicitly, never `switch_program`). Name it per
   `docs/architecture/adr/0005-ghidra-conventions.md` once confidence allows,
   with a `docs/ghidra/functions/ps2-pulse-eu/` evidence page and a
   `names.tsv` row in the same change.
2. Once the layout is measured rather than read off dimensions, extend
   `mesh::ship_skin::apply` (or a PS2-specific sibling) to write the four
   blocks into the atlas at the recovered offsets, with a ground-truth test
   on the disc.
3. Re-screenshot the skinned craft on PS2 at player size once it draws, next
   to the PSP shot already on file, to confirm the two liveries agree.
