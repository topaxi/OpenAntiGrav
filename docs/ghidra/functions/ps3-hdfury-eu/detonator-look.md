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
- **Track violet.** Separated, read and wired 2026-11-23 (`det-track-look`): ["The floor's violet"](#the-floors-violet-the-visualiser-read-and-fixed-2026-11-23-det-track-look). The dashes are `zoneTexVis` bar segments, the floor its smooth slot 164; neither is a billboard.

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
   table's). **The radial bloom** (`0.1` / `0.1` / `0.6`, enabled) draws nothing in a running round: its strength
   is `1.0` only while the craft is out of the race ([zone-frame.md](zone-frame.md), "The radial pass", 78), so its absence
   in ours matches a racing frame.

Result on the three Detonator pairs (Talon's Junction, Feisar; the upper band is `x 200..900, y 60..300`; original `(17..18, 10..13, 10..24)`):

| Pair | RMSE before to after | Mean grey before to after | Upper band ours before to after |
| --- | --- | --- | --- |
| `detonator-talons-mines-ahead` | 0.297 to 0.223 | 41.5 to 20.9 | `(96, 111, 121)` to `(16, 8, 8)` |
| `detonator-talons-cannon-volley` | 0.289 to 0.205 | 35.9 to 20.6 | `(84, 91, 98)` to `(16, 8, 8)` |
| `detonator-talons-pad` | 0.264 to 0.159 | 28.3 to 12.6 | `(65, 68, 74)` to `(22, 13, 14)` |

The billboards draw the stage target in Detonator (the pad pair with `--zone-spectrum-test` shows violet bars where the
original's are); the pairs themselves show none because their re-render is silent, a zero spectrum being a black grid on black.
The slot 8 gate (`Mode::Zone`, the card's Zone-grade exemption) was also tried on the mask: a Detonator start frame came out
pixel-identical (whether the exemption fires there was not separated), so it stays. What the frame still lacks: the lit scenery at upper right (the
original draws it dark), the lit blue strip and cyan wing edges (the pale hull itself is fixed, below) and the mine and bomb colours (`det-colour-re`).

## Fog: a stage that authors density 0 has no fog (2026-11-23, `det-scene`)

**Law (confidence 80: the file's keys and the exp2 curve are read, which fog block each draw takes is not).** The stage's `Lighting.Fog
density` is the coefficient of HD's own curve `exp(-(density * depth)^2)` (`shade`'s `authored_fog`), which is 1 at zero: a stage that
authors density `0` draws no fog. The colour is not a fog on its own: `detonatormode.effectsettings` stage 2 (`Venom`) authors an orange
`Fog colour 1.8 0.338824 0.007059` beside density `0`, and stages 0 to 1 author `0 0 0` beside `0`; borrowing the circuit's density for
a colour like that would invent a dense orange haze. Ours had read a zero density as "this stage adds no fog" and kept the circuit's
own `.envsettings` fog (Talon's Junction: `0.26 0.22 0.38`, density `0.001`), which is a pale lilac over every far surface and the cause
of the lilac skyline blocks in the upper-middle third. `ZoneGrade::fog` now returns the fog-off state for a stage that authors density
0 (keys absent: the circuit's fog still stands).

Found by measurement: fog forced off for a zero-density stage moved the upper-middle third (3 x 3 grid, mean absolute grey difference)
`mines-ahead` 32.3 to 24.7 and the skyline blocks left the frame; keeping the circuit's density `0.001` and forcing only the fog
*colour* to black gave the same picture (whole-frame 16.16 against 16.23), so what shows is the lilac colour, and the two readings
differ by 0.07 grey: density 0 is chosen as the law because stage 2 pairs an orange colour with density 0 (above), which a
borrowed density would turn into a haze. Ambient to zero (`amb0` knob, reverted) darkened the plain track surfaces and left
the cell unmoved, so the ambient `1 1 1 0` is not what lights the upper wall. Whole-frame, before to after, page scorer, same boots
and flags as `det-regrab`:

| Pair | Mean grey | RMSE | Upper-middle cell (MAD, signed ours minus original) |
| --- | --- | --- | --- |
| `pad` | 10.48 to 8.81 | 0.137 to 0.130 | 31.0 (+12.3) to 21.7 (+2.3) |
| `mines-ahead` | 18.53 to 16.23 | 0.195 to 0.187 | 32.3 (+13.4) to 24.7 (+4.2) |
| `cannon-volley` | 17.42 to 14.88 | 0.187 to 0.178 | 33.5 (+13.6) to 25.7 (+4.3) |

Controls, pixel-identical (0 differing pixels): `talons`, `tech-de-ra`, `zone-vineta-k-z5` (Zone stage 5 authors density `0.003`).
**Zone's own stages 0, 13 and 14 author density 0 as well** (`zonemode.effectsettings`, black colour; DLC3 table stage 0 and 13), so
a Zone race on those stages now draws no circuit fog where it drew the circuit's: no original frame of those stages is on the
compare page (the one Zone pair is stage 5), so that move is by the data and unchecked against a capture.

**The bright ribbed slab is a holographic billboard (closed 2026-11-26, `det-dark-scenery`, confidence 80).** Talon's slot 435, `adverts/scanlinebillboard.rcsmaterial`, found by `OAG_ONLY_SLOT` bisection on `cannon-volley-hot`. Its Zone variants drop the advert and draw the generic Zone surface, which Detonator stage 1's black `Scene.*` colours leave dark; ours returned the advert overlay before the Zone code ran, which is why the per-class debug build found the slab's scene-class fragments black. Law, evidence and pairs: [hd-zone-recolour.md](../../../rendering/hd-zone-recolour.md), "Holographic billboards in a Zone frame" (`cannon-volley-hot` 10.32 to 8.96, `mines-ahead` 12.46 to 10.63, `cannon-volley` 14.33 to 12.37, `pad` 7.60 to 5.88). Still open in that third: the HUD score panel (the original's chevrons carry white stripes ours lacks).

~~**Still open in the upper-middle third:** a bright ribbed slab (a vertical-louvre texture) over the track at the right of the cell,
drawn white-grey in ours and near black in the original, and the HUD score panel (the original's chevrons carry white stripes ours
lacks; the panel box is about 45 % of the cell and its own difference is not a scene term: the cell excluding it reads 23.1 to 10.5
`pad`, 25.3 to 15.0 `mines-ahead`, 24.6 to 14.1 `cannon-volley`). Ruled out by a removed or muted term, one at a time: the circuit's
lilac fog (fixed above), the stage ambient, the sky reflection map, the billboard equaliser bands (`--zone-bands` dropped), the
scene-class zone surface (its lit sum reads black at the slab), the rim-glow classes, the shadow tier, circuit lighting, track shine.
A debug build that replaced the fragment output per class showed the slab's scene-class fragments black while the normal frame is grey
there, so its source is a draw or pass outside the class split and is not found.~~

## The floor's violet: the visualiser, read and fixed (2026-11-23, `det-track-look`)

**Which draws what (settled, 90).** `/data/tex/detonatormodetrack1.gtf` (DATA00, 256x256 DXT5, stage 1's track texture; the
index is the stage) carries the `zoneTexVis` lookup index in its alpha ([zone-shader.md](zone-shader.md), "`zoneTexVis` is a
256-entry lookup"). Mip 0's decoded alpha: **57,136 of 65,536 texels are `164`** (smooth slot `161 + 3`, band 3), 6,144 are `255`
(unlit), and ten values `1, 19, 36, 54, 72, 89, 107, 125, 142, 160` (72 texels each, `60` x512) are bar segments of bands 0 to 15:
2,256 texels, 3.4 %. Stage 0 and stage 2 (`detonatormodetrack0/2.gtf`) have the same shape (`164` on 53,760 and 54,244 texels). So the
**floor is band 3's smooth slot and the dashes are bar segments**: no billboard is involved, which closes the "equaliser billboards or the
visualiser glow" item above. Reproduce:

```sh
python3 scripts/psarc.py cat data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC /data/tex/detonatormodetrack1.gtf > t1.gtf
# GTF: data offset = big-endian u32 at 0x10 (0x80); mip 0 is 65,536 bytes of DXT5 blocks, alpha = 8 bytes per 4x4 block
```

**The two colours (zone-equaliser.md, "The colours", read live, 90).** The lit bar segments carry the cross-faded `EQ colour tint`
(`171 9 255` at stage 1); slot `161 + b` carries `EQ analogue colour tint` (`14 1 55`) times band `b`'s level. Ours wrote the bar tint
into both halves (`mesh_render::zone::vis_pixels` took one tint), so the floor, which is the smooth slot, came out red-violet
(blue over red 1.2 at every spectrum level tried, against the original's 3.0 to 3.5 and the analogue tint's 3.9). Fix: `EqTints { bars,
smooth }`, `ZoneGrade::eq_tints` (cross-faded palette, both words). The floor is spectrum-driven: with the spectrum silent
(`--no-audio`, every published pair) nothing of it draws in ours, before or after.

**Boxes** (mean RGB, 1920x1080; box A `x 250..800, y 780..850`, box B `x 1090..1230, y 780..1000`, on `detonator-talons-mines-ahead`,
floor between the red strips, clear of the hull, HUD and mines; ours re-rendered through the page's own rerender path on a scratch
copy, `--zone-spectrum-test`, the existing ramp `band / 15`):

| | Box A | Box B |
| --- | --- | --- |
| original (`theirs.png`) | `(5.9, 0.0, 18.0)` | `(9.0, 0.0, 31.5)` |
| ours, silent (before and after) | `(0.0, 0.0, 0.0)` | `(0.6, 0.0, 0.0)` |
| ours, ramp, before | `(33.7, 5.0, 41.3)` | `(65.7, 10.8, 80.1)` |
| ours, ramp, **after** | **`(9.5, 0.0, 21.7)`** | **`(19.8, 0.0, 42.9)`** |

The ramp puts band 3 at `0.2`, a texel blue of `55 x 0.2 = 11`; box A renders `21.7` and box B `42.9` in ours, so ours' texel-to-pixel gain is about 2 in A and 3.9 in B (a flat `0.6` after the fix: texel `33`, A `37.9`, B `70.2`: 1.1 and 2.1), not constant across the floor and not shared with the original. **The original's band 3 level is therefore not determined** by these frames (one frame, a position-dependent gain, the other bands unread); the held bands in the capture's RAM are the measurement. The pad and cannon-volley pairs under the ramp move
the same way (their box A `(42.9, 7.1, 52.5)` to `(12.5, 0.0, 27.3)` and `(41.8, 6.9, 51.1)` to `(15.7, 1.2, 29.2)`). Mean grey of the
three Detonator pairs under the ramp: `mines-ahead` 23.89 to 19.61, `pad` 17.41 to 11.79, `cannon-volley` 22.42 to 19.13 (silent: 20.34,
12.18, 20.33, unchanged by the fix). The 18 other HD race and loading pairs re-render **pixel-identical** to the tree before (silent
spectrum), and `zone-vineta-k-z5` is also identical under the ramp (its stage's track texture does not index the smooth slots).

**What is still open.** (1) ~~The spectrum is the original's, not ours~~ (closed 2026-11-23, `det-pair-bands`: the pairs record the held bands, `+0x32f8`, and `--zone-bands` feeds them to ours, `docs/tools/compare-page.md`, "The original's music bands in a Detonator pair"). **New:** fed the original's own band 3 (0.51 to 0.93) ours' floor blue is 2.2 to 3.1 times the original's, and matching it would take a presented band 3 near 0.07; whether the original's floor tracks band 3 is undetermined (the frame is 0 to 3 ticks older than the RAM read, and band 3 was rising on three of five frames); next, grabs where raw is below held or a lag-scanned frame. (2) The Track Fog (`1.8 0.141 0.007`, the orange-red
authored on the track set) was not separated from the red residual in box B (ours red `19.8` against `9.0`). (3) Ours holds bands with
`Hold` (frame-rate decay `0.1`), the original's cadence is read ([zone-visualiser.md](zone-visualiser.md)).

**Cross-title.** 2048: checked, differs - its effect-settings tables author no `EQ` tints, so the lookup stays blank. Omega: not
checkable here - its band source is another library's (`zone-visualiser.md`, "Omega") and its `Environment_UpdateStageBlend` is unread; HD's
tables are what its Detonator grade carries, so the two-tint split applies the day its spectrum is read.

Tests: `oag-mesh` `mesh_render::zone::tests::the_bars_take_the_eq_tint_and_the_smooth_slots_the_analogue_tint` (red with one tint for both
halves), `oag-raceplay` `zone_sky_ground_truth::a_detonator_race_feeds_the_visualiser_its_bar_and_its_analogue_tint` (stage 1's `171 9 255` and
`14 1 55` off the disc).

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

## The pad's paint and the hull's glass (2026-11-24, `det-pad-glass`)

Two materials looked wrong against the `pad` pair (original: grey slab with white walls and red squares, a blue cockpit dome).

**The pad (`Detonator_Pad.rcsmodel`, confidence 85).** Two materials on one chunk: `diffuse_normal_specular_emmissive_alpha` (236 triangles, `ds_weaponup_cs` picture, `ds_weaponup_ne` mask, `0x7611a2d8` = `(0.59568, 0, 0)`) and `emissivecolourchange` (72 triangles, the walls, authored `0x8a3cc4bf` = `(1, 1, 1)`; fog over that colour plus the vertex light, no sampler: `3dc31258` is the fog colour as on every fogged program, `hd-unlit-programs.md`). The `_ne` alpha is exactly the squares (`pads.md`). Ours drew it as a hull: the `_ne` RGB went on as an additive glow (lavender) and the squares were black because `pad_ne` was not run on an object model and the glow index was ORed onto the pad colour's. Fixed: `pad_ne` runs in `material_setup` for that material (the program match still decides) and clears the old index and `ADD_SECOND` first. The walls lie 0.76 below the model origin; at the pad's transform alone they sink under the road. **Chosen, not measured:** each pad matrix is raised by the loaded model's own depth so the slab stands on its plane (the original's loop that binds the model to the transform is unread); the crop agrees to about 1 px. Zone Battle pad (`zone_battle_pad.rcsmodel`) ships the same `_alpha` material and takes the same binding.

**The hull's glass (`emissive_constant`, state `0x39`, 55).** Picture `detonator_glass`, normal map `detonator_n`, alpha scalar `0xf8f3ade0` = 0.55, blend `SrcAlpha`/`One`. The program patches ambient `0x81db67ea`, sun `0x2dba643d` and light direction `0x02df31e5`, and drawn through the stage's near-black rig it contributes nothing. Unlit (`slots::EMISSIVE`, the picture at its own alpha: the 0.55 scalar is not applied, a possible cause of the 20 % shortfall) the dome box reads `(25, 56, 74)` against the original's `(34, 70, 91)`; lit `(6, 0, 6)`. Chosen from the frame: the instructions were read only as far as the shape above. Only the Detonator hull ships the family on HD.

**2048/Omega (wired 2026-11-24):** Omega ships the same pad (both materials, 236 + 72 triangles; its `emissivecolourchange` state is `0x3e`, HD's `0x7c`) and the same glass (`emissive_constant`, state `0x39`, 166 triangles). **Omega's pad program** (`gcn_dump`, `diffuse_normal_specular_emmissive_alpha` block `@0xe04`, 60 instructions, confidence 80): `Constant1 * Texture2.a + constantAmbientColour * Texture1.rgb`, then fog: the `_ne` alpha **unsquared**, the same law as HD, and not the Vita program's squared `Texture2Alpha`. `mesh::rcs::psp2::emissive_add` takes it as the plain family's member for PS4, and for a Vita file that names `Texture2` and no `Texture2Alpha` (2048's Detonator pad; **inherited from Omega**, its own program unread). Before: black squares on Omega and 2048. The pad lift is generic and applies to both. **Omega's glass** is drawn lit and cyan already by Omega's light rig: nothing wired (checked, differs). Only the block that takes no sun was read.

## Cross-title

- **Omega: the grade is wired, the sky and billboards differ** (`det-parity`, 2026-11-23; the grade was
  recorded "differs" before, because Omega's Zone grade was unwired then). Omega's environment loader `FUN_0162a390`
  forks on the circuit path, not on the mode: HD-lineage `Data\Environments\<n>_<name>` take the HD fallback, "the
  Detonator pair when the mode is `0xe`" through `FUN_01623120`
  ([zone-speed-class-ladder.md](../ps4-omega-eu/zone-speed-class-ladder.md), "Which table a race loads", 70).
  Both Detonator tables and the fifteen `Data/tex/detonatorModeTrack<n>.gnf` (65,792 bytes each, no
  `detonatorMode<n>` general set) ship in `data00.psarc`. Wired as `oag_omega::race::DETONATOR`'s
  `fallback_palette` and `fallback_stage_textures` (general slot: the flat white, inherited from HD), held back
  on `Data\Environments2048` through `ZoneFallback::own_model_under` like Zone's: before, the default
  `tech_de_ra` race drew the circuit's blue-grey day colours; now black scenery, red edge strips and a violet
  floor under `--zone-spectrum-test`, as HD's pair reads
  (`detonator_ground_truth::an_omega_detonator_race_runs_fourteen_stages_on_hd_furys_grade`, and
  `...on_a_2048_circuit_keeps_its_own_colours` for the held-back side). Unchanged and still differing:
  `oag_omega` carries `zone_sky: None` (Omega's Zone sky source is unread in its `eboot.bin`,
  [zone-sky.md](zone-sky.md), "2048 and Omega") and the Detonator billboard share stays off
  (`detonator_shares_one_texture: false`). The primary branch (an `Environments2048` circuit's own
  `ZoneMode2048` table in Detonator) is unread there: **checked, differs, not wired**.
- **2048: the per-stage textures are wired on the fallback** (`det-parity`, 2026-11-23). `Environment_LoadHDFuryContent`
  loads `data/Tex/DetonatorModeTrack0..14.gxt` in the branch HD's ported circuits take
  ([zone-environment-fallback.md](../vita-2048-eu-v104/zone-environment-fallback.md), "`Environment_LoadHDFuryContent`", 75),
  so `oag_2048::race::DETONATOR.fallback_stage_textures` names `data/Tex/detonatorMode` (the base package's
  general set, 349,568 bytes) and `data/Tex/detonatorModeTrack` (the base ships it too; the v1.04 patch's is 262,208 bytes) as
  `.gxt`, 15 stages, for those circuits alone: a circuit with an environment tree of its own keeps its own
  materials. Test: `a_2048_detonator_race_on_zone_1_takes_the_hd_fury_dlc3_table` (red with the field `None`: no
  `15/15 per-stage Zone` line). Before, the fallback table graded nothing visible (no stage texture, so no
  recolour); on Vineta K now black scenery and red edge strips, the same Detonator look as HD's pair. **Open:** the
  road draws black on Vineta K in Detonator under `--zone-spectrum-test`, where the same flags in Zone mode draw
  its blue spectrum pattern on the road (shot, same circuit, tick 700): the cause is not separated. The thread notes a black
  floor on `zone_1`..`4` in every mode. A candidate is the Track/Scene bit ([zone-environment-fallback.md](../vita-2048-eu-v104/zone-environment-fallback.md),
  "Open: the Track/Scene bit": every chunk draws from the Scene group, and Detonator's stage 1 authors its Scene colours
  `0`), but Zone's pattern on the same chunks shows that the group alone does not explain it; the other is the EQ tint lookup on
  2048's path. No 2048 Detonator frame of the original exists to compare against. The sky: not checkable here; its Detonator reaches `<circuit>\ZoneMode2048.effectSettings` or HD's Detonator
  pair through `Environment_Load`'s Zone-family gate ([detonator-mode.md](detonator-mode.md)); whether its sky draw
  is gated by the same byte is unread (its Zone sky draw is unread, [zone-sky.md](zone-sky.md)), and there is no
  Detonator frame pair of 2048 to read it from.

## Track fog and the glass floor (2026-11-24, `det-mines-ahead-structure`)

**The question.** On `detonator-talons-mines-ahead` ours drew grey-green road panels and unfogged walls at the far curve. The
original does not lack them: brightened, its far curve shows the same panels and arcs, dark, under an orange-red fog. So the
difference was shading, not a node the original hides (nothing in the mode `0xe` loops, a stage set or the PVS removes it).

**Two causes, both the original's own rule.**

1. **A track chunk takes the stage's Track fog.** The fog pick at the head of the draw loop reads the chunk's flag bit 0 (the
   Track bit, `oag_rcs::rcsmodel::Mesh::is_track`) in a Zone-family frame and binds the Track buffer
   (`Scene_GetZoneTrackFog`; live `fogColour` values on [zone-plain-chunks.md](zone-plain-chunks.md), "The fog pick",
   confidence 80). Detonator stage 1 authors `Lighting.Track Fog colour` `1.8 0.141176 0.007059` and `density` `0.0015`, and
   its primary `Fog density` is `0`, so a scene chunk is unfogged and a track chunk (road, walls) orange-red. Ours fogged every
   chunk with the primary pair. Now `oag_mesh::mesh_render::TrackFog` (`Scene::track_fog`) carries the stage's pair and
   `fog.wesl`'s `fogged` takes it on `slots::ZONE_TRACK` chunks, `ZoneGrade::track_fog` reading it from the palette
   (`StagePalette::track_fog_colour`/`track_fog_density`). The pick is Title data (`oag_title::ZoneFrame::track_fog`, HD only): 2048 and Omega read the same
   HD tables for their HD-lineage circuits but mark a chunk track by a shape-name rule (`psp2::is_track_shape`) with no measured fog pick, so
   they keep the primary fog until measured.
2. **The glass floor's Zone variants run no facing-ramp sheen.** The grey-green panels are `etched_glass_tech`
   (slot 417, flags `0x1`, Track). The material ships ten Zone variants (fragment blocks 10 to 19, those declaring
   `zoneColourTint`); none samples the facing ramp `0x94b2b285` that the sheen combine
   ([rcsmaterial.md](../../../formats/rcsmaterial.md), "Talon's Junction's missing floor is a glass floor") is built on, and they
   sample the Zone textures (`0xd5e000d1`, `0x1f6f85a3`) instead. A lightmapped glass chunk (Tech de Ra, the Amphiseum) keeps the grid as its coverage in Zone; only the sheen's colour is dropped. The census covers every material
   with a ramp-sampling variant and Zone variants (`no_zone_variant_of_a_facing_ramp_material_samples_the_ramp`: `etched_glass_tech`, `etched_glass_amph`, `etched_glass`). Ours ran the sheen over the Zone surface. `shade.wesl`'s
   `ramp_sheen` is now off with `zone_program`, as the magstrip wave is. Confidence 80 (census over the material's blocks:
   `hd_zone_wave_census_ground_truth::no_zone_variant_of_a_facing_ramp_material_samples_the_ramp`). **Since 2026-11-25** the
   vertex-lit glass keeps its grid coverage in Zone, as every non-redraw Zone block writes it; the paraboloid reflection is read
   and not drawn: the tint is read live as `(1, 1, 1, 0)` with weight 1, but drawing it with those values moves the mines-ahead glass box
   9.23 to 20.55 ([rcsmaterial.md](../../../formats/rcsmaterial.md), "The Zone glass program").

**Numbers** (`mines-ahead`, the pair's own mean grey, lower is better): whole frame 14.65 to 13.74; far-curve box
`x1000..1500 y250..520` mean absolute grey difference against the original 29.1 to 19.4 (whole frame by the same measure 15.5
to 14.6). `pad` 8.17 to 7.68, `cannon-volley` 15.00 to 14.18. Track Fog alone took `mines-ahead` to 14.41; the glass gate the
rest. `talons` (a Time Trial race, no Zone grade) re-renders at 3.32 to 3.31 on the recorded value (no Zone stage, so neither
change reaches it: `track_fog.authored` is `0` and `zone_program` is `0`). `zone-vineta-k-z5` 6.55 to 6.28 (a Zone race: the
same two rules apply there).

**The "8 low" was the mine, not the glass (2026-11-25, `det-glass-box-colour`, confidence 90).** The box `x1000..1500 y250..520`
holds a mine at its bottom-left (`x1000..1130, y400..520`, 14 % of the box): the original draws it pale violet and lit, ours a
black sphere with a ring (`det-colour-re`'s mine colour bind, below). Per channel, mean RGB, ours against the original:

| Region | Ours | Original |
| --- | --- | --- |
| whole box | `(47.4, 21.6, 23.8)` | `(53.1, 29.0, 31.4)` |
| box without the mine | `(44.4, 21.8, 22.3)` | `(43.8, 21.4, 20.5)` |
| the same, top 90 rows (buildings) also dropped | `(63.8, 28.0, 29.1)` | `(67.2, 31.8, 30.6)` |
| orange road pixels (`R > 120`) | `(168.5, 113.0, 112.7)` | `(181.0, 115.5, 115.3)` |

Without the mine the box is within 0.6, 0.4 and 1.8 grey of the original per channel (within 3.8 on the panels alone), so no term
of the glass floor is short: not the Track Fog colour or density, not the grade's ambient, not the glass base. The strip's
sRGB decode ([zone-vis-gamma.md](zone-vis-gamma.md)) is not needed to close this box, so it stays unwired (not run against it). No code changed. A mine inside the box is what any later colour read of it must mask.

**Still open.** The far curve's panels are dim in the original and in ours now; the mid-distance red is closer. Which of `Fog`, `Alt Fog` (flag bit 5) and
`Track Fog` a given draw takes is read for the track bit only. **2048 and Omega:** checked, applies, not wired (gated off by Title data above): 2048's
tables author `Fog.Track Fog Colour` (density in the fourth lane, `key::TRACK_FOG_COLOUR_2048`) and ship the same
`etched_glass_tech`-style materials in the HD lineage, but their per-chunk Track bit is the chosen shape-name rule and no 2048
or Omega Detonator frame of the original exists to measure against.

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
| The floor is band 3's smooth slot (analogue tint), the dashes are bar segments (EQ tint) | alpha histogram of `detonatormodetrack1.gtf`, `zone-equaliser.md` "The colours", boxes above | 90 |

## Reproduce

```sh
ISO=data/images/hdfury-ps3-eu-dec.iso
python3 scripts/psarc.py cat $ISO:PS3_GAME/USRDIR/DATA00.PSARC /data/environments/detonatormode.effectsettings \
  | grep '"1 Sub Venom\.'
ELF=data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf
llvm-objdump -d --mcpu=pwr6 --start-address=0x3f4030 --stop-address=0x3f4170 $ELF   # the mask
llvm-objdump -d --mcpu=pwr6 --start-address=0x3ad938 --stop-address=0x3ad970 $ELF   # bloom arms
```
