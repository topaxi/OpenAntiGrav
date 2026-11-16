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

> **2026-11-16, lane `hull-albedo-re`: the shared factor is the vertex colour; see ["The vertex colour"](#the-vertex-colour-2026-11-16-hull-albedo-re).**
> Both race hull programs multiply `VertexColour1` (baked occlusion, a raw UNORM grey) into `albedo * light`, and ours' PS4
> decoder draws it as white. It supersedes "Read 2: is the factor the albedo? No" below. The housing box alone still runs
> `1.2` to `1.5` times after it (its sun term); the hull as a whole comes within 9 %.

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

### Wired (2026-11-16, lane `hull-sh-ambient`)

The ambient is wired for Omega's race hulls (every craft on the grid, opponents too; wrecks keep the constant ambient).
`oag_vex::shprobes` reads the file (little-endian, `SHPB`, count a multiple of 3, 120-byte records); the census of every
shipped circuit is `omega_lightmap_ground_truth` (34 files: 12 forward, 12 reversed, the ten 2048-lineage; the four Zone
circuits ship none, and none authors a `(shadow)` twin there). `Lighting.Ship diffuse brightness` is authored by all 38
circuit files (0.2 Vineta K, Sol 2; 0.25 Modesto, Metropia, Moa Therma, Chenghou, Talons; 0.35 Tech De Ra; 0.4 Anulpha;
0.5 Ubermall, Sebenco, Amphiseum; 1.0 on the 2048-lineage Altima, Sol, Square and the Zones, 0.25 to 0.6 on its other circuits), and its `(shadow)` twin equals it
wherever present, so the pair is `(K, K)` today. `oag_raceplay::sh_probes` blends per craft (nearest of the three lanes in
the nearer triple, along-track weight by projection, clamped, the way HD's `locate` does it) into a ten-`vec4` row of a
scene-group uniform table (`mesh_render::SH_PROBE_*`, binding 18; a buffer, not a texture, because the scene group already
binds the 16 sampled textures WebGPU guarantees); `shade.wesl`'s `sh_hull_ambient` evaluates the law of the program section
with `shadow` = ours' hull sun occlusion. A circuit with no file, or no `K`, keeps the constant ambient and says so in the
load report. HD, 2048, Pulse and Pure bind no table and draw pixel-identically (`hd/the-amphiseum`, `2048-vita/anulpha-pass`:
no pixel differs from main).

| Pair (ours, engine light off, tick 100) | Housing box `(890,690)-(1010,800)` before | after | Original (stored frame, light on) |
| --- | --- | --- | --- |
| Modesto Heights | `(0.101, 0.097, 0.105)` | `(0.097, 0.102, 0.075)` | `(0.045, 0.051, 0.044)` (light off, live) |
| The Amphiseum | `(0.265, 0.281, 0.338)` | `(0.124, 0.180, 0.191)` | not comparable (another livery) |
| Moa Therma | `(0.115, 0.122, 0.125)` | `(0.021, 0.027, 0.025)` | not comparable (light on, another pose) |

The brief predicted `(0.095, 0.093, 0.065)` for Modesto: the red is within 0.002, the green `+0.009` and the blue `+0.010`
(the shares were good to 25 %). **The blue cast is gone and the hue is the original's**: the spine box `(930,600)-(990,680)`
is `(0.153, 0.157, 0.113)` (blue `0.72` of green) against the original's `(0.071, 0.072, 0.051)` (`0.71`), from `(0.161, 0.153,
0.156)` (`1.02`) before. The floor box `(1140,650)-(1260,800)` moved by `0.0003, 0.0008, 0.0034` (the adaptive exposure), the
track pixels are untouched, and every difference in the frame is on the eight craft. **The axes of the basis are not
discriminated by the box**: the coefficients vary only some 10 to 20 % across directions around a 1.0 to 1.4 DC term, so a swap
of `y` and `z` moves the housing by under the read's noise; the order is the static read's (88). The brightness is unchanged
and still the open sun-term read above; the next paragraph says what the wiring did to its share.

**The ambient share, by the page's own method** (engine light off; the hull's sun term zeroed only on draws that name a row,
so the track keeps its sun: the floor box moves `0.114 -> 0.106` red and by under 2 % in green and blue):

| Housing `(890,690)-(1010,800)` | Constant ambient (before) | SH ambient (after) | Original |
| --- | --- | --- | --- |
| ambient alone (hull sun off) | `(0.016, 0.015, 0.052)` (reproduces the stored decomposition) | `(0.014, 0.020, 0.026)` | `(0.010, 0.011, 0.012)` |
| increment on the whole frame (shipped minus ambient off) | `(0.023, 0.021, 0.063)` | `(0.019, 0.026, 0.032)` | `(0.010, 0.011, 0.012)` (`K = Ks = 0` against shipped) |
| sun only (ambient off) | `(0.078, 0.076, 0.043)` | same | `(0.025, 0.022, 0.008)` |

So the claim's second half is **half true**: the blue cast halves (`0.052 -> 0.026` alone) and the red falls 12 to 18 %, but the
green rises (`+35 %`) and ours' ambient is still `1.4, 1.8, 2.1` times the original's alone and `1.9, 2.3, 2.7` times by the
increment, not the `(0.010, 0.011, 0.012)` the recipe expected. The spine reads the same way (`(0.0335, 0.0301, 0.0861)`
before, `(0.0265, 0.0349, 0.0439)` after as an increment, against the original's `(0.023, 0.026, 0.023)`). **That is the same
order as the sun term's `2.7` to `3.5` times, and both are on the dark body panels**: with the original's own ambient law in
ours, a common factor on those panels (the albedo: texel times vertex colour, or the lit normal) is at least as likely as a
sun-only cause. The earlier "albedo is about equal" (Read 2 above) compared the *constant* ambient's share with a predicted
`0.25 * SH`, which this read replaces; it should be redone with these numbers (confidence 55: the basis' axes are unverified
and move the ambient by 10 to 20 %, and the `K = Ks = 0` states drift 25 %).

## The vertex colour (2026-11-16, `hull-albedo-re`)

Lane `hull-albedo-re`, the same pair, five boots of the native port (1.07), ours at `47f6c99b0` plus scratch knobs that are
not in the tree. Driver: `scripts/wipeout-pc-hull-vcol-live.py`.

| Question | Answer | Confidence |
| --- | --- | --- |
| What do the hull's ambient and sun both multiply that ours does not? | **`VertexColour1.rgb`**, a baked occlusion grey per vertex, raw. Both race hull programs end on `albedo.rgb * VertexColour1.rgb * light`; ours' PS4 decoder (`oag_mesh::mesh::rcs::psp2`) keeps the colour at `1` on every submesh that is not a Vita `NOVAColor` surface | 90 the law (static, four programs and the fetch format), 80 that it is the shared factor (live slopes below) |
| Is the colour decoded (sRGB, squared) before the multiply? | **No.** The vertex programs export the fetched value untouched and the fetch format of declaration type 11 is `R8G8B8A8 UNORM` (`0x00fac00a`) in the engine's own table | 85 |
| Does it close the acceptance? | **No, on the housing box; yes to 9 % over the hull.** Housing `(0.097, 0.102, 0.075)` becomes `(0.068, 0.074, 0.054)`, still `1.54, 1.45, 1.20` times the original's `(0.044, 0.051, 0.045)`. The hull body box `(820,560)-(1100,860)` becomes `(0.069, 0.092, 0.082)` against `(0.064, 0.085, 0.079)` | measured |
| What is left on the housing box? | Its sun term: with the ambient off the box is `(0.056, 0.056, 0.031)` in ours (vertex colour on) against `(0.035, 0.040, 0.032)` in the original, where the original's value still holds the reflection ours lacks. The hull body's is equal (`(0.061, 0.080, 0.064)` against `(0.056, 0.074, 0.066)`). So it is where the sun falls on the rear panels (shadow or `N.L`), not a factor | 70 that it is local to the sun term, cause unread |
| Is the hull's normal map bound in ours on Omega? | By the code, yes: `psp2::normal_map::plan` binds it third for a PS4 submesh with a tangent, and `diffuse_vcol` (the main hull, 19,752 of Feisar's vertices) has no normal map in the original either. Not measured against the original's `N.L` | 60 |
| The second hull program's texel? | `diffuse_vcol`'s shadowed probe variant (`0x64a4`) is the same law with no specular, no reflection and no normal map: `texel.rgb * VertexColour1.rgb * (kmix * SH(n) + sun * sat(n . dir) * shadow)`, then the fog lerp. No other factor | 85 |

### Static

`data08.psarc`, `data/materials/ships/`, through `gcn_dump`:

| Program | What it does with `VertexColour1` |
| --- | --- |
| `diffuse_with_specular_from_alpha_n_vcol` vertex `0x7404` (the shadowed variant's) | `fetch slot 4 -> v20..v22`, then `exp param3 v20, v21, v22, v0` at `@228`: exported as fetched |
| the same material's vertex `0x1964` | `exp param2 v20, v21, v22, v0` at `@1c4` |
| its pixel `0x7c44` | `attr3.xyz` is that export (`attr2.xy` is `Uv1`, the export before it); `@76c v_mul_f32 v1, v5, v2`, `@770`, `@774` multiply the diffuse sample's three lanes by it, and `@778`, `@784`, `@788` multiply the product by the light. The specular (`@790`-`@798`) and the reflection are added after and are not multiplied by it |
| `diffuse_vcol` vertex `0x5d84` | `fetch slot 3 -> v16..v18`, `exp param3 v16, v17, v18, v0` at `@1b0` |
| `diffuse_vcol` pixel `0x64a4` (263 instructions: `shadowCoeffs, shCoeffs, probeOffsetAndDiffuse, liveLighting0diffuse, liveLighting0direction`) | `@4b0 v_mad_f32 v7, v26, v3, s46` is `kmix`, `@4bc`-`@4d4` build `kmix * SH + sun * shadow * N.L` per lane, `@4dc`-`@50c` multiply the texel by the colour lanes (pre-multiplied by the fog factor) and `@4e0`-`@518` by the light and lerp the fog |

The fetch format: `VertexDecl_InitAttributeFormats` (`0x017f3ae0`, was `FUN_017f3ae0`, a static initialiser, confidence 75) fills
`g_vertex_attribute_formats` (`0x022129c0`, confidence 80), sixteen rows of four GNM data formats, row the declaration's type
nibble and column the component count less one. Four rows are pinned by types `oag_rcs` already reads from the files:

| Type | Row | Four-component format | Read by ours as |
| --- | --- | --- | --- |
| 0 | `0x022129c0` | `0x00fac70e` (32-bit float) | `PS4_FLOAT2` |
| 1 | `0x022129d0` | `0x00fac70c` (16-bit float) | `PS4_HALF2` |
| 11 | `0x02212a70` | `0x00fac00a` (8-bit, channel type 0, **UNORM**) | `PS4_UBYTE4_NORM`, `VertexColour1` |
| 14 | `0x02212aa0` | `0x00fac10a` (8-bit, channel type 1, SNORM) | `PS4_SBYTE4_NORM`, `tangent` and `normal` |

No row holds the sRGB channel type (`9`; `0x00fac90a` occurs 34 times in the image, all as immediates in code). The table's
reader was not followed; the four rows agreeing is the evidence.

The data (Feisar `c1`, the player's model of the pair, a print in the reader that is not in the tree): every hull submesh
carries a grey colour with alpha 1. `diffuse_vcol`: 19,752 vertices mean `0.234`, 2,989 mean `0.522`, 1,934 mean `0.133`,
1,206 mean `0.515`; `diffuse_with_specular_from_alpha_n_vcol`: 3,940 mean `0.406`, 681 mean `0.611`, 342 mean `0.438`,
300 mean `0.483`, 180 mean `0.123`; `carbonfibre`: 682 mean `0.312` (its program was not read). The other seven craft of
the grid read the same way (means `0.08` to `0.92`).

### Live

Every state below changes the hull alone, so the adaptive exposure holds (a sunlit track panel `(700,580)-(760,620)` reads
`(0.352, 0.562, 0.509)` to `(0.369, 0.583, 0.526)` over the run, and ours `(0.318, 0.516, 0.498)`, `0.9` times). The probe
records are written flat (`sh[0] = 1`, the other eight zero: the probe term is `kmix`), `K = Ks` at 1, 2 and 4, each less
the `K = Ks = 0` state. Ours renders the same states through its probe row. Green lane, boot 5, three frames a state:

| Box | | `K = 1` | `K = 2` | `K = 4` | Slope 1 to 4, ours over the original (R, G, B) |
| --- | --- | --- | --- | --- | --- |
| hull body `(820,560)-(1100,860)` | original | `0.0314` | `0.0767` | `0.1238` | |
| | ours, vertex colour on | `0.0372` | `0.0740` | `0.1377` | `1.12, 1.09, 1.09` |
| wide `(760,520)-(1160,900)` | original | `0.0153` | `0.0517` | `0.0854` | |
| | ours, vertex colour on | `0.0263` | `0.0517` | `0.0945` | `1.02, 0.97, 0.98` |
| housing `(890,690)-(1010,800)` | original | `0.0334` | `0.0885` | `0.1804` | |
| | ours, vertex colour on | `0.0574` | `0.1217` | `0.2291` | `1.15, 1.17, 1.24` |

At `K = 1` on the hull body ours is `1.14, 1.18, 1.29` times the original with the colour, `1.67, 1.67, 1.77` without it
and `0.86, 0.92, 1.04` with the colour decoded by the sRGB curve first. On hand-placed panels of very different colour
(run 4: the housing block's top, both wing panels, the spine block, the lower body; ours' colour factor `0.91, 0.67, 0.71,
0.67, 0.36`) the raw colour gives `1.35` to `1.73` everywhere, none gives `1.5` to `4.3` and the decoded one `0.4` to `1.36`:
only the raw colour makes the ratio the same on every panel. The original's response is not a line through zero (its
`K = 2` over `K = 1` is `2.4` to `2.7`), which is why the slopes are quoted and the `K = 1` ratios run high; the cause (the
curve's toe at these levels, or the frame's own motion) was not separated.

With the flat probe at `K = 1` and no sun, **ours without the colour shows two round fittings beside the housing block as
white discs and with it shows them shaded into the body, as the original's same state does**.

Shipped state, engine light off, ours' tick 100 against the original's three shipped states of boot 5:

| Box | Original | Ours, main | Ours, vertex colour on |
| --- | --- | --- | --- |
| housing `(890,690)-(1010,800)` | `(0.044, 0.051, 0.045)` | `(0.097, 0.102, 0.075)` | `(0.068, 0.074, 0.054)` |
| hull body `(820,560)-(1100,860)` | `(0.064, 0.085, 0.079)` | `(0.087, 0.112, 0.097)` | `(0.069, 0.092, 0.082)` |
| spine `(930,600)-(990,680)` | `(0.072, 0.074, 0.053)` to `(0.080, 0.083, 0.059)` | `(0.154, 0.158, 0.114)` | `(0.103, 0.106, 0.074)` |
| left fin top `(800,640)-(880,700)` | `(0.19, 0.22, 0.20)`, frames `0.12` to `0.25` | `(0.141, 0.170, 0.135)` | `(0.132, 0.161, 0.130)` |
| floor `(1140,650)-(1260,800)` | moves with time | `(0.113, 0.353, 0.363)` | `(0.112, 0.354, 0.364)` |

The hull body and wide boxes take in track pixels at their edges; those pixels do not move with a hull-only state, so
the slopes above are the hull's, and the shipped means are diluted by a track that ours draws within 10 %.

Ours' shares on the housing with the colour on: ambient alone `(0.010, 0.014, 0.018)` (the original's `(0.010, 0.011,
0.012)`), sun alone `(0.056, 0.056, 0.031)` (the original's about `(0.025, 0.022, 0.009)`). **The ambient share closes; the
sun share on this box does not.**

What did not work, so nobody repeats it:

- **Whitening the colour bytes in the port's memory proves nothing.** The twelve buffers of the player's model are found in
  guest memory by their first bytes (13 copies, near `0x10d36c6000`) and written through the port's memfd, and the frame does
  not change (`0.96` to `1.04` on every box). The control says why: raising the 19,752-vertex buffer's positions by half a
  unit does not move the hull either. The port draws from its own copy of the vertex data.
- **The shadow term by the `K` / `Ks` split is noise at three frames a state.** `(K = 1, Ks = 0)` over `(K = Ks = 1)`,
  each less `K = Ks = 0`, gives `0.39` on the hull body in one boot and `1.19` in the next: the craft sways at rest and the
  three states are three poses. It needs a dozen frames a state, or the sway stopped.
- The craft leaves the grid about 40 s after it appears, so a run has about 20 s of states after the 12 s settle; a scan of
  all guest memory takes 24 s and has to be bounded (`--scan-lo`, `--scan-hi`).
- The original's and ours' cameras differ on this pair (ours looks further down the hull's rear), so a box is not the same
  panels on both sides; the hull-wide boxes are the safer comparison.

### Recipe

1. `crates/mesh/src/mesh/rcs/psp2.rs`, the vertex loop: for a PS4 submesh that is lit, is not a twin-blend, has no
   `lightmapUV`, carries `VertexColour1` and draws a hull program that multiplies it (`diffuse_vcol`,
   `diffuse_with_specular_from_alpha_n_vcol`; decide off the material's GCN vertex program naming the input, the way HD's
   `skin::light_inputs::colour_factor` decides off its microcode, not off the file name), emit `colour = rgb / 255` raw and
   `sun_mask = factor_sun_mask(1.0)`.
2. No shader change: `shade.wesl`'s `colour_factor` arm (`in.colour.rgb * (hull_ambient + sun_diffuse + in.spu_light)`) is
   the original's product, and it leaves the specular unmultiplied as the original does. The scratch build used exactly this.
3. Tests that fail if it is dropped: a ground truth that Feisar `c1`'s Omega hull emits a non-white colour with the factor
   marker on its `diffuse_vcol` vertices, and the box test on `omega/modesto-heights` tick 100: housing below
   `(0.075, 0.080, 0.060)` (it is `(0.097, 0.102, 0.075)` without), hull body within 15 % of `(0.064, 0.085, 0.079)`.
4. `carbonfibre` carries the same colour stream and its program is unread: read it (`gcn_dump`) before giving it the factor.
5. Then the housing's sun term (the owed shadow read, with more frames a state), and the reflection after that: adding the
   reflection first moves the housing the wrong way.

### Wired (lane `hull-vcol-wire`)

`oag_rcs::gcn::input_is_factor` reads a material file's programs for `VertexColour1`: a vertex program that fetches it and
exports the fetched registers unchanged to a parameter, and the pixel program listed after it (a file lists each vertex
program directly before the pixel program it feeds) reading that parameter's three lanes only as multiplicands (`mul`,
`mac`, `mad`, `fma`, first two sources), the lanes tracked until their register is written again, with one of those pixel
programs reading `liveLighting0diffuse`. `psp2::hull_colour`
runs it on each PS4 material that has a colour stream and emits `colour = rgb / 255` (UNORM, raw, alpha 1) with
`sun_mask = factor_sun_mask(1.0)` on its lit submeshes that are not twin-blend, lightmapped, tarmac or ice. No shader
change. Confidence 85 for the wiring (the recipe above is the evidence), 70 for "every admitted family multiplies it the way
`diffuse_vcol` does" (the reader decides off program shape, not names).

**Census** (`omega_vertex_colour_factor_ground_truth`, every `.rcsmaterial` in a `ships` directory of both base and patch
archives, stems without the `_1`/`_2` variant suffix). The rule has a third clause the first reading lacked: a pixel
program that takes the scene's light (`liveLighting0diffuse`) must be among the factor-only ones, or the colour is a factor
on a flat texture and not on a light (`leacheffectmat` and `2048_ship_zonevertexcolour_emissive*` pass the multiplicand
test and have no light; without the clause they were admitted). **Take it as a factor on the light (24):** `carbonfibre`,
`diffuse_vcol`, `diffuse_with_specular_from_alpha_n_vcol`, `diffuse_with_specular_from_alpha_vcol`, `ship_lod_vcol`,
`debris`, `fighter_version`, `ship_main_livery`, `nitro_perspex_new`, the `detonator_*` hull families (`basicalpha_vcol`,
`diffuse_vcol`, `diffuse_with_specular_from_alpha_n_vcol`, `ship`, `zoneship_basic`), and ten `2048_*` ones
(`2048_ship_paint_*`, `2048_ship_plastic`, `2048_ship_tech`, `2048_ship_carbon_fibre_shiny[_lod]`, `2048_ship_zonevertexcolour`,
`2048_nitro_ships`). **Fetch it and do not (32), so left white:** the shields, trails, flames, holograms and engine flares
(`*shield*`, `trail_test*`, `flame_test`, `jet_test`, `hexagonalshield*`, `holographic_projector*`, `2048_engine*`,
`2048_magnetic_shield`, `*absorbinternal`), the two unlit colour materials above, and `nitro_body_new`, `colourtest`,
`2048_ship_zonefacing`. Built over every `.rcsmodel` Omega ships, the materials that reach a drawn submesh are the hull
families above (Feisar `c1`: three of its five materials, 13 of 16 submeshes, `omega_hull_vertex_colour_ground_truth`;
`2048_ship_paint_shiny_final` 575 submeshes and `diffuse_with_specular_from_alpha_n_vcol` 470 lead) and, outside the ships,
one submesh each of `vertexcolourandtexture` and `vertexcolourandtexturealpha` (lit by their own programs). The unlit effect
materials (`basic_vertexemissive`, the HD bomb glows) are not admitted.

**`carbonfibre` takes the factor (checked, confidence 70).** Its vertex program `0x17e4` fetches slot 4 into `v20..v22` and
exports `exp param2 v20, v21, v22` (`@1ec`, the same shape as `diffuse_vcol` `0x1964`); the shadow-variant vertex `0x27c4`
does the same. Its lit pixel programs (`0x1ea4`, `0x2fc4`) interpolate `attr2.xyz` and multiply it into the three lanes of
the diffuse sample (`@2ac`, `@2f0`, `@32c`, `v_mad_mixlo_f16 d, attr2.c, sample.c, 0`; the `2x - 1` sample at `s[24:31]` is
the normal map); each product then feeds an opcode our table does not name (`VOP2 op 53`, `@2c4`, `@308`, `@334`) before the
fog lerp (`@348`-`@368`, `s16`-`s18`). That the second multiply is by the light is read from shape, not from the opcode name,
which is why 70 and not higher. Every one of its six vertex/pixel pairs passes the reader.

**Boxes** (1920x1080 `ours.png` of the stored `omega/modesto-heights` pair's own flags, tick 100, Feisar `c1`; linear means
as in "The measurement"; engine light off and colour off against on are the same binary with a local environment switch
that is not in the tree, as in the earlier knockouts):

| Box | Original | Colour off, engine light off | **Colour on, engine light off** | Shipped before | Shipped now |
| --- | --- | --- | --- | --- | --- |
| housing `(890,690)-(1010,800)` | `(0.045, 0.057, 0.049)` | `(0.097, 0.102, 0.075)` | **`(0.068, 0.074, 0.054)`** | `(0.158, 0.126, 0.084)` | `(0.084, 0.077, 0.055)` |
| hull body `(820,560)-(1100,860)` | `(0.064, 0.082, 0.078)` | `(0.087, 0.112, 0.097)` | **`(0.069, 0.092, 0.082)`** | `(0.109, 0.118, 0.099)` | `(0.074, 0.093, 0.082)` |
| floor `(1140,650)-(1260,800)` | moves with time | `(0.113, 0.353, 0.363)` | `(0.112, 0.354, 0.364)` | `(0.112, 0.350, 0.360)` | `(0.112, 0.353, 0.364)` |

The engine-light-off column pair is the lane's acceptance (the prediction above held to the third decimal: housing
`(0.068, 0.074, 0.054)`, body `(0.069, 0.092, 0.082)`); "shipped" is what a player sees, with the engine light on. The floor
moves by at most `0.002` (1.3 %), the stored track draw within its own frame-to-frame drift.

Second pairs, hull body `(820,560)-(1100,860)` of the stored `omega/the-amphiseum` and `omega/moa-therma` frames, Feisar `c1`,
shipped state, before to now (original): **the Amphiseum** `(0.123, 0.132, 0.144)` to `(0.065, 0.087, 0.103)`
(`(0.102, 0.121, 0.135)`); **Moa Therma** `(0.059, 0.051, 0.039)` to `(0.040, 0.041, 0.032)` (`(0.162, 0.153, 0.131)`). Both
fall as Modesto's does. On both the hull ends below the original (Amphiseum `0.64, 0.72, 0.76` of it, from
`1.2, 1.1, 1.1`; Moa Therma `0.25, 0.27, 0.25`, from `0.36, 0.33, 0.30`), so on those two circuits the terms ours still lacks
(the housing sun term, the reflection) are no longer hidden by the over-bright colour. The hull crops (ours before, ours
after, the original) show the round fittings beside the housing block shaded into the body, as the original's flat-probe
state did, where they were white discs.

**HD and 2048 are pixel-identical.** `hd-race/talons-pack` (HD) and `2048-vita/modesto-heights` (Vita), main against this
build, `max |difference| = 0` over every pixel. Neither title reaches `psp2::hull_colour`: HD multiplies the colour already
(`skin::light_inputs`), and the Vita decoder (`vertex_colour1()`) accepts the PS4 type only.

**Open.** `shade.wesl`'s `colour_factor` arm also multiplies the specular by the colour (HD's law); the Omega pixel program
adds the specular after the product and leaves it unmultiplied (`@790`-`@798`, above). It is not changed here (no shader
edit in this lane); the share is small at the dark panels but it is a difference. The housing's sun term (the owed read) and
the reflection stand as in "Recipe" 5. The admitted families beyond Feisar's three (`2048_ship_*`, `debris`,
`fighter_version`, `ship_main_livery`, the `detonator_*` ones) were not rendered: they take the colour by the same program
reading and nothing else.

### Cross-title

- **2048: checked, differs on the file, not wired.** (Wired on Omega above, including its ten 2048-lineage circuits, which ship
  `track.shprobes`.) The Vita 1.04 executable registers `Lighting.Ship diffuse brightness`
  (`0x815088e4`, in `Environment_RegisterLightingSchema`, `0x81017b80`) and names `probeOffsetAndDiffuse` (`0x815094e0`), so
  the weight exists there. It has no `shCoeffs` name and no `(shadow)` key, and its probe file is `track.probes` (`0x81422ba8`,
  1,112-byte records), not `.shprobes`: what `K` scales on the Vita is unread. Next address: the references to `0x815094e0`
  and the GXP hull program's use of the constant.
- **2048, the vertex colour: checked, applies as far as the declaration, not wired.** The Vita's `Diffuse_Vcol` and
  `Diffuse_With_Specular_From_Alpha_N_Vcol` vertex programs declare `VertexColour1` with four components
  (`scripts/vita-gxp.py grep <data.psarc>:materials/ships/diffuse VertexColour1`: the base archive and both patch archives,
  e.g. `Diffuse_Vcol.rcsmaterial` program 2 at `0xda0`), so the stream is there. What the fragment programs do with it is not
  read (the GXP reader stops at the container). Ours reads no hull colour on the Vita: `vertex_colour1()` accepts the PS4
  type only. HD multiplies it and is wired (`skin::light_inputs`).
- **HD**: its hull ambient is `4 * irradiance` from `track.probes` and is wired (`docs/rendering/hd-ship-materials.md`,
  "Finding 5"); it has no `K`.
