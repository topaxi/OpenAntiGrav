# Omega's hull tone: which term of the lit law the 1.6x lives in

Measured 2026-11-16, lane `omega-hull-tone`, live on the native port (`wipeout_pc`, the 1.07 eboot, image base
`0x800000000`; runtime address = `0x800000000 + address - 0x01000000`) against ours' render of the same pair.
Driver: `scripts/wipeout-pc-hull-tone-live.py`; box means in linear light: `scripts/hull-tone-box-linear.py`.
Companion pages: [engine-light.md](engine-light.md) (the light that was ruled out as the cause),
[sun-intensity.md](sun-intensity.md), [tonemap.md](tonemap.md).

## Summary

| Question | Answer | Confidence |
| --- | --- | --- |
| Is it one term? | **No single term identified.** Narrowed: the excess sits in the factor ours' ambient and sun terms **share** (the hull's albedo: texel times vertex colour, or the normal's length and direction), not in a constant | 55 |
| Are the light constants different? | **No.** The port's per-draw sun constant is exactly `Sun diffuse colour * Sun diffuse intensity` = `(6.4, 5.7725, 3.4635)` (12 copies read in the process); ours multiplies the same intensity (`light.rs`) | 90 |
| Is it `Lighting.Ship diffuse brightness` (key `0x0188d117`, object `+0x164`, Modesto `0.25`)? | **No, and it is live-read by the hull.** It is a separate term that ours lacks, about 20 % of the box on the original at `0.25` (slope about `+0.03` linear per unit). Ours reading it would raise ours, not lower it | 90 (three boots) |
| Specular? | **No.** Specular power `60 -> 4` and `-> 1` moves the box by less than the frame noise | 85 |
| Reflection (`Ship reflective scale` `1`, chrome `1.5`)? | **No.** Zeroing either or both leaves the box inside the noise | 80 |
| Premultiplied ambient (`ambient * K`)? | **No.** The process holds no `0.25 * ambient` triple; it does hold `sun * 1.6` and no `sun * 1.6 * 0.25` | 85 |
| Post (exposure, tonemap)? | **Not the cause of the hull's excess:** the same frames' other panels run `1.2` to `1.4` times in linear light, the hull's housing `1.8` to `2.3` | 70 |
| Livery / texel? | **Not compared** (see "Livery"); the housing box sits on panels both models draw | n/a |

## The measurement

`omega/modesto-heights`, Feisar_c1 Fury, engine light off on the original (the producer at `0x0173e350` returns at
once; ours: the engine light's records dropped), housing box `(890, 690)-(1010, 800)` of the 1920x1080 frame.
Means are of **linearised** pixels (sRGB decoded per pixel, then averaged), channels R, G, B.

Original, 4 frames per state, three boots (stored pair's `theirs.png` box, same state: `(0.045, 0.057, 0.049)`):

| State written into the live lighting object (`base = 0x800eb2898`) | Box (linear) | Boot |
| --- | --- | --- |
| as shipped | `(0.048, 0.054, 0.045)`, `(0.0452, 0.0508, 0.0442)`, `(0.048, 0.055, 0.047)` | 1, 2, 3 |
| `Ship diffuse brightness` (`base+12`) `0` | `(0.043, 0.046, 0.035)`, `(0.033, 0.040, 0.036)` | 2, 3 |
| `1.0` | `(0.065, 0.078, 0.078)`, `(0.072, 0.083, 0.080)` | 2, 3 |
| `4.0` | `(0.151, 0.181, 0.196)` | 2 |
| `(shadow)` variant (`base+16`) `0` / `1` / `4` | `(0.046, 0.051, 0.041)` / `(0.058, 0.064, 0.055)` / `(0.085, 0.103, 0.104)` | 3 |
| specular power (`base+28`) `60 -> 4` / `-> 1` | `(0.044, 0.051, 0.045)` / `(0.050, 0.055, 0.046)` | 3 |
| reflective scale `0` / chrome scale `0` / both | `(0.050, 0.054, 0.045)` / `(0.038, 0.045, 0.043)` / `(0.051, 0.055, 0.045)` | 3 |
| `Sun color` (`base-24`) zeroed | `(0.054, 0.057, 0.044)` | 3 |
| the live sun colour triple (`0x800eb4810`) zeroed | `(0.023, 0.027, 0.031)`, `(0.022, 0.029, 0.035)` | 2, 3 |
| the settings object's ambient (`0x800eb4800`) zeroed or doubled | no move | 2 |

Reading, term by term:

1. **The sun triple at `0x800eb4810` is the sun's live colour for the whole scene** (zeroing it also darkens the track and
   walls; the delta image is scene-wide, so it is not a hull specular term). The housing keeps `0.022` to `0.035` without it:
   the sun's share of the original box is about `0.023, 0.022, 0.010` (R, G, B), blue-poor as the sun is. The sibling triple at
   `0x800eb47c0` (the same values on this circuit) has no live effect.
2. **`Ship diffuse brightness` is a live weight on a neutral term** of about `0.012, 0.011, 0.009` at `0.25` (box minus the
   `K = 0` state), linear in `K` (`0 -> 1`: `+0.022` to `+0.039`; `1 -> 4`: about `+0.026` per unit). It is not a scale on the whole
   hull: at `K = 0` the box keeps 75 to 90 % of its baseline. The `(shadow)` field (`base+16`) has a slope of about `0.0096` per
   unit, a quarter of `base+12`'s: it is not the bay's weight.
3. What remains with the sun and `K` removed is a blue-tinted, K-independent part, about `(0.010, 0.018, 0.026)`: the ambient
   term times the hull's albedo. The settings object's ambient is not what the shader reads (zeroing it changes nothing); the
   per-draw copies of the raw ambient exist (2181 hits) and the premultiplied `0.25 * ambient` does not.

Ours (`eng` off, same pose, tick 100): box `(0.101, 0.097, 0.105)` (sRGB `(65.5, 63.2, 72.7)`).

| | Original | Ours | Ratio |
| --- | --- | --- | --- |
| whole box | `(0.045, 0.051, 0.044)` | `(0.101, 0.097, 0.105)` | `2.2, 1.9, 2.4` |
| whole box minus the original's `K` term (ours has none) | `(0.033, 0.040, 0.036)` | same | `3.1, 2.4, 2.9` |
| sun term (ours: box minus the box with the sun off) | `(0.023, 0.022, 0.010)` | `(0.059, 0.059, -)` | `2.6, 2.7, -` |

**Ours' per-term shares are confounded** and carry no confidence: `raceplay/src/zone_grade.rs` builds its graded rig through
`Light::authored` as well, so scaling the ambient or the sun in the rig also moves the grade. The blue channel shows it (ambient
only `0.145` is above ambient plus sun `0.105`). The constants themselves agree (finding 1 of the summary), so what is left is
how the hull draw combines them.

## What this narrows

The original's inputs (ambient, `sun * 1.6`) equal ours'. The hull is `(ambient + sun * N.L [+ vertex light]) * texture *
vertex colour [+ the K term]` on the original's own programs ([engine-light.md](engine-light.md), "Evidence"). Both of ours'
terms run `2.4` to `3.1` times the original's. A factor common to **both** is what the equation says they share:

- the **albedo**: the hull texel and the **vertex colour** (the material is `diffuse_with_specular_from_alpha_n_vcol`: a baked
  darkening in the recesses would be the vertex colour, and the housing is a recess);
- the **normal**: the original uses the world matrix times the vertex normal unnormalised (the thread's note on `N`; a hull
  scale below 1 weakens the sun and, through `N.L`, only that term).

Ruled out on the original: the constants, `K` as the cause, specular, reflection, a premultiplied ambient. Not separable here: texel
against vertex colour against ours' own grade.

## Livery

The stored pair's two craft are not the same part set: the original draws a curved rear arc and two tall rear pylons that ours does
not (`omega/modesto-heights/side.png`). The housing box does not contain them (the arc runs below `y = 800`, the pylons are right of `x = 1100`), and the
panels in it (intake, spine, the two grilles) are drawn by both models, so the box compares like with like; **the texel of the
housing was not compared** (our decoded hull texture against the port's bound one), because the port's bound texture was not read.
The port's patch list carries "Bug Fix - Model slots"; which part set it loaded for this craft was not read either.

## The discriminating read (not done)

Read the Feisar_c1 housing submesh's vertex colours and albedo texels from the Omega archive and take their product's mean over the
housing's covered pixels. If it is about `1 / 2.8` (`1 / 2.4` to `1 / 3.1` by channel) of the value ours' draw multiplies (ours treats the vertex colour as `1`, or reads a
different texel), the cause is the albedo factor; if it is about `1`, the cause is the normal (length or direction) and the next read
is the hull's world matrix scale in the port (the flare object's `row2` length, or the live draw's constant buffer). One live
read settles the other half: with the port's hull constants at hand, write the housing's vertex colour to `1` (the vertex buffer
of the draw) and see whether the box rises by the factor.

## Method notes

- The lighting object is found by value: the four floats `Sky height offset 0, Sky scale 1, Sky brightness 2.264, Ship diffuse
  brightness 0.25` appear once, at `0x800eb2898` (the image's data, the same on every boot). `+16` is the `(shadow)` weight, `+20`
  reflective scale, `+24` chrome scale, `+28` specular power, `+32` chrome power; `-24` is `Sun color`.
- The same boot's frames drift by a few percent in the box (the craft sways at rest); every state is four frames and the
  per-state spread is in the run's `note.json`, so a difference under about `0.004` is noise.
- Ours' knockouts were made with a local, uncommitted debug variable in `Light::authored` and in the engine-light list; it is not
  in the tree.
- Not checked: 2048 (Vita). Its hull uses the same material family, but its executable was not run in this lane and the live object
  is Omega's.
