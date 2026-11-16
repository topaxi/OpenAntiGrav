# Omega's hull tone: which term of the lit law the 1.6x lives in

Measured 2026-11-16, lane `omega-hull-tone`, live on the native port (`wipeout_pc`, the 1.07 eboot, image base
`0x800000000`; runtime address = `0x800000000 + address - 0x01000000`) against ours' render of the same pair.
Driver: `scripts/wipeout-pc-hull-tone-live.py`; box means in linear light: `scripts/hull-tone-box-linear.py`.
Companion pages: [engine-light.md](engine-light.md) (the light that was ruled out as the cause),
[sun-intensity.md](sun-intensity.md), [tonemap.md](tonemap.md).

## Summary

| Question | Answer | Confidence |
| --- | --- | --- |
| Is it one term? | **No single term identified.** Narrowed: the excess is in how ours' hull combines its diffuse terms (whole box `2.2, 1.9, 2.4` times, `3.1, 2.4, 2.9` after the K term the original has and ours lacks). Which term carries it is open: per term, only the sun's ratio was measured (`2.6, 2.7`, from knockouts confounded by ours' zone grade) | 55 |
| Is the sun constant different? | **No.** The port's per-draw sun constant is exactly `Sun diffuse colour * Sun diffuse intensity` = `(6.4, 5.7725, 3.4635)` (12 copies in the process); ours multiplies the same intensity (`light.rs`). **The hull's bound ambient constant was not verified**: the 2181 raw-ambient copies may belong to other draws | 90 for the sun, unknown for the ambient |
| Is it `Lighting.Ship diffuse brightness` (key `0x0188d117`, object `+0x164`, Modesto `0.25`)? | **Not as a multiplier, but the hull reads it live.** It is a separate neutral term worth about 20 % of the box on the original at `0.25` (ours reads it nowhere). The hull program as read has no `K` input and no `K * ambient` or `K * sun` constant exists in the process, so `K` reaches the hull through a bound constant ours does not compute; where is open | 90 for the live effect (three boots), 40 for its path |
| Specular? | **No.** Specular power `60 -> 4` and `-> 1` moves the box by less than the frame noise | 85 |
| Reflection (`Ship reflective scale` `1`, chrome `1.5`)? | **No.** Zeroing either or both leaves the box inside the noise | 80 |
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

The sun's input equals ours' (ambient unverified). The hull is `(ambient + sun * N.L [+ vertex light]) * texture * vertex colour
[+ the K term]` on the original's own programs ([engine-light.md](engine-light.md), "Evidence"). The whole box runs `2.4` to `3.1`
times (K removed). Candidates, each with what it predicts:

- **a grey albedo factor** (the vertex colour, the texel): predicts one ratio across channels and across the sun and ambient
  terms. The per-channel spread (`3.1, 2.4, 2.9`) is larger than the per-state noise (about `0.004` in `0.04`, 10 %), so a pure
  grey scalar is **not supported as it stands**: a coloured texel difference or a coloured ambient fits better;
- **the normal's length** (the original does not normalise `N`; a hull world scale below 1 weakens `N.L`): scales the sun term
  alone, so the ambient would have to be right and is unmeasured;
- **the hull's bound ambient constant**: unverified; if it is lower than the raw ambient, the ambient term carries part of it;
- **ours' own grade** (`zone_grade.rs`), which moves with the rig and confounds every ours-side knockout.

Ruled out on the original: the sun constant, `K` as a hull multiplier, specular, reflection, a premultiplied `K * ambient` or
`K * sun`. Not separable here: texel against vertex colour against ours' grade.

## Livery

The stored pair's two craft are not the same part set: the original draws a curved rear arc and two tall rear pylons that ours does
not (`omega/modesto-heights/side.png`). The housing box does not contain them (the arc runs below `y = 800`, the pylons are right of `x = 1100`), and the
panels in it (intake, spine, the two grilles) are drawn by both models, so the box compares like with like; **the texel of the
housing was not compared** (our decoded hull texture against the port's bound one), because the port's bound texture was not read.
The port's patch list carries "Bug Fix - Model slots"; which part set it loaded for this craft was not read either.

## The discriminating reads (not done; no new boot was spent), in this order

1. **Where `K` enters.** On the port, write `K` (`base+12`) to `0` and then `1` and diff the floats in the per-draw constant
   region (where the 12 `sun * 1.6` copies and the raw-ambient copies sit): what changes with `K` is the constant the hull
   binds, and the ambient the hull really uses is the one next to it.
2. **The albedo.** Read the Feisar_c1 housing submesh's vertex colours and albedo texels from the Omega archive and take their
   product's mean over the housing's covered pixels. About `1 / 2.8` of what ours' draw multiplies (ours treats the vertex colour
   as `1`, or reads another texel) says albedo; about `1` says the normal or the ambient, and the next read is the hull world
   matrix's scale in the port (the flare object's `row2` length).

## Method notes

- The lighting object is found by value: the four floats `Sky height offset 0, Sky scale 1, Sky brightness 2.264, Ship diffuse
  brightness 0.25` appear once, at `0x800eb2898` (the image's data, the same on every boot). `+16` is the `(shadow)` weight, `+20`
  reflective scale, `+24` chrome scale, `+28` specular power, `+32` chrome power; `-24` is `Sun color`.
- The same boot's frames drift by a few percent in the box (the craft sways at rest); every state is four frames and the
  per-state spread is in the run's `note.json`, so a difference under about `0.004` is noise.
- Ours' knockouts were made with a local, uncommitted debug variable in `Light::authored` and in the engine-light list; it is not
  in the tree.
- Modesto Heights (`data08.psarc` `data/environments/modesto_heights/track.EnvSettings`), the values the driver scans for:
  `--sky-brightness 2.264 --ship-diffuse 0.25 --ambient 0.309804,0.258824,0.713726 --sun-diffuse 4,3.607843,2.164706` (the sun's
  `Sun diffuse intensity` is `1.6`, its specular colour and `Sun color` `1 0.898039 0.541176`). `--track-steps 8` selects Modesto.
  The port ran in the pair script's own headless sway on seat 2, not on an Xvfb display.
- **2048 cross-check: checked, applies, not wired.** The 2048-lineage circuits Omega re-ships author the same key (Sol `1`,
  Queens Mall `0.5`; the XML tables are byte-identical to 2048's, `docs/formats/omega-status.md`), and `engine-light.md` records the
  2048 hull as the same material family. The Vita executable's reader of the key was not read and nothing is wired.
