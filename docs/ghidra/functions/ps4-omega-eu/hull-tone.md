# Omega's hull tone: which term of the lit law the 1.6x lives in

Measured 2026-11-16, lane `omega-hull-tone`, live on the native port (`wipeout_pc`, the 1.07 eboot, image base
`0x800000000`; runtime address = `0x800000000 + address - 0x01000000`) against ours' render of the same pair.
Driver: `scripts/wipeout-pc-hull-tone-live.py`; box means in linear light: `scripts/hull-tone-box-linear.py`.
Companion pages: [engine-light.md](engine-light.md) (the light that was ruled out as the cause),
[sun-intensity.md](sun-intensity.md), [tonemap.md](tonemap.md).

> **2026-11-16, lane `hull-tone-re`: the two reads were made; see ["The bound constants"](#the-bound-constants-2026-11-16-hull-tone-re)
> at the end.** It supersedes three statements below: the sun constant has thousands of copies, not 12 (the 12 was the
> driver's `[:12]` cap); the hull binds **no** `constantAmbientColour` in a race (its ambient is a probe); and ours' knockouts
> were confounded by the adaptive exposure, not by `zone_grade` (a single race builds no Zone grade).

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

## The bound constants (2026-11-16, `hull-tone-re`)

Lane `hull-tone-re`, the same pair and box, four boots of the native port (1.07). Driver:
`scripts/wipeout-pc-hull-probe-live.py` (frames first, then dumps of the floats around every copy of the sun constant in the
port's guest memory, `/memfd:gow3-guest-memory` below `0x2000000000`; the anonymous heap holds only stale copies).

| Question | Answer | Confidence |
| --- | --- | --- |
| Read 1: where does `K` enter? | **`probeOffsetAndDiffuse.y`** of the hull's pixel constants. The player's live block reads `(0.875, 0.25, 0.25)` as shipped and `(0.875, 0.0, 0.25)`, `(0.875, 1.0, 0.25)`, `(0.875, 4.0, 0.25)` with `K` written; `.z` is `Ship diffuse brightness (shadow)`, `.x` the craft's probe slot over 8 | 92 |
| What is the hull's ambient? | **`(Ks + (K - Ks) * shadow) * max(0.01, SH9(N))`**, a nine-coefficient probe per craft (`shCoeffs`), not `Constant ambient colour`. The race hull draws the `shCoeffs` variants of its material (`0x7c44`, `0xa224` with the vertex light); no hull block carries the authored ambient | 90 |
| Where do the coefficients come from? | **The circuit's `track.shprobes`**, read raw: the live block equals one record's 27 floats to 5 decimals, or a two-record blend along the track | 90 the file and the raw use, 60 the blend law |
| Read 2: is the factor the albedo? | **No.** Ours' and the original's ambient shares agree within the ambient's own difference on the housing and the spine, and the sun-lit fin tops match (ours `0.85` times the original). Not `1 / 2.8` | 65 |
| Is it the normal's length? | **No, statically.** The pixel program normalises the normal-mapped `N` before the probe and the sun (`0x7c44` `@484`-`@4a4`); only the vertex light uses the unnormalised vertex normal | 88 |
| Which term carries ours' `1.9` to `2.4` times? | **Ours' sun term on the dark body panels**: `(0.078, 0.076, 0.043)` of ours' `(0.101, 0.097, 0.105)` box against about `(0.025, 0.022, 0.009)` on the original. The constant ambient adds the blue (`0.052` against `0.012`). Why ours' sun term is three times the original's there is **not identified** | 75 that the excess is the sun term, no cause named |
| A term ours lacks | The reflection: `albedo.a * (6.87 * (1 - N.V)^9.23 + Chromeness + 0.1) * cubeReflectionTex`, about `(0.013, 0.016, 0.017)` of the original's box (sun and both `K` zeroed); ours' hull is black with its ambient and sun off | 70 the term, 50 its share |

### The program (static, `gcn_dump ... diffuse_with_specular_from_alpha_n_vcol.rcsmaterial --at 0x7c44`)

The material's pixel programs come in two families. `0x2064` / `0x3104` (the ones [engine-light.md](engine-light.md) read) declare
`constantAmbientColour`. `0x5ac4`, `0x6764`, `0x8c04` declare `Chromeness, shCoeffs, probeOffsetAndDiffuse` and two cube
samplers instead, and `0x7c44` / `0xa224` add `shadowCoeffs` and `shadowMapSampler`. `0x7c44`'s constant block (304 bytes):

```text
dw0  Chromeness            dw4  shadowCoeffs (4x4)     dw20 shCoeffs (28 floats: 9 x rgb, one pad)
dw48 probeOffsetAndDiffuse dw52 liveLighting0specular  dw56 liveLighting0diffuse   dw60 liveLighting0direction
dw64 reflectiveBlurCoeffs  dw68 fogColour              dw72 fadeResCoeffs
```

The law, with `n` the normalised normal-mapped normal and `sh[k]` the k-th rgb triple:

```text
shadow  = (1 / 9) * nine compared taps of shadowMap                    @2e4-@30c, @3fc  (0x3de38e39)
kmix    = probe.z + (probe.y - probe.z) * shadow                       @320, @330, @440
SH(n)   = sh[0] - sh[1] n.z + sh[2] n.y + sh[3] n.x - sh[4] n.z n.x - sh[5] n.z n.y
          + sh[6] (2 n.y^2 - n.x^2 - n.z^2) + sh[7] n.x n.y + sh[8] (n.x^2 - n.z^2)      @494-@560
light   = kmix * max(0.01, SH(n)) + liveLighting0diffuse * sat(n . dir) * shadow         @554-@5c4, @588-@5f8
colour  = albedo.rgb * vertexColour.rgb * light                                          @76c-@788
        + liveLighting0specular * albedo.a * shadow * 0.0698 * sat(r . dir)^p / max(0.0698, 0.1745 g)
        + albedo.a * (6.87 * (1 - n.v)^9.23 + Chromeness + 0.1) * cubeReflectionTex      @634-@674, @74c, @78c, @824
          p = 102.76 - 77.07 g,  g = min(1, blur.z + 0.7 * blur.w)
then the fog lerp
```

`0x5ac4` is the same without the shadow (`kmix = probe.y`). The axis order was read off the sun's dot product
(`dir.x * v13 + dir.y * v17 + dir.z * v15`). Confidence 88 (decompilation of one variant, a second variant agreeing, and the
live constants below matching its layout).

### Read 1, live

Every hull block in guest memory has the layout above: the sun constant `(6.4, 5.7725, 3.4635)` with `(20, 18.04, 10.82)`
(`Sun specular colour * Sun specular intensity`) four floats before it, `probeOffsetAndDiffuse` eight before, 28 probe floats
before that, and the shadow matrix's translation before those. The eight craft carry `probe.x` `0, 0.125, ... 0.875`; the
player (8th on the grid) is `0.875`. Its `probe.y` follows the write to `base+12` (`0`, `1`, `4`), `probe.z` stays at the
`(shadow)` field's `0.25`. Blocks of the other craft and blocks in the process's anonymous heap do not move: they are stale
copies, which is why the first scan (heap only) found the layout but no change. Blocks of a second program
(`sun` directly after the probe, no specular: `carbonfibre` or `diffuse_vcol`) carry the same triple.

The sun constant is not 12 copies: a scan finds about 3,500 in the heap and 9,000 in guest memory.

### The probe file

`track.shprobes`, beside the circuit's `track.vex`. Loader: `0x013c102c`-`0x013c11c7` in the 1.07 track load
(`FUN_013bc180`), path format `%s\%strack%s.shprobes` (`0x01890082`).

```text
u32 magic   'SHPB' (0x53485042; the bytes read "BPHS")
u32 count   refused unless a multiple of 3
count x { f32 position[3]; f32 sh[27]; }     0x78 bytes each, sh in the shader's own order
```

Modesto Heights: 1,105,928 bytes, 9,216 records, every position distinct; consecutive records are three points across the
track 18.6 apart, and consecutive triples step 1.5 along it. Present for Modesto Heights, The Amphiseum, Tower, Mall and Sol
(the five tried that resolved; a full census is open). **The earlier census that found only two (`hd-probes.md`) was wrong for
the HD circuits.**

The manager is `g_sh_probes` (`0x020084e8`, confidence 80): `+0x20` the records, `+0x28` their count, `+0x2c` a byte of eight
slot bits (a craft's model instance takes one in `FUN_016265a0` and frees it in `FUN_01627740`), `+0x30 + slot * 0x70` the
slot's 28 floats, which four draw paths copy into the constant block (`0x0127abc6`, `0x01619db8`, `0x015ff8f9`, `0x0160f31e`),
`+0x3b0 + slot * 0x10` the craft's position (`0x015f3bb7`). `Scene_PrepareFrame` (`0x015f434f` on) looks each used slot's
position up through `FUN_0161f620` (hypothesis, 50: the enclosing triangle of the probe strip) and writes the blend.

Live against the file (`track.shprobes` of Modesto, the player's blocks in two boots): the block equals record 1385
(position `(-283.87, 195.44, -285.99)`), 1388 or 1382 exactly, or `record 1388 + 0.076 * (record 1391 - record 1388)`
(also `1379` and `1382` at `0.131`) with no residual at 5 decimals: a blend of the same lane's records in adjacent
triples. A third record's weight was never seen (the craft sat on the lane); the lateral law is unread.

### The decomposition (housing box, post-tonemap linear, engine light off)

Original, boot 4 (frames taken within 6 s, baseline and restored equal):

| State | Box |
| --- | --- |
| as shipped / restored | `(0.047, 0.051, 0.044)` / `(0.047, 0.054, 0.046)` |
| `K = 1` | `(0.077, 0.087, 0.082)` |
| `K = 0, Ks = 0` (no ambient) | `(0.037, 0.040, 0.033)` |
| live sun colour zeroed | `(0.021, 0.029, 0.036)` |
| sun, `K`, `Ks` zeroed (the reflection) | `(0.013, 0.016, 0.017)` |

So about: ambient `(0.010, 0.011, 0.012)`, sun `(0.025, 0.022, 0.009)`, reflection `(0.013, 0.016, 0.017)`. The `K = 0`
states drift across their four frames (the housing reads `0.023` to `0.040`), so a share is good to about 25 %; the sun-zero
states also move the adaptive exposure ([tonemap.md](tonemap.md)).

Ours, with a hull-only scale on its ambient and on its sun term (a scratch shader edit, not in the tree; a floor box stays
within 4 %, so the exposure does not move):

| State | Housing | Left fin top `(800,640)-(880,700)` | Spine `(930,600)-(990,680)` |
| --- | --- | --- | --- |
| ours | `(0.101, 0.097, 0.105)` | `(0.144, 0.170, 0.145)` | `(0.161, 0.153, 0.156)` |
| ours, sun only | `(0.078, 0.076, 0.043)` | `(0.134, 0.161, 0.122)` | `(0.128, 0.123, 0.070)` |
| ours, ambient only | `(0.016, 0.015, 0.052)` | `(0.019, 0.027, 0.051)` | `(0.020, 0.018, 0.064)` |
| ours, neither | `(0, 0, 0)` | | |
| original | `(0.047, 0.051, 0.044)` | `(0.168, 0.190, 0.165)` | `(0.071, 0.072, 0.051)` |
| original, sun (shipped minus sun zeroed) | `(0.025, 0.022, 0.008)` | `(0.111, 0.123, 0.080)` | `(0.048, 0.046, 0.024)` |
| original, ambient (shipped minus `K = Ks = 0`) | `(0.010, 0.011, 0.012)` | `(0.019, 0.021, 0.022)` | `(0.023, 0.026, 0.023)` |

Reading:

1. **The fin tops match** (ours `0.85` times, its sun term `1.2` to `1.5` times the original's exposure-inflated one): same
   texel, same sun, same `N.L` there.
2. **The ambient shares run `0.7` to `1.6` times in red and green and `2.3` to `4.5` times in blue**: what the constants
   predict (`0.25 * SH` is about `(0.25, 0.31, 0.37)` against ours' `(0.31, 0.26, 0.71)`), so the albedo ours multiplies is the
   original's to within the read's error. Read 2's `1 / 2.8` is not there.
3. **Ours' sun term on the spine and the housing is `2.7` to `3.5` times the original's**, and that is the whole red and green
   excess (`+0.053` of `+0.054` on the housing). Ours' own hull shadow is live on Omega and takes only `8 %` off the box
   (bypassing `sun_occlusion` gives `(0.110, 0.108, 0.112)`), so it is not a missing shadow pass. What is left for those
   panels: the original's shadow term being darker there than ours' (its nine-tap map against ours' four), the normal map's
   `N.L` (whether ours binds the hull's `_n` picture on Omega was not checked), or the second hull program's texel. Not
   separated.

### What a wiring of the reads gives, and what it does not

Replacing ours' constant ambient on the hull with `kmix * max(0.01, SH(n))` moves ours' ambient share from
`(0.016, 0.015, 0.052)` to about the original's `(0.010, 0.011, 0.012)`: the box becomes about `(0.095, 0.093, 0.065)`,
`2.0, 1.8, 1.5` times the original. It removes the blue cast and a tenth of the excess. Adding the reflection term raises
the box again. **The acceptance (within 15 %) is not reachable from the two reads**; it waits on the sun term.

### Next read, for the sun term

On the port, with the live sun colour zeroed and `K = 4`, `Ks = 0` then `K = 0`, `Ks = 4` (each alone on a black sun, so the
box is `shadow * SH * albedo` and `(1 - shadow) * SH * albedo`): their ratio is the original's `shadow` per pixel, free of
`N.L`. Wait two seconds after each write and grab eight frames (the `K = 0` states drift). The same pixel's ours-side
`sun_occlusion` is ours' total over ours' total with the occlusion bypassed. If the two shadows agree on the spine, the
factor is `N.L`: dump ours' `n . dir` for that panel and compare with the original's sun share over its ambient share times
`kmix * SH / sun`.

### Cross-title

- **2048: checked, applies in part, not wired.** The Vita 1.04 executable registers `Lighting.Ship diffuse brightness`
  (`0x815088e4`, in `Environment_RegisterLightingSchema`, `0x81017b80`) and names `probeOffsetAndDiffuse` (`0x815094e0`), so
  the weight exists there. It has no `shCoeffs` name and no `(shadow)` key, and its probe file is `track.probes` (`0x81422ba8`,
  1,112-byte records), not `.shprobes`: what `K` scales on the Vita is unread. Next address: the references to `0x815094e0`
  and the GXP hull program's use of the constant.
- **HD**: its hull ambient is `4 * irradiance` from `track.probes` and is wired (`docs/rendering/hd-ship-materials.md`,
  "Finding 5"); it has no `K`.
