# Wipeout HD's unlit weapon programs

2026-09-25. What the LeachBall's and the Plasma bolt head's own materials
compute, read off their fragment programs rather than guessed from their names
or from the black vertex colour both models carry, and the shading path
`mesh.wgsl` now has for them. Both were loaded and placed but gated off,
because this engine's one shared `mesh::rcs` shader lights everything it draws
and neither program takes a light at all.

Tools: `scripts/ps3-microcode.py fp-file`/`vp-file` for the listings,
a one-off read for the raw material state,
vertex layout and winding, `crates/render/examples/hd_unlit_census.rs` for the
disc-wide survey, and `crates/raceplay/tests/hd_rim_glow_ground_truth.rs` for
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
`+0x0a` is finite on every one reads `+0x0a`. Counted off the data: 62 chunks qualify, 12 are non-finite in both places and keep the
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
  inward (a one-off read of the built mesh: 760 of 760 triangles counter-clockwise
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

## The Bomb's fireball and shockwaves

2026-10-07. The three materials of HD's Bomb detonation
([weapons.md](../ghidra/functions/ps3-hdfury-eu/weapons.md), 2026-10-07), read
off their resolved fragment programs with `scripts/ps3-microcode.py` and the
vertex programs beside them. Also `hd_bomb_programs.rs` (which block each
model resolves to).

**Four material parameters are bound by pointer** (`FUN_00677018`): `AlphaAnim`,
`V_Anim` and `Shockwave_scalar` to the model's own clock (`node + 0xc0`, what
`AnimNode_UpdateTransformTree` sets), `ColourAnim` to a float of the blast
object. The blast sets the clock from its own arithmetic and the models carry
no moving `Anim Transform` key, so each is a number the blast writes
(`bomb_blast::hd`).

| Material | Block | Models | State | Bit |
| --- | --- | --- | --- | --- |
| `hd_bombfire_glow` | `@0x1d90` (vertex `@0x1c20`) | `HD_bomb_sphere`, `HD_bomb_sphere_white` | `0x6e`, cut-out, no cull | `BOMB_FIRE` |
| `hd_bombfire_bloomring` | `@0x16a0` | `hd_bomb_sphere_bloomring` | `0x29`, `SrcAlpha`/`One` | `EMISSIVE` (existing) |
| `hd_bombfire_shockwaves_glow` | `@0x1950` (vertex `@0x17c0`) | `hd_bomb_shockwaves` (x8) | `0x29`, `SrcAlpha`/`One` | `BOMB_SHOCK` |
| `hd_bomb_halo` | `@0x19c0` (vertex `@0x1860`) | `HD_bomb_halo`, one chunk of `HD_bomb` | `0x79`, `SrcAlpha`/`One` | `BOMB_HALO` (bit 21; see `weapons.md`, 2026-10-08) |

**Both new bits are pairs of existing ones** (`BOMB_FIRE = RIM_GLOW | RIM_EDGE`,
`BOMB_SHOCK = CLOCK_SCROLL_RING | CLOCK_SCROLL_HALO`): the role word keeps
`MATERIAL_SHIFT = 23`, because bits 23 and up are the material index and a
circuit needs hundreds of those. No material earns both single bits, and every
test of one of them alone now excludes its pair. The mesh uniform's second pad
becomes `model_colour` (`ColourAnim`).

**`hd_bombfire_glow` `@0x1d90`** (vertex: `TC0 = (eye - P, u)`, `TC1 = (N, v)`,
`TC2.w = u`, `TC3 = (v, clip w)`). The LeachBall's skeleton with the blast's two
scalars in it; `DIV` takes its numerator from the destination component of its
first source (`.w` here) and its divisor from the second source's `.x`, while
`RCP` and the transcendentals read `.x` - so `RCP H0.w, H3` is `1 / (1 -
ColourAnim)` and the alpha's numerator is `H3.w`:

```text
nA   = tex(u, 0.3 t + v).a                              @0x0d
B    = tex(u, nA + v + 0.04 t)                          @0x33
c    = tex(6u, 6 (0.3 t + v)).rgb                       @0x1b
rim  = sat(1 - N.V);  h = sat(0.86 - 0.86 rim^5)^50     @0x26-0x2e
rgb  = (B.r + 100 h, B.g + 100 h, B.b) / (1 - c) / (1 - ColourAnim)
a    = sat((0.5 B.a + 0.5 - AlphaAnim) / (1 - AlphaAnim))   then * gas.y + gas.x
```

`AlphaAnim = 1` (the first 0.75 s) divides by zero: a negative numerator
saturates to nothing, and the `0 / 0` of an opaque texel is **chosen, not
measured** to be nothing too. **The material's alpha test is `GL_LESS` against
`0.5`** (`alpha_func 0x0201`, read off the `.rcsmodel` by `hd_bomb_programs`):
the one cut-out here that keeps the *low* alpha, so the fireball is whole while
`AlphaAnim` is 1 and burns away texel by texel as it falls to 0.44. The engine's
cut-out pipeline hardcodes `GL_GREATER` (`cutout::of`), so the loader reports
this surface "alpha test unread, drawn opaque" and `mesh.wesl` makes the test
itself for `BOMB_FIRE`. `(1 - c)^-1` and `(1 - ColourAnim)^-1` are held at
the largest finite half, `65504`, as the LeachBall's is. The white core is the
same program with `ColourAnim = 0.9` (`x 10`).

**`hd_bombfire_shockwaves_glow` `@0x1950`** (vertex: `TC3 = (Uv, VertexColour2.w,
clip w)`, `TC0.xyz = VertexColour1.xyz`; the chunk is an inline stride-22 one,
see below):

```text
n   = tex(u, v + 0.01 S).a                              @0x08
c   = v + 0.05 n + 0.45 S                               @0x0d
rgb = tex(u + n, c).rgb * VertexColour1.rgb             @0x14, @0x19
a   = VertexColour2.a * sum(tex(c, c).rgb)              @0x11-@0x16
```

`TEX H0.xyz, R1.xxxx` samples at `(c, c)`, the diagonal; the colour tap is the
2D one. `S` is `Shockwave_scalar`, the model's clock (0 to 1 over each ring's
window).

**The coordinate of an inline stride-22 chunk** that ends in two colours reads
`NaN` out of its last four bytes (`ff 9f 00 4c` twice on
`hd_bomb_shockwaves`). `oag_rcs::rcsmodel` now reads it at `+0x0a`, as it
already did for stride 18: a one-off read of the undeclared chunks finds exactly four such
stride-22 chunks (`hd_bomb_shockwaves`, both of `hd_plasma_ring`'s and
`hd_missile_explosion`'s), eleven others keep their tail. **This moves the
Plasma explosion's ring** (its coordinate was `NaN`, zeroed): the same program
shape, `(u, v) = TC3.xy`, so the texture it samples varies across the disc now
(`the_explosion_ring_and_halo_earn_their_clock_scroll_bits` still holds). **Not
looked at in a frame**: a detonation of the player's own Plasma is out of reach
of a headless run (the bolt times out ten seconds away).
The two colour fields fold into the vertex's `colour` and `sun_mask`
(`Mesh::inline_two_colours`), only for a surface that earned `BOMB_SHOCK`.
Confidence 85 on the arithmetic of both programs, instruction by instruction;
what is chosen is named above.

**What the picture shows** (Talon's Junction, a laid Bomb tripped by a rival,
camera posed from the far side; `--force-bomb-trip`): a white-hot textured
fireball and a flat orange disc at 0.2 s, the cooled fireball with brown smoke
puffs at 0.67 s, the fireball whited out at 1.17 s, then the seven rings as a
stacked glow at 1.45 s. **No capture of the original exists to compare with.**
Pinned by `hd_bomb_blast_ground_truth.rs`.

### The Bomb's units, read live (2026-10-26, `hd-render-fidelity`)

**Claim**: the fireball's texture is sRGB-decoded on fetch; the shockwave ring's is not.
**Confidence 80** (one live blast, one frame; the draws are named by their state and order,
which match `weapons.md`'s `bomb1` table: fireball, core, bloom disc, first ring).

RPCS3, Racebox Single Race on Talon's Junction (`hd-racebox-talons-venom-weapons-grid`), the player
standing, `scripts/rpcs3-hd-whiteout.py --bomb --delay 0.1 --attempts 4 --no-targets` (the command
stream was read whole; the run then stalled reading vertex spans, which the unit words do not need).
`SET_TEXTURE_ADDRESS` of unit 0, gamma nibble in bits 20-23:

| draw | state | unit 0 | address word | nibble |
| --- | --- | --- | --- | --- |
| 584 | opaque, alpha test `LESS`, cull off | 1024x1024 DXT45 | `0x60710101` | **7**, decoded |
| 585 | the same (the core) | the same texture | `0x60710101` | **7**, decoded |
| 586 | `SrcAlpha`/`One`, no depth write (the bloom disc) | 256x128 DXT45 | `0x60710101` | **7**, decoded |
| 587 | `SrcAlpha`/`One`, no depth write (the first ring) | 256x256 DXT45 | `0x60010101` | **0**, raw |

The lines are kept in `data/reference/hd-flame-tap/bomb-blast-units.txt`, the command stream in
`data/reference/hd-flame-tap/bomb-blast/` (`python3 scripts/rsx-draw-list.py
data/reference/hd-flame-tap/bomb-blast bomb0 --samplers --from 584 --to 587`).

What it changes:

- **The fireball** (`BOMB_FIRE`) read its two colour taps raw and drew pale, near white, where
  the console decodes them; `shade.wesl` now decodes `B.rgb` and `c` (the divisor's tap too) on HD's
  encoded store (`encoded_out`), and leaves both alphas raw. Pinned by
  `hd_bomb_blast_ground_truth` (yellow pixels at 0.67 s: 2,263 raw, 21,247 decoded; near-white
  4,230 to 1,270). The film of 2026-10-07 (`hd-bomb-match`) showed the original's ball yellow
  where ours was white; it was a Write-Color-Buffers-off capture, so it is direction, not a
  number. No WCB-on frame of the blast exists yet.
- **The bloom disc** is on the generic emissive path, which already decodes on the linear target:
  matches, unchanged.
- **The shockwave rings** (`BOMB_SHOCK`) read their tap raw, and so does the console: matches,
  unchanged. So the rings burning white at 1.8 s since the encoded store is the console's law
  applied to a raw source, not a missing decode; whether the original's rings are as white is
  still a picture nobody has taken (a WCB-on frame at about 1.8 s).

**Omega**: checked, does not discriminate, not wired: Omega ships the Bomb's textures
(`Data/weapons/Textures/HD_Bombfire_BloomRing_FireRamp_128x32.gnf` and others) and every one read
is `ChannelType::Srgb`, the ring's too, so the file does not tell what its samplers decode; its
programs are GCN and unread and its scene blends linearly (`hd-track-tone.md`), so nothing was
ported. **2048**: not checkable from the file (a `.gxt` header carries no gamma field) and no
Vita3K draw state of a blast has been read.

### The weapon units, read live (2026-11-14, `hd-weapon-taps`)

**Claim**: of the weapon programs the Bomb page left unread, the Missile blast's core and rays,
the LeachBall and its ring, the Plasma head and halo and the Rocket's smoke read their colour
tap **decoded on fetch** (unit 0's gamma nibble 7); the Missile's shockwave ring and the Plasma
ring read it **raw** (nibble 0). **Confidence 85** for the Missile blast (two shots agree draw
for draw), **80** for the LeachBall and ring, the Plasma head and the Rocket smoke (two frames
each), **70** for the Plasma ring and halo (one explosion). Falsifier written before the first
capture: a draw attributed to the program whose unit 0 reads another nibble than the one
claimed, or a draw that cannot be attributed (it stays unread).

**Method.** RPCS3 on the EU disc, Racebox Single Race on Talon's Junction with Weapons On
(`hd-racebox-talons-venom-weapons-grid` state), the player given the pickup and, for a hit,
put 6 to 12 units behind the nearest rival at its heading and speed:

```sh
uv run --with evdev python3 scripts/rpcs3-hd-whiteout.py --image <iso> --out <dir> \
    [--fire-state N --behind | --leach] --delay 0.25 --no-targets --no-baseline \
    --attempts 14 --want-mats <dir of the effect's rcsmaterial files>
python3 scripts/rsx_fp_match.py <dir> <frame> <rcsmaterial dir>
```

A pause lands anywhere in the frame the game is building, so a dump may stop before the effect's
draws. `--want-mats` keeps the pause-and-dump going (resume 0.08 s, pause) until the frame holds a
draw of one of the named programs; `--want-name` narrows it to one material. `rsx_fp_match.py`
attributes each draw by its **fragment program**: it disassembles the live program out of the
dump, strips the constants and compares it with every fragment block of the effect's
`.rcsmaterial` (`psarc.py cat <image>:<archive> <path>`), so a draw is named by what it runs,
not by its order. Blocks of under four instructions are skipped (the one-tap copies read the
same in every material). The LeachBall's two copies (`DATA00`, `DATA02`) are both matched; the
live draws match the `DATA00` copy, as the race's own archive order says.

**Control**: the Bomb, frame by frame the same as the 2026-10-26 read (fireball and core 7,
bloom disc 7, first ring 0), draws 556 to 559 of 678.

| Program | Frame, draws (of N) | Unit 0 address word | Nibble | In ours |
| --- | --- | --- | --- | --- |
| Missile core, `hd_missile_explosion_core_glow` | 0.067 s, 553 of 702; 0.117 s, 689 of 833 | `0x60710101` | **7** | decoded (`missile_core`) |
| Missile `bloom` and `rays`, `hd_missile_explosion_lightrays_glow` | 554, 555 of 702; 690, 691 of 833 | `0x60710101` | **7** | decoded (`missile_rays`) |
| Missile shockwave ring, `hd_missile_explosion_shockwaves_glow` | 556 of 702; 692 of 833 | `0x60010101` | **0** | raw, unchanged |
| LeachBall, `hd_leachbeam_ball_glow` (`RIM_GLOW`) | held beam, 937, 939, 941 of 1048; the Missile head (the same material) 472, 474 of 610 | `0x60710101` | **7** | decoded (`rim_c`) |
| LeachBall ring, `hd_leachbeam_bloomring` | 938, 940, 942 of 1048; 473, 475 of 610 | `0x60710101` | **7** | generic emissive path, decodes already |
| Plasma head, `plasmasphere_subtractive_glow` (`RIM_EDGE`) | in flight, 351, 352 of 455; 636, 637 of 735 | `0x60710101` | **7** | decoded (`rim_c`) |
| Plasma explosion halo, `hd_plasmahalo_glow` | 769 of 882 | `0x60710101` | **7** | generic combine, decodes already |
| Plasma explosion ring, `hd_plasmaring_glow` | 765, 766 of 882 | `0x60010101` | **0** | decoded by the generic combine, **not changed** (below) |
| Rocket smoke ribbon, `hd_rockettrail` | 486, 488, 490 of 623; 560, 562, 564 of 711, both units | `0x60710101` | **7**, both taps | raw, **not changed** (below) |
| Engine light cone, `dc_lightcone` | Amphiseum draws 724, 730 (`rcsmaterial.md`, "HD's light cone") | `0x60710101`, `0x60750102` | **7** | decoded already |

Format and size of the units (`rect` word, width then height): core `000b8829` 32x1024, rays
`00018829` 256x64, Missile shockwave `00018829` 256x256, ball and ring `00098829` 256x128, Plasma
ring `00098829` 256x256, Plasma halo `000a8829` 256x512, smoke `00018829` 1024x128, all DXT45.

What it changes:

- **Decoded now, on HD's encoded store only** (`encoded_out`, as the flame's, the trail's and the
  fireball's are): the Missile blast's core and rays, and the one `rim_c` tap both rim programs
  (LeachBall and Missile head, Plasma head) share. Alphas stay raw. On the same scenes: the Missile
  frame's mean saturation 8.8 to 15.5 %, whole-frame luma 233.5 to 229.9 at 0.2 s; the LeachBall's
  box 1,518 near-white pixels to 1,330; the Plasma bolt adds 569 dark pixels to its box and now
  1,024. Pinned by `hd_missile_blast_ground_truth`, `hd_leach_ball_ground_truth` and
  `hd_plasma_ball_ground_truth`.
- **The Plasma ring reads raw on the console and the generic combine decodes it.** An edit that
  skipped the decode for the ring's slot bit moved no pixel of ours' Plasma explosion frame
  (tick 487; a probe that returned pure red and green for the ring's and the halo's slot bits
  moved one pixel), so the ring and the halo are not drawn through that branch in any frame ours
  makes, and the change was dropped unverified. **Open**: which path draws them.
- **The Rocket's smoke reads 7 on both taps, but ours stays raw** (`beam.wesl`, `decodes_source`
  false): `rocket-trail.md` found a decoded smoke grey against the film's white, and no Rocket
  film is on this host to settle it, so the nibble is recorded and nothing is wired (the absorb
  shell's precedent). Falsifier for a later lane: a Rocket pair with the decode on.
- **The Missile's mean saturation moved up and the 3-frame film does not close on the original's
  in every age** (`hit0` ages 0.08, 0.15, 0.33 s, whole-frame mean saturation original 36.9, 33.4,
  62.2 %, before 46.0, 35.0, 57.6, after 52.1, 56.9, 63.4; luma original 222.8, 224.1, 199.7,
  before 231.4, 248.5, 226.2, after 228.9, 245.2, 221.4): luma is nearer the original at all three
  ages, saturation only at 0.33 s. The frames are framed by a camera moved from the original's
  relative to the blast, so this is a direction, not a score.

**Omega and 2048**: checked, applies, not wired for the three title-neutral findings. Omega ships
the same textures as `.gnf` (`ChannelType::Srgb`) and its programs are GCN and unread; 2048's
`.gxt` header carries no gamma field and no Vita3K draw of these effects has been read. The decode
is therefore left on HD's measured store (`encoded_out`); neither title's frame changed.

## Open

- **Not written:** the Repulser field and mag floor. (The Bomb's blast is
  written since 2026-10-07 and framed with `--force-bomb-trip`; see above.)
- Which archive's `hd_leachbeam_ball_glow` the original serves; the two copies
  differ by the vertex alpha only.
- No capture of the original's LeachBall or Plasma head exists under
  `data/traces/` or `data/scratch/`, so the pictures are judged against the
  programs, not against the game.
