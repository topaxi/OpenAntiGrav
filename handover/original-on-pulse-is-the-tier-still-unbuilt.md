# `original` on Pulse is the tier still unbuilt; `off` and `blob` are built

2026-09-04. Rewritten from the file that split off
[shadows-are-planned-and-pulses-occluder-payload-closes.md](shadows-are-planned-and-pulses-occluder-payload-closes.md)
on 2026-09-03 - **steps 2 and 3 of
[`docs/rendering/shadows.md`](../docs/rendering/shadows.md)'s plan have landed
and this file is what is left**, which is step 5.

## What landed

- **The setting.** `oag_game::display::Shadows`, `off` and `blob`, in
  `[render_profiles.<title>] shadows`; a `SHADOWS` row on the GRAPHICS page;
  `--shadows` as a per-run override. `original` and `mapped` are refused by
  `FromStr`, and `display/tests.rs` names both so whoever lands one deletes
  their line beside a new variant.
- **`blob`.** `oag_render::shadow` draws it, `oag_game::race::shadow` places
  it. HD's own nine `ambient_shadow.gtf` are the silhouette where the source
  ships them, a generated falloff elsewhere, and the load report says which
  happened per slot. **Diffed `off` against `blob` on four titles** rather
  than eyeballed: Pulse PSP 8,595 pixels changed at a worst delta of 43,
  Pulse PS2 4,637 at 75, Pure 16,924 at 166, HD/Fury 25,022 at 77.

Two things worth carrying forward from doing it:

- **A shadow can be uploaded, drawn, and invisible.** The first fade faded
  linearly from the ground, so a craft resting at its own ride height came out
  at strength `0.030` - 5,262 pixels changed by a maximum of 2, which reads
  exactly like "the pass never ran". An `off`-versus-`blob` PNG diff with the
  threshold at **zero** is what told the two apart; at any threshold above 4
  the frames looked byte-identical.
- **`raycast` returns the nearest hit of any surface**, so the placement casts
  `raycast_all` and takes the nearest `Floor`/`MagFloor`. A nearest-hit query
  puts a craft's shadow up the barrier it is scraping.

## Open

- **Step 5, `original` on Pulse: draw the 119 local-space hulls.** Everything
  it needs is decoded - the `0x3c3` payload
  (`crates/formats/tests/shadow_occluder_ground_truth.rs`),
  `Shadow_RenderOccluderVolume`'s own projection math (its **own local axis**,
  never a light), and the static class link. This is now a rendering task.
- **The `m` face-to-vertex index mapping's exact byte layout**, per
  [`shadow-occluder.md`](../docs/ghidra/functions/psp-pulse-usa/shadow-occluder.md)'s
  own Open section. Resolve it *before* step 5's draw code needs it: a
  plausible-looking but wrong winding is exactly the invented stand-in
  `CLAUDE.md`'s rule warns against.
- **Whether the 10 unnamed/world-space occluders are a track feature at all**,
  or authored-and-inert the way `DirectionalLight` turned out to be. If step 5
  draws only the 119 named ones, say so in the implementation rather than
  silently dropping the rest.
- **Whether `original` should be the default once it exists.** A design
  question needing eyes on both tiers side by side, which is now possible for
  the first time on one of them.
- **Nobody has looked at `blob` in a window.** Every judgement here is from a
  headless capture; the fade's shape, the falloff's darkness and `LIFT`'s size
  are all ours and none has been seen in motion.
- **HD's `ambientShadowBlendFactor`** is a named engine parameter
  ([`shadows.md`](../docs/rendering/shadows.md)) whose value has never been
  read. `blob` currently draws the disc's coverage at face value; if that
  constant scales it, ours is too dark or too light by whatever it says.
- **The padded-bbox field's exact selection rule** is still not fully
  explained - does not block drawing, wanted for an exact reproduction.

## Next Steps

- **Step 5**, in the order the design page gives: resolve the `m` indexing
  question, then draw Pulse's 119 local-space hulls through
  `Shadow_RenderOccluderVolume`'s own projection.
- Look at `blob` in a window on one title and judge the fade, the darkness and
  the lift - the three numbers that are ours.
