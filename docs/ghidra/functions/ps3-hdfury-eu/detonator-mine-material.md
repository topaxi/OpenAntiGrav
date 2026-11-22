# Detonator: what writes the mine's and the bomb's shader constants

Functions in `EBOOT.elf` (WipEout HD Fury, PS3, EU, `BCES00664`), Ghidra program
`/hdfury/EBOOT-ps3-hdfury-eu.elf`. Static reading only, 2026-11-22: nothing here ran. Bodies read
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
MasterVolume m+0x104 = clamp01(5.0 * Level0())           ; 0x304cc8, K[0x10]: unread
BassHit m+0x110 = clamp(s * 0.25, 0, 0.25)               ; K[0x28] = 1.0, K[0x2c] = 0.25
PitchOffset m+0x108 += step ; PitchVolume m+0x10c = clamp(Level1() * 1.0, 0, 0.4)
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
tangent-space normal (`TEX R0, f[TC3]`, then `* 2 - 1`), `TC3` the second coordinate set. Writing
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
literal is read; the order of the two `x4`/`/2` scales is the encoding's), 60 for "the render
state is opaque" (the engine's reading of the chunk's state word lists no cutout and no
see-through chunk for either model, unlike `HD_bomb_sphere`; the original's blend is not observed).

The bomb body (`detonator_bomb_normal`, `@0x1750`) is a longer program: a normal-mapped facing
term `0.92 N.V` and `0.2 BombColour` through an overlay-shaped `x / (1 - ...)`, a power by
`4 sat(1 - |TC2|^2) + 1`, an environment picture read at `4 dot(TC2, TC0)` on unit 1 (the
`Envmap` weight patches the first constant), a second overlay, and **a last multiply by
`BombBrightness`** (`@0x3f`, confidence 80). Only that last step is used (below).

## Wired

HD Detonator races draw the mine from the program above (`oag_mesh::mesh::rcs::mine_rim`,
`Emissive::program` mode 23, `shade.wesl`): the football and the spikes each earn a glow-table
entry holding the authored `MineColour` and `MineBrightness`, the `_n` map is bound third and the
chunk carries its tangent. The values per instance are `oag_raceplay::detonator_look`'s: a live mine
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

**Chosen, not measured:** the normal is the map through the chunk's tangent frame in the world frame
(the program dots a tangent-space `V`; the angle is the same); the draw is opaque and the output
alpha unused; a divisor at or below zero draws black; the band levels are the visualiser's held
levels (`zone_hold`, a fall of `0.1` a frame), not the raw array `SoundSystem_GetBandLevel` returns;
the envelope steps once per tick, not per rendered frame; the bomb flash is `0` (no writer read).

**Not wired:** the equalisers' rays (`Colour`, `Brightness`) and the glow edges (`RainbowColour`,
`Brightness`) keep their authored values (their names are shared with other families and their
programs are unread); the mine's size (`scale`, `spikeScale`, `m+0xd8`) stays at `1`; the visualiser
(`Electric_Visualiser`) is still not loaded.

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
4. **Where it reaches the draw.** The constants are per-mine and per-bomb, set every frame. Today no
   per-draw named-constant path exists in `oag-mesh`: the HD material readers in
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
   shader reads them as authored. See the next section.

## What stays open

- **How the fragment program combines the constants.** Unread: the `detonator_mine_normal_alpha`
  fragment program (`/data/weapons/materials/detonator_mine_normal_alpha.rcsmaterial`, 31,872 bytes)
  and its bind of `MineColour`/`MineBrightness`. Next step: `oag_rcs::rcsmaterial::fragment::Program`
  on that material's variant (the readers `alpha_gated_parameter`, `output_depends_on` and
  `patches`, as `mesh/rcs/alpha_emissive.rs` uses them), then whether `MineBrightness` multiplies
  the colour or the glow. Until then the dark mine's `(2,2,2)` against `0` and the live mine's
  `[0,2]` brightness against a `[0,1]` colour cannot be turned into a picture. Today's white
  football is the missing binds on a material the generic path does not know.
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
  equaliser brightness) are `.bss` and read `0` in the file; the level helper `0x00304cc8`
  (`MasterVolume`, `PitchVolume`) is unread. None of them feeds the football, spikes or bomb
  colours, only the visualiser and the equaliser brightness.
- **A live capture.** Nothing here ran. A read watchpoint on `m+0x100` and `m+0x138` of a laid
  mine in RPCS3 (the football's `MineBrightness` against a known stage colour) would lift the law
  to 85+; `det-pairs` holds the RPCS3 session.

## Lineage (2048 and Omega)

- **2048 v1.04: checked, applies, not wired.** `eboot.elf` carries the plain strings `MineColour`,
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
