# Wipeout HD's unlit weapon programs

2026-09-25. What the LeachBall's and the Plasma bolt head's own materials
compute, read off their fragment programs rather than guessed from their names
or from the black vertex colour both models carry, and the shading path
`mesh.wgsl` now has for them. Both were loaded and placed but gated off,
because this engine's one shared `mesh::rcs` shader lights everything it draws
and neither program takes a light at all.

Tools: `scripts/ps3-microcode.py fp-file`/`vp-file` for the listings,
`crates/render/examples/hd_unlit_probe.rs` for the raw material state,
vertex layout and winding, `crates/render/examples/hd_unlit_census.rs` for the
disc-wide survey, and `crates/game/tests/hd_rim_glow_ground_truth.rs` for
what is pinned.

## The variants the race resolves

The lit race pass's key is `HalfBrightAmbientSunSpot0SVC0` for every chunk
here, and every resolved block is a **fogged** one: `fogColour` (`0x3dc31258`)
and `globalAlphaScaler` (`0x4c13d3af`) are patched in around the arithmetic
below, which `mesh.wgsl`'s existing `fogged` and the identity scaler (see
below) already cover.

| Material | Block | Blend (`src`/`dst`) | State | Draws |
| --- | --- | --- | --- | --- |
| `hd_leachbeam_ball_glow` | `@0x19b0` (`DATA00`) | `0x0302`/`0x0001`, `SrcAlpha`/`One` | `0x39`, culls | the LeachBall's sphere, `hd_missile_ball_bloomring`'s, `detonator_pickup`'s |
| `hd_leachbeam_bloomring` | `@0x16a0` | `0x0302`/`0x0001`, `SrcAlpha`/`One` | `0x29`, does not cull | the LeachBall's ring |
| `plasmasphere_subtractive_glow` | `@0x18d0` (`DATA02`) | `0x0302`/`0x0303`, `SrcAlpha`/`OneMinusSrcAlpha` | `0x79`, culls | `HD_plasma_ball`'s two shells |

**Two copies of `hd_leachbeam_ball_glow` ship, and they differ.** `DATA00`'s
(28,368 bytes) bakes `0.9` into block `@0x19b0`; `DATA02`'s (29,984) moves it
into a model parameter, `0x743ea80b` (no preimage), which the LeachBall's own
`.rcsmodel` authors as `0.1` - `(1 - p)` is the same `0.9` - and additionally
multiplies the output alpha by `VertexColour1.w`. The race's own
`oag_assets::Archives` for HD serves `DATA00`'s (pinned by
`the_race_serves_the_data00_copy_of_the_leachball_glow_material`), which is
the one routed below; the `DATA02` shape is not matched. Which copy the original serves is the same open question
`docs/formats/hd-frontend.md` carries for `skin.xml`. Confidence 90 on the
arithmetic of both, read instruction by instruction.

## What they compute

The vertex programs (`hd_leachbeam_ball_glow` block `@0x1860`,
`plasmasphere_subtractive_glow` `@0x1780`) are the engine flare's shape:
`TC0 = (eye - P, Uv1.x)`, `TC1 = (normal, Uv1.y)`, `TC2 = (P, clip w)`, the
normal untransformed. Confidence 92.

**`hd_leachbeam_ball_glow` `@0x19b0`:**

```text
noise = tex0(u, v + 0.0001 time).a              @0x00-0x08
c     = tex0((u, v) + 0.2 noise + 0.4 time).rgb  @0x0d, @0x15, @0x17 - both axes
rim   = saturate(1 - N.V)                       @0x0a-0x13, DIVSQ-normalised
a     = (saturate(0.9 - 0.9 rim^5))^5           @0x1a-0x2a, LG2/MUL 5/EX2 twice
rgb   = a * c / (1 - c)                         three RCPs of half registers
alpha = a * globalAlphaScaler.y + globalAlphaScaler.x
then the fog lerp                               @0x2b-0x33
```

Brightest face-on, gone at the silhouette, and `c / (1 - c)` expands the
texture's bright end towards infinity - a bloom source. `hd_leechbeam_ball.gtf`
reaches 255 in every channel, where the original's half-precision `RCP` of
zero is an infinity; `mesh.wgsl` holds it at 65504, the largest finite half,
**chosen, not measured**, rather than let an infinity into the float target.

**`hd_leachbeam_bloomring` `@0x16a0`:** one tap at the authored `(u, v)`,
`rgb = tex.rgb`, `alpha = tex.a * gas.y + gas.x`, fog. The "ramp" in its
texture's name (`hd_leechbeam_ball_bloomring_ramp_128x32.gtf`) is indexed by
the ring's own authored `u`, which runs `0.0016..0.99` round it with `v` held
at `1.0` - not by any computed scalar. This is exactly what
`slots::EMISSIVE` (a program fed neither ambient nor sun) already draws, so it
needed nothing new.

**`plasmasphere_subtractive_glow` `@0x18d0`:** the same two taps, then

```text
rgb   = 1000 rim^5 * c                          @0x19-0x22
alpha = 0x7611a2d8 * gas.y + gas.x              the model authors 1.0
```

Dark face-on, blown out at the silhouette, and opaque over what is behind it.

## `globalAlphaScaler` is the identity, and the blend equation is `ADD`

`globalAlphaScaler` is engine parameter slot 76
([renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md#the-engines-own-parameter-table-read-from-its-initialiser-2026-08-24)).
`Scene_PrepareFrame` publishes `block + 0x7c50` into it (`stw r4, 0x998(r11)`
at `0x003ab590`, `r4 = r31 + 0x7c50`), and `Scene_InitRenderBlock` fills that
vec4 from stack words it has just written `(0.0, 1.0, 0.0, 0.0)` (`lvx` of
`r1 - 0x10` at `0x003aa7c0`, `stvx` to `+0x7c50` at `0x003aa7e0`). An
operand search for `0x7c5` over the whole executable finds those two and two
more `li r11, 0x7c50` (`FUN_000ddd58`, `FUN_000dfd90`) that were not traced to
this block. So `alpha * 1 + 0` for
every draw that does not override the table entry itself - which
`Ship_DrawModels` and `FUN_003eb890` do for craft, and nothing on the weapon
path was found to. Confidence 80.

The RSX blend equation is written by one function, `Rsx_SetBlendEquation`
(`0x005c2130`, method `0x40320`), and every caller - 13 direct, 6 through the
thunk at `0x00678238` - passes `0x8006`, `GL_FUNC_ADD`. See
[material-state.md](../ghidra/functions/ps3-hdfury-eu/material-state.md). So
"subtractive" in the Plasma material's name is not a blend mode; it is the
opaque dark core the program paints.

## The path, and how it is selected

Two `slots` bits, `RIM_GLOW` (bit 11) and `RIM_EDGE` (bit 12), set in
`mesh::rcs::skin::roles` by `mesh::rcs::rim_glow::classify` from the resolved
program alone: its mnemonic sequence, every literal the file authors that no
parameter patches over, its declared parameter set and a single sampler. The
literals are the load-bearing part - `mesh.wgsl` hard-codes exactly those
numbers for each bit, so a program with the same skeleton and a different
constant is refused rather than drawn with these. `RIM_EDGE` also requires the
material's own `0x7611a2d8` to be `1.0`, since the path carries no
per-material alpha.

`mesh.wgsl` computes both results unconditionally and `select`s on the bits
after everything else, so a draw with neither bit returns exactly what it
returned before. **Measured, not assumed**: Pulse PSP and HD race frames at
tick 90 (`--race --ticks 90 --hold cross`), HD's LeachBeam lock at tick 15 and
a Plasma bolt at tick 106 all `cmp` byte-identical before and after this path
with both gates still off. No pipeline layout, vertex layout or override
constant changed, so `mesh_render::pipeline_cache::Key` needed nothing.

## Who else uses these programs

`hd_unlit_census.rs`, every drawn material slot of every `.rcsmodel` in all
seven archives (10,877, 267 with no resolvable lit-race program):

- `RIM_GLOW`: one material, three models - the LeachBall, the Missile's head
  (`hd_missile_ball_bloomring`, not wired to anything yet) and
  `detonator_pickup` (not drawn by this engine).
- `RIM_EDGE`: one material, `HD_plasma_ball`.
- No near misses: no program opening either skeleton is refused.
- The texture-only shape the bloomring has: the four circuits'
  `pvsblocker/*/emissivealpha`, `screen_test`, `hd_bombfire_bloomring` and the
  bloomring itself - all already on `slots::EMISSIVE`.

## An inline stride-18 vertex can carry its colour last

The three spheres above are **inline** chunks, which declare no vertex layout,
and `rcsmodel::Mesh::texcoords` read their `Uv1` from the last four bytes, the
reading every inline chunk got. Here those bytes are a `VertexColour1`
(`ff ff ff cc` on every vertex), two `NaN` halves, and `Uv1` sits at `+0x0a`
right after the normal. Declared stride-18 layouts carry both orders - 834
chunks put `Uv1` first, 554 the colour - and nothing in the chunk or in the
material's attribute slots predicts which (38,054 declared chunks run their
offsets in slot order, 834 do not). So the rule is read off the data: an
inline stride-18 chunk whose tail is non-finite on some vertex and whose
`+0x0a` is finite on every one reads `+0x0a`. `hd_unlit_probe.rs
--undeclared`: 62 chunks qualify, 12 are non-finite in both places and keep the
tail, 118 have a finite tail and are untouched. Confidence 85.

**This one changes pixels that were already drawn.** Of the 62, the Plasma
blast's halo (`HD_plasma_halo`) and HD's shield cockpit (`vr_shield_cockpit`)
are on screen today; the rest (`frontendscene_hd_atg`, `aurora`, `drone`,
`rockettrail_triangle`, the Detonator and Missile explosion models, the flyer)
are not drawn by this engine. The halo now shows its texture instead of one
flat sample: `--give plasma` with a grid-shot input script, tick 190, before
and after differ, and the after frame's dome carries the streaks its
texture authors. Measured on that frame: 310,143 of 1,175,040
pixels move, 3,654 of them by more than 16 levels, mean luminance 132.0 to
131.5 - the dome picks up texture detail, nothing gets worse.

The 62 by model: `frontendscene_hd_atg` 43, `hd_plasma_ball` 2,
`detonator_bomb_shockwave` 2, `00_flyer` 2, `drone` 2, and one each for
`aurora`, `pvs_blocker/amphiseum`, `detonator_bomb_glowedges`,
`detonator_mine_explosion_kaleidoscopic`, `lightbarrier_new_shockwave`,
`rockettrail_triangle`, `hd_leachbeam_ball_bloomring`,
`hd_missile_ball_bloomring`, `hd_missile_explosion`, `hd_plasma_halo` and
`vr_shield_cockpit`. The shield cockpit's tail is `7f 7f 7f 7f`, two `NaN`
halves, so before this it sampled one texel; now its `Uv1` spans `u
0.20..0.40`, `v 0..1`. **It was not looked at in a frame**: `--give shield`
never activates the shield in a headless run (`holding Shield` through tick
420 with `--press square`, internal camera), the same limit HD's shield-flash
work recorded, and `oag-view` draws the additive sphere as nothing against
its black background either way.

## The clock-scroll programs (2026-10-05)

The Plasma explosion's ring and halo (`hd_plasmaring_glow` `@0x1900`,
`hd_plasmahalo_glow` `@0x1960`) are the same skeleton as the two rim glows
over the declared `UV_offset` - the model's own clock, the blast's age -
rather than `time`. They earn `slots::CLOCK_SCROLL_RING`/`_HALO` through
`rim_glow`'s fingerprint; only the colour tap's coordinates are played. The
programs, the evidence and the frames are in
[plasma.md](../ghidra/functions/ps3-hdfury-eu/plasma.md), 2026-10-05.

## Gates

- **LeachBall: drawn.** `load::weapon_models::LEACH_BALL_DRAWN` is deleted
  rather than flipped. Its model does not cull (the ball's material sets
  state bit 4, the ring's does not, and `cull_as_authored` refuses a mixed
  model); a face turned away from the eye has `rim = 1`, so `a = 0` and the
  additive blend adds nothing there. In play it reads as a small cyan energy
  ball between the craft and its target.
- **Plasma head: drawn (2026-10-06).** `blast_models::HD_PLASMA_BALL_DRAWN` is
  deleted rather than flipped. The defect that held it off was not the cull
  (below), and the 2026-09-25 reading of it was wrong on that point.
  `HD_plasma_ball` is two coincident spheres, one wound outward and one
  inward (`hd_unlit_probe.rs --built`: 760 of 760 triangles counter-clockwise
  about their own normal in both, 760 and 0 wound outward, 760 and 0 with an
  outward vertex normal), under a material that culls; the cull and the
  winding are consistent. What was wrong is the eye: `rim = 1 - N.V` reads
  the camera out of the drawable's own scene block (`Fog::camera`), and the
  weapon drawables are written no scene block at all, so theirs held
  `Scene::off`, whose camera is the origin. With a bolt at about `(6, -48,
  -190)` the eye vector came out as `-position`, which is `(0.48, 0.62, 0.99)`
  as a colour - the same value the debug frame showed - and `N.V` flipped sign
  across the middle of the disc, so most of it was `rim` near 1 and white.
  Proofs, on Talon's Junction at tick 112 with `--give plasma`: the flipped
  cull (`Face::Front` on the blended pipelines) changed 2,379 pixels and the
  disc stayed white; writing the hull's scene block to the ball's drawable
  (`Scene::write_ship_scenes`) gave a dark core inside a blown-out rim, which
  is what the program's arithmetic predicts. Pinned by
  `crates/game/tests/hd_plasma_ball_ground_truth.rs` (387 dark pixels in the
  bolt's box with a bolt, 16 without, 0 with the write dropped). **Checked
  against Omega: not checkable** - Omega's bolt model is not built (its race
  is incomplete, `omega-status.md`).

## Weapon scene blocks (2026-10-06, hd-weapons)

`weapon_models::write_fog` writes the circuit's own scene block (fog, light
rig, **eye**), Zone half off, onto every weapon drawable whose model a PS3
program shades (`Drawable::is_ps3_shaded`: some material resolved to an
`.rcsmaterial` variant; a Pulse or Pure body answers no, and a Pulse
`--give rocket/mine/bomb/plasma/cannon` frame is pixel-identical either side).
Zone is off because the disc's 39 weapon materials carry no Zone variant.
Pinned by `hd_weapon_scene_ground_truth.rs`, `hd_leach_ball_ground_truth.rs`
and `hd_plasma_ball_ground_truth.rs`, each written / write dropped, and all
seven fail with the write dropped:

| Drawable | Metric | Written | Dropped |
| --- | --- | ---: | ---: |
| LeachBall (`RIM_GLOW`) | near-white in the ball's box, close camera | 1,186 | 369 |
| Plasma head | dark core pixels | 387 | 0 |
| Plasma explosion shells | violet pixels | 10,521 | 38 |
| Mine halo spikes | green pixels | 5,751 | 115 |
| Rocket body | near-black in its box | 57 | 1,675 |
| Bomb body | near-black in its box | 1,463 | 2,297 |
| Cannon muzzle flash | near-white in its box | 2,916 | 2,849 |

The LeachBall, Plasma and Mine rows are programs that read the eye. The
Rocket, Bomb and Cannon rows rest on the engine's globals alone:
`material-state.md` names no program for them, so this is **chosen, not
measured** against an original capture (none exists); the Rocket's body is flat
black without the rig, which is the visible defect. The Cannon's margin is 67
pixels. **Checked against Omega: checked, applies, not wired** - Omega's
`WeaponModels` is `EMPTY`, so it builds no weapon drawable; once named through
`mesh::rcs` they take the write by the same predicate.

## Open

- **Not written:** the Bomb's blast pair, its Repulser field and mag floor. A
  detonation needs a rival to run onto the bomb (lap 2 with `--opponents`), so
  no AI-independent frame was kept.
- Which archive's `hd_leachbeam_ball_glow` the original serves; the two copies
  differ by the vertex alpha only.
- No capture of the original's LeachBall or Plasma head exists under
  `data/traces/` or `data/scratch/`, so the pictures are judged against the
  programs, not against the game.
