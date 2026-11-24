# Pure's weather: the screen mist and the rain and snow it was never playing

Lane `pure-race-worst-two`, 2026-11-24. Wired on Pure's PSP disc by the `circuit_weather` rule
(`oag_pure::TITLE.looks`); Pulse's law, run on Pure's own circuit data. Confidence is per claim, rubric as in
[`confidence-rubric.md`](../reverse-engineering/confidence-rubric.md).

## What the pairs showed

Of the fourteen Pure race pairs in `data/reference/compare-page/pure-psp/`, two sat far above the rest:
`sinucit` (mean grey against the original 26.96) and `sebenco-climb` (25.88); ten of the other twelve read 6 to 14 (the
two Zone pairs read 52 and 64 for their own reasons). Their whole frame was darker than the original's by a near-constant 23 grey, in the sky, the road, the
walls and the hull alike. A regression of the original's grey on ours over both frames read
`theirs = 0.87 to 0.95 * ours + 28 to 39` per channel; the offset was flat from the shadows to the mid-greys and spatially uniform (a
blurred signed difference with a 5th to 95th percentile spread of 6 to 42, no structure). A flat additive lift over the
whole frame is a screen-space additive layer, not a lighting term. It is not the sun lens flare that explained the same
shape on Vineta K and Chenghou (`docs/rendering/lens-flare.md`): Sinucit authors no `lensflare` node, and Sebenco's is
already wired and drawn in these frames.

## The cause

Both circuits author a `<Weather>` element in `TrackStartup.xml`; none of the other eleven circuits the disc names
(Vineta K, Modesto Heights, Chenghou Project, Blue Ridge, Citta Nuova V, Sol 2, Zone 1, the four Classics) does
(`crates/raceplay/tests/pure_weather_ground_truth.rs`, `only_sinucit_and_sebenco_climb_author_weather`):

| Circuit | `EnvPsys` | `ScreenPsys` | `Tex` | `Alpha` | `DisplayScale` | `TexScale` | `MistInside` |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| Sinucit (`08_Sinucit`) | `WO_RAIN` | `WO_RAIN_LENS` | `Data\Tex\ScreenFX\Mist.mip` | 0.2 | 0.8 | 3.1 | 0 |
| Sebenco Climb (`10_Sebenco_Climb`) | `WO_SNOW` | none | `Data\Tex\ScreenFX\Mist.mip` | 0.3 | 0.8 | 3.1 | 1 |

`Mist.mip` (128 x 64), `WO_RAIN`, `WO_RAIN_LENS` and `WO_SNOW` are in Pure's `Data.wad`
(hashes `9121871f`, `f100dfd9`, `106ae186`). Pure's executable carries the element's attribute names (`ScreenPsys`
`0x08a47800`, `DriftMistMult` `0x08a47814`, and `EnvPsys`, `MistInside`, `WindBase`, `WindRange`, `DisplayScale`,
`TexScale`, `AspectRatio`) and the two source names `FxWeather.cpp` (`0x08a7acd0`) and `FxScreenMist.cpp`
(`0x08a7af80`): the same pair of classes Pulse's `Weather_Construct` and `WeatherMist_Construct` are
([`psp-pulse-usa/weather.md`](../ghidra/functions/psp-pulse-usa/weather.md)). Confidence **85** that Pure's engine
parses and plays this element.

Our loader played it on Pulse only: `Looks::circuit_effects` bundled the placed particle nodes, the destroy camera's
stations and the weather, and Pure leaves that rule unread. So Pure's race loaded none of it: no mist layers, no
rain, no snow, no droplets on the lens. The two scenes' haze is the mist: two additive layers of `Mist.mip` over the
whole screen at `Alpha` (0.2 and 0.3), which is a lift of a few tens of grey on a dark frame.

## The fix

One new rule, `Looks::circuit_weather` (`oag_title::effects`), split out of `circuit_effects` so a title can play
its weather without also switching on its placed nodes and camera stations, which Pure has not been measured on.
Pulse sets it exactly as it set `circuit_effects` (PSP and PS2, measured). Pure sets it on the PSP, **inherited from
Pulse** (`Origin::InheritedFrom("Wipeout Pulse")`): the loader's `place_weather` runs unchanged on Pure's
`TrackStartup.xml`, the mist, the lens droplets and the rain or snow emitter all through the existing
`oag_raceplay::scenery_fx::weather`, `oag_fx::mist` and `oag_fx::psys` code. Pure's `title.weather` stays `Unplayed`: the WEATHER setting row (`original` or `enhanced`) is not offered on Pure and a
track module's `set-weather` does not borrow Sinucit's `<Weather>`, until the enhanced weather has been looked at on Pure.
Flipping it later is one line in `crates/pure/src/lib.rs` (`Played { rain_circuit: Some(Sinucit) }`) plus the three menu
and settings test expectations that name Pure.

## Result

Mean grey (480 x 272, against the original), the pair as stored against re-rendered by this tree, `3D` being the
frame with the HUD boxes masked out; signed is the original minus ours (positive: ours darker):

| Pair | Frame mean | Frame `>40` px | 3D mean | 3D `>40` px | 3D signed |
| --- | ---: | ---: | ---: | ---: | ---: |
| `sinucit` | 26.81 -> 15.64 | 14,698 -> 7,026 | 25.81 -> 14.42 | 8,680 -> 4,355 | +23.8 -> +7.9 |
| `sebenco-climb` | 25.65 -> 15.72 | 15,326 -> 9,137 | 26.09 -> 15.01 | 12,077 -> 5,969 | +23.3 -> +2.9 |

The other twelve Pure pairs, Zone pairs included, re-render pixel-identical. A Pulse (Fort Gale White, Outpost 7 White),
an HD (Talons pack), a 2048 (Sebenco Climb) and an Omega (Metro Park) pair re-render pixel-identical to the stored
frames: the rule only reads Pure's and Pulse's tables.

Frames at 500 and 650 ticks (the pair's pose held) show the droplets running on Sinucit's lens and the snow falling on
Sebenco, so it plays, not only the still.

## What is left on these two pairs

- **The mist's pattern is random in the original.** `WeatherMist_UpdateLayer` rolls the layers' UV offsets with
  `Psys_RandFloatRange` on every re-roll (`weather.md`, "The mist overlay"), and ours seeds one fixed table. The frame
  mean is right (signed +7.9 and +2.9 in the 3D region) while the haze sits in different places: on Sebenco's lower left
  ours is now 30 grey brighter than the original there. That is the draw, not the law.
- Sinucit's advert screens above the road (the original shows an animated cyan and yellow picture, ours three cyan
  strips): `trackstartup.xml` names five billboard slots by colour and the loader draws none of them
  (`23 billboard-slot placeholder draw(s) suppressed`). Open; the ranked next region on this pair.
- The blue strip of light across the track at the craft's height is wider and bloomier in ours than the original's thin
  line on both pairs (about 8 to 11 % of each pair's `>40` pixels). Not read.
- The HUD (about a third of the remaining `>40` pixels on Sinucit) is out of this lane: the pilot message line and the
  speed gauge belong to the Pure HUD item.

## Not measured

- Whether Pure's `FxScreenMist` blends additively with Pulse's constants, its layer phases and its `0.02` step. The
  evidence is the two frames: the 3D signed difference falls from +23 to +8 and +3 and the 3D `>40` count roughly halves
  on both, which a wrong blend could not do. Pure's draw function is unread (the `lui`/`addiu` relocation trap on this
  import breaks the string-to-function route, `psp-pure-eu/string-anchors.md`). Confidence **70**.
- The rain and snow emitters' motion on Pure (wind table, `WrapBox`), and the lens: Pulse's law, no Pure frame of them
  paired. The enhanced setting (wet road, splashes, spray) is this project's own and was not looked at on Pure.

## Cross-title

- **Pulse:** the same code, unchanged behaviour (pixel-identical pairs above).
- **HD, 2048, Omega: not checked, not wired.** Their `title.weather` stays `Unplayed` and the new rule is `UNREAD`;
  2048 and Omega ship no `WO_RAIN`, `WO_RAIN_LENS` or `WO_SNOW` entry (`oag_2048`, `oag_omega` effect notes), and HD's
  weather is not read.
