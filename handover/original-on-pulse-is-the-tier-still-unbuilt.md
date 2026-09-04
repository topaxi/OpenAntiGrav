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
- ~~The `m` face-to-vertex index mapping's exact byte layout~~ **Read
  2026-09-04**, and the parser is `oag_formats::shadow_occluder`: `u16[4]` of
  per-edge adjacent faces at `+0x10`, `u16[4]` of vertex indices at `+0x18`,
  the fourth repeating the first on a triangle. Reciprocal on **14,328 of
  14,328** edges disc-wide, every index in range on 4,381 of 4,381 faces, and
  every triangle's declared normal within **0.028 degrees** of the geometry of
  the vertices it indexes. `Occluder::silhouette` is the edge walk step 5
  needs.
- ~~The projection direction is what now blocks the draw~~ **Read
  2026-09-04**: `normalize(0.5, -5, 1)`, a *local* axis the node's own world
  matrix carries into world space. It sits in `.bss` at `g_shadow_direction`
  (`0x08b62540`), written by `Shadow_RegisterClass` (`0x08923518`) - the same
  function that registers class `0x3cb`, `shadow`. **The reason it read as
  unfindable is worth carrying**: both candidate addresses are reached through
  PRX relocations with `addr_base = 1`, so the obvious reading of the
  `lui`/`addiu` pair lands in `.text` and decodes as instructions - the
  `shield-pickup.md` trap, one segment over. Read `.rel.text` before believing
  an address has no bytes behind it.
- **`shadow` `0x3cb` is a direction override, not dead weight**, and the census
  line calling it inert now says so: `g_shadow_node_count` is a reference count
  incremented by its constructor and decremented by its destructor, and while
  one is alive every shadow projects along that node's own negated vector
  instead of the constant. No `.vex` on the disc authors one, so nothing
  shipped ever takes that path. What `0x0892342c` (the function that fills the
  override) belongs to is unread.
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
