# PS2 track texture sets: a duplicated node name shifts every later ordinal

Supersedes `12-tracks-sky-draws-one-white-face.md` (closed 2026-09-05): that
thread's specific white face is now a known symptom of this broader
mechanism, not a standalone mystery, and its "genuinely absent texture"
framing turned out to be wrong.

## What's established (see `docs/formats/ps2-texture.md`'s "near-miss
shortfall" section for full evidence, confidence 88)

The five PS2 circuits whose per-model texture set is short by 1-2 entries
against their `.vex`'s own `Texture` node count (`02_Track`,
`02_Track_reversed`, `06_Track_reversed`, `12_Track`, `12_Track_reversed`)
are short because one or two `Texture` nodes in each file share an identical
declared name with an earlier node, and the nested per-model texture-set WAD
holds one physical entry per **unique** name, not one per node. Every
circuit's duplicate-name count matches its shortfall exactly, checked
directly on all five.

Swept exhaustively on `12_Track` (all 152 of its `Texture` nodes, not just
the one with a visible symptom): name-hash lookup against the outer archive
(the already-95-confidence `.tga`->`.pct` rewrite) plus a sha256 match
against the 151 extracted nested-set entries gives, per node, the *correct*
physical slot independent of raw position. Result: ordinals 0-62 are
unshifted, ordinal 63 (the duplicate's second occurrence) resolves to the
same slot as its first occurrence (55), and every ordinal from 64 to 151
resolves to `slot = ordinal - 1`, with zero exceptions and a second,
unrelated content-identical pair (nodes 24 and 113) independently confirming
the same shift point.

`crates/render/src/mesh.rs`'s `build_with_textures` was read directly: it
resizes a short external set with `set.resize_with(set.len().max(slots),
|| None)`, appending padding at the end rather than reindexing anything -
a flat "node ordinal `N` reads external-set entry `N`" scheme with no
awareness of the packer's dedup. Under that scheme, `12_Track`'s ordinals 64
through 150 (87 of 152) each currently bind a real, validly-decoded texture
that is not the one the node actually asks for - its neighbour's, one slot
early - and only ordinal 151, past the padded end, falls back to the white
1x1. The white sky face this project already noticed is the one visible
symptom; the other 86 mis-bindings were not visible in anything checked so
far because nothing has swept `12_Track`'s main art mesh face-by-face the way
the sky/pads sweep did.

A control render (`01_Track`, an exact-match circuit with no duplicate
names, same flags) looked coherent; `12_Track`'s did not (large flat
incongruous-textured planes, an unexplained large diagonal shape) - weak,
suggestive evidence consistent with the finding above, not proof by itself.

## Open

- **What the original PS2 build's own loader does.** Everything above is a
  data-agreement finding (per-node content identity via a lookup route
  independent of node position), not a runtime trace. It does not say
  whether the original's loader is dedup-aware (matching the name-hash
  route) or whether it *also* reads this flat and has always mis-textured
  these 86-odd surfaces on real hardware, unnoticed. Two ways to settle this:
  - `Texture_FindOrLoad` (`0x0010c1e0`, already recovered for the
    `.tga`->`.pct` rewrite) - find its call sites for a PS2 track model and
    see whether the index it's given is the raw node ordinal or something
    already dedup-adjusted.
  - A real console or frame-exact PCSX2 capture of `12_Track`'s forward
    layout, at one of the surfaces the sweep says is mis-bound (e.g. node 69,
    `silo_plate.tga`, expected to actually show `stadium_side2.tga`'s pixels
    under the current flat scheme) - see
    `docs/reverse-engineering/pcsx2-debugger.md` for the frame-exact drive
    technique.
- **Whether the fix should ship without that corroboration at all.** An
  argument for yes: the dedup-aware mapping isn't a guess about the
  original's hardware, it's a correction to make *this project's own*
  documented invariant ("one nested-WAD entry per `Texture` node, in node
  order") hold in the one case where it's currently false, using data this
  project already trusts (the same name-hash lookup rule, at 95 confidence,
  used elsewhere without a runtime trace). An argument for no: shipping
  behaviour "corrected" to what the *packer* did without checking what the
  *runtime* did risks trading one wrong picture (white) for another
  (plausible-looking, but still not what the original draws) if the
  original's own loader turns out to also read flat - decide this before
  writing the fix, don't default silently either way.
- Whether the fix, if any, should live in `mesh::build_with_textures`
  (general, affects ships too, though no duplicate-name case is known there)
  or be scoped to the track code path in `oag_game::race::load` alone.
- Whether the white face (or any of the other mis-bound surfaces) is visible
  during an actual PS2 race on `12_Track` forward, or only from angles a
  flyable circuit's camera never reaches - not checked, from the original
  thread and still true.

## Next Steps

- Spend the corroboration step first (either recovered-function call sites
  or a PCSX2/console frame) - resolves the "what should this even mirror"
  question before any renderer change, per this project's own
  reverse-engineering workflow (observe -> hypothesise -> verify -> document
  -> implement).
- If dedup-aware indexing corroborates: implement a dedup-aware
  node-ordinal-to-set-index mapping (count unique `Texture` node names in
  file order; a repeated name reuses its first occurrence's index) at
  whichever call site the previous step points to, add a ground-truth
  regression test asserting no slot goes silently mis-bound on any of the
  five near-miss circuits (not just `12_Track`), and re-run the 2026-08-27
  visual sweep's method on all five to confirm nothing regresses on the 27
  already-exact circuits.
- If flat indexing corroborates instead (the original genuinely has this
  bug): document that finding at whatever confidence the evidence supports,
  and leave the current implementation as a faithful reproduction rather than
  "fixing" a bug the original also has - update `ps2-texture.md` and
  `skycube.md` to say so explicitly rather than leaving today's "not yet
  corroborated" language standing.
- Either way, once resolved, sweep `12_Track`'s (and the other four
  near-miss circuits') main art mesh face-by-face the way sky/pads already
  were, since this thread's evidence says the current mis-binding - if it is
  one - is not confined to the sky.
