# Detonator's look: Zone's effect gate, its own settings file, and what ours leaves on the circuit's day scene

`EBOOT.elf` (Wipeout HD/Fury, PS3, `ps3-hdfury-eu`), image base `0x00000000`. Read 2026-11-22
(`det-look-re`), static, off the pages named below plus one pixel sample of the adopted pair
`data/reference/compare-page/hd/detonator-talons-mines-ahead` (`theirs.png`, the original on RPCS3, HD/Fury EU,
Talon's Junction, Feisar, lap clock 2.66 s, stage 1). Nothing here ran live; the live legs are the pair's frame.

**The question.** The original's Detonator frame is near black with a violet and red tinted track and a dark
hull; ours draws the circuit's day sky (`detonator-talons-*` pairs, thread "Render accuracy", row 1, 31 % of the
frame). Does mode 14 take Zone's visual mode (the effect-settings, sky and grade path), or its own settings,
and with which values?

**The answer.** Both, and they are two different gates:

1. **The effect block is Zone's, on the same gate byte.** `Environment_LoadRaceScene` (`0x003f3fb0`) sets
   `g_ZoneEffectsActive` (`0x00d45f84`) for `(1 << mode) & 0x00206040`, modes 6, 13, **14** and 21
   ([zone-sky.md](zone-sky.md), 82). Every piece of the Zone look that tests that byte, and not the mode,
   therefore runs in Detonator: the sky swap to `Data/Tex/ZoneSky.gtf` and the gradient dome, the stage block's
   publication in `Scene_PrepareFrame`, the stage pass that draws the equaliser target on billboard slots 1 to 7.
2. **The values are Detonator's own.** The mode-14 arms pick `Data/Environments/DetonatorMode.effectSettings`
   (`DetonatorModeDLC3` on `25_Track`..`28_Track`), the `Detonator*` texture sets, and the stage from
   `RaceManager+0x2e10` instead of the per-craft field. Same 73-key schema, same 15-row ladder, other numbers.

## The gate byte and what reads it

| What | Test | Where | Detonator | Confidence |
| --- | --- | --- | --- | ---: |
| Sky: `ZoneSky.gtf` for the circuit's `sky.gtf`, same handle | byte | `0x003f4270` | takes it | 82 |
| Gradient dome, `Sky horizon colour` and `Sky zenith colour` of the blended stage | byte | `0x003ad598` | takes it | 80 |
| Light, ambient, fog, scene and track colours, `+0x7bb0` and neighbours | byte | `Scene_PrepareFrame`, `0x003ab65c` | takes it | 85 |
| Billboards 1 to 7 show the stage target (the equaliser) | byte, `slot == 8` exempt | `Billboard_LoadModelAndBind` `0x003a4da0` ([billboards.md](billboards.md)) | takes it | 80 |
| Hull sun from the stage (`directionalLight0Colour`) | byte | [zone-palette-select.md](zone-palette-select.md), section 1 | takes it | 85 |
| Bloom block (`HDR and Bloom` fields) | mode id, `0x003ad940`: 6 Zone arm, **14 Detonator arm**, else common tail | [zone-frame.md](zone-frame.md) | **own arm** | 80 |
| Stage index | mode id, `0x003da638`: `0xe` -> `RaceManager+0x2e10`, `0xd`/`0x15` per-viewport tables, else `craft+0x640` | [zone-effectsettings-loader.md](zone-effectsettings-loader.md), tenth pass | **own arm** | 85 |
| Settings file and texture sets | mode id, `0x003f4138`, `0x003d71ec` | [zone-palette-select.md](zone-palette-select.md), section 2 | **own arm** | 88 |

The mode id and the byte are different tests, and the Detonator arms of the three "own arm" rows are the only
places the original distinguishes Detonator from Zone in the look. Rows 1-5 have no mode test at all: the same
code runs for 6, 13, 14 and 21.

## What the file says at stage 1

A Detonator race is constructed at stage `1` (`SPDetonator`'s two constructors, `li r9,1; stw r9,0x2e10`;
[detonator-mode.md](detonator-mode.md)), so lap 1 shows row `1 Sub Venom` of `DetonatorMode.effectSettings`
(`DATA00.PSARC`, `/data/environments/detonatormode.effectsettings`, 43,240 B; read off the disc, not
transcribed from a page). Talon's Junction is not one of `25_Track`..`28_Track`, so it is this file and not the
DLC3 one, which ours already loads (its load report names `detonatormode.effectsettings`, 15/15 track and 15/15
scene art, Track recolour `zoneEffect [0.972549, 0, 0]`, Scene chunks black).

| Key at stage 1 | Value | Reading |
| --- | --- | --- |
| `Sky horizon colour`, `Sky zenith colour` | `0 0 0`, `0 0 0` | the dome is black from horizon to zenith |
| `Lighting.Sun colour` | `0 0 0 0` | no sun: the hull's `directionalLight0Colour` is black |
| `Lighting.Constant Ambient Colour` | `1 1 1 0` | |
| `Lighting.Fog colour`, `Fog density` | `0 0 0 0`, `0` | no scene fog |
| `Lighting.Alt Fog colour`, `density` | `0.007843` grey, `0.001` | |
| `Lighting.Track Fog colour`, `density` | `1.8 0.141176 0.007059 0`, `0.0015` | an orange-red fog on the track set |
| `Scene.Texture Colour`, `Near`, `Base Colour` | all `0 0 0` | scene materials (everything but the track) draw black plus their rim and glow terms |
| `Scene.Base Colour Highlight` | `0.078431` grey | |
| `Track.Texture Colour` | `0.972549 0 0` | the track set's red strips |
| `Track.Base Colour Highlight` | `0.156863 0 0` | twice this, `(0.314, 0, 0)`, is the hull's `iblScalePower` ([zone-hull-light.md](zone-hull-light.md)) |
| `EQ colour tint`, `EQ analogue colour tint` | `171 9 255 255`, `14 1 55 255` | violet, the equaliser bars' colour |
| `Aurora Colour`, `Airbrake Colour` | `204 51 51 255`, `0.498 0.004 1` | |
| `Detonator Mine Colour`, `Electricity` | `122 25 255 255`, `109 0 232 255` | the mine's violet |
| `Detonator Bomb Inner`, `Outer` | `255 128 65 255`, `255 112 25 255` | the bomb's orange |

(`Scene.Luminance Power`, `Track.Luminance Power` and `Radial Bloom Intensity` are never authored by any of the
four files, [zone-effectsettings-loader.md](zone-effectsettings-loader.md).)

**The pair agrees with the file.** On `theirs.png` the upper band (`x 200..900, y 60..300`) averages
`(18, 10, 11)`: no cloud, sea or blue texel anywhere, which is what a black dome authors. Ours averages
`(96, 111, 121)` there, the circuit's day sky. The floor's dashes are violet, `(132..143, 6..7, 199..211)` at
their brightest, against `EQ colour tint` `(171, 9, 255)`: the same hue, about 0.8 of the authored brightness
after the resolve (hypothesis, confidence 60: one frame, the tonemap and the bar's own fade not separated).
Confidence for the file's values being what the frame shows: 80.

## How ours applies Zone's look today, and where Detonator stops short

`oag_raceplay::load::environment::zone_grade` (`crates/raceplay/src/load/environment.rs`) already builds a grade
for `Mode::Detonator` from `oag_hd::race::DETONATOR` (its own file, its DLC3 split, its two texture sets, stage
from the lap counter through `ZoneGrade::with_lap_stages`). That grade is applied per frame in
`scene/frame.rs` whatever the mode: fog, light rig, the Track and Scene recolour, the stage-light hull sun, the
hull probe tint, the EQ tint. So the track and scene colours of a Detonator frame are already the file's
(row 3's red strips on a near-black track in ours).

The pieces that are gated on **`Mode::Zone`** where the original's gate is the byte (mask 6, 13, 14, 21):

| Ours | Gate in ours | Original | Change for Detonator |
| --- | --- | --- | --- |
| Sky: `ZoneSky.gtf` as the reflection map and `sky_cube::zone_dome` | `load/environment.rs:71`, `(mode == Mode::Zone).then_some(race.zone_sky)` | byte | `Mode::on_zone_mask()`. Stage 1's dome is black. |
| The dome's per-frame recolour (`shade_zone_dome`) | follows the dome | byte | none beyond the line above; the grade already carries the stage's `Sky horizon/zenith` |
| Billboards 1 to 7 on the shared stage target (`zone_equaliser`, `zone_shares_texture`) | `load/gantry_visibility.rs:25`, `mode == Mode::Zone && spec.zone_shares_one_texture` | byte | `on_zone_mask()`; the stage's EQ tint colours the bars. Style 0 only exists for stages 0-2, the other styles draw the grid ([zone-equaliser.md](zone-equaliser.md)). |
| Gantry start board | `gantry_mode()`: Detonator already has its own | mode-keyed in the original too | none |
| Bloom block (`bloom::for_mode`) | `load/environment/bloom.rs:170`, `mode == Mode::Zone` | mode id, **own arm** | A separate change, not "same as Zone": the Detonator arm takes the common head (`Bloom feedback -0.4`, sizes `1.0`/`1.0`, frame exponent `0.1`, alpha contribution `1.0`, radial enabled), radial contribution `0.1` (`g_ZoneBloomTable +0x28`), radial exponent `0.1`, distance scale `0.6`, and **keeps the circuit's frame contribution and adaption boost**, where Zone's arm forces `0.5` and `5.0`. `oag_title::ZoneFrame` carries Zone's; Detonator needs the radial value and the "keep" rule as data. |
| Track Zone variant of the circuit model | `view.rs:214`, `track_model(.., options.mode == Mode::Zone, ..)` | not read for Detonator | Not a look change: it picks the `wohdmeshrefract` handling for 2048/Omega. Leave. |

What does not need to change: the palette file, the stage textures, the stage source (lap counter), the
craft model and handling, all already Detonator-specific.

What is **not** read and stays a measurement for the wiring lane:

- **The hull.** Ours drew a pale silver hull; the original's is dark gunmetal with a lit blue strip and cyan wing
  edges. Stage 1 authors a black sun and `iblScalePower (0.314, 0, 0, 1)`, and ours already takes both through the
  grade (`lit_by_stage_alone`, `hull_probe_tint_scale`, `scene/frame.rs:476`), so the pale hull was not the mode's
  light (70, now confirmed: "The hull's paint" below). Closed to +6.4 grey on the side panels (was +51.9; "The hull's paint").
- **The mine and bomb colours.** The four `Detonator Mine/Bomb` colour keys exist per stage (table above) and are
  parsed; where the mine and bomb materials bind them is the `det-colour-re` lane's function set and is not
  read here.
- **Track violet.** Whether the floor's violet dashes are the equaliser bars (billboards on the floor) or the
  `zoneTexVis` visualiser glow tinted by `EQ colour tint` ([zone-visualiser.md](zone-visualiser.md)) is one frame
  away from settled: both share the tint and the byte gate; which one draws these dashes was not separated.

## Wired

Wired 2026-11-22 (`det-look`); the three steps below are code, each with a test that fails when it is dropped.

1. **Sky and dome.** `hd_sky_model` takes `Mode::on_zone_mask()` for the `zone_sky` choice, so Detonator loads `ZoneSky.gtf` and
   draws the gradient dome in the stage's colours; stage 1's are `(0, 0, 0)`. Titles whose `zone_sky` is `None` (Pulse,
   Pure, 2048, Omega) do not change. Test: `zone_sky_ground_truth::a_detonator_race_draws_the_zone_dome_in_its_own_stage_colours`
   (Talon's Junction, the same circuit as the Single Race load).
2. **Billboards.** `zone_shares_texture` reads `Adverts::detonator_shares_one_texture`, `true` on HD alone. Omega and 2048
   carry `zone_shares_one_texture` for Zone but their Detonator frames are unread, so they keep the per-slot adverts. Test:
   `gantry_visibility::tests::a_detonator_race_shares_the_stage_target_on_hd_alone`.
3. **Bloom arm.** `bloom::for_mode` runs `ZoneFrame` for Detonator when `ZoneFrame::detonator_arm` (HD): feedback, both sizes,
   frame exponent and alpha contribution taken, the circuit's frame contribution and adaption boost kept. Test:
   `bloom::tests::a_detonator_race_takes_the_table_but_keeps_the_circuits_gain` (a circuit whose every value differs from the
   table's). **Not wired: the radial bloom** (`0.1` / `0.1` / `0.6`, enabled): `oag-post` has no radial pass, so Zone's `0.04`
   is equally absent.

Result on the three Detonator pairs (Talon's Junction, Feisar; the upper band is `x 200..900, y 60..300`; original `(17..18, 10..13, 10..24)`):

| Pair | RMSE before to after | Mean grey before to after | Upper band ours before to after |
| --- | --- | --- | --- |
| `detonator-talons-mines-ahead` | 0.297 to 0.223 | 41.5 to 20.9 | `(96, 111, 121)` to `(16, 8, 8)` |
| `detonator-talons-cannon-volley` | 0.289 to 0.205 | 35.9 to 20.6 | `(84, 91, 98)` to `(16, 8, 8)` |
| `detonator-talons-pad` | 0.264 to 0.159 | 28.3 to 12.6 | `(65, 68, 74)` to `(22, 13, 14)` |

The billboards draw the stage target in Detonator (the pad pair with `--zone-spectrum-test` shows violet bars where the
original's are); the pairs themselves show none because their re-render is silent, a zero spectrum being a black grid on black.
The slot 8 gate (`Mode::Zone`, the card's Zone-grade exemption) was also tried on the mask: a Detonator start frame came out
pixel-identical (whether the exemption fires there was not separated), so it stays. What the frame still lacks: the floor's violet dashes and the lit scenery at upper right (the
original draws it dark), the lit blue strip and cyan wing edges (the pale hull itself is fixed, below) and the mine and bomb colours (`det-colour-re`).

## The hull's paint: the second texture was added as a glow (read and fixed 2026-11-23, `det-hull-look`)

**The cause.** The pale hull was not the light, the livery, the `zoneship01` rim or the program variant. It was
`mesh::rcs::emissive`'s structural accumulate: a material whose program reads unit 1 gets its second texture added to the
paint as a tinted glow, unless the second texture's role is measured to be a normal or specular map. Two Detonator hull
materials slipped through it:

| Material | Share of the hull | Second texture `Pick::aux` landed on | What the program does with it |
| --- | --- | --- | --- |
| `detonator_ship_dg_iridescent` (the two side panels and wings, 814 triangles) | the big panels | `detonator_stripe.gtf` (`0x215b724c`, 32x32 DXT5: GCM format `0x88`, 1,344 bytes with three mips) | a ramp: `TEX H6.xyz, R1.wwww unit3`, coordinate a scalar `dot(N, R3)` (the `IBL` block `0x46d0`, `@0x22`), folded into the overlay below |
| `detonator_ship_rich_iridescent` (the fuselage, 8,341 triangles) | the body | `detonator_ao.gtf`, the picture itself (the ordinal fallback on entry 1, where the program's only picture sits: block `0x3550`, `@0x00` `TEX H0.xyz, f[TC3] unit1`) | nothing: no second texture |

Adding either at full weight is the pale wash. **Fix:** `emissive.rs` refuses the accumulate for a facing-ramp sampler
(`SHIP_FACING_RAMP_SAMPLERS`, the stripe; the hash binds nothing else in all 643 HD models of the seven archives) and for a family
that reads one picture (`SINGLE_PICTURE_FAMILIES`, the body); the load report counts both (`emissive_lookup_excluded`).
`carbonfibre`, whose second texture is a plain diffuse, stays on its old path (the existing test pins it). Confidence 90 on the cause
(the one switch that moves the panels off `(109, 97, 98)` by 55 grey, four that do not), 85 on the two program reads (static).

**Discriminating switches, in order** (one scratch copy of `detonator-talons-mines-ahead`, ours re-rendered through the page's own
rerender path per switch, panel box mean RGB; the box is the two side panels, `x 885..940` and `972..1025` at `y 685..780` of ours,
`x 887..935` and `975..1020` at `y 755..870` of the original, 10,260 and 10,695 pixels; ours' hull sits 70 px higher, so the boxes
differ and cover the same panels):

| Switch (on the tree before the fix) | Panel box ours |
| --- | --- |
| none | `(109.4, 97.4, 98.1)` |
| hull ambient forced to 0 | `(105.7, 97.4, 98.1)` |
| lit sum forced to 0 | `(105.7, 97.4, 98.2)` (so the grey is not light: a glow) |
| `OAG_ALBEDO_ONLY=1` (every picture, no light) | `(210.0, 203.7, 205.7)` (the paint is pale, it is the glow that wins) |
| `OAG_SKIP_MATERIAL=rich_iridescent` | `(110.3, 97.5, 98.2)` |
| `OAG_ONLY_MATERIAL=dg_iridescent` / `rich_iridescent` | `(107.6, 103.4, 103.3)` / `(121.0, 121.0, 120.9)` (the box holds both materials) |
| **second binding refused on both materials (the fix)** | **`(54.1, 30.9, 32.0)`** |
| the fix and the overlay below | **`(67.7, 50.1, 50.6)`** |
| original (`theirs.png`) | `(60.5, 44.2, 44.5)` |

Mean grey on the box: ours 101.6, the original 49.7 (gap **+51.9**); with the glow refused 39.0 (**-10.7**); with the overlay 56.1
(**+6.4**). The three Detonator pairs (RMSE, mean grey, ours before to after both changes): `mines-ahead` 0.223 to 0.221, 20.85 to
20.34; `cannon-volley` 0.205 to 0.204, 20.64 to 20.33; `pad` 0.159 to 0.155, 12.63 to 12.18. Of the 23 other HD pairs 22 re-render
identical to the pre-fix tree and `zone-vineta-k-z5` differs in 2 pixels by one level of green (a recompiled shader's rounding on
the Zone hull).

**The overlay, read and wired.** With the glow refused the red was close and green and blue were 12 to 13 low: an untinted additive
term, and the program has one. In the `IBL` block `0x46d0` the colour is `light * AO + H8` with `H8 = 2 * Colour * (AO *
stripe(R1.w))` while `Colour < 0.5` (`@0x3a`; `Colour` `0x02ab9f07` is `0.10948` on `dg_iridescent`; the overlay blend `1 - 2 (1 -
AO*ramp)(1 - Colour)` above `0.5` is read but no material authors it, so it is not wired). Its coordinate `R1.w = dot(R1, R3)` takes
`R3 = normalise(f[TC1])`; the hull vertex block `0x3a20` writes `o[TC1].xyz = eye - position` (`@9`, `scripts/ps3-microcode.py
vp-file` block 2), so the ramp is read at `N.V`, positive on a front face; `R1` is the normal-mapped world normal (`@0x1d`, `DIVSQ R1`
over the tangent-frame sum). Wired as `mesh::rcs::hull_overlay` (a glow-table entry of rate `HULL_OVERLAY_RATE` with `Colour` in
`tint.x`, the stripe the slot's second binding) and a `shade.wesl` term added after the light multiply, untinted, next to
`hull_rim`. Confidence 85 for the term and its coordinate (the shape, the vertex write and the measurement agree). It lands 6 grey
above the original (`+7.2, +5.9, +6.1`, uniform): something else the program does is not drawn (the fog and `/2` are applied; the
stage's sun and ambient on the panels were not separated) and is open. One block of the material's 25 was read for the coordinate
(`0x46d0`, the `IBL` variant); the others address the same unit through other registers (12 of 25 as a scalar splat, 13 as a plain
register, unread). The fuselage material (`rich_iridescent`) has its own overlay through the iridescence ramp at another coordinate
and is not wired.

**Cross-title.** 2048 and Omega: checked, differs in the reader. Their hulls draw through `mesh::rcs::psp2`, which does not call
`emissive::emissive`; its `glow` plan sets `ADD_SECOND` only for a measured glow program. Whether their Detonator hull shows the same
wash is not checkable: no Detonator frame of either title exists. The stripe and picture reads above are HD's programs.

Tests: `oag-mesh` `hd_hull_glow_role_ground_truth::detonators_hull_paints_no_glow_from_its_ramp_or_its_own_picture` (red with the fix
removed: both families carry `ADD_SECOND`; it also pins the panel's overlay entry, `Colour` `0.10948`), and
`hd_hull_env_probe::a_tagged_hull_panel_adds_twice_colour_times_picture_times_its_ramp` (red with the shader term removed).

## Cross-title

- **Omega: checked, differs** (and the Detonator billboard share stays off: `detonator_shares_one_texture: false`). `oag_omega` carries `zone_sky: None` and Omega's Zone sky source is unread in its
  `eboot.bin` ([zone-sky.md](zone-sky.md), "2048 and Omega"); its Detonator grade table is HD's (the four paths are
  in its `eboot.bin` and `data00.psarc`, [zone-palette-select.md](zone-palette-select.md)). Nothing to wire until
  the sky draw is read.
- **2048: not checkable here.** Its Detonator reaches `<circuit>\ZoneMode2048.effectSettings` or HD's Detonator
  pair through `Environment_Load`'s Zone-family gate ([detonator-mode.md](detonator-mode.md)); whether its sky draw
  is gated by the same byte is unread (its Zone sky draw is unread, [zone-sky.md](zone-sky.md)), and there is no
  Detonator frame pair of 2048 to read it from.

## Confidence

| Claim | Evidence | Confidence |
| --- | --- | ---: |
| Mode 14 turns on `g_ZoneEffectsActive` | mask `0x00206040` at `0x003f4064`, [zone-sky.md](zone-sky.md) | 82 |
| Sky, light, grade publication and the equaliser target ride the byte, not the mode | `zone-sky.md`, `zone-palette-select.md`, `billboards.md` (static, 85 each; mode 14 inherits the byte, never watched live) | 80 |
| Detonator's settings file, textures and stage source are its own | `0x003f51b4`, `0x003d71ec`, `0x003dd5dc` | 85 |
| Stage 1 authors a black sky, no sun, no fog, black scene colours | the file, read from the disc | 95 (file), 80 (as the frame) |
| Bloom has a Detonator arm that keeps the circuit's frame contribution and adaption boost | `0x003ad940`-`0x003aebb0`, static only | 70 |
| The pale hull was not the mode's sun; it was the second texture added as a glow | five switches on one pair, "The hull's paint" | 90 |
| The panels' iridescent overlay is `2 * Colour * AO * stripe(N.V)`, `TC1 = eye - position` | `0x46d0` `@0x22`..`@0x4a`, vertex block `0x3a20` `@9`, box `(67.7, 50.1, 50.6)` against `(60.5, 44.2, 44.5)` (without it `(54.1, 30.9, 32.0)`) | 85 |
| The floor's violet dashes are the EQ tint | one pixel sample of one frame | 60 |

## Reproduce

```sh
ISO=data/images/hdfury-ps3-eu-dec.iso
python3 scripts/psarc.py cat $ISO:PS3_GAME/USRDIR/DATA00.PSARC /data/environments/detonatormode.effectsettings \
  | grep '"1 Sub Venom\.'
ELF=data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf
llvm-objdump -d --mcpu=pwr6 --start-address=0x3f4030 --stop-address=0x3f4170 $ELF   # the mask
llvm-objdump -d --mcpu=pwr6 --start-address=0x3ad938 --stop-address=0x3ad970 $ELF   # bloom arms
```
