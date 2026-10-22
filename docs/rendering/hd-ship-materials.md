# HD ship hull materials: what each resolved variant computes, and a live bug it exposed

2026-09-13. Scope: `feisar_c1/Ship.vex`'s five drawn materials (the craft the
matched-camera reference frames, `docs/reverse-engineering/rpcs3-capture.md`),
corroborated disc-wide over every ship family. Confidence per the
[rubric](../reverse-engineering/confidence-rubric.md). Produced with
`crates/render/examples/hd_ship_material_dump.rs`.

Prior reading this builds on: `docs/formats/rcsmaterial.md` ("The ship hull's
dark materials: a wrong colour-set match, not a wrong vertex class", 2026-09-13)
fixed every hull material's *resolution* (100/178 -> 178/178). This page is
about what the now-resolved variants actually compute, and a rendering bug the
resolution fix exposed rather than caused.

## The per-material term table (`feisar_c1/Ship.vex`, 5/5 resolve)

All five resolve `Static/HalfBrightAmbientSunSpot0SVC0` (the ordinary lit-race
key, no lightmap, no colour set - confidence 95, direct decoder output).

| Material | chunks | ambient | sun | specular chain | unit 0 | unit 1 | unit 2 |
| --- | ---: | --- | --- | --- | --- | --- | --- |
| `diffuse_with_specular_from_alpha_n_vcol` | 2 | yes | yes | found, literal `0.0`, unpatched by `SpecularPower` -> falls to the shared `32.0` | `feisar_c1_livery.gtf` | `feisar_c1_livery_norm.gtf` (normal map) | declared `0x9edd3243`, no `.gtf` bound (see below) |
| `diffuse_vcol` | 6 | yes | yes | **no chain at all** (`None`) -> falls to `32.0` | `feisar_c1_livery.gtf` | none besides the unused lightmap placeholder | - |
| `carbonfibre` | 1 | yes | yes | found, literal `0.0`, unpatched -> `32.0` | `carbon.gtf` | `carbon_n.gtf` (normal map) | declared `0x9edd3243`, same as above |
| `glass_texture_clamped` | 1 | yes | yes | none | `feisar_c1_glass.gtf` | - | - |
| `detonator_emissive_bloom` | 2 | **no** | **no** | none | `colours_flashing_glow.gtf` (an `ag_systems` texture, shared cross-team) | - | - |

Confidence 90 for `ambient`/`sun`/`specular chain` (mechanical decoder output,
`Declared::takes_constant_ambient`/`takes_directional_light`,
`fragment::Program::specular_exponent`), already read and wired by prior work
(`mesh::rcs::skin::roles`, `crates/rcs/src/rcsmaterial/fragment.rs`). Nothing
in this table is new *wiring* - it is new *reporting* over an existing reader,
confirming the resolved hull materials feed the same ambient/sun/specular-
exponent path every other resolved HD material does. `diffuse_vcol` (6 of 12
chunks - the majority of the hull) genuinely has no specular chain at all,
which the existing `DEFAULT_SPECULAR_EXPONENT` fallback already treats
correctly (it still draws a specular term at the shared exponent, a documented,
tested prior decision - see `crates/mesh/src/mesh/vertex.rs`'s own doc
comment - not revisited here).

`detonator_emissive_bloom`'s texture is worth flagging rather than acting on:
it's shared from `/data/ships/ag_systems/...`, not a Feisar asset, which reads
as a decal/pad-light slot rather than a hull surface proper (2 chunks, likely
small trim geometry). Not chased further.

## Finding 1: a ship's own second texture is added to the hull as a glow, disc-wide

**The mechanism.** `mesh::rcs::skin::picks` chooses the material's "aux"
(second) texture entry per slot. Where the alpha lane traces to nothing
(`Texel::Untraced` - true of every hull material's output alpha above) and the
material has no lightmap, `aux` falls back to raw ordinal 1 of the model's own
sampler table - `Pick::default()`'s `aux: Some(1)`, unconditional. That
fallback is documented as "what this renderer always bound" and is meant as a
conservative no-information default.

`mesh::rcs::emissive::emissive` separately asks each material's resolved
fragment program `Program::accumulates(1)` - a structural test (a `MAD`/`ADD`
whose destination is also one of its own sources, fed by unit 1's sample) -
and where that is true and a second texture decoded, sets `slots::ADD_SECOND`:
the shader adds that texture to the albedo as a tinted, optionally-scrolling
glow (`mesh.wgsl`'s `glow`/`glow_linear` terms).

**Where the two combine wrongly.** A ship hull material's raw ordinal-1 entry
is whatever the artist put second in the table - on `feisar_c1`'s two affected
materials, literally the ship's own **normal map** (`feisar_c1_livery_norm.gtf`,
`carbon_n.gtf`). Its tangent-space unpack and the specular/diffuse math it
feeds (`crates/rcs/src/rcsmaterial/fragment.rs`'s decoded instruction stream,
61 instructions on `diffuse_with_specular_from_alpha_n_vcol`) trips
`accumulates(1)` - the same `MAD dst, dst, x, y`-shaped register reuse a real
scrolling glow has, produced here by ordinary lighting arithmetic rather than
an additive combine. Confidence 85 that `accumulates(1)` is a genuine
structural true positive here (the instruction shapes were read, not just
counted) and not a decoder bug; confidence 95 that the *consequence* is wrong,
verified by the tint check below.

**Verified, not inferred: neither affected material declares `TINT`.**
`emissive()` reads `material.parameters` for `TINT` (`0xe8bcd7f5`),
`OFFSET`/`SCALE` (`0x78256a45`/`0x78787596`). `diffuse_with_specular_from_alpha_n_vcol`
declares `0xab31c2b1`/`0x4232e459` and `carbonfibre` declares
`0xebecee0f`/`0x24212379` - none of the six is the emissive triple. So the
fallback fires: `tint = [1.0, 1.0, 1.0]`, `scale = 1.0`, `offset = 0.0`,
`rate = 0.0`. The normal map's own RGB (unpacked tangent space is typically
`~(0.5, 0.5, 1.0)` - blue-dominant) is added **at full weight**, gated only by
the diffuse texture's alpha (`first.a`, which is `1.0` everywhere on a DXT1
diffuse). Confidence 95 - read directly off the material's own parameter
table, not estimated.

**Disc-wide count, corrected.** A first census (raw ordinal-1 path string,
`_norm`/`_n` name match) over-counted; the real number is off the actual build
pipeline (`mesh::rcs::build`, `Model::material_slots`). Over every ship file
plus Talon's Junction (forward and reversed): **249 `ADD_SECOND` slots total,
176 on ship files.** Splitting those 176 by material family:

| Family | slots | reads as |
| --- | ---: | --- |
| `diffuse_with_specular_from_alpha_n_vcol` | 52 | hull paint, normal map at ordinal 1 |
| `nitro_body_new` | 46 | hull paint, but ordinal 1 is a **different diffuse texture** (`*_tp_nolivery.gtf`), not the normal map - see below |
| `leacheffectmat` | 37 | weapon-trail effect overlay, not a hull surface - plausibly a real glow, not evidenced either way here |
| `carbonfibre` | 20 | hull trim, normal map at ordinal 1 |
| `hexagonalshield_alpha` | 6 | shield effect overlay, same caveat as `leacheffectmat` |
| `diffuse_with_specular_from_alpha_n` | 6 | hull paint, normal map at ordinal 1 |
| `detonator_diffuse_with_specular_from_alpha_n_vcol` | 4 | hull paint, normal map at ordinal 1 |
| `glass_texture_n` | 2 | canopy glass, normal map at ordinal 1 |
| `zonebattle_shield`, `detonator_ship_rich_iridescent`, `detonator_ship_dg_iridescent` | 1 each | mixed - see below |

**132 of 176** are hull-paint families where ordinal 1 is confirmed (by name
and, for two of them, by direct dump) to be a texture that is not a picture to
add - a normal map on six of the nine families. The other 44
(`leacheffectmat`, `hexagonal_shield_alpha`, `zonebattle_shield`) are effect
overlays where an additive layer is plausible by design; not evidenced as
wrong here, and not included in the 132.

**Not one shape.** `nitro_body_new` (46 slots, the nitro-variant hull)
resolves ordinal 1 to `auricom_tp_nolivery.gtf` - a **base, no-livery diffuse
texture**, not a normal map at all (its actual normal map is at ordinal 2:
`auricom_c1_tp_norm.gtf`). So the bug's root cause (`aux` falling back to raw
ordinal 1 regardless of what that position holds) is uniform, but its visible
symptom differs by family: some ships add their own normal map onto their
hull, this one adds a *different skin's diffuse texture*. Either way the added
layer is not what the material's own microcode structurally intends
`accumulates(1)` to answer for - a glow sprite - and both are symptoms of the
same ordinal-1 fallback.

**Two fixes were tried and both failed on measurement - reported rather than
landed.**

1. *Gate on whether `aux` was a positive trace* (a new `Pick::aux_traced`
   field, true only via the lightmap identification or a resolved
   `Texel::Unit` alpha-lane read, false on the ordinal-1 fallback). This is
   the fix that would read as "obviously correct" from the mechanism above.
   **Measured and refuted**: rerunning the same before/after census found it
   deletes real, working glows too - `scroller_glow_v3`/`v4`, `tunnel_fx_noalpha`,
   `bluemetal`, `mt_uvanim_diffuse_emissive2`, `scanlinetext`,
   `etched_glass_tech`, `mageffect08`, `chevron_facing_material`,
   `nr_scalinguvs`, `and_arrowmaterial` - every one of them lost `ADD_SECOND`
   too, because a real glow's *alpha* lane is routinely `Untraced` as well (its
   coverage is usually a constant; the accumulate lives entirely in RGB). So
   "was `aux` traced" is not a proxy for "is this a real glow" - both
   populations get to `aux = Some(1)` by the identical fallback, and the two
   cannot be told apart at the `Pick` level.
2. *Hash-exclude the ship normal-map samplers*, matching this codebase's own
   established idiom (`skin::NOT_A_PICTURE`, a disc-measured hash list already
   used to keep a ramp/lookup from being picked as a picture). Not landed:
   `nitro_body_new`'s 46 slots show the wrong-added-texture isn't always a
   normal map (see above), so a normal-map hash list would fix six of nine
   affected families and silently leave the rest - an incomplete fix presented
   as complete. Confirming the exclusion is also safe for `bluemetal`
   (`blue_metal_facing_ramp.gtf`, already in `NOT_A_PICTURE` for its `picks()`
   role - would this exclusion also cost `bluemetal`'s existing glow, correctly
   or not?) needs a decision this session did not reach.

**What a correct fix needs, for whoever picks this up next:** `emissive()`
should resolve unit 1's *actual* declared-and-bound sampler by hash cross-
reference against `Declared.samplers` (the same lookup `skin::units` already
does for `ALBEDO_FROM_SECOND`/`ALPHA_FROM_SECOND` routing), not accept
whatever `Pick::aux` (ordinal-based) happened to load - and then decide,
per that resolved sampler's role, whether an accumulate is a real glow. That
is more than this session's remaining budget allowed to land safely, given the
first attempt's regression; **no code change is committed for this finding**,
by design, so `just test-data`'s current green stays green and describes the
bug rather than a broken fix.

**Player-eye corroboration.** Our own default-chase render of `feisar_c1` at
the grid (`/tmp/oag-drive/lane-hull-shading/before-default-chase.png`) against
the same-framing RPCS3 reference (`data/reference/hd-capture/talons-ships/00.png`)
shows exactly the colour shift this bug predicts: the original's top fuselage
reads warm khaki-olive with orange wing-panel accents, ours reads
distinctly cooler/blue-grey with a visible blue-purple cast over the
canopy/wing-top surfaces that use `diffuse_with_specular_from_alpha_n_vcol` -
consistent with a raw, blue-dominant tangent-space normal map added at full
weight. Not a controlled A/B (no code changed this session to compare against),
but the direction and location of the colour shift match the mechanism.

## Finding 1, resolved: the fix reads the role of the texture `Pick::aux` actually loads

**Fixed 2026-09-13, in `mesh::rcs::emissive::emissive`.** The prior section's
"what a correct fix needs" prescribed resolving unit 1's declared-and-bound
sampler by hash cross-reference against `Declared.samplers` (the same lookup
`skin::units` performs) and deciding a role from that. **Measured and
refuted before landing**: `tunnel_fx_noalpha` declares its *specular* map
(`SpecularTexture`, `0x20c3e476`) at hardware unit 1 and its own emissive
texture (`EmissiveTexture`, `0xb1f2a176`) at unit 2, while `Pick::aux` (no
lightmap, an untraced alpha lane) resolves to ordinal 1 - the emissive one,
the texture `Model::lightmaps[slot]` already holds and the one this material
genuinely glows with. Asking "what sits at hardware unit 1" independently of
`Pick::aux` would have refused that real, working glow - repeating, by a
different route, the regression the first (`aux_traced`) attempt already hit.

**The fix instead reads the role of `Pick::aux`'s own choice**:
`material.samplers[pick.aux].0` - the same ordinal `skin::skin` already
decoded into `seconds[slot]` - cross-referenced against a disc-measured
table of sampler roles, not against "whatever the declaration says is at
unit 1". Where that role is a normal or specular map, the accumulate is
refused (`Report::emissive_surface_map_excluded`); where the role is a named
glow, or cannot be settled at all, the material is left exactly as it drew
before this fix (`Report::emissive_role_unresolved` for the second case).
Confidence 90 on the mechanism and the fix (read directly in
`crates/mesh/src/mesh/rcs/emissive.rs`, cross-checked against the disc-wide
census below and guarded by `hd_hull_glow_role_ground_truth.rs`).

**The role table itself is narrower than a preimage name suggests, and that
is measured, not assumed.** Of the seven "Surface maps" preimages, only
three (`NormalTexture`, `NormalTexture2`, `Normal`) bind zero counter-examples
disc-wide; `SpecularTexture` binds a glow (`and_power_glow.gtf`) and a
diffuse (`and_station4_diff.gtf`) somewhere on the disc, `SpecMap` binds an
advert live on `scanlinetext` at Talon's Junction, and `Spec` binds an
ambient-occlusion map - all three excluded from the table rather than
trusted on the name alone. Four further, per-material-family hashes with no
preimage at all (`0x436d3929`, `0xc8f18561`, `0x0617f872`, `0xc78c9866`) are
included because each was independently measured to bind only a normal map,
disc-wide, with zero exceptions. Full evidence table:
`crates/mesh/src/mesh/rcs/emissive.rs`'s own doc comment and
`crates/render/examples/hd_emissive_role_census.rs`.

**The nitro_body_new claim above was itself measured off the wrong
texture.** Resolving `nitro_body_new`'s ordinal 1 (`Pick::aux`'s naive
choice, as the previous section's census did) lands on
`auricom_tp_nolivery.gtf`, a base diffuse - which is what made the
hash-exclude attempt look incomplete. But `nitro_body_new`'s **declared**
unit-1 sampler is a different model-table entry (ordinal 2 in the
`auricom_n1` case measured), and that entry is the ship's own normal map on
every one of 13 distinct paths measured, disc-wide, with no exception -
exactly the same shape as the other eight affected families. The two
readings disagree because `Pick::aux`'s ordinal-1 default and the material's
*declared* unit 1 are not always the same entry; `emissive()`'s fix reads
`Pick::aux`'s own chosen entry (see above), and `nitro_body_new`'s
`Pick::aux` happens to choose ordinal 1, `auricom_tp_nolivery.gtf` - so this
family's real second texture stays unresolved and un-refused (see below),
not because it is a genuine exception to "the hull adds its own normal map,"
but because this project has no settled role for a plain diffuse.

**Disc-wide, post-fix**: of **53** ship `ADD_SECOND` hits the real
production pipeline (`mesh::rcs::build_scene`) produces today, **26** refuse
(`diffuse_with_specular_from_alpha_n_vcol`, `diffuse_with_specular_from_alpha_n`,
`detonator_diffuse_with_specular_from_alpha_n_vcol` - all resolve `Pick::aux`
to the ship's own normal map) and **27** are left exactly as they drew
before: `carbonfibre` (`Pick::aux` resolves to `carbon.gtf`, a plain
diffuse, not a normal map - its actual normal map sits at a *different*
ordinal this material's own microcode does not accumulate through),
`nitro_body_new` (as above, `Pick::aux` resolves to a base diffuse texture
even though its declared unit 1 is a normal map) and `glass_texture_n`
(`Pick::aux` resolves to a real normal map, `assegai_glass_n.gtf`/
`piranha_glass_n.gtf`, but its declared hash is the generic `Texture2`
preimage, used elsewhere on the disc for a genuine picture, so this project
does not guess at its role). The same shape recurs on **80** circuit slots
(down from an unfixed 985 total), mostly window/glass materials
(`windowsnormaldiffusespecular*`, `dc_diffusenormalspecular`,
`j_rgb_colourtintnormalreflect`, `pb_window*`, `glass_reflect_seperateopacity_normal`)
sharing the ship hull's own material family or a real normal map at
`NormalTexture`.

**One circuit case found and closed the same way, 2026-09-14: Tech De Ra's
mountains drew cyan/purple.** Its `tech_de_ra_rocks.rcsmaterial` names four
samplers - `rocks_01_sand.gtf`, `rocks_01_normal_alpha.gtf`,
`rocks_01_colour.gtf` and the `lightmap` slot - and of the circuit's 22
slots on that material, slot 343 (58 chunks, the mountain range itself) is
the only one whose `lightmap` entry carries no path. `picks()` therefore
took its non-lightmap branch, `Pick::aux` landed on entry 1 - the normal
map, 512x512, mean RGB (127, 126, 246) - and `emissive()` added it untinted
to the sand albedo: exactly Finding 1's shape, on scenery. Its sampler hash
`0x0cddca48` binds that one path and nothing else across all 643
`.rcsmodel`s on the disc (a brute-force sweep of about a million compound
names against `rcsmaterial::name_hash` lands `Normal_Spec` on it, beside
`Dirt` - already a recovered preimage - and `Rock` for the other two
samplers, three hits for three targets where chance predicts 0.0007; a
reading that agrees with the file, not the evidence the refusal rests on),
so it joins the role table as `emissive.rs`'s `CIRCUIT_SURFACE_MAP_SAMPLERS`.
Verified at the reported pose (`--pose 410.5,-47.2,-291.4`): the range now
draws the same brown rock as the lightmapped slots beside it. Pinned by
`tech_de_ras_mountains_no_longer_glow_their_own_normal_map`.

**Then the same census over every accumulating slot on the disc, by pixels
rather than by name, same day.** Decoding all 254 distinct second textures
the 932 `ADD_SECOND` slots load and classifying each by its channel means
found **29 normal maps** still being added as glows, on **16 (family, hash)
pairs** across five hashes. None of those hashes can be refused alone -
`Texture2` (`0xa2d555b9`) binds a normal map under `diffuse_normal_specular`
and a picture elsewhere, `0x3bdc0403` binds 1,438 paths including adverts -
but keyed on the material family *and* the hash every pair is unambiguous:
136 paths bound disc-wide across the 16 pairs, 119 with the canonical
tangent-space signature and 17 DXT5 two-channel packings reading as flat
`(128, 128, 128)`, all named `*_n`/`*_ne`/`*normal*`, no picture among them.
`emissive.rs`'s `CIRCUIT_SURFACE_MAP_SAMPLERS` is therefore a `(family,
hash)` table with the full row list. What that changes on screen, measured
before/after at fixed ticks: **Zone 1-4's whole floor** (`tracktexture_with_normal`,
whose diffuse is a 1x1 black texel - the Zone look is the unread Zone
shader's, `hd-zone-stage-textures-are-grounded.md`) goes from lavender to
black, an honest absence in place of an accident; Zone's pads
(`weapon_pads`, the `_ne` normal map pads.md already measured) lose the same
lavender - on every other circuit the pads carry a lightmap and were never
accumulating, so pads.md's "none of that reaches the frame" still stands;
Sol 2's `diffuse_normal_specular` wall pieces lose a lilac tint; Moa
Therma's chevron overpass underside (`pb_diffalphaspecnormal`) goes from
pink-tinged to its yellow; the ship cockpit glass (`glass_texture_n`, the
open case above) no longer adds its normal map either. Every other circuit's
fixed-tick frames moved under 2 %. Pinned by
`zone_1s_floor_and_pads_no_longer_glow_their_normal_maps`.

**Guarded, not just measured**: `hd_hull_glow_role_ground_truth.rs` pins
`feisar_c1`'s `diffuse_with_specular_from_alpha_n_vcol` losing `ADD_SECOND`
while its `carbonfibre` keeps it, and that `scroller_glow_v3`,
`tunnel_fx_noalpha` and `mageffect08` - three of the ten families the first
attempt regressed - all still carry `ADD_SECOND` on Talon's Junction.
`scripts/hd-frame-compare.py --pair-dir talons-matched --pose 00` and
`--pair-dir amphiseum-matched --pose 00` report the same per-region numbers
before and after this fix (whole-frame luma 0.481 vs 0.481 on Talon's
Junction, 0.423 vs 0.424 on Amphiseum, the latter a floating-point rounding
difference, not a real one) - the fix reaches no circuit scenery at all,
only ship hulls (and the handful of circuit materials sharing their shader
family).

**Player-eye, at `--race --team feisar_c1 --size 1280x720`, ticks 5 and
300**: the wing-panel accents move from a purple/pink cast to yellow at both
tick counts - the same direction the reference's warm khaki-olive/orange
palette predicts, though not a full match. The top fuselage
(`carbonfibre`) stays grey rather than warm, which is expected and separate:
`carbonfibre`'s own **albedo** may independently be reading the wrong
sampler entry (`picks()`'s colour-lane trace returns `Mixed` for this
material, falling back to the first non-`NOT_A_PICTURE` entry - ordinal 0,
which is `carbon_n.gtf`, the normal map, not `carbon.gtf`) - the same shape
as the `EmissiveTexture` fix in `247114b6`, on a different binding
(`picks.albedo` rather than `picks.aux`). Flagged for whoever picks this up
next; not measured further here and not folded into this fix, which is
`ADD_SECOND` only.

## Finding 2: unit 2's specular-from-alpha term is not achievable as written

The handover brief named "wire the unit-2 specular-from-alpha binding" as the
step-2 minimum. It is not achievable with what the disc supplies, for two
independent reasons, each sufficient on its own:

1. **No `.gtf` to bind.** `diffuse_with_specular_from_alpha_n_vcol`'s declared
   unit-2 sampler hash is `0x9edd3243`. The *model's own* sampler table
   (`rcsmodel::Material::samplers`, the thing `mesh_render` can actually load
   a path from) is `[0xfb17503f, 0x436d3929, 0x37b5db58]` - none of the three
   is `0x9edd3243`. `skin::picks`'s `index_of` requires a model-sampler entry
   whose hash matches *and* whose path is `Some`; there is no such entry, so
   this unit can never resolve to a texture this project can load, regardless
   of any shader work. Confidence 95 - a direct table lookup, not an
   inference.
2. **Its coordinate is computed, not the surface UV.** Block #49
   (`TEX H1.xyz, coord unit2`) reads a coordinate built at
   `R1.z = R0.w + 0.25`, `R1.w = (R2.y + R3.y) * 0.5 + 0.5` (traced by hand
   through the register-write chain, confidence 60 - the individual `MAD`/`ADD`
   steps are read directly off the decoded instruction stream, but the chain
   was not cross-checked against a second material or a runtime trace). `R2`/
   `R3` are the same registers the specular dot-products (`#8`, `#13`, `#15`,
   `#17`, `#19` - saturated `DP3`s) write into earlier in the same program, so
   this reads as a **lighting-scalar-indexed lookup** (the same shape as
   `NOT_A_PICTURE`'s `0x94b2b285`, sampled at `dot(V,N)`), not a picture
   sampled at the diffuse UV the way `mesh.wgsl`'s existing second-texture path
   assumes. Sampling it at `in.texcoord` - the only coordinate this renderer's
   second-texture path currently has - would not reproduce what the file
   computes, on top of there being no texture to sample in the first place.

Both points independently close this as "name the gap," per `CLAUDE.md`'s rule
against inventing a stand-in.

## Finding 3, resolved (2026-10-09, `hd-hull-bright`): `VertexColour1` is a factor on the light

The write-mask question is closed by decoding the blocks instead of the vertex
program (`scripts/ps3-microcode.py fp-file`). `diffuse_vcol`'s lit block
(`@0x32e0`, `HalfBrightAmbientSunSpot0SVC0`) is

```text
@0x04  DP3_SAT H0.w, N, sunDirection
@0x06  MAD H0.xyz, H0.w, sunColour, ambient     ; ambient + sun * N.L
@0x0e  MUL H2.xyz, f[TC0], H0                   ; VertexColour1 * that
@0x11  TEX H1.xyz, f[TC3] unit0                 ; albedo (TC3, not TC0)
@0x12  MAD R1.xyz, H1, H2, -fog
@0x15  MAD /2 H0.xyz, ...  END
```

`f[TC0]` is `VertexColour1` (the vertex block writes it to `o[7]` = `TC0`) and
the albedo samples `f[TC3]`; the `/2` ends all three ship variants read, and
the ship scene already receives the track's `0.5`-scaled light, so the `/2` is
matched and was never the cause. The same factor shape holds in
`diffuse_with_specular_from_alpha_n_vcol` (`MOV H2.xyz, f[TC0]`, then
`H1 = H2 * light`, specular `* H2`), in the `SVC1` twin (`@0x7c90`:
`H2 = f[TC0] * (f[TC1] + ambient + sun * N.L)`, the SPU light *inside* the
product) and in the `ShadowMap` variant (`@0x36e0`, sun gated by the two map
taps, then `* f[TC1]`). The `IBL` variants (`@0x3950`) swap the constant
ambient for a unit-1 sphere-map lookup; the n_vcol program also adds a unit-2
environment lookup (sampler `0x9edd3243`, no `.gtf` bound, still unread).

**What drew the housing 3x bright.** `mesh.wesl` added `VertexColour1` to the
light sum as a baked light (`colorSet1`'s shape), gated the sun by its fourth
byte, and drew the shared-32 specular on `diffuse_vcol`, whose program has no
`LG2` at all. Now `mesh::rcs::skin::light_inputs` classifies a material from
its programs (the vertex block routes `VertexColour1` into `o[TCn]`; the
fragment block only `MOV`s or `MUL`s `f[TCn]`; `Program::input_is_only_a_factor`),
marks its chunks' `sun_mask` as `-1 - mask`, and a hull (`receives_shadow` not
the track's `2`) lights `colour * (ambient + sun * N.L * occlusion + spu light)`
with no prelit or added vertex term, specular `* colour`, and none at all for a
material with no `LG2` (`NO_SPECULAR`). **Tracks keep the additive reading**:
the rule also matches materials on `01_vineta_k`, `04_chenghou_project` and
`amphiseum`, whose frames move only through the craft's exposure effect (<= 2
levels outside the craft on Vineta K) but are unmeasured against their own
references, so they are left as they were. Open: whether the track programs
multiply too.

**Numbers** (Metropia grid, `feisar_c1` concept1, box (880,670,1000,750) in the
`metropia-grid-wcb-on-s150` frame; ours at the same hull panel, box
(880,452,1000,532) with `--pose 530.17,-13.17,169.75`, 1882x1058, bloom on):

| | luma |
| --- | ---: |
| original, WCB on (boot 1 / boot 2) | 0.087 / 0.091 |
| ours before, `--shadows off` (comparison setting) | 0.297 |
| ours before, `--shadows original` (HD's default) | 0.196 |
| ours after, `--shadows off` | 0.205 |
| ours after, `--shadows original` | 0.110 |

Under `original` the sun-occlusion map zeroes the sun at this indoor grid, so
the whole 0.110 is ambient times the vertex factor plus specular and glow. The
`off` tier has no occlusion, so its sun term stays and it reads 0.205: not a
match, and not a bug of this fix. Whole frame at the same pose, `off`: 0.2757
before, 0.2742 after. **Open**: the 0.02 residual under `original`, the orange
rim (likely the unbound unit-2 environment lookup and/or the `IBL` variant,
unmeasured which program the original binds), and the rest-pose pylons.

**Other titles.** Omega and 2048: checked, not wired - their materials are
PS4/Vita shader blocks this microcode reader does not decode, so the
classification has nothing to read; the same `VertexColour1` factor shape is a
question for their own shader reads. Pulse, Pure: untouched (race screenshots
byte-identical, see the thread).

## Finding 4 (2026-10-20, `hd-hull-reflection`): the original draws the hull with an IBL, shadowed, SVC1 variant, and the IBL is a live environment map

> **Superseded in part by [Finding 5](#finding-5-2026-10-20-hd-craft-ibl-probe-the-environment-maps-are-baked-probes-blended-at-the-craft-and-now-wired).**
> The variant, the law and the maps' sizes below hold. The claims that the maps are **built at runtime**, that
> **`.probes` is not them (75)**, that the producer is the cube-to-paraboloid program, and that "nothing here was built"
> are refuted: the maps are a blend of `track.probes`' baked images, and the hull now draws them.

**Why the original's hull is pale and ours is dark and saturated.** Read live on RPCS3 (Harimau classic on Talon's
Junction, `hd-timetrial-talons-harimau-classic-start`, Write Color Buffers on, one frame at rest, one after 10 s of
thrust, and the Assegai classic start as the second craft). Hull-wing regions of the same framing, original against ours
(`data/reference/hd-flare-material/rest-sheet.png`, top row against the middle row, boxes picked by eye): mean luma
0.60-0.74 against 0.24-0.29, mean saturation 0.21-0.31 against 0.67-0.79. Single frame, not pixel matched: the size of
the gap, not a law.

| | permutation word | name | lit by |
| --- | --- | --- | --- |
| ours, every hull material | `0x001` | `HalfBrightAmbientSunSpot0SVC0` | `constantAmbientColour` + sun |
| the original's live hull draw | **`0x847`** | **`HalfBrightIBLSunShadowMapSpot0SVC1`** | `paraboloidIblTex` + sun x shadow + `f[TC2]` light |

**How the variant was named (confidence 90).** The hull draws are draws 487-504 of the captured frame. Their first
texture is `0x0a76ee80`, and five 64-byte samples (at 0, 0x400, 0x1000, 0x4000, 0x10000) equal
`harimau_livery.gtf`'s payload byte for byte, so these are Harimau's hull chunks. The live fragment program of the
`diffuse_vcol` chunks (37 instructions) equals, after the constants are dropped, fragment block 17 at `0x87c0` of
`diffuse_vcol.rcsmaterial`, and `hd_material_variants` (new, `crates/render/examples/`) names that block: variant 17
(and 43 for `RigidBody`), word `0x847`. The `diffuse_with_specular_from_alpha_n` chunks (draws 487-488 and 493-495)
share the first 12 instructions with its block 17 at `0xdae0` and are the same family (the live dump is cut at 70
instructions, so the whole stream was not compared). Re-run: `python3 scripts/rsx-fp-match-material.py <dir> <stem> <material.rcsmaterial>... -- <fp>...`
after a `rpcs3_draw_hook.py` capture and `just psarc cat` of the three ship materials.

**What the `diffuse_vcol` variant computes (block 17 of `diffuse_vcol`, read off the live microcode, confidence 85).**

```text
ibl   = tex(unit 3 here, (u, v)) * 4        u = 0.25 + (n.x > 0 ? 0.5 + n.z / 4 : -n.z / 4),  v = 0.5 - 0.5 n.y
light = ibl + sunColour * sat(N.L) * shadow(unit 1, unit 2, both TXP of f[TC0]) + f[TC2]
out   = fog(albedo * f[TC1] * light) / 2
```

Unit numbers depend on the variant (the `diffuse_with_specular_from_alpha_n` one reads the livery at unit 0, the
normal map at 1, two shadow maps at 2 and 3, the IBL at 4 and a 128x64 reflection at 5, and scales both by 4). `f[TC1]`
is the `VertexColour1` factor of Finding 3. So the ambient the hull gets is **a texture lookup by the normal into an
environment map**, `x4`, where ours adds one constant. The mean texel of the 64x32 map at the rest frame is
`(0.21, 0.31, 0.36)`, so the ambient is about `(0.85, 1.23, 1.43)` before the final `/2`, brightest for normals that face the
sky. That is the "pale, washed" look, and it is what we lack. The `hd-hull-wash-2` engine-light wash on boost (confidence 85) is a different
term and is unchanged.

**What the two environment maps are (confidence 80 for what, 70 for the cadence).**

- Unit 4 is a 64x32 `A8R8G8B8` linear texture and unit 5 a 128x64 one, `CLAMP`, in **main memory** (the texture format
  word's low two bits are 2, not VRAM: reading them through `0xC0000000` returns another buffer's bytes, reading
  `0x40000000 + offset` returns the image). Harimau's rest frame binds `0x03493000` and `0x0348b000`; Assegai's binds
  `0x03498100` and `0x03490100`: a pool of slots, not one address.
- Decoded (`data/reference/hd-hull-reflection/harimau-rest-*.png`): a **dual paraboloid** of the surroundings. Two
  discs side by side, sky with clouds and the green slopes beside the track above the middle row, the track and its
  walls below, the 64x32 one a blurred version of the 128x64 one. The row split at `v = 0.5` is the same coordinate law
  `rcsmaterial.md` read for the Vineta K sea (`v = 0.5 - 0.5 d.y`).
- **They are runtime data.** No draw in the frame renders into either address (the render-target list holds none of
  them, the command stream names them only as texture sources), and their content changes while the game runs:
  against the rest read, 26 % of the bytes match after 10 s of thrust; five reads at 0.4 s, 0.8 s, 3.8 s and 6.8 s
  match 83 %, 87 %, 50 % and 51 % of the previous read. So the original rebuilds its environment map from the scene
  as the craft moves. HD's executable carries `liveRsxPanXForm_CubemapToDualParaboloide_fp` and
  `liveRsxPanXForm_CubemapConvolve_fp` (`renderer.md`), which is the likely producer: a cubemap rendered at the craft,
  folded into the 128x64 map and convolved into the 64x32 one. The producer's own draws were not in the frames
  caught (confidence 40 that they run on a slower cadence than the hull draw).
- **Many draws read them, not only the hull.** The Assegai frame has 60 draws with unit 4 at `0x03498100`: the hull,
  other craft and props. It is an engine resource, not a ship material.
- **~~`.probes` is not it (confidence 75).~~ Refuted 2026-10-20: it is exactly them, see Finding 5.** `Pvs_LoadForTrack` (`0x003f5550`) calls `0x003c4d78`, which reads
  `<track>.probes` as `u32` count (`0x35e` = 862 for Talon's Junction), `u32 16`, then 862 records of 32 bytes (two
  `vec3` boxes, 12 units apart along the track) and builds a cell tree from them with `Pvs_BuildCellTree`; the ~6.6 MB
  that follow are about 7.7 KB per record at 7.6 bits of entropy per byte, with no zlib stream and no image in them.
  That is a visibility structure. The same function does allocate 8 pairs of 128x64 and 64x32 textures (format word
  `0x500aae4`, the very format unit 4 and 5 carry), so it also **creates the probe pool**; it does not fill it from the file.
  Not renamed: the pool allocation and the PVS tree sit in one function and the evidence for a name is below 50 for the split.

**What would make ours match, and why it is not in this change.** The variant switch and the probe have to land
together: resolving `0x847` without a probe samples nothing (flat black ambient on the whole hull), and a probe pasted from the
RPCS3 dump would be a stand-in for data this project did not decode. A faithful port is a renderer feature: build a
cube (or two paraboloid hemispheres) of our scene at the craft each frame or every few, fold it into a 128x64 dual
paraboloid, convolve a 64x32 copy, bind both at the units the resolved variant declares (`paraboloidReflectionTex`
`0x9edd3243`, `paraboloidIblTex` `0x872ed3bd`), pick the `IBL` word (`0x007`/`0x047`, plus `0x800` for the craft light) in
`skin::variants`, and scale by 4 as the program does. Nothing here was built.
The sub-terms that stay ours until then are labelled as before: the constant ambient and the shared specular
exponent. The variants for `Spot1`/`Spot2` and `SVC1` are read, not exercised.

**Nozzle interior.** The cyan-pink "jewel" in the rest nozzle is not a flame and not a lit material. Draws 521 and 522
(258 and 24 indices, blended, 32x32 `DXT1` at unit 0) are Harimau's two `emissive_bloom` chunks: the program is
`fog(tex(unit 0, f[TC4])) / 2` and nothing else (14 instructions, no light), and the texture is
`colours_flashing_glow.gtf`, a strip atlas of vertical gradients (cyan, white, red, olive, blue, black columns on a
purple band). The pink and cyan pixels are texels of those columns. Ours draws the same material as an unlit texture
and shows a black rectangle, so what differs is which texel the chunk's own `TC4` picks: that coordinate comes out of the
material's vertex program, which was not read (the `.rcsmaterial`'s first vertex block is position-only and the
programs of the rest were not matched to the live vertex programs). Confidence 70 that this material is the jewel, no
score for why ours is black. Left open.

**Other titles.**

- 2048: checked, differs. It ships both an authored `skyParaboloid.gxt` per circuit (14 of them, 699,072
  bytes each, `2048-sky.md`, `rcsmaterial.md`) **and** 28 `.probes`, in a different (little-endian, 1,112-byte
  record) layout than HD's: [hd-probes.md](../formats/hd-probes.md), "Cross-title". Whether 2048's hull materials sample
  either is unread.
- Omega: checked, applies, not wired. It carries HD's engine, its fragment programs are a different, unread
  container, and the PS4 has no emulator here, so whether its hull draws an IBL variant is not checkable; its two
  2048-lineage circuits ship `.shprobes` instead.
- Pulse, Pure: untouched, they do not use `.rcsmaterial`.

## Finding 5 (2026-10-20, `hd-craft-ibl-probe`): the environment maps are baked probes blended at the craft, and now wired

**What a player sees now.** An HD hull is pale and metallic where it was dark and saturated: at rest and at speed,
on Harimau and Assegai. The silver of the yellow panels, the pastel orange and the pale blue wing tips are the
reflection of the circuit's sky in the livery's gloss mask, and the ambient is the circuit's own light at the craft.

**The producer is disc data** ([hd-probes.md](../formats/hd-probes.md), [env-probes.md](../ghidra/functions/ps3-hdfury-eu/env-probes.md)).
`track.probes` holds, per probe, a `64x32` irradiance image and a `128x64` reflection image in a 12-byte-per-4x4-block
format (two `BGR888` endpoints and sixteen 3-bit indices), and the original blends the probes around a craft by its
position on an SPU. Read live on RPCS3: `0.652 * probe[852] + 0.340 * probe[854]` equals both maps of Harimau at rest to
0.4-0.65 of 255, and 52 pauses of two drives round Talon's Junction fit to 0.4-0.6 where the weights are fitted and
within 10 of 255 for 50 of 52 with the law ([hd-probes.md](../formats/hd-probes.md)). Confidence 85 the file and the blend,
70 the weights' law, 55 that the `liveRsxPanXForm_*` programs are the offline bake.

**What was wired** (`oag_vex::probes` reader, `oag_raceplay::env_probes`, `shade.wesl`):

- the circuit's probes load with the race (`Loaded::probes`, 862 on Talon's Junction, forward and reversed), and each craft
  blends its own at its position **one tick back** (measured, [hd-probes.md](../formats/hd-probes.md)) into two layers of
  a 128x64 array bound beside the shadow maps: the irradiance map in the corner, the reflection map whole;
- the hull's ambient is `4 * irradiance[n]` where the draw names a craft, the constant ambient elsewhere. The `4` is the
  engine's probe tint `0x7480de6d`, `(4, 4, 4, 0)` on Talon's Junction and on Metropia;
- a hull material whose `IBL` variant declares `paraboloidReflectionTex` is tagged in the glow table
  (`mesh::rcs::probe_lit`, `rate` 8) with its own reflectivity (`0xab31c2b1`, `0.5` on Harimau), and adds
  `4 * reflection[reflect(-V, n)] * (albedo.a * reflectivity)` after the albedo multiply and before the fog;
- **opponents**: the original binds a pair of maps per craft (the Metropia grid frame holds eight pairs, 0x8000 apart, and
  every draw of a craft its own), so each craft here blends at its own position.

**The decode domain was chosen by the pixels - superseded by ["Hull brightness"](#hull-brightness-2026-10-20-hd-hull-brightness-the-brighter-and-whiter-gap-was-a-capture-artefact-and-the-probe-maps-are-srgb): the reference was WCB-off, the maps are sRGB.** The unit's sampler word carries the gamma nibble `7`, but the project
settled that the RSX multiplies raw bytes ([renderer.md](../ghidra/functions/ps3-hdfury-eu/renderer.md)); the probe enters as a
light, raw, and the reflection is **added raw**. Measured on the rest frame in four boxes of the hull (luma and
saturation, 0-1; the original 0.56-0.91 and 0.08-0.27, boxes picked by eye, one frame, not pixel matched):

| | luma | saturation |
| --- | --- | --- |
| original | 0.56-0.91 | 0.08-0.27 |
| before (constant ambient) | 0.18-0.30 | 0.53-0.77 |
| after, reflection decoded (sRGB) | 0.40-0.49 | 0.23-0.42 |
| after, both maps hardware-sRGB | 0.31-0.41 | 0.21-0.29 |
| **after, raw (shipped)** | **0.56-0.65** | **0.11-0.29** |

Assegai (original 0.39-0.62 luma, 0.29-0.36 saturation over four boxes): before 0.11-0.22 and 0.32-0.49, after 0.29-0.46
and 0.22-0.45: most of the gap closed, the wings and pods still a little low (their pose differs from the capture's).
Chosen by pixel match, no score: the raw domain of both maps (the sampler word says otherwise). Measured, as counts: the
one-tick lag, best for 41 of 52 pauses (0 ticks for 4, 2 for 7).

**Thrust, matched against the previous lane's frames** (`data/reference/hd-flare-rpcs3-check/`, the nozzle crop of
`sheet2.py`; luma / saturation / fraction of pixels at 255): Harimau original 0.71-0.87 / 0.17-0.34 / 0.48-0.67; before
0.43-0.46 / 0.57-0.58 / 0.03-0.08; after 0.60-0.61 / 0.39 / 0.08-0.12. Assegai (no "before" frame was taken): original
0.65-0.72 / 0.23-0.24 / 0.36-0.46, after 0.48-0.49 / 0.38-0.39 / 0.04-0.07. The hull moves toward the original and
nowhere near clips more than it does; the original's remaining lead at thrust is its boost wash and bloom, which are
measured elsewhere and **not tuned here**.

**Cost.** `just bench-render hd` (release, one fixed race, 1280x720, RX 7800 XT, one binary run with and without the
probes; load 12-16, so the numbers are relative; `data/bench/ibl-before`, `ibl-after`, `ibl-after2`, `ibl-after3`):

| | cpu p50 / p99 | frame p50 / p99 | gpu p50 |
| --- | --- | --- | --- |
| before (no probes) | 1,604 / 2,382 us | 1,906 / 2,751 us | 669 us |
| first cut: decode, blend and resize per craft per frame | 3,594 / 4,718 us | 3,895 / 5,085 us | 678 us |
| decoded probes cached, no resize | 2,413 / 3,663 us | 2,721 / 4,043 us | 678 us |
| **shipped: vectorised blend, a craft that has not moved uploads nothing** | **2,024 / 3,033 us** | **2,323 / 3,397 us** | **682 us** |

**+0.42 ms of CPU a frame at the median, +22 % of the frame, nothing on the GPU** (one more array fetch). On the Steam
Deck the CPU is two to three times slower per thread, so about 1 ms a frame: not measured there (no Deck run was offered),
and affordable against a 16.6 ms frame, so the original's cadence (every tick, the position one tick old) is kept and no
render-profile setting was added. If a Deck run says otherwise, the lever is blending an opponent every second tick.

**Cross-title.** 2048: checked, differs (a different `.probes`, named as such in its load report, and an authored sky).
Omega: checked, applies, not wired (not checkable); it ships no `.probes`, so nothing was ported by accident.
Pulse, Pure: no `.rcsmaterial`.

**How brief step 3 was done.** The brief asked to resolve the `IBL` permutation words in `skin::variants`. They are not:
`variants` still resolves the constant-ambient block of every material, which also feeds `colour_factor`, the specular
exponent and the `StaticQuake` tag, and re-keying it moves all of those for every title at once. Instead the shader keys on
the draw's craft layer (a hull that names one is lit from its probe) and `probe_lit` resolves the `IBL` block
(`0x007`) of each material on its own, only to read whether it declares the reflection sampler. The `IBL` block has no
`constantAmbientColour`, so a `NO_AMBIENT` gate keyed on the resolved variant would have switched the probe off; keying on
the layer avoids that. The `Spot`/`SVC1`/`ShadowMap` words (`0x047`, `0x807`, `0x847`) are carried by the existing SPU light and shadow paths.

**The nozzle jewel, resolved (2026-10-20, `hd-hull-nozzle`): it is the engine flare's shapes, not the `emissive_bloom` chunks.**
Both earlier readings above are wrong. (1) Ours does draw the two `emissive_bloom` chunks (Harimau `ship.vex` chunks 6 and 7,
`LightsShape` 444 indices and `LightsShape1` 24; `oag-view --draws` lists both). (2) The original draws the same two chunks
(258 and 24 indices, texture `colours_flashing_glow`): their vertex buffers are the file's own (240 and 16 vertices, 14-byte
stride, the buffers sit `0x11b0` apart in the capture and in the file), but the index buffer is rebuilt every frame and submits
only 86 of the 148 triangles of `LightsShape`: the cockpit and wing lights that are on. Projected with the draw's own
`viewProj` and model matrices, none of the 16 surviving pieces lands on the jewel; they are the two rings at x +-1.5, the
small blocks at z -3.09 and -3.37 beside the vents, and four cockpit dots. The jewel's pixels are covered by the five draws
right after them: the flame shapes (`flame_test` program, unit 0 `flame_01`, additive), whose projected box is 45 x 45 px at
the jewel. At 0 km/h their model matrices are body 0.75 x (1, 1, 0.25) and spikes 0.75 x 2.0 x a flicker in 0.65-0.85
(1.361, 1.606, 1.380 / 2.0), exactly `Flame::new()`'s rest scales. Confidence 90 (draw lists, projection, matrices; two boots).
**The flame is drawn from the first frame of a race, countdown included**: a Restart Race walked at 0.1 s steps shows the jewel
from the first visible frame through the countdown and after GO, flickering between cyan and pink (Harimau classic and
Assegai classic, 0 km/h). The earlier "no `flame_test` draw at 0 km/h" (`engine-trail.md`) was a miss, and
`Race::hd_flame_resting`, built on it, hid the flame at a standstill. Removed: ours now draws the jewel at rest
(`data/reference/hd-hull-nozzle/{harimau,assegai}-rest-orig-before-after.png`, original / before / after). Residual: ours
reads a little larger and sits lower than the original's, which stays inside the nozzle opening; the poses are matched to a
few pixels only (the original start sits 1.8 m higher than ours), so size is not measured. Whether the `LightsShape` index
rebuild is a flash schedule (it is why the hull lights blink in the original) is unread; ours draws all 148 triangles.
Cross-title: Omega ships HD's hulls on its `.rcsmodel`s; checked, applies (flare shapes are the same `ef_*` nodes in
`engineflare.vex`), not measured (no PS4 capture); 2048 inherits the same wiring, not compared.

**Open.** The `LightsShape` per-frame light schedule (above). The weights at two tight corners (a section pair two apart).
The tail arrays of the file (24 + 4 + 3 bytes a probe) are read by the loader and unread here. The word resolution is
the one the previous paragraph names.
Whether 2048's hull programs sample its `.probes` or its authored sky paraboloid.

## What has not moved

`mesh::rcs::skin::roles`'s existing per-material reads
(`NO_AMBIENT`/`NO_SUN`/`specular_exponent`/`ALBEDO_FROM_SECOND`/
`ALPHA_FROM_SECOND`/`SECOND_IS_LIGHTMAP`) are confirmed correct and unchanged
for this hull by the table above - the "no per-material lighting branch"
framing this thread's title carries is narrower than it reads: most of the
per-material branching already exists and is already correctly wired for
`feisar_c1`. The ADD_SECOND misclassification (Finding 1) is resolved as of
2026-09-13 - see "Finding 1, resolved" above. What remains open is the
TC0/TC1 question (Finding 3), the three ship-hull families with no settled
`ADD_SECOND` role, and `carbonfibre`'s own possibly-wrong albedo pick, not a
missing branch structure.

## The Zone-mode hull, matched against RPCS3 (2026-10-20, `hd-zone-hulls`, `hd-zone-material`)

`/data/ships/zone/ship.vex` takes its geometry from the `.rcsmodel` beside it like every team hull: 29 of 29
nodes, 32,065 triangles, 9 of 9 drawn materials resolved to a shipped variant (33 of 33 chunks), 42,919 authored
vertex normals, albedo `zone_tp_1024.gtf`. The earlier reading "untextured, computed normals" was stale: the
normal decode landed 2026-08-17. Geometry and paint agree with the original.

**Zone mode flies `Data\Environments\Zone_1`, not the circuit picked** (the original's load log, two captures); the
first sheet of this section was framed on Vineta K and is not the comparison. Two draw dumps of Zone mode on RPCS3
(zones 3 and 4, both the `Sub Flash` stage; `rpcs3_draw_hook.py`, `rsx-draw-list.py`, `rsx-fp-match-material.py`,
`ps3-fp-live.py`) read the hull's draws:

- **Every hull draw is an `IBL` permutation**, `diffuse_with_specular_from_alpha_n` block 17 (`0x847`,
  `HalfBrightIBLSunShadowMapSpot0SVC1`) and `dg_zonelights1` block 17, with the probe maps bound (units 4 and 5 of the
  64x32 and 128x64 images). So the gloss is not a variant choice.
- **The probe tint is a draw value.** `iblScalePower` (`0x7480de6d`) is `(0.69804, 0.69804, 0.69804, 1.0)` in the
  live program of both frames, where Talon's Junction and Metropia read `(4, 4, 4, 0)`. `.w` lerps the tinted probe
  towards its own luma (`@0x28`-`@0x31` of the `_n` block), so a Zone hull's probe light and reflection are grey and
  a fifth as strong. Ours multiplied by a constant 4, which is the chrome and the iridescence. **Wired**:
  `oag_raceplay::env_probes::ProbeTint` (`ENGINE` and `ZONE`), carried in the model uniform's two spare floats
  (`env_gain`, `env_desat`), `shade.wesl`'s `probe_tint`. Confidence 85 for the two values at that stage; **what moves
  the value inside a Zone race is not read** (both frames were at `Sub Flash`; the stage table's per-stage
  `Lighting.Sky reflection colour` is `0 0 0` at stages 0, 7, 13 and 14 and magenta at 6, the obvious untested
  driver), so the one value is carried for the whole mode. Source: `Scene_PrepareFrame` copies a vec4 at
  `object+0x510` to `block+0x7c40` (`0x003ab100`), which is the shader parameter; `(4, 4, 4, 0)` is written there at
  `0x003abb10`-`0x003abb28` on the path where the byte at `r17` is 0, and the other path (`0x003ad748`) is unread.
- **The hull's gloss gap that remains** is brightness: the original's hull reads white, ours light grey. The live
  `directionalLight0Colour` is `(1.144, 1.091, 1.906)` in both frames and its direction `(0.577, 0.577, 0.577)`
  (Zone_1's `Sun direction`); where the colour comes from, and what ours feeds, is not compared. The original frame
  is also visibly bloomed and tone-mapped brighter overall.
- **`dg_zonelights1`** (five chunks, one material) is the hull's side light cells. Block 17, from the microcode:

  ```text
  uv2   = TC6.xy                           vertex attribute 0x1aefe524 .zw (Uv2)
  mask  = tex(unit 4, dg_flasha, (uv2.x + 0.4 * time, uv2.y - 0.09 * K))
  blue  = tex(unit 5, dg_flash3, (uv2.x + 0.09 * K, uv2.y))
  flash = mask * (blue - C) + C
  P     = tex(unit 0, cf_damage_lights, uv) + flash
  out   = P * (light + 1)                  the picture is added unlit as well
  ```

  `time` is `0x906b67ba` (186.4 and 712.7 in the two frames), `C` is `0xef18f362` and `K` is `0xffede334`. The file
  authors `C` and `K` as 0; live `C` is `(0, 0, 0)` and `K` is 6.0 at zone 3 and 7.0 at zone 4. `dg_flash3` is blue
  with one red corner and `dg_flasha` a white mask with notches in its top rows, so the cells are lit blue on an
  undamaged craft and the mask scrolls. **Wired** (`mesh::rcs::damage_lights`, role `MAG_WAVE` with glow rate 3, the
  second UV set, `shade.wesl`'s `is_cell`); `K` is the **chosen** constant 6 (what it follows is not read: the zone
  number plus three fits two points and is not claimed). `hd_zone_light_cells_ground_truth` pins the binding and that
  `texcoord2` is read and varies, which a still cannot show (blue everywhere also comes from a zero coordinate).
  `texcoord2` is now also filled for the magstrip chunks (`ice::second_uv`), unread there.
- **`zone_tp_1024_n.gtf` is sampled by the original**: `@0x05` of the `_n` block reads unit 1 as a tangent-space
  normal (`2x - 1`) through the vertex tangent frame. **Wired since `hd-hull-normals`** for this hull and every other
  HD ship hull, see "Hull normal maps" below.

Judged against `data/reference/hd-zone-material/`: original frames at zones 3, 4 and 5 (top) against ours before and
after (bottom), Zone_1, `--zone-stage 3`, the chase camera of each; the camera could not be picked from the capture, so
the pose is not pixel matched. After: matte, form-shaded, blue cells in the original's places; the original's hull is
brighter and its panel normal detail finer.

Cross-title: Omega ships the same hull as `hdships/Zone` and it **includes `dg_zonelights1`** and `zoneship01` (6
materials; HD's nine add `diffuse_with_specular_from_alpha_n`, `diffuse` and a second `basicalpha`) with `.gnf`
textures: **checked, applies, not wired** (Omega's hulls build through the PS4 path, which has none of this; it ships
no `.probes`, so the tint is moot there). 2048's is **not checkable** from its `.pkg` here (PFS-encrypted).

## Hull normal maps (2026-10-20, `hd-hull-normals`)

The lit hull programs (`diffuse_with_specular_from_alpha_n`, `_vcol`, `carbonfibre`, `nitro_body_new` and the Fury
`detonator_*` kin) read **texture unit 1 as a tangent-space normal** and light with it. Until this lane ours bound the
map and ignored it, on every HD ship.

**What the program does** (`diffuse_with_specular_from_alpha_n.rcsmaterial`, `scripts/ps3-microcode.py fp-file|vp-file`;
read statically, **not from a live dump of the draw**). The fragment combine reads three interpolators the same way in
blocks 0 and 17 (`x * A + y * B + z * C`, `A`, `B`, `C` the tangent, the binormal and the normal); every vertex block
that writes a binormal (5, 6, 9 and 10 of the same file) computes `cross(N0, T) * w`, so that is `B`. Which vertex block
pairs with which fragment block, and so which `TC` is which, was not traced (the numbering below is block 5's):

```text
t   = tex(unit 1, uv)                         plain RGB, not DXT5nm: MAD x*2-1 on x, y and z
N   = (2 t.x - 1) T + (2 t.y - 1) B + (2 t.z - 1) N0       normalised (DP3 + DIVSQ)
vertex program:  T = 2 v[tangent].xyz - 1                  attribute 0xdbe5f417, four ubyte-norm
                 B = cross(N0, T) * v[tangent].w           the fourth byte, raw 0..1
                 TC4 / TC2 / TC6 = N0 / B / T              (numbering differs per block)
```

`N` then feeds every term the program lights with: the sun's `N.L`, the specular's `N.H` and both probe lookups
(`-N.y * 0.5 + 0.5` of the irradiance map, `reflect(-V, N)` of the reflection map). The frame is **object space** in
the original (no matrix on `T` in these blocks); ours turns `T` by the node and model matrices like the normal.

**The tangent is authored on every normal-reading hull chunk** (`hd_hull_normal_census` over the 32
`/data/ships/*/ship.vex` hulls: 240 chunks of materials that read unit 1 as a normal, 117 of them
`diffuse_with_specular_from_alpha_n_vcol`, 25 `nitro_body_new`, 13 `diffuse_with_specular_from_alpha_n`): the `tangent`
attribute `0xdbe5f417` is four normalised bytes in all 240, perpendicular
to the authored normal (mean `|T.N0|` 0.004), unit length on average (0.99 to 1.04 per material; a few vertices are
short, and a few padding vertices decode to `(-1,-1,-1)`, the shader's off value). The fourth byte `w` is **`0` or
`255`** (about 45 % `255` on the paint materials), so `B` is the zero vector on the `w = 0` vertices and only the
`x`/`z` of the map reach `N` there, the same finding `pads.md` made for the pads. No hull declares the second
attribute `0x868f8229` the live dump named (0 chunks); it is not a tangent stream of the hull.

**Wired** (`mesh::rcs::hull_normal`, `GpuVertex::tangent`, `shade.wesl`'s `hull_normal`): the map binds in the third
slot (`Model::pad_masks`, the pad mask's), the material is marked `Model::material_tangent`, and its chunks carry
their authored tangent as four raw bytes (`Unorm8x4`, 4 bytes a vertex, zero = none, the shader's off switch). A
node transform that is not the identity rotates the tangent like the position and the normal and packs it again.

- **Binding and gamma.** The map is the sampler the program declares on unit 1, found by hash in the model's sampler
  table (`nitro_body_new`'s model lists its diffuse second and the `_n` third; the hash picks `assegai_c1_livery_n.gtf`,
  checked on `assegai_n1`). It uploads raw (`Rgba8Unorm`, `Bc*Unorm`; ADR-0020), which is what `x * 2 - 1` needs; the
  original sets texture gamma `0` on normal maps (`rcsmaterial.md`).
- **Scope: opaque `data/materials/ships/*` materials whose resolved program reads unit 1 as a normal.** Left as they
  drew, unmeasured: the blended ones (`glass_texture_n`, `nitro_perspex_new`), `detonator_shield` (normal on unit 2),
  `detonator_ship_rich_iridescent` (unit 0), and every circuit material that reads a normal map
  (`tracktexture_with_normal`, `diffuse_normal_specular`: not ships, other programs).
- **Chosen, not measured: `B = cross(N0, T) * w` is built per pixel** from the interpolated `N0` and `T`, where the
  original builds it per vertex and interpolates it. Equal at every vertex; a saved varying (the vertex stage has the
  sixteen locations wgpu guarantees).
- Not changed: the facing ramp and the fog read the vertex normal, as the programs do.

Judged against `data/reference/hd-hull-normals/`: `harimau-classic/hull-sheet-harimau-classic.png` and
`assegai-c1/hull-sheet-assegai-c1.png` (original from RPCS3 on Talon's Junction, ours before and after, the original's
frame at rest at the start line against our `--camera-view far` at tick 120, so **hull-sized, not pixel matched**; the
camera could not be placed on the original's ship), and `zone/` (ours before and after at Zone stage 3 against
`hd-zone-material/original-zone3.png`, a different spot of the tunnel). Read as a player would: the change is real but
small at this size. Before against after differs across the whole hull (wings, nose, the probe's reflection tint; a
difference image scaled x6 shows it, the raw pictures a few levels), none of it a change of look; the
lighting level and the probe colour, not the normal, are what still separate the frames (the original is brighter and
whiter - **retracted: that reference was a `Write Color Buffers`-off capture, see "Hull brightness" below**, the bloom halo ours draws around the paint is not there). **Not claimed a match**; the claim is the data path:
authored tangent, authored map, the program's own formula.

**Zone hull brightness (the brief's item 5): not a clear data miss, left open.** Ours feeds the stage's authored
`Sun colour` (`zonemode.effectsettings`, stage 3 `Sub Flash`: `1 1 1`; `zone_1/track.envsettings` authors
`1.5 1.294 1.0`, which the stage replaces). The live `directionalLight0Colour` `(1.144, 1.091, 1.906)` equals neither
and no stage authors it (the stage's `EQ colour tint` is `123 154 253`, `Sky horizon colour` `0.055 0.004 0.365`), so
what composes it, a blend with a bluish term, is unread. Feeding the live number would be a hand-transcribed constant.

**Cross-title.** Omega: **checked, applies, not wired.** `hdships/Zone` and the other hulls ship `*_N.gnf` normal maps
(66 `_n.gnf` entries under `hdships/` in `data01.psarc`, `Zone_TP_1024_N.gnf` among them) beside the same hull
materials, and 2048's reader `psp2` already decodes the `tangent` stream (`unpack_tangent`, signed bytes, `w` `+-1` on 82.6 %
of vertices; the `w` convention differs from HD's `0`/`255`), but Omega's hulls build through the PS4 path, which reads
no material program and fills no `GpuVertex::tangent`; wiring is the PS4 lit path's lane. 2048: **not checkable**
(PFS-encrypted `.pkg`).

## Hull brightness (2026-10-20, `hd-hull-brightness`): the "brighter and whiter" gap was a capture artefact, and the probe maps are sRGB

The brief: HD hulls read darker and greyer than the original on Harimau classic and Assegai_c1 (Talon's Junction) and
on the Zone hull. **Two findings, one a retraction.**

**1. The reference frames were taken with `Write Color Buffers` off (confidence 90).** The `hd-hull-normals` and
`hd-zone-material` lanes ran RPCS3 from the stock config, whose `Write Color Buffers` is `false` (both lanes' scratch
`rpcs3-config.yml`, line 277), which reads 1.6-1.8x too bright
(`docs/reverse-engineering/rpcs3-capture.md`, "A screenshot's brightness depends on `Write Color Buffers` too").
Direct check at one camera: `hd-hull-normals/harimau-classic/00.json` and this lane's WCB-on `00` have the same eye
(-6.54, -47.11, -195.1), and their whole-frame lumas (no HUD band) are 0.602 and 0.347, a ratio of **1.74**. Re-shot with it
on (`Used configuration:` beside the frames: `Write Color Buffers: true`, `Resolution Scale: 150`), Harimau classic on
Talon's Junction at three matched camera poses (steps: restore the saved Talon's Harimau start, `hd-grid-capture.py` for the
camera, the player's body pose read through `rpcs3_place.read_pose` at the same pause, ours rendered with `--camera-pose`
and `--pose`, shadows `original`, MSAA off, shifted by one tick):

| mean luma, frame without HUD | original | ours before | ours after |
| --- | ---: | ---: | ---: |
| rest on the grid | 0.347 | 0.379 | 0.364 |
| original at 398 km/h (ours posed there at rest) | 0.418 | 0.444 | 0.438 |
| original at 427 km/h (ours posed there at rest) | 0.387 | 0.435 | 0.431 |

"Before" is this branch's base with the probe format alone reverted (so it includes `hd-hull-normals`). The whole picture
is within 5-12 % of the original and brighter than it, not darker: no global gap, so no exposure, tone-map or albedo-decode
term is missing on a race hull (the exposure `scale` and the bloom were already measured unity, renderer.md, "The exposure
is unity"). **The "speed" rows compare ours at the original's position but at rest (the HUD shows 6 km/h), and the
original's hull region includes its engine plume and wash, which ours draws differently**, so only the rest row is a clean
like-for-like.

**2. Every lit term on a race hull equals the authored value (confidence 90, one boot).** Read live on the same hull
(draw dump of the rest frame, `rsx-draw-list.py`, `ps3-fp-live.py`, fragment block 17 of
`diffuse_with_specular_from_alpha_n`): sun colour `(2, 1.82745, 0.886275)`, sun direction `(-0.7776, 0.58152, -0.23911)`,
fog `(0.2627, 0.2157, 0.3804)` at density 0.001, `iblScalePower (4, 4, 4, 0)`, all equal to Talon's `track.envsettings`; the
scene target is `0x3148`, A8R8G8B8 2x MSAA, sRGB on write (`SET_SHADER_PACKER`, read by `hd-exposure`). The Zone hull's live
sun colour `(1.144, 1.091, 1.906)` is a constant of that draw, not a capture artefact, and stays unexplained.

**3. The one real hull-specific term: the probe maps are sRGB-decoded (confidence 85 for the register read, 70 that this
is what moves the picture).** Both probe units of the live hull draw (4 and 5) carry the address word `0x60730303`,
gamma nibble 7, the decode the albedo (`0x60710101`) has; the Zone dump of `hd-zone-material` shows the same word on its
units 4 and 5 (`0001a52a`, 128x64 and 64x32). Finding 5 read the maps raw ("chosen by pixel match") against the too-bright
reference; against the right one the raw read is the pale, pastel hull and the decoded one the original's saturation. The
array is now `Rgba8UnormSrgb` (`mesh_render::ENV_PROBE_FORMAT`), the engine's tint of 4 scaling linear light. Hand-boxed
hull regions of the rest and 398 km/h frames (mean over pixels above 0.25 luma; boxes picked by eye, poses not pixel exact,
the speed box holds the plume): rough figures.

| | luma | saturation |
| --- | ---: | ---: |
| original, rest | 0.461 | 0.313 |
| before (raw probes), rest | 0.610 | 0.283 |
| **after (sRGB probes), rest** | **0.436** | **0.326** |
| original, 398 km/h | 0.461 | 0.433 |
| before, posed there at rest | 0.604 | 0.336 |
| **after, posed there at rest** | **0.508** | **0.446** |

Judged as a player would (`data/reference/hd-hull-brightness/harimau-classic/`: the original over before and after at
three poses, and a hull-crop sheet): the orange and yellow panels and the blue wing tips read saturated like the original,
not chalky. Pinned by `hd_hull_env_probe::the_probe_maps_are_decoded_as_srgb` (a stored byte of 128 must light a hull to
byte 238; the raw read saturates at 255).

**Open.** The Zone hull's brightness (the brief's third case) needs a re-shoot with `Write Color Buffers` on: its two
frames are WCB-off, and its probe maps are now decoded as well (the thread of Zone races carries it). The Assegai_c1 sheet
of `hd-hull-normals` is WCB-off too. `emu-env.sh` copies the stock `~/.config/rpcs3/config.yml`, which has the option off:
set it before any brightness capture.

**Cross-title.** Omega: checked, differs (it ships no `.probes`, and its hulls build through the PS4 path, so the decode
has nothing to touch). 2048: checked, differs (a different `.probes` layout the reader names and refuses, so no hull reads
a probe array). Pulse and Pure: no probes, nothing to check.

## The hull's sun gate (2026-10-20, `hd-hull-pearl`): the occlusion map is sRGB-encoded

The fresh WCB-on pairs (`data/reference/compare-page/hd/`) read "the hull is a saturated orange with blue wing tips where the
original's is pearlescent". Cropped at 1:1 that is not the whole picture: on the Talon's pair (Harimau classic) the original's
wing tops are **warm orange-khaki**, `(114, 103, 71)` on the left wing top, where ours was a cool grey, `(80, 89, 75)`; the blue
tips are in both. The cause is the sun, not the paint or the reflection: with the hull's occlusion map forced to 1 the same
facets read `(188, 146, 79)` (too bright), forced to 0 `(80, 89, 75)` (ours), and the original sits between, at a gate near 0.24.

**Read off the original** ([ship-sun-occlusion.md](../ghidra/functions/ps3-hdfury-eu/ship-sun-occlusion.md), "Read live on RPCS3"):
the hull's occlusion map (unit 3, 256x256 `B8`, pitch 512) holds **61** on the road where the lightmap alpha is the byte **12**, and
13 where it is 1: the pass writes the mask sRGB-encoded and the hull reads the byte raw. Ours wrote it linear, so the gate was 0.047.
**Fixed** (`crates/render/shaders/occlusion.wesl`): the left wing top is now `(112, 104, 75)` against the original's `(114, 103, 71)`.

| facet, Talon's Harimau | original | before | after |
| --- | --- | --- | --- |
| left wing top | (114, 103, 71) | (80, 89, 75) | (112, 104, 75) |
| right wing top | (156, 160, 145) | (140, 155, 146) | (157, 163, 148) |
| right wing tip | (127, 163, 200) | (123, 160, 206) | (129, 170, 214) |
| fuselage, left | (121, 117, 112) | (100, 108, 108) | (117, 116, 110) |
| nose, green | (54, 63, 54) | (50, 62, 61) | (61, 69, 63) |
| right upper wing | (106, 131, 131) | (100, 124, 125) | (108, 130, 126) |

Mean absolute difference over the six, **8.5 before and 4.1 after**. Circuits whose road lightmaps are fully lit (alpha 255) do not
move (`sol-2`, `tech-de-ra`, `modesto-heights`, `metropia`, `chenghou-project`, `vineta-k`: 0 changed pixels; the encode of 1 is 1).

**What is not closed.** Over the whole hull the mean luma went from matched to about 10 levels bright (`talons` 109.4 to 119.9 against
the original's 108.9; `talons-assegai-c1` 62.8 to 72.1 against 62.4; `the-amphiseum` 81.1 to 86.6 against 81.3), and the facets
that got brighter are not the ones where the original is: the gate is right, the sun term it multiplies is not wholly. Not specular
(zeroed, the six facets move 4.1 to 3.4). Open: the self-shadow tap (its depth map cannot be read back), the `x8` on the specular's
saturate, and the hull-wide silhouette offset that makes a pixel metric unusable. **Open-air circuits keep the pearl gap**: on
Sebenco Climb and the Amphiseum our sun-facing facets read whiter than the original's orange and yellow, and the sun gate does not
reach them (their lightmap alpha is 255). Overlap scores (whole frame, hull is a small part of it): Talon's Harimau stored pair
0.18252 to 0.18289 (same `theirs.png`), a live rerender 0.1854 to 0.1810 RMSE and edge NCC 0.2524 to 0.2362 (two different
captures).

**Superseded in part (2026-10-21):** the overshoot and the open-air pearl were mostly the specular exponent, see
["The hull's specular exponent"](#the-hulls-specular-exponent-2026-10-21-hd-hull-colour-every-hull-drew-its-highlight-with-the-shared-32).

**Cross-title.** Omega: checked, not checkable (its original shadow tier is `Unbuilt`, hulls build through the PS4 path). 2048: checked,
differs. The ground ring and the pearl on open-air circuits stay open.

## The hull's specular exponent (2026-10-21, `hd-hull-colour`): every hull drew its highlight with the shared 32

**What a player sees.** The hull's sun-facing panels read white and chalky where the original's are saturated orange, yellow
and blue. The brief's "pearl gap" on open-air circuits was mostly this, and the part of the whole-hull overshoot that
`hd-hull-pearl` could not place.

**The cause (confidence 90, two circuits read live).** A hull's fragment program is `diffuse_with_specular_from_alpha_n[_vcol]`'s
`IBL`/`SunShadowMap` block (block 17, `0x740` bytes of code). Its live copy on **Sebenco Climb** and **Anulpha Pass**
(`HOOK_FP_LEN=0x800`, [rpcs3-capture.md](../reverse-engineering/rpcs3-capture.md); the default dump cut the block at instruction
`0x60`) carries these constants, every one equal to the circuit's own `track.envsettings`:

| slot | constant | Sebenco Climb | Anulpha Pass |
| --- | --- | --- | --- |
| `0x18`/`0x36` | sun direction | `(-0.842, 0.337, 0.421)` | `(-0.701, 0.504, 0.504)` |
| `0x1f`/`0x68`/`0x6c` | sun colour, `.w` the specular scale | `(4, 4, 4)`, 3 | `(1.087, 1.694, 1.790)`, 1 |
| `0x2c`/`0x5e` | probe tint | `(4, 4, 4, 0)` | `(4, 4, 4, 0)` |
| `0x48` | fog | `(0.646, 1.184, 1.852)`, density `0.0004` | density `0.0004` |
| **`0x58`** | **the exponent `MUL` after `LG2`** | **300** | **300** |
| `0x6f` | reflectivity | 0.5 | 0.5 |

The exponent's `MUL` constant is patched at draw time by parameter **`0x4232e459`**, and `0x4232e459` is the exponent: Harimau's
`.rcsmodel` material carries `0x4232e459 = 300`, `feisar_c1`'s `160`, `assegai_c1`'s `160`. The reader asked the patch table for
`SpecularPower` (`0x81e0e773`) only, found nothing, and fell back to the shared stand-in **32**: visualised in ours (the exponent
as a grey level), 46,325 of Sebenco's hull pixels drew at 32, 5,379 at 400 (a second material) and 14,092 at 0 (`diffuse_vcol`,
no highlight). A highlight `pow(N.H, 32)` against `pow(N.H, 300)` is a wash over a third of a panel instead of a point.

**Fixed.** `fragment::Program::specular_exponent_parameter` names whichever parameter patches the exponent's own slot, and
`skin::light_inputs::specular_exponent` reads it for `SpecularPower`, `0x4232e459` (`SPEC_EXPONENT`) and `carbonfibre`'s
`0xebecee0f` (`CARBON_EXPONENT`, 80 on `feisar_c1`, 70 on `assegai_c1`). Harimau's paint is now 300, `feisar_c1`'s 160, their
carbon 80 and 70 (pinned by `hd_hull_colour_factor_ground_truth::a_hulls_specular_exponent_is_its_materials_own_parameter`, which
reads 160 and 80 off `feisar_c1`).

**Why only three hashes (a census, `hd_specular_patch_census`).** Over the 16 circuits and four hulls, 732 blocks whose literal is `0.0`
have the exponent slot patched by a parameter other than `SpecularPower`: `0x2924e4ad` 476 blocks (authored 0.01 to 100), `0x7611a2d8` 99 (1
to 200), `0x4232e459` 46, `0x2fae3d80` 41 (all 1.0, Anulpha's `mt_diffuse_specular`), `0xebecee0f` 30, `0x2a2dc10b` 16, `0x71db09a8` 4,
`0xef18f362` 4, `0x05fec07d` 5, `0x46ce6dda` 2, `0x438ddeff` 2, `0xbd4582d3` 2, `0x6be7990e` 1, `0x3dc31258` 1. No two parameters ever
patch one slot (7,183 blocks). Reading all of them was tried first and moved the **scenery** (Amphiseum 150,462 pixels, Anulpha 181,392,
Modesto 20,070, Tech de Ra 47,681 differ from the three-hash reading) with a mixed whole-frame score, and a value of 1.0 as an
exponent is not credible next to a material whose own name says `specscalar`, so only the three whose slot, value and name agree
are read; the rest stay on the stand-in and are listed as open. The three hashes do reach the scenery (carbon and the `0x4232e459` materials on circuits, and the `sat(8 N.L)` factor below), which
`hd-scenery-materials` owns: outside the hull box (x 680-1240, y 540-960 of the 1920x1080 frame) the pictures changed on Anulpha Pass
(284,859 pixels, at most 62 levels), Modesto Heights (58,088), Vineta K (299,260), Tech de Ra (27,712, at most 1) and Talon's (13,704,
at most 6), and the grey difference to the original there did not get worse on any of the thirteen pairs (Anulpha -0.045, Modesto -0.162,
Vineta K -0.020, Tech de Ra +0.002, the rest 0.000; mean 24.752 to 24.735).

**The facing term is `sat(8 N.L)`, not `N.L` (confidence 85).** In every HD lit program the specular multiplies `MOV_SAT x8` of the
**unsaturated** `DP3` of the normal and the sun (`@0x26` here, `@0x36` of the track's `track_surface` block 5), so a highlight is full
strength from about seven degrees above the horizon instead of scaled by `N.L` itself. Wired for the hull and scenery
(`receives_shadow` not the track's); the track keeps `N.L` because applying it there moved four of the thirteen pairs the wrong way and
the track is the scenery lane's. Hull box difference, Vineta K 11.25 to 10.90.

**Read, measured, not wired: the diffuse `N.L` is unsaturated too.** The same `DP3` feeds `H3 = N.L * sunColour` unsaturated
(`diffuse_vcol` alone has `DP3_SAT`), so a facet turned from the sun would be darker than its ambient. Wired, Vineta K's hull
difference went 10.90 to 13.16 (worse) and Anulpha's 33.95 to 34.51, so the diffuse stays saturated. The original's self-shadow pass
(front faces culled) zeroes most of the back-facing sun, which is the likely reason the negative term is rarely seen; not checked.

**Measured (the 13 HD compare pairs, Harimau classic on 12 circuits and Assegai_c1 on Talon's Junction).** The pixel metric on a hull is
dominated by the silhouette: two captures of one state differ in camera (0.078 units on Anulpha Pass, about ten pixels at the hull), and the hull-box grey
difference of the re-made set swung from -9.2 (Talon's) to +6.4 (Vineta K) against the pairs it replaced with no change to the hull in
between. So each number below is taken on **one** original frame and **one** pose, ours rendered before and after, aligned to the
original by a scale and shift search (`0.97-1.02`, at most 14 px) over the hull, the hull pixels picked by a flat-colour render of ours
(`uniforms.env_layer`) eroded by 14 px, 25,000 to 32,000 pixels a pair:

| pair | luma orig | luma before / after | sat orig | sat before / after | hull mean abs diff before / after | 6 flat facets before / after |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| anulpha-pass | 100.6 | 93.1 / 84.9 | 0.342 | 0.281 / 0.336 | 21.0 / 19.5 | 15.0 / 9.7 |
| chenghou-project | 85.2 | 81.5 / 81.5 | 0.517 | 0.467 / 0.467 | 15.4 / 15.4 | 7.3 / 7.3 |
| metropia | 65.9 | 63.0 / 63.0 | 0.426 | 0.338 / 0.338 | 9.1 / 9.1 | 3.0 / 3.0 |
| moa-therma | 179.3 | 166.2 / 160.9 | 0.391 | 0.355 / 0.400 | 26.6 / 24.7 | 15.0 / 15.3 |
| modesto-heights | 119.5 | 124.1 / 118.1 | 0.276 | 0.234 / 0.245 | 15.9 / 11.4 | 11.0 / 3.9 |
| sebenco-climb | 168.2 | 159.4 / 155.8 | 0.304 | 0.291 / 0.324 | 22.0 / 20.2 | 14.1 / 12.5 |
| sol-2 | 111.2 | 105.0 / 105.0 | 0.305 | 0.293 / 0.293 | 16.2 / 16.2 | 6.1 / 6.1 |
| talons (Harimau) | 106.3 | 111.9 / 107.8 | 0.284 | 0.249 / 0.258 | 15.5 / 13.2 | 4.3 / 4.1 |
| talons-assegai-c1 | 52.9 | 46.6 / 46.3 | 0.446 | 0.401 / 0.407 | 19.6 / 19.7 | 4.2 / 4.2 |
| tech-de-ra | 136.3 | 150.4 / 140.4 | 0.357 | 0.347 / 0.377 | 28.0 / 22.2 | 26.4 / 26.3 |
| the-amphiseum | 70.7 | 71.7 / 70.2 | 0.362 | 0.292 / 0.316 | 8.3 / 7.5 | 6.5 / 3.4 |
| ubermall | 138.9 | 142.1 / 137.2 | 0.393 | 0.355 / 0.393 | 24.9 / 20.6 | 11.6 / 11.6 |
| vineta-k | 104.2 | 114.3 / 103.4 | 0.386 | 0.334 / 0.352 | 16.6 / 8.1 | 8.1 / 3.2 |
| **mean abs gap, mean** | | **6.70 / 5.84** | | **0.042 / 0.029** | **18.4 / 16.0** | **10.2 / 8.5** |

The facets are six 10x10-pixel cells, one per colour cluster, flat in the original (std below 9) and wholly inside the eroded hull, at
the same aligned coordinates in all three images. Examples (original / before / after, RGB): Modesto Heights' orange wing panel
`(149,110,98) / (182,149,123) / (148,108,92)`, its blue wing tip `(117,143,192) / (149,167,204) / (130,153,197)`; Anulpha Pass' orange
panel `(139,110,81) / (153,150,139) / (123,97,71)` and olive panel `(138,170,81) / (148,183,134) / (118,147,69)`; Talon's Harimau's
six sit at 4.3 and 4.1 mean levels (the tunnel's gate keeps the highlight small there). **The re-made pairs, the same code, the same capture**
(ours rendered with the old reading at each re-made pair's own pose and camera): box grey difference **26.53 to 25.95**, whole-frame RGB
difference 24.944 to 24.890; no pair worse but Anulpha Pass (+0.22 box, -0.06 whole frame, the luma shortfall below), Vineta K -3.61,
Tech de Ra -1.06, Ubermall -0.98, Modesto -0.73, Moa Therma -0.52, Sebenco -0.50, Talon's -0.36, the rest within 0.04.

**Not closed, by circuit.** Three circuits turn darker than the original once the highlight is narrow: **Anulpha Pass** (hull luma
-7.5 to -15.7), **Moa Therma** (-13.1 to -18.4) and **Sebenco Climb** (-8.8 to -12.4), saturation now level with it (Anulpha -0.061 to
-0.006, Moa -0.036 to +0.009, Sebenco -0.013 to +0.020). What is read is not the gap: the live constants (above), the program (every
instruction of block 17 matched against `shade.wesl`: the normal-map frame, the `(u, v)` law of both probe maps, `reflect(-V, N)` with
`V` the vertex program's `eye - position`, the `x4` probe tint, reflectivity `0.5` times the livery's alpha, the `/2` and the fog), the probe
content (our blend at the original's position against the live 128x64 and 64x32 maps on Sebenco: mean 82.5 / 92.9 / 102.1 against
80.2 / 91.2 / 100.8, 11.6 mean absolute per texel: the blend's weights, not isolated) and the livery (its first 576 rows
are the disc's `harimau_livery.gtf` byte for byte, and its DXT5 alpha is 255 everywhere, so the reflection reaches every hull texel in both).
The residual on Anulpha's and Chenghou's orange facets is a factor of 1.13-1.14 in display value on all three channels (`(214,133,91)` against
`(188,114,81)` on Chenghou, which the exponent never touched), about 1.3 in light, not an additive grey. Candidates, unmeasured: the post
bloom around the very bright highlights (sun 4 and a specular scale of 2 to 3 on Sebenco and Moa), and the craft's SPU vertex light
(`f[TC1]`), which the hull programs add before the albedo multiply and which ours feeds from the weapon point lights
(`weapon_light.rs`); whether the original's list also carries the circuit's own lights is unread. The glass chunk
(`glass_texture`, exponent parameter `0x4232e459` = 115) still draws with 32: its chain is not the hull program's.

**Cross-title.** Omega: checked, not checkable here - its hull shaders are PS4 programs this reader does not decode, so
`light_inputs::specular_exponent` gets no program for them and they keep the stand-in; nothing was ported by accident. 2048: checked, differs -
its materials name their uniforms by their own GXP programs ([2048-material-params.md](../formats/2048-material-params.md)), not by HD's
hash table, so `0x4232e459` has no counterpart to look up. Whether their hull highlights are the stand-in 32 is open for both.

## The engine bay: Assegai_c1's Fury light, and the two circuits that switch the SPU lights off (2026-10-21, `hd-hull-bay`)

**What a player sees.** Behind the cockpit the engine bay (the housing around the nozzle) is lit in the engine light's colour in
the original. Ours lit Assegai_c1's bay blue where the original's is orange, and on Metropia and Sol 2 ours draws Harimau's bay black
where the original's is blue.

**Assegai_c1, fixed (confidence 85).** The engine light's Fury colour `(40, 10, 4)` is selected by the same `_c1`/`_n1`/`detonator`
directory test as the red trail, and the test was case-sensitive: the pair scripts pass `--team ASSEGAI_C1`, so the Fury hull flew
the classic blue light (and the classic trail colour). The test is case-blind now (`load::roster::hd_trail_red`). The front end
passes the directory's own lower-case id, so only the command line and the pair path were affected. On the compare pair the
housing and its rim line turn orange like the original's (bay crop mean absolute difference 24.56 to 24.18 levels; the poses
differ by a few pixels, see "The drawn hull is not the body", so the number moves little).

**Metropia and Sol 2, open (confidence 75 on the reading, no score on the cause).** Both circuits' `.envsettings` author
`Lighting.Enable spu vertex lights = 0` and ours takes the engine light off every receiver there, the hull included. The original's
bay is blue on both pairs. Ours with the light forced onto the hull reproduces that picture (walls and yellow rim line), **but the
original does not get it that way**: on RPCS3 (interpreter, the Sol 2 and Talon's Junction Time Trial states) the light is still
submitted on Sol 2 (`EngineFlare_SubmitSpuLight`'s `0x0029ff8c`, 2 stops in 25 s on both circuits), yet the hull's `SVC1` branch
(`0x003eb368`, `r19` the hull record) was taken **0 times in 25 s on Sol 2 against 2 on Talon's Junction** (list count 1). So the
hull draws without the SPU light there, as ours does, and the blue comes from a term not read yet. Ours is left as it was, an honest
black, rather than lit by a light the original does not apply. The sample is small (rate about one stop in 12 s on the interpreter);
a longer Sol 2 run, and what the non-`SVC1` variant reads in the bay, are the next steps.

**Cross-title.** Omega and 2048 keep the engine light off (`engine_light: false` in their `Flare`, chosen), so neither finding reaches
them; whether their originals light the bay is not checkable (no PS4 capture) and not compared on the Vita. The case-blind Fury test
applies to every title that reads it.

## The drawn hull is not the body: a per-frame shake from `ship+0x6b00` (2026-10-22, `hd-hull-bay`)

**What a player sees.** On every one of the 13 HD pairs our craft sits 3 to 13 pixels away from the original's (Talon's Junction
`(0, -9)`, Sol 2 `(+13, -6)`, Modesto Heights `(-12, +1)`), while the floor right beside it lines up within one pixel. Shifting our craft
onto the original's halves the craft difference (Tech de Ra 49 to 16, Sebenco 53 to 24, Vineta K 33 to 12 levels, a mask of the
craft's own pixels from a render with the craft lifted out of view).

**Not ours, not lag (measured).** Ours draws exactly the RAM pose (no between-ticks blend, lean or roll at the screenshot frame), and
the pair poses equal the RAM body. The original's craft does not move between its two captured frames (0.12 s apart), and the near
floor aligns, so neither presented-frame lag nor the camera's eye explains it.

**The original draws the hull away from its body (confidence 80).** On RPCS3 the main hull's `EdgeGeom` matrix (`0x005d8488`, `r11`,
scale 0.75) equals the matrix the craft struct holds at `ship+0x40` (a copy at `+0x80`), and it is **1.1 to 1.8 degrees** off the body
rows at `body+0x1d0` and 0.05 to 0.09 units off its position, about an axis that differs between reads (Talon's Junction 1.13 to 1.33
degrees, Sol 2 1.76). Posing ours with the Talon's read moves that pair's craft from `(0, -9)` to `(-2, -1)` pixels and its craft
difference at zero shift from 32.37 to 16.04. In `Ship_UpdateEntity` (`0x000eadb8`) the display block builds the node matrix from the
body rows and the drawn lean (`ship-lean.md`, "The display matrix"; lean and roll phase are 0 at rest): stopped at `0x000ebc78` the
node matrix equals the body to 0.016 degrees, at `0x000ebc88` it is 1.12 to 1.13 degrees and `(-0.013, +0.087, -0.086)` /
`(-0.008, +0.054, -0.052)` off on two consecutive frames. Between the two sit `FUN_000d0c70(ship+0x6b00, f2 = *(r9+0x4c4))` and
`FUN_000d39d0(ship+0x6b00, node, 0)`: an object at `ship+0x6b00` that adds a changing rotation and offset every frame. Neither
function decompiles (VMX code, 0x258 and 0x4a4 bytes). **Read from the disassembly (2026-10-22,
[hull-shake.md](../ghidra/functions/ps3-hdfury-eu/hull-shake.md)):** `HullShake_Update` runs a uniform cubic B-spline over random
points, one new point a second (x in `4 * (r - 0.5)`, y and z in `1.8 * (r - 0.5)`). The hull turns by roll `0.025 * x`, yaw
`-0.01 * x` and pitch `0.025 * (y - z)` of the previous frame's value, and moves by `4 * (p - q)` (the spline's change over one frame).
Both are scaled by `clamp((80 - thrust) * 0.02, 0, 1)` on a 0.99-per-frame smoothed engine thrust (body `+0x4c4`), so the shake is
full at rest and nearly gone under full throttle. `HullShake_Apply` composes the rotations in the hull's own frame. `rand` returns 30
bits, so the points are centred on zero. A still pair cannot match a shake frame for frame; what can be ported is the law, and until then the pair pixels
carry it.

**Next.** Sample `ship+0x6b00` live for the lane masks, the frame time and what body `+0x4c4` is (thrust or speed), then port the shake to `oag_raceplay`'s drawn model
matrix (view only, unhashed). Cross-title: Omega carries HD's engine, so the same
object is expected, not checkable here; 2048 not compared.
