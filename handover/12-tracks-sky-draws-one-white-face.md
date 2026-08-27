# `12_Track`'s sky draws one white face

A visual sweep of all 32 PS2 circuits' sky/pads texture resolution
(2026-08-27, see [`skycube.md`](../docs/formats/skycube.md#what-this-engine-does-with-it))
found the shared texture-set resolution generalises to 63 of 64 renders (32
circuits, sky and pads, both directions). The one exception: `12_Track`'s
`Skycube` (forward direction only - `12_Track_reversed` is clean) has a face,
`skycube1_nolightShape`, whose material names texture ordinal 151 - the model
wants 152 texture slots and the preceding archive entry's resolved set decodes
only 151, so that face binds the renderer's white 1x1 fallback and draws pure
white. Reproducible: `oag-view --sky 'Data\Environments\12_Track\track.vex'
--textures 2043 --draws --screenshot out.png` against
`data/images/pulse-ps2-eu.chd:54748/WADS2.WAD` (index `2043` is the entry
directly preceding the track model, found by directory position - see
[`ps2-texture.md`](../docs/formats/ps2-texture.md#how-a-model-finds-its-texture-set-directory-position-not-a-name)).
The white face is not visible from the CLI's default camera angle; spinning
the yaw around (`--yaw 4.04` from that same command showed it) is what makes
it show up, which is presumably also why it went unnoticed until this sweep
photographed every face rather than one default angle.

`12_Track` is one of five PS2 circuits whose resolved texture set is short by
1-2 slots against what the model's `Texture` nodes ask for
(`02_Track`, `02_Track_reversed`, `06_Track_reversed`, `12_Track`,
`12_Track_reversed` - `ps2-texture.md`'s "How a model finds its texture set"
section). The other four's missing ordinals happen to fall outside what their
sky/pads actually reference, so only `12_Track` shows a visible consequence.
**The root cause of the shortfall itself was never identified** - `ps2-texture.md`
already named it as an open question (a handful of `Texture` nodes per track
that reference no unique pixel data, a merge during packing, or something
else) before this white face gave it a concrete, visible cost.

## Open

- Why `12_Track`'s (and the other four circuits') resolved texture set is
  short by exactly 1-2 entries against the model's `Texture` node count -
  never identified, see `ps2-texture.md`'s near-miss paragraph
- Whether the missing ordinal 151 is genuinely absent from the disc's data, or
  present somewhere the directory-position rule does not reach (a second
  preceding entry, a different archive, a name-addressed fallback)
- Whether this white face is visible during an actual PS2 race on `12_Track`
  (forward direction), or only from viewing angles the flyable circuit's
  camera never reaches - not checked

## Next Steps

- Diff `12_Track`'s `Texture` node list (name, size fields) against the
  preceding entry's decoded blob count to see which specific texture(s) the
  model asks for that the set does not supply - the same kind of check that
  resolved the ship and boost-plume findings in `ps2-texture.md`
- If the missing texture is genuinely nowhere on the disc, this project's
  stance is to draw nothing and say so rather than invent a stand-in - a white
  face from an honestly-absent texture is consistent with that, and no fix is
  owed. But check what the original PS2 game actually shows on this face
  first (real hardware or an emulator capture) - if the original also draws it
  white, that closes the question; if it draws something else, the
  directory-position rule itself needs revisiting for this one entry
