# Detonator: what writes the mine's and the bomb's shader constants

Functions in `EBOOT.elf` (WipEout HD Fury, PS3, EU, `BCES00664`), Ghidra program
`/hdfury/EBOOT-ps3-hdfury-eu.elf`. Static reading only, 2026-11-22: nothing here ran in the original (the wiring below, 2026-11-23, runs in ours). Bodies read
off `llvm-objdump -d --mattr=+altivec,+vsx` over their real extent (Ghidra's decompile of both
`AnimateToMusic` functions is garbled by the AltiVec words); TOC slots through
`scripts/ps3-toc.py resolve <fn>` (both functions declare TOC `0x008ad4d8`, "exact"). Tuning
constants are the initial `.data` words at `0x008c19dc` (mine) and `0x008c18d8` (bomb), read with
`scripts/ps3-toc.py u32`; nothing was seen to rewrite them. This answers the open lines of
[detonator-hull.md](detonator-hull.md) ("the mine's music-driven binds", "the draw path of the
mine's three models") for the colour and brightness constants. The objects' own laws are
[detonator-mine.md](detonator-mine.md) and [detonator-bomb.md](detonator-bomb.md); where the
colour keys come from is [zone-effectsettings-loader.md](zone-effectsettings-loader.md).

## Result

| Constant | Material | Writer | Law | Confidence |
| --- | --- | --- | --- | ---: |
| `MineColour` | football and spikes (`detonator_mine_normal_alpha`) | `DetonatorMine_AnimateToMusic` `0x001395f0` | live mine: the stage's `Detonator Mine Colour` bytes / 255 (order inferred); dark mine: `(2,2,2)` football, `(10,10,10)` spikes | 74 |
| `MineBrightness` | both | same | live: `2.0 * s`, `s` a band-level envelope; dark: `0` | 76 |
| `ElectricityColour` | `Electric_Visualiser` | same | live: the stage's `Detonator Mine Electricity Colour` bytes / 255 * 1.5; dark: `(2,2,2)` | 74 |
| `BombColour` | `detonator_bomb_final` | `DetonatorBomb_AnimateToMusic` `0x001340d8` | the stage's `Detonator Bomb Inner Colour` bytes / 255 (order inferred) | 74 |
| `BombEQColour`, `Colour`, `RainbowColour` (glow edges) | equalisers, equalisers, glow edges | same | the stage's `Detonator Bomb Outer Colour` bytes / 255 | 74 (`Colour`: 72) |
| `BombBrightness`, `BombEQBrightness` | final, equalisers | same | `0.2 + 2.0 f`, `0.8 + 2.0 f`, `f` a flash value that decays | 72 |
| equalisers `Brightness` | equalisers | same | `clamp(scale / 4 - g + f, 0, 1)`, `scale` from a band-4 envelope | 66 |

How the **mine's** shader combines `MineColour` and `MineBrightness` is read now, from the material's
own microcode ("The mine's fragment program", 80). How the **bomb's** programs combine theirs is not
(only that the brightness is the last multiply); see "What stays open".

Both functions take the player's colour from the same place: `RaceManager_GetInstance()+0x13e8`
is the player craft, and its word `+0x6284` is the index passed to the getter. The getters are
`Environment_GetStageMineColour` (`0x003cdbd0`), `..MineElectricityColour` (`0x003cdc00`),
`..BombInnerColour` (`0x003cdc30`) and `..BombOuterColour` (`0x003cdc60`), all named already
([zone-effectsettings-loader.md](zone-effectsettings-loader.md), 85). The code reaches them through
the TOC-switching stubs `0x00677cf8`, `0x00677d08`, `0x00677cd8`, `0x00677ce8` (each is a three-word
stub and a `b` to the getter; they are not renamed). Each getter is

```text
word* getter(idx):  e = table[idx * 56]      ; table = *(TOC -0x5a80), entry +0 = committed stage
                    return stagesBase + e * 592 + 0x1230 + {0, 4, 8, 12}   ; mine, electricity, inner, outer
```

so the value is **the colour of the stage the viewport has committed** (`entry[n].+0`, written by
`Environment_UpdateStageBlend`, [zone-effectsettings-loader.md](zone-effectsettings-loader.md),
"a ninth pass"). The getter returns a pointer to a packed word; there is no cross-fade of these four
keys: **the colour snaps when the stage commits** (confidence 75: the read of the getter is exact,
that nothing else blends the word is the absence of a writer found). The caller unpacks the word as
red `w >> 24`, green `(w >> 16) & 255`, blue `(w >> 8) & 255` (the `rldicl` extracts at
`0x001397d4`-`0x001397e4`), then `* 1/255` (TOC `-0x2b74` = `0x3b808081`). The low byte is never
read, so an authored alpha of `0` (stage 11, "Super Zen") is harmless.

**Channel order is inferred, not read.** The key's parser (`0x005d40b8` registers it as type
`(1, 4, 3)` and hands the text to `0x005d13c0`) was not read, so that the file's first integer
lands in the word's top byte is not shown by a store. It is corroborated by hue: the same stage
authors `Airbrake Colour` as floats in plain `R G B`, and the mine colour matches it in file order
on every stage checked (stage 2 airbrake `(0.004, 0.541, 1.0)` against mine `0 142 224`; stage 3
`(0.682, 0.004, 1.0)` against `164 0 232`; stage 4 `(0.004, 0.918, 1.0)` against `0 213 232`;
stage 5 `(0.518, 1.0, 0.004)` against `146 248 0`). Any other order would put the authored alpha
`255` in a colour lane. Confidence 74 for the order, which caps the colour rows below.

## The authored values: `detonatormode.effectsettings`

`/data/environments/detonatormode.effectsettings` (`DATA00.PSARC`, 43,240 bytes); the DLC3 file
`detonatormodedlc3.effectsettings` authors the same fifteen rows (a `diff` of the `Detonator` lines
is empty). Each is four integers `R G B A`. In a Detonator race the stage counter starts at `1`
([detonator-placement.md](detonator-placement.md): `rm+0x2e10 = 1` on the opening stage), so stage
`0` ("Start", all black) is not the first stage a race shows; `oag_raceplay::zone_grade`'s
`show_stage` is the port's counter for it.

| Stage | Name | Mine | Mine electricity | Bomb inner | Bomb outer |
| ---: | --- | --- | --- | --- | --- |
| 0 | Start | 0 0 0 | 0 0 0 | 0 0 0 | 0 0 0 |
| 1 | Sub Venom | 122 25 255 | 109 0 232 | 255 128 65 | 255 112 25 |
| 2 | Venom | 0 142 224 | 0 132 224 | 101 255 81 | 99 253 110 |
| 3 | Sub Flash | 164 0 232 | 175 0 232 | 73 184 255 | 73 189 255 |
| 4 | Flash | 0 213 232 | 17 207 255 | 255 41 192 | 235 65 255 |
| 5 | Sub Rapier | 146 248 0 | 141 240 0 | 197 26 254 | 178 57 255 |
| 6 | Rapier | 123 0 248 | 123 0 248 | 65 226 255 | 123 221 253 |
| 7 | Sub Phantom | 231 255 25 | 228 255 1 | 255 49 233 | 255 89 234 |
| 8 | Phantom | 79 255 1 | 91 255 1 | 255 112 49 | 255 107 73 |
| 9 | Super Phantom | 23 255 17 | 10 254 10 | 255 96 41 | 254 91 26 |
| 10 | Zen | 9 177 255 | 17 168 255 | 252 108 44 | 255 111 33 |
| 11 | Super Zen | 23 1 239 | 1 1 239 | 74 255 65 | 64 255 33 |
| 12 | Subsonic | 23 1 231 | 25 1 255 | 111 255 41 | 83 255 49 |
| 13 | Mach 1 | 0 215 240 | 33 216 255 | 255 25 25 | 184 13 0 |
| 14 | Supersonic | 255 255 255 | 255 255 255 | 255 255 255 | 255 255 255 |

The materials author their own defaults, so an unbound constant is not zero. Read with
`cargo run -p oag-rcs --example hd_material_dump -- <entry> DATA00.PSARC`, hashes are
`~crc32(name)` (`oag_rcs::rcsmaterial::name_hash`):

| Model | Constant (hash) | Authored default |
| --- | --- | --- |
| `mine_rita_football` | `MineBrightness` (`c420e90d`) | `0` |
| | `MineColour` (`d7883cf6`) | `(0.239, 0.5596, 1.0)` |
| `mine_rita_spikes` | `MineBrightness` | `0` |
| | `MineColour` | `(0.329, 0.7275, 1.0)` |
| `detonator_bomb_final` | `BombBrightness` (`6cf44555`) | `2.24584` |
| | `BombColour` (`1eb1279c`) | `(1.0, 0.5843, 0.9922)` |
| | `Envmap` (`15128046`) | `0` |
| `detonator_bomb_final_equalisers` | `BombEQBrightness` (`9df50e6b`) | `0` |
| | `BombEQColour` (`659bbb32`) | `(1, 1, 1)` |
| | `Brightness` (`69d42acc`), `Colour` (`02ab9f07`) on the rays materials | `0`, `(1,1,1)` and `0.4659`, `(0.911, 0.822, 1.0)` |
| `detonator_bomb_glowedges` | `Brightness` | `4.7362` |
| | `RainbowColour` (`d01ab395`) | `(1, 1, 1)` |

## `DetonatorMine_AnimateToMusic` (`0x001395f0`)

Called from `DetonatorMine_Update` while the mine is visible and not triggered
([detonator-mine.md](detonator-mine.md)). `m` is the mine; `m+0xcc`, `+0xd0`, `+0xd4` are the
football, the spikes and the visualiser ([detonator-hull.md](detonator-hull.md)). The binds are made
in `DetonatorMine_Construct` (`0x0013a7d8`) through `0x0010ebb8(m, model, hash, &field)`: a pointer
to a field of `m` is bound, so the constants take whatever the field holds at draw time.

| Field | Bound as | On |
| --- | --- | --- |
| `m+0x100` | `MineBrightness` | football, spikes |
| `m+0x138` (3 floats) | `MineColour` | football (`lwz 4, 204(31)`, `0x0013a9f8`) |
| `m+0x144` (3 floats) | `MineColour` | spikes (`0x0013aabc`) |
| `m+0x104`, `+0x108`, `+0x10c`, `+0x110`, `+0x114`, `+0x118` | `MasterVolume`, `PitchOffset`, `PitchVolume`, `BassHit`, `TrebleHit`, `Tempo` | visualiser |
| `m+0x120` (3 floats) | `ElectricityColour` | visualiser |

Confidence 80 for the table: the name strings are `TOC -0x2b40` (`MineBrightness`), `-0x2b3c`
(`MineColour`), `-0x2b28`..`-0x2b10`, and every `addi 6, 21, off` pairs with the model it follows.

`m+0x154` and `m+0x158` are two float band indices written once, in the constructor
(`0x0013a8c8`-`0x0013a91c`): `a = Libc_Rand() % 4` (`0x006762f8`, `Libc_Rand`, 88), `+0x154 = a`,
`+0x158 = a + 11.0` (`TOC -0x2b0c` = `0x41300000`). So **each mine picks one of four low bands
(0..3) for its football and the matching high band (11..14) for its spikes, at random, once.** The
band source is `SoundSystem_GetBandLevel` (`0x00304530`, 85, [zone-visualiser.md](zone-visualiser.md)):
`g_sound + 0x24 + 4 * band`, `band < 16`, otherwise `0.0` ([zone-visualiser.md](zone-visualiser.md)). Ours has the same sixteen
levels in `0..1` as `oag_audio::spectrum` (`BANDS = 16`; `Spectrum::levels`).

### Live mine (`m+0x160 != 0`)

```text
phase m+0x150 = wrap(m+0xc0 * 4.0)                       ; spin; presentation only
bA = clamp01(Band((int) m+0x154))                        ; 0x001398b8-0x001398e4
s  = m+0xf4;   s = (s + 0.3 < bA) ? bA : s * 0.95       ; K[0x18], K[0x1c]; m+0xf4 = s
scale = clamp(s * 2.3, 1.0, 2.3)                         ; football size (K[0x20])
bB = clamp01(Band((int) m+0x158))
t  = m+0xf8;   t = (t + 0.3 < bB) ? bB : t * 0.95       ; m+0xf8 = t
spikeScale = clamp(4.0 * t, 1.0, scale + 0.95)           ; spikes size (K[0x30], K[0x38])
MineBrightness (m+0x100) = s * 2.0                       ; K[0x3c]; 0x00139a80-0x00139aa0
MineColour football m+0x138.. = spikes m+0x144.. = RGB(stageMineColour) / 255
ElectricityColour m+0x120..0x128 = RGB(stageMineElectricityColour) / 255 * 1.5   ; K[0x14]
MasterVolume m+0x104 = clamp01(5.0 * L)                  ; L = 0x00304cc8(), K[0x10]; see "The visualiser's level"
BassHit m+0x110 = clamp(s * 0.25, 0, 0.25)               ; K[0x28] = 1.0, K[0x2c] = 0.25
PitchOffset m+0x108 += step ; PitchVolume m+0x10c = clamp(1.0 * L, 0, 0.4)   ; a second call to 0x00304cc8
Tempo m+0x118 = 1.0 ; TrebleHit m+0x114 = *(0x00994464 + 4)   ; a .bss word, unread
m+0xd8 = scale
```

The two models are drawn through `0x00327500(model, matrix, 0)` with the rows of the mine's matrix
scaled by `scale` (football) and `spikeScale` (spikes). Presentation only: nothing here reads back
into a simulation word except `m+0xd8`, whose reader is unread.

**What the envelope means for the picture.** `MineBrightness` is `2 s`; `s` jumps to the band
level when the band rises by more than `0.3` in one call and otherwise falls `5 %` per call. With
no music, `s` falls to `0` and the mine's brightness is `0`, the same number its dark state writes.

### Dark mine (`m+0x160 == 0`; entered only while `byte(0x008c19e4) != 0`, initial `1`)

```text
MineBrightness m+0x100 = 0 ; m+0x104, 0x108, 0x10c, 0x110 = 0 ; m+0x12c..0x134 = 0
MineColour football m+0x138.. = (2.0, 2.0, 2.0)          ; TOC -0x2b60
MineColour spikes   m+0x144.. = (10.0, 10.0, 10.0)       ; TOC -0x2b5c
ElectricityColour m+0x120.. = (2.0, 2.0, 2.0)            ; K[0x54]
Tempo m+0x118 = 1.0 ; TrebleHit m+0x114 = *(0x00994464 + 4) ; m+0xd8 = 0
```

Both models and the visualiser are re-submitted (`0x00327500` at `0x00139ccc`, `0x00139cec`). A
mine is dark after a craft contact darkens it and for every mine laid on an earlier stage
([detonator-mine.md](detonator-mine.md), "Live and dark mines"; `DetonatorMine_SetDark`,
`0x00688280`). **A dark mine writes no stage colour at all**: its constants are the literals above.
Confidence 76: the stores are read, what a shader does with `(2,2,2)` against `0` brightness is not.

## `DetonatorBomb_AnimateToMusic` (`0x001340d8`)

`b` is the bomb; `b+0xb4`, `+0xb8`, `+0xbc` are `detonator_bomb_final`, `_final_equalisers` and
`_glowedges`. The binds are in `DetonatorBomb_Construct` (`0x00135328`) through
`Material_BindInstanceParamPointer` (`0x00677018`, 85, [plasma.md](plasma.md)); each group names
the hash through `0x00676ff8` and `~` of it.

| Field | Bound as | On |
| --- | --- | --- |
| `b+0xd0` | `BombBrightness` | final |
| `b+0xe4` (3) | `BombColour` | final |
| `b+0xdc` | `Envmap` | final and equalisers |
| `b+0xd8` | `BombEQBrightness` | equalisers |
| `b+0xf0` (3) | `BombEQColour` | equalisers |
| `b+0x108` (3) | `Colour` | equalisers |
| `b+0xfc` | `RainbowColour` | equalisers |
| `b+0xd4` | `Brightness` | equalisers |
| `b+0xe0` | `V_Offset` | equalisers |
| `b+0x114` (3) | `RainbowColour` | glow edges |
| `b+0x120`, `+0x12c`, `+0x130`, `+0x134`, `+0x138` | `U_Offset_Speed`, `Distortion`, `Speed`, `V_Offset`, `Brightness` | glow edges |

Confidence 76 (name order against `r6` hash registers; the `lwz` name strings are
`TOC -0x2d08`..`-0x2cdc`).

```text
inner = RGB(stageBombInnerColour) / 255 ; outer = RGB(stageBombOuterColour) / 255
BombColour   b+0xe4.. = inner                            ; 0x001341cc
BombEQColour b+0xf0.. = outer                            ; 0x00134240
Colour       b+0x108.. = outer   (only if byte(0x008c18d8+0x24) != 0, initial 1)
RainbowColour b+0x114.. = outer                          ; glow edges, always
f = b+0x154 (flash);  BombBrightness b+0xd0 = 0.2 + 2.0 f ; BombEQBrightness b+0xd8 = 0.8 + 2.0 f
f = clamp(f > 0 ? f - 0.02 : f, 0, 1) ; b+0x154 = f      ; decays 0.02 per call, kicked elsewhere
bandB = byte(0x008c18d8+0x3c) ? Band(4) : 0   ; initial 1: band 4
s = b+0x124 ; s = (s + 0.3 < clamp01(bandB)) ? clamp01(bandB) : s * 0.95 ; b+0x124 = s
scale = clamp(4 s, 2.0, 4.0)                             ; body size
Brightness b+0xd4 = clamp(scale / 4 - *(0x00994420 + 0xc) + f, 0, 1)
```

**The bomb does read the music, through one band: `Band(4)`**, behind a tuning byte that is `1`
in the shipped `.data`. Its brightness constants additionally read a flash value `b+0x154` that a
hit or a detonation sets (the writer is unread) and `*(0x00994420 + 0xc)`, a `.bss` word. Spin:
`b+0x15c * 4.0` wrapped by `2 pi` feeds the rotation of the matrix (`0x006772d8`). The equalisers'
`RainbowColour` (`b+0xfc`) and `Envmap` (`b+0xdc`, a product of the matrix rows, `0x0013444c`) are
not set to a stage colour by this function.

Confidence 74 for the three colours (getter and field read, channel order inferred), 72 for the two
brightness words (constants resolved, flash writer unread), 66 for equaliser `Brightness`
(a global of unknown value is in the formula).

## The mine's fragment program

`detonator_mine_normal_alpha.rcsmaterial` (`/data/weapons/materials/`, `DATA00.PSARC`, 31,872 bytes),
read with `scripts/ps3-microcode.py fp-file`. The race's lit pass resolves the fogged variant at
`@0x1a30` (static class, feature `0xfb61d927`); its unfogged twin (`@0x15d0`) is the same program
without the closing fog lerp. One sampler, unit 0, `0xc78c9866`: the model's `_n` map, read as a
tangent-space normal (`TEX R0, f[TC3]`, then `* 2 - 1`), `TC3` the program's fourth coordinate interpolator. Writing
`C` for `MineColour`, `B` for `MineBrightness`, `V` for the normalised tangent-space eye vector
(`f[TC1]`) and `N` for the map's normal:

```text
rim = sat(1 - dot(V, N))                      ; DP3_SAT, then ADD 1 - x
p   = 4 sat(1 - B) + 1                        ; ADD_SAT 1 - B, MOV x4, ADD 1
rgb = rim^p / (1 - 0.9 C)   per channel       ; LG2, MUL, EX2; RCP of MAD(-C, 0.9, 1)
out = lerp(fogColour, rgb, fogFactor) / 2     ; the fogged variant; the unfogged one has neither
a   = the map's alpha, through globalAlphaScaler (identity)
```

So the mine is an **unlit** Fresnel rim glow, no light, no albedo: with `B = 0` (silence) it is a thin
`rim^5` edge, with `B >= 1` a wide `rim^1` shell; `C` dodges per channel, so a saturated colour
(`122 25 255`: `1.75, 1.1, 10` times the rim) goes blue-white at the edge. **A dark mine's literal
`C = (2, 2, 2)` makes the divisor `-0.8`**, a negative output; `(10, 10, 10)` on the spikes `-8`. What
the target does with a negative is unread. Confidence 80 for the formula (every instruction and
literal is read; the order of the two `x4`/`/2` scales is the encoding's), 70 for "the draw is a
cutout on the map's alpha": both models' state word is `62` and the engine sorts each into its
alpha-tested draw list (`mesh::Model::alpha_tested_draws`, one draw each), so the map's alpha, the
program's output alpha, is tested; the reference value and what the original's blend does with a
partial alpha are not observed.

## The bomb's programs

The bomb body (`detonator_bomb_normal`, `@0x1750`) is a longer program: a normal-mapped facing
term `0.92 N.V` and `0.2 BombColour` through an overlay-shaped `x / (1 - ...)`, a power by
`4 sat(1 - |TC2|^2) + 1`, an environment picture read at `4 dot(TC2, TC0)` on unit 1 (the
`Envmap` weight patches the first constant), a second overlay, and **a last multiply by
`BombBrightness`** (`@0x3f`, confidence 80; that is the unfogged block, the race resolves the
fogged variants `@0x1e00` for the body and `@0x1d80` for the equalisers, which end in the same fog
lerp as the mine's, `MAD /2`, so the brightness is the last multiply before the fog). Only that last
step is used (below).

## Wired

HD Detonator races draw the mine from the program above (`oag_mesh::mesh::rcs::mine_rim`,
`Emissive::program` mode 23, `shade.wesl`): the football and the spikes each earn a glow-table
entry holding the authored `MineColour` and `MineBrightness`, the `_n` map is bound third and the
chunk carries its tangent; the map's alpha is the output alpha, which the cutout draw tests. A chunk
with no albedo draws through the model's shared row 0, which holds no normal map, so the slot also
gets a 1x1 skin. The values per instance are `oag_raceplay::detonator_look`'s: a live mine
takes the committed stage's `Detonator Mine Colour` (`EffectSettings::detonator_colours`) and
`2 s`, a dark one `(2, 2, 2)` / `(10, 10, 10)` and `0`, written into the mine pool slot's own
glow-table row each frame (`Drawable::write_program_glow`). The envelope `s` steps once per race
tick for a live mine only, keyed by the mine's survey entry and lane (not its pool slot), against
the frame's sixteen band levels: the football band is **chosen** from the same key (the original's
`Libc_Rand() % 4`), the spikes' `+ 11`. Bombs: a material that patches `BombColour` +
`BombBrightness` or `BombEQColour` + `BombEQBrightness` draws as colour times brightness, unlit
(`PROGRAM_BOMB_TINT`, mode 24, **chosen, not measured**: the rest of the bomb programs is unread),
with the stage's inner colour on the body and the outer on the equalisers, brightness `0.2` and
`0.8` for a flash of `0`.

**Which coordinate set the map reads** (`det-mine-lit`, 2026-11-25; confidence 85): the mine's `.rcsmodel` chunks declare **no
`lightmapUV` and no `Uv2`**, only the diffuse set, so `TC3` is that set. The wiring first sampled `lightmap_texcoord`, which
the chunk does not carry (zero), so every pixel read one texel: a flat normal and a flat alpha, a smooth sphere with no hubs
and no cut-out holes even at full brightness. `shade.wesl` now samples `in.texcoord`; with the map live the football shows its
hubs, screws and holes (`hd_emissive_glow_ground_truth::the_mine_football_shows_its_normal_map_relief`: 623 luminance steps
in the ball's inner half against 145 for the smooth one). Pair `mines-ahead` mean grey 13.74 -> 13.73, `pad` 7.68 and
`cannon-volley` 14.18 unchanged: the pairs feed raw bands under 0.3, so ours' envelope never attacks and `MineBrightness`
stays 0 (a thin `rim^5` edge); the original's `s` carries history ("not done" below), so its mines are lit.
**The envelope history is carried in a pair** (`det-mine-lit`, second grab, RPCS3 lap clock 2.68 s, 80 mines): `s` read
`0.0` to `0.95` (the near three `0.57, 0.46, 0.57`, spikes `t` `0.53, 0.86, 0.48`), `--detonator-mine-envelopes`
([compare-page.md](../../../tools/compare-page.md), "Detonator pairs") holds them, and ours' near mines draw as lit, spiked balls of the
original's size: whole-frame mean grey 14.76 to 14.29, rmse 0.177 to 0.171.
**Read this session, the dark centre is not the formula** (confidence 85): the vertex block `#1` (the lit pass's) writes
`TC1 = (R0.T, R0.B, R0.N)` with `R0 = c[209] - pos` and the fragment program normalises it (`DIVSQ`), as read above; the map's
flat base is `(0.5, 0.5, 1)` (BC3 block `0x841f`, alpha `ff`), so a flat area facing the eye is `rim` 0 in the program as read. The
original's ball is white there, so **`c[209]` is not the model-space eye the port builds** (suspect: the world camera, not
transformed by the mine's inverse matrix); the value is not in the push-buffer regions dumped, so settling it needs a live read of
`c[209]` at the mine's draw (a breakpoint at `0x001399a4` for the matrix, then the draw). **Superseded** by "The lit draw, read
live" below: the race runs block `#15`, not `#1`, and `c[209]` is the world camera, which is what the port feeds.
**Still unlike the original** with the envelope forced high (raw bands 0.8): the ball's centre is dark in ours (`N.V` near 1
there) and bright in the original; the eye-vector space was the suspect; it is ruled out below, and the cause stays open.

### The lit draw, read live: `c[209]` is the world camera (2026-11-25, `det-mine-eye-centre`)

**Method.** RPCS3 restored from `hd-detonator-talons-feisar-grid-late`, driven to lap clock 2.733 s (80 mines laid), paused,
and the whole command ring dumped (`scripts/hd-draw-dump.py` with `HOOK_RING_ONLY=1`, `RING_DUMP=0x40000000:0x120000`: 464
draws). Every `TRANSFORM_CONSTANT_LOAD` and vertex-program upload was replayed per draw. A draw is the mine's when its
uploaded vertex code is a byte run of `detonator_mine_normal_alpha.rcsmaterial` and its unit 0 is the football's or the
spikes' `_n` map. A second dump with the fragment programs (`HOOK_LIGHT=1`), RPCS3's own decompiled shaders
(`Log shader programs: true`) and a GDB read of the bound VRAM were taken in the same session.

**What the lit pass runs** (confidence 90). The 26 football and spikes draws of the 13 mines on the scene target `0x1920000`
(football, spikes, then the mine's visualiser, per mine) run **vertex block `#15`** (`@0x5150`, code at file offset `0x5200`), not `#1`. Its table puts
`world` (`0xc588eebc`) at `c256` and `viewProj` at `c260`:

```text
R0 = v[0] * c[210] + c[211]                 ; positionScale, positionBias
R5 = world * R0                             ; world position
N' = world3x3 * v[1]   T' = world3x3 * (2 v[2] - 1)   B' = cross(N', T') * (2 v[2].w - 1)
V  = c[209] - R5                            ; c[209] = c465 = eyePositionWorldSpace
o[TC1] = (V.T', V.B', V.N')   o[POS] = viewProj * R5
```

Blocks `#1` and `#0` (`c256` = `viewProj`, no world) drew nothing lit in this frame: `#0` matched only orthographic passes
(`c259.w = 1`, `w` constant).

**`c[209]` at the draw** (confidence 90). `c465 = (202.888, -41.344, -194.880)` on every mine draw of the frame. Pushed through
the same draw's `c260..c263` it lands on clip `(0, 0, -0.954, 0)`, `w = 0` to the printed digit: it is the eye of the frame's own
view-projection. The RAM rig eye after the pause read `(210.11, -41.01, -194.65)`, about 7 units ahead, the 2-tick pause skew.
`c256..c259` on the football draw is the mine's own world matrix: translation `(211.8179, -43.3238, -199.5839)`, the RAM
position of mine 0 to the digit, columns of length 1.69 (the football scale), determinant `+4.83` (no reflection). The mine's
`c260` equals the circuits' `c256` element for element. **So `c[209]` is the world camera, the vector is formed in world space,
and the port's `normalize(scene.fog.camera - in.world)` against a world normal is the same angle.** The brief's suspect
(an untransformed world eye against a model-space position) is false.

**Everything else the facing surface reads matches the read formula** (each confidence 85 or more):

| Input | Live read | Port |
| --- | --- | --- |
| Fragment program | the fogged program as read above, constants `C = (0.478, 0.098, 1.0)`, `B = 1.157` (so `p = 1`), fog density 0; RPCS3's decompiled GLSL is the same instruction for instruction | mode 23 |
| Vertex decode | position `S32K x3` at `+0`, normal `CMP` (11:11:10) at `+6`, tangent `UB x4` at `+10`, uv `SF x2` at `+14`, stride 18; RPCS3's `CMP` fetch is the same 11:11:10 split | `rcsmodel::unpack_normal` |
| Normal map | unit 0 VRAM `0x0cb0e480` byte-identical to `mine_rita_football_n_1024x1024.gtf` at six offsets; remap `0xaae4` (identity), unsigned; the body is `(128, 127, 255)` on 816,206 of 1,048,576 texels | same file |
| Alpha test | `GREATER 0x7f` | `GREATER 0.5` |
| Cull | enabled, `BACK`, front `CCW`, window `0x1438`: the same state as every other culled draw on the scene target (396 in the frame, the circuit's included) | `cull_as_authored` (state 62) |

**The football is two coincident shells** (240 vertices, 232 triangles, the live buffer and ours agree vertex for vertex): 116
triangles wound outward with outward normals and 116 wound inward with inward normals. In the frame's window space every
triangle facing the eye is counter-clockwise and kept, and every kept triangle but 12 at the silhouette has its vertex normal
toward the eye. So the program as read gives `rim` near 0 at the facing centre: dark, in RPCS3's own shader too. **The
original's white facing body is not explained by any input read here.** Ours keeps the same shells (a `Face::Front` probe keeps
the inner one and turns the ball white, but it breaks every circuit, so it is not the law).

**A separate defect found on the way** (confidence 90): ours builds the football and the spikes with **no tangent at all** (0 of
240 and 0 of 630 vertices non-zero), where the live buffer carries the `UB x4` tangent at `+10` (unit vectors perpendicular to
the normal, `w = 255`). So ours' relief drops the map's `x` and `y`; only `z` and the alpha reach the rim. It changes the hubs,
not the flat body.

**Wired** (2026-11-25, `det-mine-eye-centre` send-back; confidence 85). The mine's chunks are inline: they declare no
layout, so `Mesh::tangents` (declaration only) gave nothing. `Mesh::inline_tangents` reads the four bytes after the packed
normal (`+10`, the offset the live arrays state) when every vertex there decodes to a unit vector perpendicular to its own
normal, and only a surface whose material draws through the mine rim program asks (`mine_rim::tangents`), so no other
inline chunk changes. Every football and spikes vertex now carries it
(`hd_emissive_glow_ground_truth::the_mine_models_carry_their_authored_tangent`; 0 of 240 and 0 of 630 before). On the
near-mine grab (lap clock 2.68 s, envelopes pinned) the ball's dark centre is gone: the hubs ring as the original's do and
the core (a 100 x 90 box on the near ball) reads mean 143.0 to 157.5 against the original's 188.0; a 200 x 180 box around it
90.2 to 95.8 against 118.8 (mean abs difference 66.7 both, limited by the grab's alignment). Pairs not worse: `mines-ahead`
13.73 to 13.62, `pad` 7.68 to 7.64, `cannon-volley` 14.18 to 14.17. **Still open:** ours' body stays darker than the
original's white (the A/B above puts the original's body rim near 0.4 to 0.7); the next read is `f[TC1]` itself (Open,
below). **2048 and Omega: checked, differs.** The inline read is HD's container
only: a `psp2` chunk declares its tangent and `rcsmodel::psp2::unpack_tangent` already decodes it; their mine rim draw itself
is not wired (`det-mine-lit`: "checked, applies, not wired").

**A live A/B says the white is this program, at a rim well above 0** (confidence 80). Same state, lap clock 2.5 s: the mine's
brightness factor `K[0x3c]` (`0x008c1a18`, read `2.0`) written to `0.0` while paused, the game run 0.6 s, grabbed, then
written back and run 0.6 s again. With `MineBrightness` 0 (so `p = 5`) the near ball's body turns **dark with bright
rings**, the port's picture; restored, the bodies are white again. `rim^5` darkening the body while `rim^1 / (1 - 0.9 C)`
whitens it puts the body's `rim` at roughly 0.4 to 0.7, not 0: **in the original the tangent-space eye vector at the flat body
is well off the map's `(0, 0, 1)`**, although every constant, the vertex buffer and the map read here say it should not be.
Frames: the lane's scratch only; the reproducing writes are the two lines above.

**Open** (next address to try): read `f[TC1]` itself. Patch the football's fragment program in VRAM (its code is at the
`SET_SHADER_PROGRAM` offset of the draw, `0x01122d00` local in the dump) to output `normalize(f[TC1]) * 0.5 + 0.5`, grab, and
compare with the port's tangent-space `V`. If it is off-normal, the next suspects are the vertex inputs as the RSX fetches them
(`VERTEX_DATA_BASE_OFFSET` `0x1738`, the `CMP` normal's scale), not the constants. The visualiser (additive, `SRC_COLOR,
ONE`) and the post chain's alpha-keyed glow are ruled out as the source of the white by the A/B (they do not read
`MineBrightness`).

### The white is before post, and the program as read reproduces it (2026-11-26, `det-mine-body-white`)

**Method.** RPCS3, state `hd-detonator-talons-feisar-grid-late`, lap clock 2.3 to 5.6 s. The fogged program's **source copy** is
in RSX-mapped main memory: the material file is loaded whole at `0x43a48700` this boot, so its four lit variants (code at file offsets
`0x1ad0`, `0x2060`, `0x2880`, `0x3130`) sit at `0x43a4a1d0` and on. Each frame the game copies the program per mine part into a
VRAM ring (`0xc1122xxx`, `0xc152xxxx`, then `0xc1170xxx`, `0xc156fxxx`: no copy survives a frame) and patches the constants
there, so a patch of the ring does nothing and a **patch of the source reaches every mine 3 to 5 frames later** (the shader
recompiles). The patch replaces slot `0x13`, `MAD R0.xyz, R1, {2, -1}.x, .y` (the map's `* 2 - 1`), with
`MAD R0.xyz, R1, c.w, c.xyzw` (stored words `0e000400 c8041c9d fe020001 c8020001`, halves swapped), `c = (nx, ny, nz, 0)`, so
the map is replaced by a constant `N` and `rim = sat(1 - dot(Vn, N))`. `K[0x3c]` (`0x008c1a18`) is held at 20 so every lit mine
sits at `p = 1`. **The scene target is read through the GDB stub, not `/proc/<pid>/mem`**: the football draws target local
`0x01920000` (pitch `0x2000`, 1300 x 1080, `A8R8G8B8`; `scripts/rsx-draw-list.py` on a `hd-draw-dump.py` ring), and with `Write
Color Buffers` on RPCS3 writes it back lazily, so a `/proc` read returns a stale picture of another moment; a stub read row by
row (`0x1450` bytes a row, about 2 min for the frame) returns the paused frame, the display grab agreeing.

**The target holds encoded colour, and the program is decoded off it to the digit** (confidence 85): on the football's body the
bytes read `(R, G) = (118, 95)` and `(145, 116)` (`B` 221 and 240); `(x / 255)^2.2` gives linear `R / G = 1.59` and `1.60`, the
program's `1.75 / 1.1 = 1.59`. On a patched frame the rim decoded off `R` (`R_lin / 0.875`) and off `G` (`G_lin / 0.55`) agree to
2 % on every ring of every mine read. So **the original's mine body is bright before any post**: rim 0.2 to 0.35 on the body.

**With `N = (0, 0, 1)` (the map flat), the rim near the ball's centre reads 0.1 to 0.35, but that is not an off-normal eye
vector** (confidence 70). A first reading here took it for one (30 to 50 degrees off-normal); a held-pose read withdrew it. At **one
pause**, with the patch applied, the frame's ring (`hd-draw-dump.py`, `HOOK_RING_ONLY=1`), every football draw's `c256..c263`,
`c465..c467` and vertex array, and the scene target were read, and the vertex block `#15` was evaluated per vertex offline
(`TC1 = (V.T', V.B', V.N')`, `T'`, `N'` through `c256..c258`, `B' = cross(N', T') (2 w - 1)`, `V = c465 - R5`). The projected
vertices land on the balls in the target (the RSX picture may be a frame older than the ring, a few pixels). Radial profile of the
nearest clean mine (projected radius 34 px, image 31 px), rim measured against rim predicted from the live constants:

| `r / R` | 0.2 | 0.4 | 0.6 | 0.8 |
| --- | ---: | ---: | ---: | ---: |
| measured | 0.24 | 0.29 | 0.47 | 0.68 |
| program as read, live constants | 0.06 | 0.16 | 0.45 | 0.75 |

The edge agrees; the inner rings are 0.1 to 0.15 high, where the spikes that point at the camera (the same program, so also
patched flat, and seen side-on: rim near 1) lie over the body. A second boot with the map unpatched gave the same pattern on a
12 px mine (`0.29, 0.31, 0.55, 0.73` measured, `0.13, 0.13, 0.45, 0.62` predicted with the `_n` map sampled at the vertices'
UVs, level 3). Two other mines of the first held frame overlap on screen and were not used. **So the program as read, fed the
live constants, reproduces the original's body to within the spikes' overlap**: the body's whiteness is the program's own output,
`rim` 0.2 to 0.35 already reading near white after the encode, and no separate input term is missing. `N = (0, 0, 100)` draws the
bodies black (the white is this program's output); `N = (0, 0, 0)` saturates.

**Ruled out as a difference in the inputs** (each 85): the map's sampling (unit 0 at the football draws, D12e's ring dump: format
`0x000b8829`, DXT45, 11 levels; address `0x60010101`, gamma nibble 0, no signed remap; remap `0xaae4`; filter `0x02063e80`,
trilinear, LOD bias `-1.5`, no signed bits; the `_n` mip chain is flat on the body); texture
coordinate control (`TC1 = 0`, `TC3 = 0`, neither forced 2D); RPCS3's fetch (`RSXVertexFetch.glsl`: `CMP` as `11:11:10` to
`[-1, 1]`, `UB` as `/255`, the port's `rcsmodel::unpack_normal`); the glow mask (the draw's colour mask is `0x00010101`, alpha off;
blend off; alpha test `GREATER 0x7f`).

**Still open:** what makes ours darker is then outside the body's program inputs. The `mines-ahead` pair shows the
original's near mine with long white spikes over the body and a ball about twice ours' on screen, and ours with no spikes visible
there: the spikes' draw (their own `_n` map, `spikeScale`) and the pair's picture lag ([detonator-mine-draw.md](detonator-mine-draw.md),
"The displacement") are the next places to measure. The bitangent stretch (`B'` scales with the part's row scale, 75) is part of
the reading above and is small at the pair's scales.

**Pair** (`mines-ahead`, re-rendered at the merged tree, no change in this lane): whole-frame mean grey 13.62; near ball 60 x 60
box centred on each image's own ball (theirs at `1060, 520`, ours at `1090, 515`, 1920 x 1080) RGB mean theirs
`(170, 146, 227)`, ours `(78, 41, 84)`. The post chain is not the gap: `oag_post::hd_bloom` already adapts on the encoded mean
([hd-track-tone.md](../../../rendering/hd-track-tone.md)), and the original is bright before it.

**2048 and Omega: not checkable** for this finding: it is a live constant of HD's draw, and the cause of the white body is
open on HD itself, so there is nothing yet to compare. Their mine programs are unread; both name the same world-space
`eyePositionWorldSpace` uniform ([gxp.md](../../../formats/gxp.md), [gcn.md](../../../formats/gcn.md)), which the port already
feeds to every title.

**Chosen, not measured:** the normal is the map through the chunk's tangent frame in the world frame
(the program dots a tangent-space `V`; the angle is the same); a divisor at or below zero draws
black; the football band (a key hash for `Libc_Rand() % 4`); the envelope steps once per tick, not
per rendered frame; the bomb is colour times brightness; the bomb flash is `0` (no writer read). The
band levels are the raw array (`Scene::set_band_levels`, fed from `Spectrum::levels()` next to
`set_blur_resolution` in the three draw sites and the capture).

**Size breathing wired** (2026-11-23, `det-mines`): a live mine's football is drawn at `scale = clamp(2.3 s, 1, 2.3)` and its
spikes at `clamp(4 t, 1, scale + 0.95)`, about the mine's centre, from the same envelope as the brightness
(`MineEnvelope::{football_scale, spike_scale}`; `RaceView::mine_sizes` is the one source for the draw, the shadows and the
blur's previous pose; the scene steps the envelope before it writes the matrices). A dark mine stays at `1` (its `m+0xd8`
is `0`; what the draw does with it is unread). **Live check** (`detonator-mine.md`, "Where a mine stands"): one laid mine
read `s = 0.5886`, `t = 0.568`, `m+0xd8 = 1.3537`, `MineBrightness = 1.1771`, `MineColour = (0.4784, 0.098, 1.0)`: `2.3 s`,
`2 s` and stage 1's `(122, 25, 255)` to the digit, so the size, brightness and colour laws are lifted to **85** (the
colour order too). Chosen: what the original reads as one `s` per mine over a history, ours steps once per tick from the
frame's bands with no history, so a pair's mine is the original's size only if its `s` is carried (not done).

**Not wired:** the equalisers' rays (`Colour`, `Brightness`) and the glow edges (`RainbowColour`,
`Brightness`) keep their authored values (their names are shared with other families and their
programs are unread); the visualiser (`Electric_Visualiser`) is drawn since `det-mine-visualiser`
([detonator-mine-draw.md](detonator-mine-draw.md), "The visualiser, wired").

## What the port needs

1. **Stage colour.** `EffectSettings::stage_rgba8(stage, "Detonator Mine Colour")` already answers
   these four keys (they are authored as integers, `crates/tables/src/effectsettings.rs`;
   `Mine Electricity`, `Bomb Inner` and `Bomb Outer` likewise). Four `key::` constants are missing.
   The table is `detonatormode` / `detonatormodedlc3` (`crates/hd/src/race.rs` already names both).
   The stage is `ZoneGrade`'s committed lap-stage counter (`zone_grade.rs`, `show_stage`, `blend.current`,
   not the requested one): the same number the original's getter reads, and the colour **snaps** on a stage step.
2. **Live and dark.** `oag_weapons::detonator::Placed::live` is the sim's reading of
   `DetonatorMine_SetDark`: a dark mine uses the literals `(2,2,2)` / `(10,10,10)` / brightness
   `0`, not the stage colour.
3. **Brightness.** `s` per mine, from `oag_audio::spectrum::Spectrum::levels()` (read in
   `crates/game/src/app/session/draw.rs`, `self.audio.output().spectrum().levels()`): football band
   `idx` in `0..4`, spikes band `idx + 11`, one `idx` per mine. The envelope is applied **per call**
   in the original, and the call rate (once per mine update; the only step the function reads is
   `PitchOffset`'s, from the stub `0x00677538`, `0x00426780`, taken to be the frame step) is not measured; a wire lane applies it once per rendered frame and says so.
   The per-mine `idx` is the original's `Libc_Rand`; the port's own choice (for example the lane
   index modulo 4) is **chosen, not measured**, and must never touch the simulation (render side
   only, like the rest of the spectrum).
4. **Where it reaches the draw** (done, "Wired" above). The constants are per-mine and per-bomb, set
   every frame. Before this lane no per-draw named-constant path existed in `oag-mesh`: the HD material readers in
   `crates/mesh/src/mesh/rcs/` (`alpha_emissive.rs`, `chevron_pulse.rs`, `curve_track.rs`) read a
   material's authored parameters once, at load, into a slot's lanes
   (`mesh::rcs::skin`). The mine and the bomb each need a per-instance override of two or three
   `float4` slots, fed from `crates/raceplay/src/detonator_models.rs` (`DetonatorModels`,
   `DetonatorDrawables`, the spikes, equalisers and glow edges pools) and the mines and bombs pools
   in `crates/raceplay/src/scene/weapon_models.rs`/`scene/frame`. `MineColour` per stage is a
   constant between stage steps, so a per-stage rebuild of the pool's slot is enough for it; the
   brightness needs a per-frame write. A new path of this kind is the larger part of the wiring
   (several hours, not minutes): size it before taking the lane.
5. **What not to do first.** Do not draw the mine with the authored defaults and call it done: the
   defaults (`MineBrightness 0`, `MineColour (0.239, 0.56, 1.0)`) are the dark-mine look only if the
   shader reads them as authored. They are read now: see "The mine's fragment program".

## What stays open

- **How the bomb's fragment programs combine their constants.** The mine's is read ("The mine's
  fragment program"). The bomb body's and equalisers' are not beyond their last multiply by the
  brightness: the overlay terms, the environment picture and `Envmap` are unread, so the draw is
  the chosen colour times brightness. The flash `b+0x154` has no recovered writer, so it is `0`.
- **What a dark mine's negative output looks like in the original.** `1 - 0.9 * 2` is negative;
  the render target's treatment is unobserved, so a dark mine draws black here.
- **Four-lane copies.** `MineColour` is bound by a pointer to three floats followed by other
  fields (`m+0x144` after `m+0x138`); whether the bind copies three or four words is unread
  (`Material_BindInstanceParamPointer`'s copy width). The w lane is harmless if the shader reads
  `.xyz`, which the authored `0.0` fourth lane suggests.
- **The bomb binds this function never writes:** equaliser `RainbowColour` (`b+0xfc`), `Envmap`
  (`b+0xdc`), `V_Offset` (`b+0xe0`) and the glow edges' `U_Offset_Speed`, `Distortion`, `Speed`,
  `V_Offset`, `Brightness` (`b+0x120`..`+0x138`): the blast's own update
  (`DetonatorBombBlast_Update`, `0x00136420`, [detonator-bomb.md](detonator-bomb.md)) is the next
  address to try. The writers of the flash `b+0x154` and the phase `b+0x15c` are unread too.
- **Runtime words:** `*(0x00994464 + 4)` (mine `TrebleHit`), `*(0x00994420 + 0xc)` (bomb
  equaliser brightness) are `.bss` and read `0` in the file. The level helper `0x00304cc8`
  (`MasterVolume`, `PitchVolume`) is read: "The visualiser's level". None of them feeds the football, spikes or bomb
  colours, only the visualiser and the equaliser brightness.
- **A live capture.** Nothing here ran. A read watchpoint on `m+0x100` and `m+0x138` of a laid
  mine in RPCS3 (the football's `MineBrightness` against a known stage colour) would lift the law
  to 85+; `det-pairs` holds the RPCS3 session.

## The visualiser's level (2026-11-26, `det-mine-pitch-volume`)

**`MasterVolume` and `PitchVolume` are one level, not two bands.** `DetonatorMine_AnimateToMusic` calls the same
reader twice in its live branch and scales the one result two ways:

```text
0x00139774  bl 0x00304cc8 ; f1 = L
0x00139788  lfs f0, 0x10(r28)          ; K[0x10] = 5.0   (r28 = TOC -0x2b78 = 0x008c19dc)
0x00139790  fsel/fsel with f30 = 0.0 (TOC -0x2b70), f29 = 1.0 (TOC -0x2b90)
0x001397a0  stfs f13, 0x104(r31)       ; MasterVolume = clamp(5 L, 0, 1)
0x00139ad4  bl 0x00304cc8 ; f1 = L again
0x00139ae0  lfs f0, 0x48(r28)          ; K[0x48] = 1.0
0x00139aec  lfs f13, 0x4c(r28)         ; K[0x4c] = 0.4
0x00139b0c  stfs f12, 0x10c(r31)       ; PitchVolume = clamp(1.0 L, 0, 0.4)
```

The `.data` words at `0x008c19dc`: `+0x10` `0x40a00000` (5.0), `+0x48` `0x3f800000` (1.0), `+0x4c` `0x3ecccccd` (0.4).

**`SoundSystem_GetSpectrumTotal` (`0x00304cc8`)**, read end to end:

```text
g   = *(TOC +0x7784)                   ; g_sound_system (0x00b6cc90 live, zone-visualiser.md)
n   = min(g[0xe8], 6)                  ; li 6 / subfc / subfe / and / add: the byte, capped at 6
if 0x002f6160():  return **(g + 0x61c) / 2.0    ; TOC +0x77cc = 0x40000000
else:             return **(g + 0x61c) / (float) n
```

`g[0xe8]` is the byte [zone-visualiser.md](zone-visualiser.md) reads as the speaker layout (`2` stereo; a channel
count, capped at 5.1's six). `0x002f6160` tests a flag byte `+0x7c` and a word `== 2` of another object (unread). In
stereo both branches divide by `2`, so on a stereo capture `L = (record +0x61c)[0] / 2`. Its third caller,
`0x002faef8`, is a bare thunk. Confidence **85** for the reader and its two uses (read at instruction level, constants
from the file, the live relation below).

**What `+0x61c` holds is the zone page's reading, confidence 70**: `SoundSystem_InitBandAnalysis` writes a third
record at `+0x61c` over bins `0..511` ([zone-visualiser.md](zone-visualiser.md), "What a band measures"), the
sixteen band records' sibling over the whole spectrum. So `L` is the music's whole-spectrum magnitude over the
channel count, **not ranged** against its own history the way the sixteen bands are. Its absolute scale (the SPU's
transform's normalisation) is not measured.

**Live** (RPCS3, the `mines-ahead` grab's shot record, `det-mine-spin`; one value for every updated mine of a read):

| Race clock | `PitchVolume` (= `L`) | `MasterVolume` |
| ---: | ---: | ---: |
| 2.5333 | 0.1414 | 0.7069 |
| 2.5500 | 0.1055 | 0.5276 |
| 2.5667 | 0.1055 | 0.5276 |
| **2.5833** (the picture's read) | **0.3154** | 1.0 |
| 2.6667 (the pause) | 0.1074 | 0.5372 |
| 2.6833 | 0.1263 | 0.6317 |

`MasterVolume = 5 x PitchVolume` at all four unclamped values and `1.0` where `5 L > 1`: one level, two scalings.
The `0.4` ceiling never binds here (static only). `L` swings threefold between neighbouring reads, so a comparison
must take the picture's read, never the pause's. Five off-screen mines hold `0.2766` on every read and sixty hold `0`:
a mine's words move only while it is animated.

**Ours is wired** (2026-11-27, `det-mine-volume-reland`): `visualiser_volumes` applies the two clamps to one level,
`oag_audio::Spectrum::total`, the sum of all 512 bins' magnitudes over 1024. **Its scale against the original's is
chosen, not measured** (over two seconds of a music WAV it spans 0.10 to 0.65, median 0.17, the original's order).
A comparison pins each mine's two words from the picture-clock read (`--detonator-mine-volumes M,P;...`, from
`original.detonator.mines`; `compare-rerender-ours` adds it to a pair recorded before the flag). The pair pins the
law's outputs, not `L`, since a mine that was not animated holds stale words. A feed with no music (a fixed or ramp
band feed, `--no-audio`) reads `L = 0` for any mine not pinned, so its ring loses the lightning; the `pad` record
carries no mines, so ours' own seeded ones draw that way.

| Pair | Before | After |
| --- | ---: | ---: |
| `mines-ahead` | 9.0662 | 9.0626 |
| `cannon-volley` | 9.1588 | 9.1513 |
| `cannon-volley-hot` | 4.8241 | 4.8247 |
| `pad` | 3.2363 | 3.2373 |
| `endrace-record`, `endrace-norecord` | 5.1548, 5.9386 | unchanged |

Reproduce: copy `data/reference/compare-page/hd/detonator-talons-*` and call `compare-rerender-ours.rerender` on each
`pair.json` (`docs/tools/compare-page.md`). Mine boxes against the original (mean absolute grey difference, before to
after): `mines-ahead` `620x340+780+330` 12.579 to 12.542, where the near ring's rim turns jagged like the original's;
`cannon-volley` `500x260+720+350` 21.649 to 21.681; `cannon-volley-hot` `360x120+880+380` 18.618 to 18.641; `pad`
`260x90+930+390` 21.305 to 21.379. The two cannon pairs' rings move by no visible amount at 1920 wide.
**`cannon-volley-hot` rises by 0.0006 and `pad` by 0.001**: the law is kept because it is read, not because a pair
improves.

An earlier wiring (2026-11-26, commit `5fac71320`, reverted) measured `mines-ahead` 12.5060 to 12.5155 on older
frames; there `MasterVolume` carried 0.25 grey and `PitchVolume` 0.01.

## Correction: the dark branch does not submit the visualiser (2026-11-24, `det-mine-look-re`)

The dark-mine block above says "Both models and the visualiser are re-submitted". The two calls at `0x00139ccc` and
`0x00139cec` pass `m+0xcc` and `m+0xd0` (the football and the spikes); `m+0xd4`, the visualiser, is posed by
`DetonatorMine_PoseVisualiser` (`0x00139308`), a camera-facing billboard scaled `1 + 0.4 (m+0xd8 - 1)` live and `0.9`
dark: [detonator-mine-draw.md](detonator-mine-draw.md). That is also the reader of `m+0xd8` this page left open.

## Lineage (2048 and Omega)

- **2048 v1.04: checked, applies, not wired.** (the mine models' vertex declaration, and so the coordinate set the map reads, is not read there.) `eboot.elf` carries the plain strings `MineColour`,
  `MineBrightness`, `BombColour`, `BombEQColour`, `ElectricityColour` and the four
  `Detonator ... Colour` keys (one hit each, `grep -c`). The per-stage values are HD's: 2048 ships
  `data/ZoneEnvironmentHDFury/DetonatorModeHDFury.effectSettings` in `data2.psarc`, **byte-identical**
  to HD's `detonatormode.effectsettings` (MD5 `1105c3ee...`). The function bodies are not read; the
  law is inherited from HD and unwired.
- **Omega: checked, applies for the stage colours, differs in how the binds are named.**
  `Data/zoneenvironmenthdfury/DetonatorModeHDFury.effectSettings` is byte-identical to HD's (same
  MD5) and the executable carries the four `Detonator ... Colour` key strings, but **no plain
  `MineColour` or `BombColour` string**: the executable holds the hashes as constants instead
  (`~crc32("MineColour") = 0xd7883cf6`: two little-endian hits; `MineBrightness`: two;
  `BombColour` and `BombBrightness`: one each). So Omega binds the same names, by precomputed hash.
  The Omega function bodies are unread. Not wired.
- **The visualiser's level** (2026-11-26, `det-mine-pitch-volume`). **2048: not checkable**: its mine animate
  function is not located and its band source is untraced (NGS audio, [zone-visualiser.md](zone-visualiser.md)).
  **Omega: checked, differs** in the source: its spectrum comes from CRI's `ACX Spectrum Meter`, not MultiStream's
  records ([zone-visualiser.md](zone-visualiser.md)), so no `+0x61c` record exists to read; its mine animate function
  is not located, so whether it reads one whole-spectrum level for both words is unread.
  **2048 and Omega run HD's law, inherited from HD** (2026-11-27): their rings draw through the same
  `visualiser_constants`, fed ours' whole-spectrum level; neither has a pair that was measured.
