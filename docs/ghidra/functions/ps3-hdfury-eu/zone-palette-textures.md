# HD's Zone palette textures: `zoneAnisoPalette*` and `GradientColour*` are built at runtime from the stage table

`EBOOT.elf` (Wipeout HD/Fury, PS3, `ps3-hdfury-eu`). Read 2026-11-11 (`hd-zone-stage`)
off `llvm-objdump -d --mcpu=pwr6` with every TOC load resolved against the function's
own OPD entry (`scripts/ps3-toc.py`, [memory.md](memory.md)), then **read live** on
RPCS3 through `/proc/<pid>/mem` during a Racebox Zone race.

**The question** ([zone-effectsettings-loader.md](zone-effectsettings-loader.md),
twenty-third and twenty-fourth passes): what fills the two-entry arrays at
`0x00c81330`-`0x00c81350` that `Scene_PrepareFrame` publishes as shader parameters
65, 66 and 68-71 (`zoneAnisoPalette`, `zoneAnisoPaletteOuter`, `GradientColour`,
`GradientColour1..3`). The earlier pages had no filling write and only 60 (then 78,
[zone-shader.md](zone-shader.md)) that they hold textures at all.

**The answer: no file. The loader creates seven pairs of tiny FP16 textures, and the
stage blend rewrites their texels from the effectSettings stage table each time a
stage commits.** The aniso palette is a 17-texel ramp over five authored colours; a
gradient is three authored colours, one per texel. Confidence 92 on the palette law
and 90 on the gradients: a static read, then two live captures (a circuit and the
`Zone_1` arena, 1,248 palettes over stages 1-4): every authored control point exact,
every interpolated texel within one FP16 ulp, every gradient texel equal to the live
stage's `Gradient<k>.Colour<j>`.

## 1. Seven arrays, not five, and who publishes which

Each array is two words, indexed by the same `max(e, 0)` the stage-texture samplers
use (`e` = `*0x008c1430`). `Environment_UpdateStageBlend` (`0x003da540`) takes `e` as
its second argument, reads `craftArray[e]->+0x640` with it, and for `e == 1` only
copies `H[0]` into `H[1]` (`0x003da678` -> `0x003dc738`), so the second entry is a
second local player's (split screen). Confidence 70 on that reading; entry `[0]` is
the one a single-player race draws, and the live capture below shows only `[0]`
written.

| array | `0x00c7dfb0 +` | texture | Scene block publishes as | Track block publishes as |
| --- | ---: | --- | --- | --- |
| `0x00c81330` | 13184 | 3x1 FP16, `Gradient1.Colour0..2` | 68 `GradientColour` **and** 69 `GradientColour1` | the same |
| `0x00c81338` | 13192 | 3x1 FP16, `Gradient2.Colour0..2` | 70 `GradientColour2` | the same |
| `0x00c81340` | 13200 | 3x1 FP16, `Gradient3.Colour0..2` | 71 `GradientColour3` | the same |
| `0x00c81348` | 13208 | 17x1 FP16, Scene ramp, stage `n` | 65 `zoneAnisoPalette` | - |
| `0x00c81350` | 13216 | 17x1 FP16, Scene ramp, stage `max(n-1,0)` | 66 `zoneAnisoPaletteOuter` | - |
| `0x00c81358` | 13224 | 17x1 FP16, Track ramp, stage `n` | - | 65 `zoneAnisoPalette` |
| `0x00c81360` | 13232 | 17x1 FP16, Track ramp, stage `max(n-1,0)` | - | 66 `zoneAnisoPaletteOuter` |

**The Scene block** is `Scene_PrepareFrame` (`0x003aa888`, TOC `0x008bd3c4`): each
`lwzx` off a TOC slot is followed by `+32` (the texture header skip) and a single
`stw` to the parameter's `+0x1c`:

```text
3ab6b8  lwz  r3,-25020(r2)   ; 0x008b7208 -> 0x00c81330
3ab6c8  lwzx r9,r3,r11       ; [max(e,0)]
3ab6f0  stw  r6,2204(r9)     ; 68*0x20+0x1c  GradientColour
3ab6f4  lwzx r9,r3,r10       ; the same array again
3ab724  stw  r4,2236(r9)     ; 69  GradientColour1
3ab718  lwz  r8,-25016(r2)   ; 0x00c81338 -> 3ab758 stw 2268  70 GradientColour2
3ab74c  lwz  r8,-25012(r2)   ; 0x00c81340 -> 3ab78c stw 2300  71 GradientColour3
3ab780  lwz  r8,-25008(r2)   ; 0x00c81348 -> 3ab7c0 stw 2108  65 zoneAnisoPalette
3ab7b4  lwz  r10,-25004(r2)  ; 0x00c81350 -> 3ab80c stw 2140  66 zoneAnisoPaletteOuter
```

**The Track block** of the seven two-block publishers (e.g. `FUN_003fc140` from
`0x003fd5a0`) loads the second TOC set in the same order: `-20700` (`0x00c81330`,
twice), `-20696` (`0x338`), `-20692` (`0x340`), then `-20644` (`0x00c81358`) and
`-20640` (`0x00c81360`) where the Scene block had `0x348`/`0x350`. Confidence 78 on
the Track pairing, read off the load order, which matches the Scene block's
one for one. The same split holds for parameter 67 `zoneAnisoPower`: every
publisher's Scene block loads `-20648` (`0x00c81460`, the Scene power pair) and its
Track block `-20604` (`0x00c814e0`, the Track pair).

Every word in the image that points into `0x00c81320`-`0x00c81368` is one of these
twelve TOC slots (a full scan of the loadable segments), and only module B
(TOC `0x008bd3c4`) can reach them.

## 2. The loader creates them: `Environment_LoadStageTextures` (`0x003d6dc8`)

Base register `r21 = 0x00c7dfb0`. Fourteen calls to the texture-creation wrapper
`0x005aa3a0` ([renderer.md](renderer.md), `Texture_BuildGcmRegisters`; arguments
`(out, 0, 1, width, height, 1, format word, ...)`, read the same way as
[zone-effectsettings-loader.md](zone-effectsettings-loader.md)'s 64x10 glyph):

```text
3d8040  li r6,17 ; li r7,1 ; ori r9,..,0xaae4 (lis 0x1a00)  ; 17x1, 0x1a00aae4
3d8098  lwz r3,13208(r21) ; stw r31,13208(r21)              ; release old, keep new
        ... eight times: 13208, 13212 ... 13236               ; 0x00c81348..0x00c81364
3d8498  lwz r11..r3, 13208..13236(r21) ; lwz rX,16(handle)   ; each texture's data pointer
3d8500  stw r0..r23, 14964..14992(r21)                       ; kept at 0x00c81a24..
3d84e4  li r6,3                                               ; 3x1, same format word
3d855c  stw r31,13184(r21)  ... six times to 13204           ; 0x00c81330..0x00c81344
3d8878  stw r0..r26, 14996..15016(r21)                       ; their data pointers
```

`0x1a00aae4`'s format byte `0x1a` is `CELL_GCM_TEXTURE_W16_Z16_Y16_X16_FLOAT` with
its two layout flags off: **FP16 RGBA, 8 bytes a texel**, which the writer's `sth`
stores and the live bytes below both confirm. The ramp needs it: DLC3 authors
`Base Colour Highlight Middle` up to `2.85`, which a unorm texture would clamp.

## 3. The writer: `Environment_UpdateStageBlend` on a stage commit

The rebuild sits on the commit path of `Environment_UpdateStageBlend` (`0x003da540`),
after `H[e].+0x00 := H[e].+0x04` (`0x003da74c`), and is skipped while the two agree
(`0x003da69c`). `r31 = n * 0x250` (the new stage), `r9 = max(n-1, 0) * 0x250`
(`0x003da7bc`-`0x003da7e8`, kept at `3840(r1)`), `r27 = 0x00c7efb0`
(`g_effect_settings_stages`), `r24 = 0x00c7dfb0 + 4e`. Offsets are the stage record's
([zone-effectsettings-loader.md](zone-effectsettings-loader.md), fourteenth pass).

**Four ramps** through `Environment_BuildAnisoPalette` (`0x003d9970`):

| call | texels to | control points `r5..r9` | `f1` |
| --- | --- | --- | --- |
| `0x003dab70` | `14964(r24)` = `0x00c81348[e]` | stage `n` `+0x10 +0x40 +0x50 +0x20 +0x30` | `n +0x248` Scene Aniso Curve |
| `0x003dabac` | `14980(r24)` = `0x00c81358[e]` | stage `n` `+0x70 +0xa0 +0xb0 +0x80 +0x90` | `n +0x24c` Track Aniso Curve |
| `0x003dabf8` | `14972(r24)` = `0x00c81350[e]` | stage `n-1`, Scene offsets | `n-1 +0x248` |
| `0x003dac44` | `14988(r24)` = `0x00c81360[e]` | stage `n-1`, Track offsets | `n-1 +0x24c` |

So the five points are, in texel order: **`Near Colour`, `Base Colour`, `Base Colour
Middle`, `Base Colour Highlight`, `Base Colour Highlight Middle`** of the Scene
(`+0x10/+0x40/+0x50/+0x20/+0x30`) or Track (`+0x70/+0xa0/+0xb0/+0x80/+0x90`) group.
The two "Middle" keys are the second and fourth points, not midpoints of anything.

**Three gradients**, written directly, no cross-fade, stage `n` only: each
`Gradient<k>.Colour<j>` vec4 (`+0xc0 + 0x30*(k-1) + 0x10*j`) is split with `vspltw`,
converted with `0x005f7a60` (float to half) and stored as texel `j` of
`0x00c81330/338/340[e]` (`0x003dac48`-`0x003db0c0`, nine texels, `sth` at `+0/+2/+4/+6` of texel
`+0`, `+8`, `+16`).

**`zoneAnisoPower`**: the same commit writes the Scene pair `(n +0x180, n-1 +0x180)`
to `0x00c81460` (`0x003da964`-`0x003da9a4`) and the Track pair `(n +0x184, n-1 +0x184)`
to `0x00c814e0` (`0x003daa94`-`0x003daac8`). So `.x` is the inner (new) stage's power
and `.y` the outer (previous) stage's, matching the shader's `pow(rim, power.x)`
inner and `.y` outer.

## 4. `Environment_BuildAnisoPalette` (`0x003d9970`), the ramp law

Arguments `(r3 = 0, r4 = texel pointer, r5..r9 = P0..P4, f1 = curve)`. Read at
instruction level (the decompile carries a phantom `auRam00000000`):

```text
3d9a44  lfs  f0,-23052(r2)     ; 0x008b79b8 = 0.5
3d9a50  fmuls f30,f1,f0        ; a = curve * 0.5
3d9a58  vsubfp v30,v28,v30     ; P1-P0     (and P2-P1, P3-P2, P4-P3 below)
3d9b4c  vmaddfp v31,v30,a,P0   ; texel 1
3d9be8  vmaddfp ..,v30,0.5,P0  ; texel 2   (0.5 from the vector at *(-22472(r2)) = 0x007b1f20)
                               ; texel 3 = P0 + (P1-P0)*(1.0 - a), 1.0 at 0x008b7974
```

```text
for segment i in 0..3:
    texel[4i + 0] = P[i]
    texel[4i + 1] = P[i] + (P[i+1] - P[i]) * (curve / 2)
    texel[4i + 2] = P[i] + (P[i+1] - P[i]) * 0.5
    texel[4i + 3] = P[i] + (P[i+1] - P[i]) * (1 - curve / 2)
texel[16] = P[4]
each lane converted to FP16 by 0x005f7a60
```

`Aniso Curve` bunches the interior texels toward the segment's ends (`0`) or its
middle (`1`); `0.5` (every stage of `zonemodedlc3.effectsettings`) is an even
quarter spacing. The shader reads the ramp at `pow(rim, zoneAnisoPower)`
([zone-shader.md](zone-shader.md)), so `Near Colour` is the head-on colour and
`Base Colour Highlight Middle` the silhouette's.

## 5. Read live (RPCS3, 2026-11-11)

`scripts/rpcs3-hd-zone-palette.py`: a cold boot into Racebox Zone (`TTY.log`:
`GetMode()==6`; the race flew `01_Vineta_K`, so the base `zonemode.effectsettings`
table, [zone-palette-select.md](zone-palette-select.md)), cross held for 150 s,
158 samples at 1 Hz of the seven handle arrays, their texel bytes (the handles'
`+0x10` data pointers, which equal the loader's `0x00c81a24`.. copies; RSX local
memory at `0xc3b0fe00`.. reads through the same mapping) and `H[0]`. The craft
climbed stages 1 to 4.

- **Only entry `[0]` of each array is written**; entry `[1]`'s texels are
  uninitialised (`ff ff ff ff ...`). Single player, as section 1 reads.
- **The ramp law, against all 632 sampled palettes** (4 ramps x 158 samples): taking
  texels 0, 4, 8, 12, 16 as `P0..P4`, every interior texel equals the law's value to
  within **one FP16 ulp** in every lane (the half conversion's rounding).
- **The control points, against the file**: every point the table authors -
  `Base Colour` and `Base Colour Highlight` for both groups, both the inner stage `n`
  and the outer `max(n-1,0)` - equals the authored value **exactly** at FP16, 1,264
  of 1,264. E.g. stage 4 Scene: `P1 = (0.6196, 0, 2.0)` = `4 Flash.Scene.Base Colour`
  `0.619608 0.000000 2.000000`, `P3 = (0, 1.4824, 7.0)` = `Base Colour Highlight`
  `0.000000 1.482353 7.000000`; its outer ramp's `P1 = (0, 0.1, 1.5)` = stage 3's.
- **What the base table does not author is not a constant.** `zonemode.effectsettings`
  carries no `Near Colour`, `Base Colour Middle`, `Highlight Middle` or `Aniso Curve`,
  and the live `P2`/`P4` differ by stage (`(1,1,1)`/`(5,5,5)` at stages 1-2,
  `(1,4,1)`/`(3,5,5)` at 3, `(0,4,1)`/`(0,5,5)` at 4): whatever the record held.
  Irrelevant to a port: no racing circuit compiles the `zoneAnisoPalette` shape
  ([zone-shader.md](zone-shader.md)), and the four Zone arenas take
  `zonemodedlc3.effectsettings`, which authors all five points, the curve and the
  power on all fifteen stages.
- **Gradients**: 3 texels `(1,0,0) (0,1,0) (0,0,1)` in all three arrays at every
  stage. The base table authors no gradient key, so this checks the layout (three
  colours, FP16, one a texel) and not the key mapping; DLC3 authors exactly these
  three values for `Gradient2`/`Gradient3` on every stage it ships.
- The palettes change exactly on the four stage commits seen, and not between.

Reproduce:

```sh
scripts/emu-env.sh <lane> <display> <gdb-port>; source data/scratch/<lane>/env.sh
OAG_EMU_STALL=300 uv run --with evdev --with pillow python3 scripts/emu-run.py --silence 300 \
  -- python3 -u scripts/rpcs3-hd-zone-palette.py --image <hdfury-ps3-eu-dec.iso> --out <dir>
```

A fresh private RPCS3 tree compiles its PPU cache on first boot (over 100 s, past the
boot stall limit); seed it from `~/.cache/rpcs3/cache` first. The Racebox remembers
the last race type, so the script verifies each RIGHT tap on the race-type box and
refuses a race whose mode line is not `GetMode()==6`.

### The arena, `Zone_1` (Pro Tozo), the same day

Same script attached to a `serve` walked by hand to Racebox, Zone, Track Select,
PRO TOZO (`TTY.log`: `Loading World "Data\Environments\Zone_1\track.vex"`,
`GetMode()==6`), so the table is `zonemodedlc3.effectsettings`, which authors all
five points, the curve and the power on every stage. 154 samples, stages 1 to 4:

- **616 palettes, all 3,080 control points** (five a ramp, all authored) equal the
  file's values exactly at FP16; every interior texel within one ulp of the law with
  the file's own `Aniso Curve` (`0.5`).
- **Gradients**: at stage 4, `Gradient1` reads `(0.8823, 1.0, 0.0902) (0.7529,
  0.8628, 0.7529) (2, 2, 2)` against `4 Flash.Scene.Gradient1.Colour0..2` =
  `0.882353 1 0.090196`, `0.752941 0.862745 0.752941`, `2 2 2`; `Gradient2`/`3` the
  file's `(1,0,0) (0,1,0) (0,0,1)`.

Frames of the same run (start grid at stage 1, the track at stages 1-4) are the
original's side of the arena comparison in
[zone-shader.md](zone-shader.md)'s port notes: grey textured track and white
scenery kept, the stage's colours only where the albedo is black.

## 6. What this does not settle

- **Which draws sample `GradientColour*`.** [zone-shader.md](zone-shader.md) has
  `GradientColour1` on unit 0 (the albedo slot) of `zone_2/materials/gradientcolour1`;
  the full census of materials that declare the four gradient samplers is not run.
- **Whether `0x005f7a60` rounds or truncates**; one ulp either way.

## Names applied

| address | kind | name | confidence |
| --- | --- | --- | ---: |
| `0x003d9970` | function | `Environment_BuildAnisoPalette` | 92 |
| `0x00c81330` | data | `g_ZoneGradient1Textures` | 88 |
| `0x00c81338` | data | `g_ZoneGradient2Textures` | 85 |
| `0x00c81340` | data | `g_ZoneGradient3Textures` | 85 |
| `0x00c81348` | data | `g_ZoneAnisoPaletteTextures` | 90 |
| `0x00c81350` | data | `g_ZoneAnisoPaletteOuterTextures` | 90 |
| `0x00c81358` | data | `g_ZoneTrackAnisoPaletteTextures` | 85 |
| `0x00c81360` | data | `g_ZoneTrackAnisoPaletteOuterTextures` | 85 |
