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

**The decode domain was chosen by the pixels.** The unit's sampler word carries the gamma nibble `7`, but the project
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
Chosen, not measured: the raw domain for the ambient (the sampler word says otherwise) and the one-tick lag, whose
weight is the 41/52 count above.

**Cost.** See "Cost" below.

**Cross-title.** 2048: checked, differs (a different `.probes`, and an authored sky). Omega: checked, applies, not wired
(not checkable). Pulse, Pure: no `.rcsmaterial`.

**Open.** The nozzle jewel (`emissive_bloom`, `TC4`) is unread. The weights at two tight corners (a section pair two apart).
The tail arrays of the file (24 + 4 + 3 bytes a probe) are read by the loader and unread here. The `Spot`/`SVC1` words of
the IBL variants (`0x047`, `0x807`, `0x847`) are carried by the existing SPU light and the shadow path rather than resolved:
the shader keys on the draw's craft layer and the material's own reflection sampler, which for the hull is the same set.

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
