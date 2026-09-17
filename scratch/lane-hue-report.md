# lane-hd-amphiseum-hue: localising the 154-degree Amphiseum hue gap

2026-09-17. Continuation of `lane-hd-track-lighting` (merged as `016ef0c6`).
This worktree already carried a committed predecessor turn (`d3718b48`,
report at `scratch/lane-amphiseum-hue-report.md`) that split the whole-frame
hue gap into a vertical tile grid; this turn cross-checks that split against
real material slots and the maintainer's own "channel order" lead, and adds
one new RE finding. Full technical account is in two places, not duplicated
here: [renderer.md](../docs/ghidra/functions/ps3-hdfury-eu/renderer.md)'s
newest two dated sections (both 2026-09-17, `lane-hd-amphiseum-hue`) and
the matching entries in the rendering handover thread named after HD's
brightness/bloom investigation (not cited by path here - nothing outside
`handover/` and `HANDOVER.md` may cite a thread file, per `CLAUDE.md`).

## Answer to the brief's three questions

**Localised: yes, to the dome/ceiling structure specifically.** A box-free
`--tiles 4x6` grid (predecessor's work, `hd-frame-compare.py`) showed the
top half of frame reads warm-to-blue-violet-wrong (hue gap 76-177 deg,
"ours brighter") while the bottom half (floor/barriers) reads a much
smaller gap (33-86 deg, "ours darker") - a genuine vertical split, not one
whole-frame number. This turn confirmed the split is a **real material
boundary**, not a coincidence of screen position: extending
`hd-material-probe.py` with `--pair-dir` (its hardcoded pair, `talons-
matched`, is gone from disk) and cross-referencing its `slot_map` by pixel
position shows the ceiling rows are drawn by `animhexlights`, `cf_diff_spec`,
`lambert` and `base_diffusespecular` (non-lightmap slots) - the largest
"ours brighter" luma deltas in the whole probe (-0.15 to -0.53) - while the
floor rows are `track_wall`/`track_surface_no_emissive`. Coverage was
**79.1%, below the script's own 90% floor** - treat as indicative, per the
script's documented caveat about its fog classifier being calibrated for
Talon's Junction by eye, not Amphiseum.

**`.gtf` decode: pixel-verified this session, clean negative.** The three
ceiling materials' own DXT1 albedo textures (`dc_hexgrid.gtf`,
`and_metaldark.gtf`, `dc_cement_base_edges.gtf`) were extracted off the disc
(`scripts/psarc.py cat`) and decoded with `crates/texture/examples/
gtf_to_png.rs` (already existed, unused until now). All three are plain
neutral grey by eye and by measured hue/saturation - no blue, violet or
gold cast. So the maintainer's own channel-order hypothesis is refuted for
these specific textures: the wrong colour is not coming from the texture
sample.

**Candidate mechanism found, not confirmed.** With near-grey albedo, three
of the four ceiling materials have no lightmap (`prelit` = 0), no colour
set on their chunks (`vertex_light` = 0), and a ceiling-facing normal that
clamps `sun_diffuse` toward zero against Amphiseum's own upward sun
direction - leaving `scene.light.ambient` (`Lighting.Constant ambient
color`, wired and read off `.envsettings`) as the only non-zero term.
Amphiseum's own ambient constant is `(0.745, 0.612, 0.925)`, hue **265.5
deg** - a hue-family match to the **285 deg** this session measured on the
rendered ceiling. Talon's Junction's own ambient constant is *also*
blue-leaning (hue 246 deg) but its HDR-magnitude warm sun dominates its
own sun-facing geometry, which is offered as why the same wired-blue
ambient never surfaces there. **`Lighting.Sky colour`** - neutral grey
(128-140) on both circuits, unlike either's tinted ambient constant - is
parsed into `oag_tables::envsettings` but **never consumed by
`crates/render` or `crates/game`** (confirmed by `rg` over both crates).
New RE: `Environment_RegisterLightingSchema` (`0x003a83d8`, confidence 80)
identified - the `.envsettings` key registrar, confirming `Sky colour` and
`Constant ambient color` land in the same live struct, offsets `+0x440`
and `+0x430`. **Whether anything downstream reads `+0x440` back out is not
traced** - that's the concrete next RE step, not this session's. No code
changed on the strength of this; it is a mechanism built from the shape of
authored data, not a read of the original's shader.

## Gate

`.py` files changed (`hd-material-probe.py`, and the predecessor's
`hd-frame-compare.py`), so the docs-only carve-out does not apply. Ran the
full `flock ... just` in the background this turn; watch
`/tmp/gate-just.log` on this machine for the result (not yet finished as
this report was written - the team lead should confirm before merging, or
ping this lane to confirm once it lands). `just check-docs`,
`just check-names` and `just check-status` (after `just gen-status`, which
was run and committed) all pass standalone.

## Committed

`9da6cc84` on `lane/hd-amphiseum-hue`: `hd-material-probe.py` (`--pair-dir`
+ center-crop fix), `renderer.md` + handover thread's new dated sections,
`names.tsv` (+1 row), `docs/overview/status.md` (regenerated). Predecessor's
`d3718b48` (tile grid) was already there when this turn started.

## Still open

- Trace `Environment_RegisterLightingSchema`'s struct (`iRam008b6fb4`)
  offset `+0x440` (Sky colour) for any reader downstream - if none exists,
  the "sky colour blended in for ceiling-facing surfaces" mechanism is
  dead on arrival and the real cause is still unknown.
- Measure the per-pixel `ndl` on the ceiling's own drawn chunks directly,
  rather than inferring `sun_diffuse ≈ 0` from the authored sun direction
  and assumed normal facing.
- `Ambient false direction` (`(0.1, 1.0, 0.1)` Amphiseum vs `(1.0, 1.0,
  1.0)` Talon's Junction) is read, unused, undecoded - a per-axis weight
  is a plausible shape for a normal-driven ambient blend but this is
  pattern-matching on the key's shape, not RE.
- The predecessor's own open items stand: extend the material probe past
  one 79.1%-coverage, bloom-off sample; the `track_surface`/`prelitScale`
  lead is still Talon's-Junction-only evidence and a separate defect from
  this one.
- Whether Amphiseum's bloom chain itself over-contributes vs. the
  original's, raised two sessions ago, still not reached.
