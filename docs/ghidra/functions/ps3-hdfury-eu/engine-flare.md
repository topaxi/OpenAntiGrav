# The engine flare: `flame_test.rcsmaterial`, read program by program

Read 2026-08-23, with
[`scripts/ps3-microcode.py`](../../../../scripts/ps3-microcode.py) over
`/data/materials/ships/engines/flame_test.rcsmaterial` - the one material every
Wipeout HD craft's `engineflare.rcsmodel` names. **No function in `EBOOT.elf`
was read for this page and it adds no `names.tsv` rows**: the microcode lives in
the material file on the disc, the way
[renderer.md](renderer.md)'s fog and bloom readings do, and a row without an
address would be a name with nothing behind it.

The asset side - which craft ship the pair, how the node tree splits it, and
what the engine now draws - is on
[trail-ribbon.md](../../../rendering/trail-ribbon.md). This page is only the
shader.

## What the flare is

`Data\Ships\<Team>\engineflare.vex` plus the `.rcsmodel` beside it, on all
fourteen craft (twelve teams, Zone and Detonator). 961 to 2,141 triangles.
**One** material each, always
`data/materials/ships/engines/flame_test.rcsmaterial`, always
`src 0x0302`/`dst 0x0001` - `GL_SRC_ALPHA`/`GL_ONE`, additive, the same
equation the ribbon's material authors. The first texture varies by team
(`data/ships/flame_test/flame_01.gtf`, `data/weapons/textures/flame_01.gtf`,
and Detonator's own), there is no second texture, and every one of them is
64x64.

## The names, by hash preimage

The same `~crc32` technique [renderer.md](renderer.md) used to recover
`fogColour`, run over this material's declaration tables. Seven preimages,
each an exact hash match on a plain-English identifier:

| Hash | Name | Where |
| --- | --- | --- |
| `0xb9d31b0a` | `position` | vertex attribute, slot 0 |
| `0xde7a971b` | `normal` | vertex attribute, slot 1 |
| `0x427214fc` | `Uv1` | vertex attribute, slot 2 |
| `0x7493d450` | `VertexColour1` | vertex attribute, slot 3 |
| `0x2e7d5f33` | `viewProj` | vertex parameter, `c256` |
| `0x3466fc0e` | `eyePositionWorldSpace` | vertex parameter, `c465` |
| `0x961a1154` | `scale1` | fragment parameter |
| `0xaa5e39a1` | `power1` | fragment parameter |
| `0xfa51b981` | `min1` | fragment parameter |
| `0x31182e0d` | `Speed` | fragment parameter |
| `0x906b67ba` | `time` | fragment parameter |

Four were unresolved: `0x92fc84bf` and `0x17d9b3d3` (both `float1`),
`0x4c13d3af` (`float2`) and `0x3fbddeab` (the sampler on unit 0). **One of them
is recovered**: `0x4c13d3af` is `globalAlphaScaler`, engine parameter slot 76 -
not from a wordlist but from the engine's own parameter table
([renderer.md](renderer.md#the-engines-own-parameter-table-read-from-its-initialiser-2026-08-24)),
which is also where `time` and the ribbon's `engineTrail` come from. The other
three are not engine parameters, so they are model-authored and their names are
not in the executable at all. Confidence 90 on the resolved names - a CRC-32 preimage on a short readable identifier is
not a coincidence, and the four that stay open are what a wordlist misses
rather than what the method gets wrong.

## The vertex program, block #1

```text
 0  MOV o[TC0].w, v[2].xxxx          <- Uv1.x
 1  MOV o[TC1].w, v[2].yyyy          <- Uv1.y
 2  MOV o[TC1].xyz, v[1].xyzx        <- normal
 3  MUL R0.xyz, v[0].xyzx, c[210]    \  position * positionScale
 4  ADD R0.xyz, R0.xyzx, c[211]      /  + positionBias
 5  MOV R0.w, v[3].wwww              <- VertexColour1.w
 6  ADD o[TC0].xyz, -R0.xyzx, c[209] <- eyePositionWorldSpace - world position
 7  MUL R1, R0.yyyy, c[1]            \
 8  MAD R1, R0.xxxx, c[0], R1         |  viewProj
 9  MAD R1, R0.zzzz, c[2], R1         |
10  MOV o[TC2], R0                   <- (world position .xyz, VertexColour1.w)
11  ADD o[POS], R1, c[3] | END       /
```

So `TC0` is the **view vector** with `Uv1.x` in its `w`, `TC1` the **world
normal** with `Uv1.y` in its `w`, and `TC2.w` is the colour set's fourth byte,
carried through untouched.

### The live vertex program, read off RPCS3 frame dumps (2026-11-26, `hd-flame-rim-capture`)

**The program the console runs is not the listing above.** Rebuilt from the stream's
`TRANSFORM_PROGRAM` uploads on every flame draw of three existing frame dumps of the original
(`data/reference/hd-flame-tap/`, the `hd-craft-ibl-probe` frames: `c1`, stem `f`, one craft at rest at Talon's
Junction's start; `c2`, stem `f`, the same craft at thrust; `d1`, stem `m`, a grid of eight at rest), the same 20 instructions
each time:

```text
 0  MOV o[TC3].yz, v[2].xxyx           <- Uv1
 1  MOV R0.w, v[3].wwww                <- VertexColour1.w
 2  MUL R1.xyz, v[1].yyyy, c[1].xyzx   \
 5  MAD R1.xyz, v[1].xxxx, c[0].xyzx    |  normal through the plain model matrix
 9  MAD R1.xyz, v[1].zzzz, c[2].xyzx   /
 3  MUL R0.xyz, v[0].xyzx, c[210].xyzx \  position * positionScale + positionBias
 4  ADD R0.xyz, R0.xyzx, c[211].xyzx   /
 6-8,10                                <- R0 = M * R0 + c[3]: the world position
11  ADD o[TC1].xyz, R1.xyzx, c[208].xxxx
12-14,16                               <- clip = viewProj (c[4]..c[7]) * R0
15  ADD o[TC0].xyz, -R0.xyzx, c[209].xyzx  <- eye - world position
17  MOV o[TC2], R0
18  MOV o[POS], R1
19  MOV o[TC3].x, R1.wwww | END
```

The program's `c[N]` is the register file's `c[256 + N]`: `c[0..3]` is the per-draw model matrix
at `c256`-`c259`, `c[4..7]` the view-projection at `c260`-`c263`, `c[208]` is `c464`, `c[209]` is
`c465`, `c[210]`/`c[211]` are `c466`/`c467` (1/128 and the chunk's bias).

| Finding | Evidence | Confidence |
| --- | --- | --- |
| **`E` (`c465`) is the camera's world position.** It equals the camera centre solved from the view-projection (the point whose clip x, y and w are zero) on all 73 flame draws, largest gap 0.00003 units | three dumps: `c1` 5 draws, `c2` 5, `d1` 63 (eight craft, eye 7.5 to 147 units away) | 92 |
| **`V = E - world position`**: the position is taken through the model matrix before the subtraction, so both are world space | the listing, every flame draw | 92 |
| **The normal is `M * n` with the plain model matrix**, not its inverse transpose, plus `c464.x`, which is 0 on every draw | the listing; `c464` read per draw | 90 |
| At rest, the length-scaled draws carry rows (0.75, 0.75, 0.187) and (0.75, 0.75, 0): the hull's 0.75 times (1, 1, 0.25) and (1, 1, 0), which is `hd_flame_transform`'s `EF_Main` and `EF_Boost` at rest. Which draw is which shape is not read here (some `d1` craft carry three zero-length draws) | `c1`; `d1`, every craft | 75 |

Seen on three frames, two at rest (`c1`, `d1`) and one at thrust (`c2`); **no boosting frame was dumped**, so
boost is covered only by the law (boost moves `M`'s scale, not where `E` comes from). Reproduce:
`python3 scripts/rsx-flame-eye.py data/reference/hd-flame-tap/c1 f --program`.

### The live fragment program is block #2, and it halves its colour (2026-11-26, `hd-flame-colour-gap`)

**The console runs the fogged permutation, not block #1.** The five rest flame draws of `c1` (518 to 522)
bind fragment programs at local `0x748500`, `0x7487c0`, `0x748a80`, `0x748d40` and `0x749000`, all the same
42 instructions, and with the inline constants masked they equal `flame_test.rcsmaterial`'s fragment
block #2 instruction for instruction (blocks #3 to #5 differ in 8 lines each). Read with the patched
constants, it is block #1's arithmetic plus a fog lerp and a closing halving:

```text
@0x07  ADD_SAT R0.w, -dot(V, N), 1           <- rim
@0x0a  MUL R0.x, TC3.x, {fog}.w               <- clip w * 0.001
@0x0d  MAD R1.w, {2}, {429.579}, Uv1.y        <- Speed * time (c1's clock: 429.579 s)
@0x17  MAD H0.x, rim^10, {0.3}, {0.45}        <- f
@0x1a  MAD H2.x, -f, {2}, {2}                 <- 2 (1 - f)
@0x1e  MUL H3.w, H2.x, TC2.w                  <- alpha, neither fogged nor halved
@0x21  MUL R0.w, -(d * 0.001)^2, 1/ln 2
@0x23  TEX H0.xyz, (2u, 2v + noise) unit0
@0x26  EX2_SAT R0.w                           <- fog factor exp(-(0.001 d)^2)
@0x27  MAD /2 H0.xyz, factor, tex - fc, fc    <- rgb = mix(fc, tex, factor) / 2
```

`fc` is `(0.262745, 0.215686, 0.380392)` with density `0.001` in `.w`, the circuit's fog: 325 of the 378
fragment programs bound in `c1` carry the same four numbers. So:

| Finding | Evidence | Confidence |
| --- | --- | --- |
| **The flame's colour is halved: `rgb = mix(fc, tex * colourScale, f) / 2`.** The `/2` is the same closing scale every HD lit program ends on (`Light::output_scale`, 0.5 on HD) | `@0x27 MAD /2` on all five draws; block #2 on the disc carries it too | 90 |
| The live program is block #2, the fogged permutation | 42 instructions equal block #2's with constants masked | 90 |
| The fog is the circuit's (`fc` above, density 0.001, EXP2 on clip `w`); at chase distance (about 12 units) its factor is 1 to four places | the constants, shared by 325 of 378 programs in the frame | 85 |
| The flame's `time` is a boot-relative clock (429.579 s in `c1`), not the race or scenery clock the comparison pairs record; it is the frame context's `+0xc4` ("The flame's clock at a pair's grab") | `@0x0d`'s patched constant | 80 |

**Wired**: the flame surfaces' block carries the circuit's output scale (`oag_raceplay`'s `flame_scene`),
which `shade.wesl`'s flame branch already multiplied by; it read `Scene::off`'s 1.0 before, so the rest
flame drew at twice the original's linear strength. The fog is not wired: the block is shared with the
disc shield shells and the cockpit sphere, nothing measured fogs those, and the factor is 1 at chase
distance. Reproduce: `python3 scripts/ps3-fp-live.py data/reference/hd-flame-tap/c1/f-c0748500.bin`.

**The blue/pink split on `hd/talons` is the noise phase, not a layer.** `flame_01.gtf` (64x64 DXT5) is a
vertical gradient, black to blue, with two pink rows near `v` 0.78 and 0.92, and its alpha is 2D cloud
noise. The colour tap reads `(2u, 2v + noise)` with the noise scrolled by `2 * time`, so which part of
the nozzle lands on a pink row turns over every 0.5 s. Ours (halved, camera eye) at `--anim-seconds`
0.0 and 0.125 shows the original's blue left and pink right; the other six of eight phases show other
patterns. The pairs made before 2026-11-26 do not record the flame's boot clock, so such a still pair
cannot match the phase; the pair script now records it ("The flame's clock at a pair's grab" below).

### The flame's clock at a pair's grab

Measured 2026-11-26 (`hd-flame-clock`), RPCS3, HD Fury EU (BCES00664). The flame's `time` is the frame context's
boot clock: `f32` at `(*0x00936fd4) + 0xc4` (`Render_FrameContextPtr_q`, the context `Game_PresentLoop_q` hands
the render calls; engine-trail.md, "The draw state's two non-material constants, placed"). Every HD grab now
records it beside the scenery clock (`grab_with_clock` writes `flame_clock_s` in `<stem>.clock.json`;
`rpcs3_hd_field.flame_clock`), the pair script stores it as `original.flame_clock_s` and runs ours with
`--flame-clock` at the recorded value less the lag below (`compare_env.flame_pin`), and a re-render adds the flag
to an HD pair whose record holds the field. Ours' `--flame-clock` sets the flame surfaces' block's clock alone
(`oag_raceplay`'s `flame_seconds`); every other animation keeps the world clock.

| Finding | Evidence | Confidence |
| --- | --- | --- |
| **The flame program's `time` is `+0xc4`**, read one to two frames behind the PPU | two frame dumps of the Racebox weapons grid state with `+0xc4` read at every pause: 53 flame programs (`@0x0d MAD R1.w, R1, {t}`, `@0x27 MAD /2`, the fog constants) all hold 41.2949 against `+0xc4` 41.3283 (lag 0.0334 s); the second dump's slot-0 time 88.4302 against 88.4469 (lag 0.0167 s) | 85 (two reads, one state) |
| It runs at game time and stops with the emulator: frozen under a GDB pause | five pauses over 12 s on the `hd-race/talons-pack` state, each read twice 1 s apart, bit-identical; it advanced 12.0 s across them | 85 (one state) |
| **Within a run its gap to the scenery clock is constant; across boots it is not.** So a pair made before the field existed is clocked only when its own timeline is a known state | `hd-race/talons-pack` state, five pauses: `+0xc4` less the scenery clock 48.57702 to 48.57722, less the lap clock 115.1591 to 115.1593. Across runs the gap was 39.77 (a grid state), 54.78 and 57.01 (two cold boots) | 85 (one state within; three runs across) |

The pin a pair passes is `flame_clock_s - 0.025` s (`compare_env.FLAME_CLOCK_LAG_S`, the mean of the two lag
reads), within 0.009 s of either read, 2 % of the noise's 0.5 s period.

**Of the 22 pairs above, one is clocked from its record.** `hd-race/talons-pack` was made on the restored
`hd-race-talons-venom-pack` state with no restart: its record's scenery-less-lap offset 66.58214 equals the restored
state's 66.58212, so its flame clock at the grab is its `world_clock_s` 73.89487 plus the state's 48.5771 = 122.4720
s (derived from the timeline's offset, not read at the grab). Every cold-boot pair (`hd/talons` and the eleven other
Time Trial grids) and the Detonator pairs (their `grid-late` state is not kept) need a regrab with the current
script (the cold-boot ones are regrabbed below); the far-hit pairs were filmed after a restart.

**The phase, on two regrabbed pairs and one clocked pair.** Ours re-rendered at the pin and at 20 clocks 0.025 s apart
across one 0.5 s period; mean absolute RGB error (0 to 255) over the pixels the flame clock moves, each render
bit-identical on a second run:

| pair | moved pixels | free (world clock) | at the pin | best of 20 | pin's rank |
| --- | --- | --- | --- | --- | --- |
| Talon's Junction, `harimau_c1`, cold boot, flame 410.2580 | 1,679 | 22.947 | **18.384** | the pin | 1 of 20 |
| Talon's Junction, `harimau_c1`, cold boot, flame 135.1249 | 1,761 | 19.698 | 19.453 | 19.122 (+0.10 s) | 12 of 20 |
| `hd-race/talons-pack` (clock 122.4720, eight phases 0.0625 s apart) | 1,251 | 34.420 | 34.403 | 34.025 | 7 of 8 |

On the first regrab the pin is the sharpest minimum of the sweep; on the second the error is flat (19.1 to 19.5)
over most of the period and the pin sits in that flat band, away from the two dips (21.4 to 23.3); on
`talons-pack` the flame is a distant craft's and the whole sweep spans 0.4 on an error of 34, so it cannot
discriminate. On the second pair's nozzle box `944,750 32x26` the pin read 30.85 against the free phase's 31.36,
and an 8-phase sweep ranked the pin 4th of 8: the falsifier written before the capture ("the pin among the best 2 of
8") failed on that pair. **Phase match at the pin: confidence 60** (one sharp confirmation, one flat, one blind).
The two regrabbed originals carry RPCS3's "Press and hold the START button to resume" toast top left and fly the
Fury hull (this profile's Ship Select lands on `harimau_c1`), so they are measurements, not replacements for
`hd/talons`, and the `hd/talons` nozzle box `935,802 57x58` (standard hull) was not re-measured.

**The cold-boot pairs, regrabbed with the clock (2026-11-27, `hd-flame-regrab`).** All 14 cold-boot pairs of the list
(`original.provenance.capture_path.kind == "cold_boot"`: the 12 Time Trial grids, `talons-assegai-c1`, `zone-vineta-k-z5`) were
regrabbed on RPCS3 0.0.43-20574 with `rpcs3-hd-cold-pair.sh`'s steps (profile `hd-fury`, hull shake patched out), each from its own
cold boot, with the hull the stored pair names checked twice: by RAM (the new `the player's hull (RAM) is the hull ours draws` check,
`Harimau` or `Assegai_c1`) and by eye against the stored frame. That day's save opened Ship Select on the Fury `Harimau_c1` and had
Pilot Assist on, so the walk took one `down` and the option was written off before `Restart Race` (compare-page.md, "The save
moved since"). Ours was re-rendered from scratch copies at the recorded pin and at 7 more phases 0.0625 s apart (one 0.5 s noise
period), twice per phase: both runs were pixel-identical on every pair. "Crop" is the mean absolute RGB difference (0 to 255) from
the original over the pixels any phase moves; "nozzle" is the same mask clipped to `880..1040 x 760..900`, where every grid pair's
own flame sits (on Vineta K, Anulpha Pass and Chenghou Project the clock also moves 11,000 to 35,000 other pixels of the scene).
Rank is the pin's place among the 8 phases (1 is best). Mean grey is the page's whole-pair score at the pin.

| pair | flame clock (s) | moved px | crop at pin | rank | nozzle at pin | rank | mean grey |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `hd/talons` | 105.9366 | 1,625 | 16.24 | 4 of 8 | 16.24 | 4 of 8 | 2.834 |
| `hd/talons`, second boot | 104.5756 | 1,593 | 15.43 | 2 of 8 | 15.43 | 2 of 8 | 3.246 |
| `hd/vineta-k` | 96.5750 | 34,783 | 3.90 | 2 of 8 | 14.93 | 2 of 8 | 2.522 |
| `hd/anulpha-pass` | 105.2061 | 33,522 | 3.80 | 2 of 8 | 7.84 | 2 of 8 | 4.637 |
| `hd/moa-therma` | 97.7607 | 1,534 | 12.51 | 1 of 8 | 12.51 | 1 of 8 | 2.617 |
| `hd/chenghou-project` | 99.7491 | 11,018 | 7.60 | 3 of 8 | 11.59 | 3 of 8 | 2.307 |
| `hd/metropia` | 100.3077 | 1,683 | 14.56 | 1 of 8 | 14.56 | 1 of 8 | 2.720 |
| `hd/sebenco-climb` | 108.7260 | 1,395 | 18.04 | 2 of 8 | 18.04 | 2 of 8 | 3.930 |
| `hd/ubermall` | 111.1978 | 1,148 | 19.95 | 3 of 8 | 19.95 | 3 of 8 | 4.226 |
| `hd/sol-2` | 103.4375 | 1,455 | 15.41 | 5 of 8 | 15.41 | 5 of 8 | 3.470 |
| `hd/the-amphiseum` | 114.2185 | 1,529 | 16.29 | 1 of 8 | 16.29 | 1 of 8 | 4.762 |
| `hd/modesto-heights` | 115.2832 | 1,481 | 15.00 | 1 of 8 | 15.00 | 1 of 8 | 3.961 |
| `hd/tech-de-ra` | 107.9301 | 1,478 | 12.77 | 2 of 8 | 12.77 | 2 of 8 | 4.001 |
| `hd/talons-assegai-c1` | 100.9166 | 351 | 42.17 | 1 of 8 | 42.17 | 1 of 8 | 2.652 |

The falsifier, written before the scores were read, was "the pin in the best 2 of 8 on at least ceil(2N/3) of the regrabbed pairs".
On the 13 pairs measured (first boot of each) the pin ranks in the best 2 on 9: the bar is 9, met with no margin. A pin dropped at
random ranks in the best 2 one time in 4 (about 3 of 13). The misses are Chenghou Project (3), Ubermall (3), Talon's Junction (4)
and Sol 2 (5); Talon's Junction ranks 2 on a second boot and 1 on a third (a boot whose original still drew the Pilot Assist
bracket under the craft, so not adoptable; scored the same way, 34.22 against 36.24 to 39.47). One boot per circuit, so every other rank is seen once. **Phase match at
the pin: confidence 70** (raised from 60: 9 of 13 against about 3 by chance, the rest within one or two places but for Sol 2).
At the pin the Talon's nozzle shows the original's pink core where the worst phase (+0.25 s) is plain blue.

The stored `talons-assegai-c1` original flies another hull than ours draws: a narrow blue Assegai against ours' black
`Assegai_c1` (read by eye; that pair predates the RAM hull check). The regrab's original reads `Assegai_c1` in RAM and matches ours,
and its whole-pair score falls from the stored 4.78 to 2.65.

**The Zone pair was not measured here.** Both grabs of that lane landed at lap clock 10.0 to 10.1 s: the capture's default 12 s settle outlasts Zone's
3 s countdown. It was regrabbed inside the countdown with the flame clock recorded (compare-page.md, "The countdown frame", 2026-11-27); no 8-phase flame
sweep was run on it.

Reproduce: `python3 scripts/rpcs3-drive.py grab <png>` on an attached HD race writes `flame_clock_s`;
`python3 scripts/hd-draw-dump.py <dir> <stem>` dumps the frame's fragment programs and
`python3 scripts/ps3-fp-live.py <dir>/<stem>-<address>.bin` shows `@0x0d`'s constant.

## The fragment program, block #1

```text
@0x01-07  normalise TC0 and TC1, dot them
@0x0c     ADD_SAT R2.x, -R1, {1,..}     <- rim = saturate(1 - dot(V, N))
@0x08-0a  MOV R0.z, {Speed}; MAD R0.y, R0.z, {time}, R0.w
@0x11     TEX R2.w, R0 unit0            <- a first sample, at the scrolled row
@0x0f-17  LG2 / MUL by {power1} / EX2   <- pow(rim, power1)
@0x16     MOV H0.w, f[TC2]              <- the vertex colour's own alpha
@0x18-1a  MAD H0.x, R0, {scale1}, {min1}<- f = pow(rim, power1) * scale1 + min1
@0x1c     MAD H1.w, -H0.xxxx, {c}, {c}  <- alphaScale * (1 - f)
@0x1e     ADD R1.xy, R1, R2.zw          <- the first sample offsets the second
@0x1f     TEX H0.xyz, R1 unit0          <- the flame's colour
@0x20     MUL H0.xyz, H0, {colourScale}
@0x22     MUL H0.w, H1, H0 END          <- alpha = alphaScale * (1 - f) * TC2.w
```

**Every `{c}` above is attributed rather than guessed.** The patch chain
[renderer.md](renderer.md) documents - `param.fslot` -> `u16` at `block+fslot`
-> the sub-header's offset list -> the code slots it patches - resolves
one-to-one for this block:

| Parameter | fslot | code slot | What it is |
| --- | --- | --- | --- |
| `power1` | `0x7e` | `0x15` | the rim exponent |
| `min1` | `0x80` | `0x19` | the floor |
| `scale1` | `0x7c` | `0x1b` | the rim term's scale |
| `0x92fc84bf` | `0x7a` | `0x1d` | the alpha term's scale |
| `0x17d9b3d3` | `0x74` | `0x21` | the colour's scale |
| `Speed` | `0x76` | `0x9` | the scroll rate |
| `time` | `0x78` | `0xb` | the clock |

`@0x1c` reads the *same* inline constant for both operands, `.xxxx` each, which
is the `alphaScale * (1 - f)` above - and "one parameter patches both inline
constants of one instruction" is exactly the idiom `renderer.md` already
records for `fogColour`.

## The values are on the disc, in the model's own material record

**The `.rcsmaterial` declares the parameters; the `.rcsmodel` supplies the
numbers.** Past everything `oag_rcs::rcsmodel::material` used to read, a
material record carries a table of named values:

```text
+0x30  u32  entry count
+0x34  u32  offset of the entries, 0x20 bytes each
       +0x00 u32 name hash   +0x04 u32 kind (0x8001 sampler, 0 parameter)
       +0x18 u32 value offset (or the sampler's texture path)
       +0x1c u32 how many vec4s
```

Feisar's `engineflare.rcsmodel` carries eight entries - two samplers and the
six parameters above:

| Hash | Name | Value |
| --- | --- | ---: |
| `0xaa5e39a1` | `power1` | **10.0** |
| `0x961a1154` | `scale1` | **0.3** |
| `0xfa51b981` | `min1` | **0.45** |
| `0x92fc84bf` | (no preimage) | **2.0** |
| `0x17d9b3d3` | (no preimage) | **1.0** |
| `0x31182e0d` | `Speed` | **2.0** |

**All fourteen craft author the same six**, which is a measurement rather than
a reason to share one - `oag_mesh::mesh::Flame` reads them per craft.

So the whole program, with nothing left unrecovered - the clock included, as of
2026-08-24:

```text
rim   = saturate(1 - dot(normalize(V), normalize(N)))
f     = pow(rim, 10) * 0.3 + 0.45
alpha = 2.0 * (1 - f) * VertexColour1.w        -> 1.1 face-on, 0.5 edge-on
noise = texture(u, v + Speed * time).a         <- Speed 2.0, time engine-supplied
rgb   = texture(2*u, 2*v + noise).rgb * 1.0
```

**Two corrections to the line this replaces** (`rgb = texture(uv + scroll).rgb`),
both re-derived instruction by instruction and then confirmed against block #2,
which schedules the same arithmetic through entirely different registers:

- **The clock moves the *noise* sample only, in `v`.** `@0x0a` computes
  `R0.y = Speed * time + R0.w` with `R0.w = Uv1.y`, and `@0x11` samples `unit0`
  at `(Uv1.x, that)` taking its **alpha**. `time` never reaches the colour
  lookup; it reaches it only through the noise the colour lookup is displaced
  by. Block #2's `@0x0d`/`@0x13` do the identical thing out of `TC3.yz`.
- **The colour lookup is at *doubled* coordinates.** `@0x10` puts `Uv1.x` in
  `R1.x` and `@0x13` puts `Uv1.y` in `R1.y`; `@0x0e` puts `Uv1.x` in `R2.z` and
  `@0x12` puts `Uv1.y + noise` in `R2.w`; `@0x1e ADD R1.xy, R1, R2.zwzz` sums
  them to `(2*Uv1.x, 2*Uv1.y + noise)`. Block #2 reaches the same pair at
  `@0x1d ADD R0.xy, R0.yzxx, R1.zwzz`. Two independent schedules, same result,
  so the doubling is the program's and not a swizzle misread.

The models' authored `Uv1` is neither `0..1` nor `0..0.5` - Feisar's ten flare
nodes span roughly `0.19..0.81` in `u` and `0.37..0.86` in `v`, with one node
running `-0.91..2.26` - so the doubling is a deliberate difference of scale
between the noise and colour taps rather than an asset authored to be halved.
A one-off read prints those spans.

**The face-on 1.1 survives a race and clamps everywhere else.** A race draws
into `post::hd_bloom::SCENE_FORMAT`, a linear float target, where a fragment
alpha above 1.0 reaches the blend intact; the asset viewer and
the capture path draw the same models into a gamma target, where it clamps to
1.0 first. So the flame is measurably dimmer in those three than in the game,
by up to 10 % of its own contribution. A property of the target, recorded
rather than worked around.

**Confidence 92 on the layout** - it is validated disc-wide rather than on the
file it was read from: 20,445 parameters across 9,757 materials of 643
`.rcsmodel`s, and every one of them is a finite float of ordinary magnitude,
which a walk that was one field out could not be. Pinned by
`crates/rcs/tests/rcsmodel_material_ground_truth.rs`.

**Confidence 88 on the arithmetic**: the instructions are the program's own and
the patch chain is resolved, but the block was read once and the two unnamed
parameters are identified by the slot they patch rather than by a name.

### Three findings that fall out and are deliberately not acted on

- **`SpecScale` (`0x4232e459`) is authored per material instance** - 235 on
  Feisar's hull paint, 500 on its glass. [renderer.md](renderer.md) records the
  specular exponent as "32, the commonest of three round values ... a stand-in,
  now labelled as one", from a sweep of *inline* literals. The exponent is a
  per-instance parameter, and reading it is what would retire that stand-in.
  Nothing read it until 2026-10-21: it is the specular exponent where it patches the exponent's `MUL`, and a hull now draws with it ([hd-ship-materials.md](../../../rendering/hd-ship-materials.md), "The hull's specular exponent"; the value is 160 on `feisar_c1`'s paint, 300 on Harimau's).
- **`Bloom` (`0xf8f3ade0`) is 1.0 on `emissive_bloom`**, the material on a
  hull's lit strips.
- **`0xab31c2b1`** is 0.2 on the paint and 0.3 on the glass, with no preimage.

### The scroll's clock is read: engine parameter slot 0

`time` is **not** in any craft's material record, which is what said it is
engine-supplied. What supplies it is now read, on
[renderer.md](renderer.md#the-engines-own-parameter-table-read-from-its-initialiser-2026-08-24):
the executable initialises **81 named engine shader parameters** in
`Shader_InitEngineParams` (`0x003f1300`), each a 0x20-byte entry in the same
format the `.rcsmodel` material record uses, and **`time` is entry 0**. Every
draw-state builder read so far - `Trail_BuildDrawState` (`0x002e30d8`) and its
sibling `0x002a51e8` - splats the frame context's `+0xc4` into four lanes and
writes that address into entry 0's value pointer with a vec4 count of 1. That
field is the **global seconds clock** engine-trail.md read live at nine pauses
marching 48.97 to 119.37 at game-time rate: one value, shared by everything
drawn that frame.

So the flame's surface scroll is `Speed * time` = **`2.0 * seconds`** in `v` on
the noise tap - two full texture repeats per second. That is a falsifiable
prediction and it needs no emulator: if HD's flame does not visibly churn at
about that rate in any recording, something in this chain is wrong, and the
likeliest wrong thing is the clock's units rather than the arithmetic.

Neither of the two draw-state builders read so far is the one that draws the
flame *model*; the claim is that entry 0 holds a live clock every frame, which
both of them establish, not that the flare's own builder supplies it.

**Drawn as of 2026-08-24.** `oag_mesh::mesh::Flame` reads `Speed` as a sixth
authored number (2.0 on all fourteen craft and on all fourteen boost plumes -
`crates/raceplay/tests/hd_engine_flare_ground_truth.rs` pins it against the disc),
`mesh_render::Scene` carries the clock the way the original binds it - one
value splatted to four lanes - and `mesh.wgsl` takes both of the program's
taps. This project's clock is the race tick over 60, the same one the trackside
texture and node animation already ride, so `--anim-seconds` pins all three at
once and a replay of a tick draws the same frame.

**The rate is confirmed on screen by its own period.** `Speed` 2.0 against a
seconds clock predicts one whole texture repeat every `1 / 2.0` = 0.5 s, so a
capture of one tick at `--anim-seconds 0.5` must come back *identical* to the
same tick at 0.0 while intermediate phases must not. Four captures at tick
1050, differing pixels against the 0.0 frame out of 1,175,040:

| `--anim-seconds` | phase | differing pixels |
| ---: | --- | ---: |
| 0.125 | quarter repeat | 132 |
| 0.25 | half repeat | 71 |
| **0.50** | **one whole repeat** | **0** |

Exact recurrence at 0.5 s and at no smaller value tested. The counts are small
because the flame is a few hundred pixels behind the hull at chase distance -
they are not a measure of the effect's strength, and the zero is the load-bearing
row: an inert surface would read zero at *every* delta, and only a surface
scrolling at exactly `Speed` repeats per second reads zero at 0.5 and not at
0.25. What this does **not** settle is whether the original's clock ticks in the
same units; that wants the matched-view harness and a recording, and the
prediction above is what such a recording would falsify.

## Our rim's eye is the camera (wired 2026-11-26, after the colour's `/2`)

**Wired 2026-11-26 (`hd-flame-colour-gap`), with the colour's closing `/2`.** `flame_scene`
(`crates/raceplay/src/scene/scene_blocks.rs`) gives the flame surfaces' block (`SceneKind::Flame`) the view's eye as
`Fog::camera` and the circuit's `Light::output_scale`. Before, the block was `Scene::off` plus the clock: its eye was
`(0, 0, 0)` and its output scale 1.0, so the rest flame was measured from the wrong point and drawn at twice the
original's linear strength ("The live fragment program is block #2"). The history below is why the eye alone was
held back.

**The 22 pairs, re-rendered twice from the branch's binary on scratch copies** (base: `main` at `04c9e4cc9`; final:
the `/2` plus the eye). "Error in them" is the mean absolute RGB difference (0 to 255) from the original over the
pixels the change moved; "whole" is the mean absolute RGB difference over the whole frame. The final binary drew both
runs pixel-identical on every pair.

| pair | pixels moved | error in them | whole |
| --- | --- | --- | --- |
| `hd/anulpha-pass` | 3,260 | 16.22 -> 10.30 | 4.4339 -> 4.4246 |
| `hd/chenghou-project` | 2,407 | 14.51 -> 14.20 | 2.8652 -> 2.8649 |
| `hd/detonator-talons-cannon-volley` | 106,369 | 25.53 -> 25.62 | 17.8640 -> 17.8690 |
| `hd/detonator-talons-cannon-volley-hot` | 30,225 | 30.35 -> 30.14 | 14.8942 -> 14.8911 |
| `hd/detonator-talons-mines-ahead` | 31,074 | 35.42 -> 35.16 | 16.9525 -> 16.9486 |
| `hd/detonator-talons-pad` | 24,604 | 26.66 -> 26.61 | 11.0953 -> 11.0946 |
| `hd/metropia` | 1,808 | 21.15 -> 23.57 | 3.6836 -> 3.6858 |
| `hd/moa-therma` | 1,435 | 27.23 -> 25.53 | 6.3930 -> 6.3919 |
| `hd/modesto-heights` | 1,520 | 23.90 -> 18.01 | 4.7668 -> 4.7625 |
| `hd/rocket-craft-far-hit-0100` | (base not repeatable) | 37.06 -> 34.54 | 15.7094 -> 15.5777 |
| `hd/rocket-craft-far-hit-0233` | (base not repeatable) | 60.00 -> 59.46 | 46.5271 -> 46.2668 |
| `hd/rocket-craft-hit-0100` | 0 | | 72.6709 |
| `hd/sebenco-climb` | 1,489 | 22.89 -> 21.03 | 5.0902 -> 5.0888 |
| `hd/sol-2` | 1,556 | 30.55 -> 24.51 | 6.7846 -> 6.7800 |
| `hd/talons` | 1,505 | 20.54 -> 16.66 | 3.5015 -> 3.4987 |
| `hd/talons-assegai-c1` | 360 | 104.84 -> 106.83 | 5.0036 -> 5.0039 |
| `hd/tech-de-ra` | 1,978 | 22.55 -> 15.04 | 4.5423 -> 4.5351 |
| `hd/the-amphiseum` | 1,430 | 21.65 -> 20.84 | 6.0316 -> 6.0310 |
| `hd/ubermall` | 1,207 | 27.49 -> 22.54 | 4.5731 -> 4.5702 |
| `hd/vineta-k` | 2,284 | 21.02 -> 21.62 | 3.5188 -> 3.5194 |
| `hd/zone-vineta-k-z5` | 3,805 | 78.14 -> 75.08 | 6.4744 -> 6.4688 |
| `hd-race/talons-pack` | 1,337 | 33.21 -> 33.95 | 23.9628 -> 23.9633 |

On `hd/talons` the nozzle box `935,802 57x58` moves from 14.81 to 13.05 (mean RGB (36, 65, 94) to (34, 61, 91); the
original's (37, 60, 89)). Five pairs read worse on this metric: `hd/metropia`, `hd/vineta-k`, `hd-race/talons-pack`,
`hd/talons-assegai-c1` and `hd/detonator-talons-cannon-volley`. `hd/talons-assegai-c1`'s original frame shows a
different view of the craft than ours (error about 105 in both), so it does not compare a flame. The far-hit pairs'
base binary drew different pixels on its two runs (0.1 and 1.0 million), so their row is not evidence either way.
The other three are within the flame's phase: those pairs predate the flame clock's recording ("The flame's clock at
a pair's grab"), so each still frame samples the pink rows at an arbitrary phase. Averaged over eight phases across one 0.5 s period (the pair's
`--world-clock-offset` plus 0 to 0.4375 s, so the scenery barely moves), the error over the moved pixels reads, base /
`/2` alone / `/2` plus eye: `hd/metropia` 11.83 / 11.69 / 11.95, `hd/vineta-k` 4.40 / 4.40 / 4.44,
`hd-race/talons-pack` 30.94 / 29.81 / 31.51, `hd/detonator-talons-cannon-volley` 30.13 / 30.15 / 30.10. So the `/2`
never loses on the phase average, and the eye still costs up to 0.6 on three rest pairs.

The `/2` alone was rendered on all 22 pairs too: 13 better, 8 worse on the single-phase metric (`hd/zone-vineta-k-z5`
76.06 to 83.55 the largest, which the eye takes back to 75.08), against the pair's 17 better and 5 worse. Both are
wired because together they are the measured program and the better of the two on this table; what still separates
ours from the original on the rest pairs is open. A still pair cannot separate it from the phase; a pair regrabbed with
the flame clock recorded can ("The flame's clock at a pair's grab").

**Before the `/2` was found, the eye alone made the rest flame worse, so it was held back.** The change tried: `flame_scene`
(`crates/raceplay/src/scene/frame.rs`) gets `fog: Fog { camera: eye, ..Fog::off() }` (commit `0c86ed0ed`, reverted by
the commit after it). All 172 comparison pairs were re-rendered on copies with the build before and after: 150 are
byte-identical (every 2048, Omega, Pulse, Pulse PS2 and Pure pair, and 9 of HD's), 22 HD pairs move. "Error in them" is
the mean absolute RGB difference (0 to 255) from the original over the pixels the change moved; the other columns are
the page's whole-frame scores. On every grid pair with the craft at rest the error rises and ours gets brighter than
the original, a dim deep blue; the Zone pair, the Detonator pad and mines pairs and the far-hit pairs improve.

| pair | pixels moved | error in them | mean grey | rmse | edge |
| --- | --- | --- | --- | --- | --- |
| `hd/anulpha-pass` | 3,108 | 16.59 -> 19.84 | 4.224 -> 4.229 | 0.0468 -> 0.0468 | 0.4395 -> 0.4396 |
| `hd/chenghou-project` | 3,113 | 12.78 -> 20.58 | 2.693 -> 2.703 | 0.0376 -> 0.0378 | 0.3937 -> 0.3938 |
| `hd/detonator-talons-cannon-volley` | 38,230 | 30.37 -> 30.07 | 14.326 -> 14.321 | 0.1727 -> 0.1727 | 0.1037 -> 0.1044 |
| `hd/detonator-talons-cannon-volley-hot` | 29,567 | 30.61 -> 30.56 | 11.470 -> 11.468 | 0.1394 -> 0.1394 | 0.1019 -> 0.1016 |
| `hd/detonator-talons-mines-ahead` | 4,584 | 33.13 -> 31.71 | 13.631 -> 13.627 | 0.1682 -> 0.1682 | 0.1053 -> 0.1055 |
| `hd/detonator-talons-pad` | 26,178 | 22.27 -> 21.59 | 7.604 -> 7.598 | 0.1190 -> 0.1189 | 0.1804 -> 0.1820 |
| `hd/metropia` | 2,580 | 17.66 -> 30.86 | 3.469 -> 3.486 | 0.0379 -> 0.0382 | 0.4478 -> 0.4478 |
| `hd/moa-therma` | 1,375 | 27.61 -> 44.38 | 6.222 -> 6.234 | 0.0570 -> 0.0572 | 0.4520 -> 0.4520 |
| `hd/modesto-heights` | 1,480 | 22.57 -> 31.00 | 4.763 -> 4.769 | 0.0541 -> 0.0542 | 0.4181 -> 0.4181 |
| `hd/rocket-craft-far-hit-0100` | 185 | 93.48 -> 90.36 | 12.431 -> 12.430 | 0.1336 -> 0.1336 | 0.2789 -> 0.2789 |
| `hd/rocket-craft-far-hit-0233` | 188 | 65.87 -> 64.52 | 42.693 -> 42.693 | 0.2538 -> 0.2538 | 0.0521 -> 0.0521 |
| `hd/rocket-craft-hit-0100` | 46 | 195.51 -> 192.94 | 74.008 -> 74.008 | 0.3694 -> 0.3694 | 0.0122 -> 0.0122 |
| `hd/sebenco-climb` | 1,523 | 22.35 -> 33.66 | 4.800 -> 4.808 | 0.0531 -> 0.0532 | 0.3947 -> 0.3947 |
| `hd/sol-2` | 1,421 | 31.01 -> 41.40 | 6.325 -> 6.332 | 0.0674 -> 0.0675 | 0.4310 -> 0.4310 |
| `hd/talons` | 1,466 | 20.87 -> 26.44 | 3.319 -> 3.324 | 0.0395 -> 0.0395 | 0.4306 -> 0.4306 |
| `hd/talons-assegai-c1` | 331 | 105.84 -> 102.64 | 4.776 -> 4.775 | 0.0599 -> 0.0599 | 0.4090 -> 0.4091 |
| `hd/tech-de-ra` | 1,662 | 23.79 -> 26.37 | 4.517 -> 4.520 | 0.0447 -> 0.0447 | 0.4535 -> 0.4535 |
| `hd/the-amphiseum` | 47,791 | 15.75 -> 16.11 | 6.107 -> 6.116 | 0.0648 -> 0.0649 | 0.4157 -> 0.4157 |
| `hd/ubermall` | 1,115 | 25.91 -> 34.29 | 4.417 -> 4.422 | 0.0417 -> 0.0417 | 0.4487 -> 0.4487 |
| `hd/vineta-k` | 35,093 | 4.84 -> 5.86 | 3.221 -> 3.239 | 0.0393 -> 0.0396 | 0.4322 -> 0.4321 |
| `hd/zone-vineta-k-z5` | 4,444 | 74.02 -> 65.02 | 6.270 -> 6.251 | 0.0764 -> 0.0761 | 0.3010 -> 0.3011 |
| `hd-race/talons-pack` | 1,918 | 31.24 -> 33.46 | 23.054 -> 23.056 | 0.1711 -> 0.1711 | 0.0916 -> 0.0916 |

With the true view angle a face turned to the camera has `rim` near 0, so `alpha` is near its 1.1 maximum; from the
origin `rim` was larger and the plume dimmer. So something else in our flame is wrong and the origin eye hides part of
it.

**The space is settled: it is world space and the eye is the camera** ("The live vertex program" above, confidence
92). So the view's eye is the right value for `Fog::camera`, and the rest pairs get worse for another reason. Re-applied
on `hd/talons` and `hd/moa-therma` (2026-11-26), the moved pixels read, mean RGB:

| pair | original | ours, origin eye | ours, camera eye |
| --- | --- | --- | --- |
| `hd/talons` | (37, 66, 103) | (33, 74, 113) | (36, 85, 128) |
| `hd/moa-therma` | (58, 77, 106) | (79, 92, 128) | (95, 115, 159) |

Ours is already brighter and bluer than the original with the origin eye; the camera eye raises `alpha`, as the
program says it should, and so widens a gap that is not the eye's. On `hd/talons`, read once at 4x, the original's
nozzle shows a blue left half and a pink right half; ours is one even blue. Whether that is the flame's colour tap
(its scroll clock, the `(2u, 2v + noise)` displacement, the texel), the spikes or the nozzle behind is not yet shown;
the colour tap is the first suspect. Ruled out on the way: the rest
scales (they match), the normal transform (both plain `M`), and an alpha clamp (the original's target is
`A8R8G8B8`, surface format `0x3148` read off `c1`, so a fixed-point target that should clamp its 1.1 alpha to 1.0,
inferred, confidence 70; clamping ours to 1.0 moved no pixel of either pair either way).

The disc shield shells and cockpit spheres read the same block and are unaffected: with the eye supplied to them alone,
18 frames (HD, Omega, 2048, Pulse, Pulse PS2, Pure; chase at ticks 400 and 600, cockpit at 400) are byte-identical.
2048 and Omega flames: checked, differs. Their flares take `oag_livery::flare`'s `psp2_flare`, which sets no `Flame`,
so they are shaded without the rim; none of their pairs moved.

## Blocks #0 and #2, in one line each

`#0` computes the same rim term, writes a black `H0.xyz` and samples nothing -
a depth or mask variant. `#2` is `#1` with `fogColour` (`0x3dc31258`, already
named on [renderer.md](renderer.md)), the `float2` still unresolved, and a
closing `MAD /2` on the colour: the fogged permutation, and the one the console
runs ("The live fragment program is block #2" above).

## The runtime, read 2026-08-23

**`EBOOT.elf` is imported now**, so this section is the executable's half. Five
findings, and the one that matters most is the boost gate this project had
been carrying as a labelled guess.

### The per-team path is the executable's own literal

`0x00782388` holds `%s\\engineflare.vex`, labelled
`EngineFlare_ModelPathFormat`. So composing
`Data\Ships\<Team>\engineflare.vex` is not this project's convention applied
to a filename it found - it is the format string the loader itself uses, with
the team directory in `%s`, exactly as `oag_pulse::race::ships::entry_name`
assembles it. **Confidence 95.**

### The eleven shapes are bound by name, and the slot order is the split

`EngineFlare_PlaceShapes` (`0x002a1f00`, vtable slot 4) looks node names up in
the loaded model **once** - the whole lookup block sits behind a bound-yet flag
at `+0x184` - and stores each handle in its own field, `+0x15c` through
`+0x184`. Every frame after that it computes the transforms and submits the
shapes it bound.

The names come from the TOC pointer table at `0x008b2fbc`, and **their order is
the finding**:

| Slot | Name | Subtree in the `.vex` |
| --- | --- | --- |
| `+0x15c` | `ef_OuterShape` | `EF_Main` |
| `+0x160` | `ef_InnerShape` | `EF_Main` |
| `+0x164` | `ef_Spikes1Shape` | `EF_Main` |
| `+0x168` | `ef_Spikes2Shape` | `EF_Main` |
| `+0x16c` | `ef_Spikes3Shape` | `EF_Main` |
| `+0x170` | `ef_BoostLeftShape` | `EF_Boost` |
| `+0x174` | `ef_SpikesLeftShape` | `EF_Boost` |
| `+0x178` | `ef_BoostRightShape` | `EF_Boost` |
| `+0x17c` | `ef_SpikesRightShape` | `EF_Boost` |
| `+0x180` | `ef_DiamondsShape` | `EF_Boost` |
| `+0x184` | `ef_FlareShape` | **authored by no craft measured** |

`ef_OuterShape` (`0x0079be80`), `ef_BoostLeftShape` (`0x0079bed0`, labelled
`EngineFlare_BoostLeftShapeName`) and `ef_BoostRightShape` (`0x0079bf00`,
`EngineFlare_BoostRightShapeName`) are the three the decompiler shows as plain
literals; the rest are the same run of strings, `0x0079be80` to `0x0079bf58`.

**The engine never looks up `EF_Main` or `EF_Boost`** - it addresses each shape
individually. But its first five slots are exactly the five the `.vex` hangs
under `EF_Main` and its next five exactly the five under `EF_Boost`, in that
order, which is a partition arrived at from the other side of the asset.
`oag_mesh::mesh::groups::split` reaches the same one by walking the subtrees.
**Confidence 90.**

Two loose ends the table names. `ef_FlareShape` is a slot the runtime binds and
no craft on this disc authors, so it is either a shape only some model has or a
name that outlived its geometry. And the entry after the eleven, at
`0x0079bf58`, is `AlphaScaler` - looked up through a different call in the same
function, and **not** a shader-parameter hash (`~crc32("AlphaScaler")` is
`0xa657095c`, which matches none of `flame_test`'s), so it is another node.
Neither is followed here.

### The boost gate is a timer, a snap and an exponential decay

`EngineFlare_Update` (`0x002a3100`, vtable slot 5) is the per-frame method,
taking `dt` as its first argument. It carries a countdown at `+0x12c`:

```c
/* EngineFlare_Update */
*(float *)(this + 300) = *(float *)(this + 300) - dt;   /* floored at 0 */

/* EngineFlare_PlaceShapes */
if (threshold < *(float *)(this + 300)) {
    *(float *)(this + 0x144) = 1.0f;                    /* snap */
}
...
blend = *(float *)(this + 0x144) + *(float *)(this + 0x150);
```

and where the snap does not fire, `+0x144` is multiplied down by `(1 - k)` once
per substep - an exponential decay. That blend then scales the flare's own
transform.

**So the boost really is timer-driven, and the shape of the gate is the shape
this engine already had**: a countdown set by something else, a hard reveal
while it is above a threshold, a decay after. `oag_fx::exhaust`'s recovered
Pulse gate - `boost_timer`, `BOOST_GATE`, `BOOST_DECAY` - is the same three
parts. Wiring `EF_Boost` to `Exhaust::plume_visible` is therefore a *matched
mechanism* rather than a guess off a node name. **Confidence 88**, up from the
80 this carried when only the name was evidence.

**What *sets* `+0x12c` is narrowed but not closed** - the 2026-08-24 session
([engine-trail.md](engine-trail.md)) read the snap threshold as
`EngineFlare_BoostGate` = 0.2, the PSP's own gate value, watched live timers
decay from at most 0.70 in multiples of 1/30, and found one writer
(`0x00090d30`, mirroring a craft countdown at `craft+0x108`, gated on mode
10) plus two offset-collision red herrings in HUD code. Nothing in the
flare's own vtable writes it - slots 6 through 10 are lerp helpers
(`0x002a3708` linear, `0x002a3730` inverse-lerp, `0x002a3788` a packed-colour
lerp). What arms `craft+0x108` from pads, rolls and the start boost is still
unread, so *how long* a boost lasts on HD remains this engine's number.

### A second flare path exists, and it is not what draws the model

`0x0079bdd8` holds `Data/Tex/EngineFlare/Engine_Flare_Rich.gtf`, and
`0x007a1000`/`0x007a1010` hold the shader-registry names `engineflare_vp` and
`engineflare_fp`. Neither resolves through
[`scripts/ps3-registry.py`](../../../../scripts/ps3-registry.py), which reaches
62 of the 121 registered programs.

**It is not the program the flare *model* is drawn by, and the parameter table
is what says so.** Each craft's `engineflare.rcsmodel` supplies six values
whose hashes are one-for-one the six `flame_test.rcsmaterial`'s block #1
declares, each landing on a resolved code slot in that block. A program
registered in the executable has no per-model material record to read them
from; the material does, and that is the binding this renderer follows.

What the registry pair plausibly draws instead is a **billboard**:
`Engine_Flare_Rich.gtf` sits under `Data/Tex/EngineFlare/`, the same
sprite-shaped directory Pulse keeps `grabbedEngineFlare128x64x8.mip` in. So HD
may well draw a sprite flare *as well as* the model, which would be a second
thing to reproduce rather than a correction to this one. Open, with a stated
leading answer.

### The ribbon, from the same TOC block

`TrailEffectManager`'s own TOC neighbourhood carries the trail's whole
interface, and one entry of it corrects an asset choice this project had made
by name rather than by evidence:

```text
0x008b444c  'Trails'
0x008b4450  0x3f4ccccd                      = 0.8f, inline
0x008b4454  'position' 'normal' 'Uv1' 'VertexColour1'
0x008b446c  -> 0x007a2d70 'Data/RibbonEffects/enginetrail_bluered_triangle.vex'
                                                        Trail_ModelPath
0x008b4478  'TrailSpeed'
0x008b4484  'EngineTrail/TrailEffectManager.cpp'
```

#### The ribbon's own vertex program, read 2026-08-24

The fragment half below was read in 2026-08-23's pass; the vertex half was
not, and the two together are what say where the scroll is applied. Eleven
instructions, one block:

```text
 0  MOV o[TC0].xyz, v[3].xyzx          <- VertexColour1.rgb
 1  ADD o[TC1].xyz, -v[0].xyzx, c[211] <- eyePositionWorldSpace - position
 2  MOV o[TC2].xyz, v[1].xyzx          <- normal
 3  MOV o[TC3].xyw, v[2].xyxy          <- TC3 = (u, v, _, v)
 4  ADD o[TC3].z,   v[2].xxxx, c[210]  <- TC3.z = u + TrailSpeed
 5  MOV o[TC4].yzw, v[0].xxyz
 6  MUL R0, v[0].yyyy, c[1]            \
 7  MAD R0, v[0].xxxx, c[0], R0         |  viewProj
 8  MAD R0, v[0].zzzz, c[2], R0         |
 9  MOV o[TC4].x, v[3].wwww            <- VertexColour1.a, the `f[TC4].x` below
10  ADD o[POS], R0, c[3] | END         /
```

The block's parameter table places `TrailSpeed` (`0x07431a35`) at **`c466`**,
`viewProj` at `c256` and `eyePositionWorldSpace` at `c467`; the code's `c[N]`
numbering is the constant index less 256, so `c466` is `c[210]` and `c467` is
`c[211]` - the same offset `flame_test`'s block confirms, where `viewProj`'s
`c256` is `c[0]`.

**`TrailSpeed` is added once per lookup path, not twice on one.** Instruction 3
keeps the raw `u` in `TC3.x` and instruction 4 writes `u + TrailSpeed` beside it
in `TC3.z`; the fragment program then samples the noise at `f[TC3].zw` - the
scrolled coordinate - and its `@0x00 MOV R0.zw, f[TC3].xxxy` recovers the *raw*
`u` from `TC3.x`, which is why `@0x02` has to add `TrailSpeed` again before the
colour lookup. Both lookups end up with exactly one `TrailSpeed` in them. A
reading of this page that had the two additions stacking on one coordinate - and
`exhaust.wgsl` carried it in a comment - was wrong about the mechanism while
right about the remedy. Confidence 90: the instructions are the program's own
and the constant-index offset closes against a second material.

#### The ribbon's own fragment program

`hd_enginetrail_bluered.rcsmaterial` disassembles to 29 instructions, and its
patch chain resolves the same way `flame_test`'s did:

| Parameter | code slot | Value | What it is |
| --- | --- | ---: | --- |
| `TrailSpeed` (`0x07431a35`) | `0x3` | 1.0 | added to the `u` coordinate |
| `0xe296b1ed` | `0xc`, `0x11` | 0.15 | the facing-fade band's bound *and* its divisor |
| `0xbb48e390` | `0x18` | 0.0 | the mix between the two textures' colours |

and the two samplers resolve by hash against the model record's own entries:
`0xcc5bb827` is `hd_enginetrail_red_alphaisnoise.gtf` on **unit 0** and
`0x2743292b` is `hd_enginetrail_blue_alphaistrail.gtf` on **unit 1**. So:

```text
@0x01  TEX R1.w, f[TC3].zwzz unit0      <- the red texture's ALPHA
@0x02  ADD R0.x, R0.z, {TrailSpeed}     <- u + the scroll
@0x06  ADD R2.z, R0.x, R1.w             <- displaced by that alpha: the noise
@0x0b  MIN H0.w, dot, {0.15}            \
@0x0e  MAX H0.x, H0.w, {0}               |  saturate(clamp(dot, 0, 0.15) / 0.15)
@0x10  DIV_SAT H2.w, H0.x, {0.15}       /
@0x0d  TEX H1.xyz, R2.zwzz unit0        <- the red texture's colour
@0x12  TEX H0,     R2.zwzz unit1        <- the blue texture, all four channels
@0x14  MUL_SAT R0.w, f[POS].zzzz, {0.75}<- a window-depth fade, inline
@0x16  MUL H2.w, f[TC4].xxxx, H2        <- times a per-vertex scalar
@0x17  MAD H0.xyz, H2, {0.0}, H0        <- mix the two colours: the identity
@0x1a  MUL H0.w, H0, H2                 <- alpha = blue.a * that fade
@0x1b  MUL H0.w, H0, R0                 <-       * the depth fade
@0x1c  MUL H0.xyz, H0, f[TC0] END       <- colour * the vertex colour
```

**Three names confirmed by three independent routes.** The filename says
`alphaisnoise`; the sampler hash puts that file on unit 0; and unit 0's alpha
is what displaces the coordinate. Same for `alphaistrail` on unit 1, whose
alpha is the output's coverage. Confidence 92.

**All of it is implemented as of 2026-08-24, and every "unread" above is
read** - the live patch values, the geometry that carries the two vectors,
the depth fade and the per-vertex `f[TC4].x` are all on
[engine-trail.md](engine-trail.md): `TrailSpeed`'s live value is the flare's
wrapping phase accumulator (`Tex Scroll Speed Delta` 0.08..0.10 per frame by
speed), the colour mix is the per-craft Fury-skin flag (1.0 turns the trail
red), the facing fade's vectors are the fin normal and the view vector the
tube's own geometry now carries, and `f[TC4].x` is the vertex alpha -
`brightness x attack x tail falloff`, measured. `exhaust.wgsl`'s
`trail_shape` path computes each term.

**`Trail_ModelPath` is the `bluered` pair, not the `enginetrail_triangle` one**
this renderer loaded until now, and the plain one is named nowhere in the
executable. `Trail_ConstructManager` (`0x002e2da8`) allocates `0x11b00` bytes
and initialises **eight** per-craft trails, each `0x1230` bytes with a
`0x1200`-byte GPU buffer. The buffer's stride and sample count are not read, so
`oag_fx::exhaust`'s geometry constants stay PSP's. The rest is on
[trail-ribbon.md](../../../rendering/trail-ribbon.md).

## Reproducing this

```sh
just psarc cat data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC \
    /data/materials/ships/engines/flame_test.rcsmaterial > /tmp/flame_test.rcsmaterial
python3 scripts/ps3-microcode.py fp-file /tmp/flame_test.rcsmaterial
python3 scripts/ps3-microcode.py vp-file /tmp/flame_test.rcsmaterial 1
```

