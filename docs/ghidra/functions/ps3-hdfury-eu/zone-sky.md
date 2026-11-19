# HD/Fury's Zone sky is a **file swap**, and the gate that does it names four game modes

`EBOOT.elf` (Wipeout HD/Fury, PS3), image base `0x00000000`. Static reading
throughout, so every score here is capped at 84 per the
[confidence rubric](../../../reverse-engineering/confidence-rubric.md).

**The question this page answers.** From the maintainer, playing HD/Fury: *"the
skybox looks visibly different in zone races, almost like solid colours or
gradients."* Two mechanisms were candidates - a gradient built from the
`Sky horizon colour` / `Sky zenith colour` keys the Zone stage table
cross-fades, or a different cubemap - and
[zone-effectsettings-loader.md](zone-effectsettings-loader.md)'s own next step
insisted the answer be read rather than guessed, because a horizon-to-zenith
lerp is exactly the kind of legible invention `CLAUDE.md` rules out.

**Both are real, and the first one is the one a port can draw today.** A Zone
race loads `Data/Tex/ZoneSky.gtf` **instead of** the circuit's own `sky.gtf`,
through the same call, into the same handle, under the same sampler-state
patch. The gradient exists too, on the same gate, but its draw is unread - see
[What is not established](#what-is-not-established).

## The gate byte: `g_ZoneEffectsActive` at `0x00d45f84`

Not a new address. `zone-effectsettings-loader.md`'s twenty-third pass already
found `Scene_PrepareFrame` testing the byte at `0x00d45f84` at `0x003ab65c` and
skipping shader parameters 52-67 when it is zero. **Its writer is found here**,
and it is `Environment_LoadRaceScene` (`0x003f3fb0`) - the same per-race
environment loader that reads `ZoneMode.effectSettings`, `sky.gtf` and
`skycube`:

```text
003f4044  lwz  r11, -0x52C4(r2)   ; TOC 0x008bd3c4 -> 0x008b8100 -> g_GameState
003f404c  lwz  r9,  0xe0(r11)     ; the mode id
003f4050  cmplwi r9, 21
003f4054  bt   29, 0x3f4150       ; > 21: clear the byte
003f4058  li   r10, 1
003f405c  lis  r0, 32
003f4060  sld  r9, r10, r9        ; 1 << mode
003f4064  ori  r0, r0, 24640      ; r0 = 0x00206040
003f4068  and  r8, r9, r0
003f406c  cmpdi r8, 0
003f4070  bt   30, 0x3f4150       ; not in the mask: clear the byte
...
003f4078  lwz  r25, -0x52A0(r2)   ; -> 0x008b8124 -> 0x00d45f84
003f4080  stb  r10, 0(r25)        ; set

003f4150  lwz  r25, -0x52A0(r2)
003f4154  li   r0, 0
003f4158  stb  r0, 0(r25)         ; clear, on every other path
```

So the byte is **`(1 << mode) & 0x00206040 != 0`**, and the mask names exactly
four mode ids: **6, 13, 14 and 21**. Confidence **82** on the byte's meaning -
the arithmetic is direct, the two writers are the only ones in the image, and
`0x00936fe8 + 0xe0` is the mode field [mode-manager.md](mode-manager.md)
identified from the executable's own `GetMode()==%d` format string.

### This settles a hypothesis `mode-manager.md` left open

That page enumerated 22 mode ids and found seven - `0`, `6`, `7`, `11`, `13`,
`14`, `15` - with no `ModeManager` subclass of their own. It wrote:

> the loading screen distinguishes 6, 13 and 14 from each other, so they are
> real modes. The likely reading is that they are the ones with a `RaceManager`
> subclass and no `ModeManager` of their own [...] Zone, Zone Battle and
> Detonator are the obvious candidates. **Not established.**

This gate mask is independent evidence for exactly that: **three of the four
modes that switch on the whole Zone/Detonator effect block are 6, 13 and 14**,
the three that page singled out. And one of them is already pinned -
`zone-effectsettings-loader.md`'s tenth pass read mode `0xe` (14) as
**Detonator** off a different branch of this same loader. So 6 and 13 are Zone
and Zone Battle, in an order this page does **not** claim to know.

**The fourth id, 21, does not fit that story and is recorded rather than
explained.** `mode-manager.md` puts 21 with `MPArcade`, alongside 16 and 18.
The reading that costs nothing is that 21 is the online form of one of the two,
sharing `MPArcade`'s manager the way 16/18/21 already share it - but that is a
hypothesis (confidence 40), and the mask is a fact.

## The sky branch itself

`Environment_LoadRaceScene` tests the same byte again a page later and takes
one of two branches:

```text
003f426c  cmpwi 7, r0, 0          ; r0 = *(0x00d45f84), loaded at 003f4268
003f4270  bf    30, 0x3f4e24      ; non-zero -> the Zone sky

; the Zone branch
003f4e2c  lwz  r16, -0x5288(r2)   ; -> 0x008b813c
003f4e30  lwz  r4,  -0x5290(r2)   ; -> 0x008b8134 -> "Data/Tex/ZoneSky.gtf"
003f4e38  lwz  r5,  -0x528C(r2)   ; -> 0x008b8138 -> "skycube"
003f4e3c  addi r3, r1, 168
003f4e44  bl   0x5dde98
003f4e68  lwz  r17, -0x5310(r2)   ; -> 0x008b80b4 -> 0x00d42c60
003f4e70  stw  r31, 0x1014(r17)   ; the handle
003f4e98  b    0x3f441c           ; join

; the circuit branch, reached by falling through at 003f4274
003f43a4  lwz  r16, -0x5288(r2)
003f43ac  ...                     ; r4 = a path built from the circuit's own
003f43b0  clrldi r4, r29, 32      ;      directory plus "\" and "sky.gtf"
003f43bc  bl   0x5dde98           ; the same loader
003f43e8  stw  r31, 0x1014(r17)   ; the same handle slot
003f441c  ...                     ; the same sampler-state patch
```

**The convergence is what makes this a swap rather than a variant.** Same
loader call, same `"skycube"` tag, same handle slot `*(0x008b80b4) + 0x1014`,
same `rlwinm`/`ori` patch of the texture-control words at `+0x20`. Nothing
downstream of `0x3f441c` can tell the two apart, so the *picture* is the only
thing that changes. Confidence **84**.

**And `Environment_LoadRaceScene` is the only sky loader in the image.**
`scripts/ps3-toc.py attrib` on all three strings - `"Data/Tex/ZoneSky.gtf"`
(`0x007b3ef8`), `"skycube"` (`0x007b3f10`) and `"sky.gtf"` (`0x007b3f28`) -
returns `0x003f3fb0` and nothing else. **And the per-hit OPD check this page
warns about below was actually run**: a byte sweep for `lwz rX,-0x5290(r2)` -
the displacement that reaches the `ZoneSky.gtf` string's own slot - finds three
instructions, but `0x000b70b8` and `0x000b7d7c` carry the *other* TOC
(`0x008ad4d8`), where the same displacement resolves to `0x008a8248`, an
unrelated slot. So `0x003f4e30` is the only instruction in the image that ever
names this file. That is what lets the branch be read as either/or rather than
as one of several routes.

## The file

`/data/tex/zonesky.gtf`, **`DATA02.PSARC`**, 12,416 bytes. Its own header:

| field | value |
| --- | --- |
| textures | 1 |
| format | `0x86` (DXT1) |
| cubemap | 1 |
| mip levels | 1 |
| size | 64x64 |
| payload | 12,288 B = 6 faces x 2,048 B |

Against `01_vineta_k`'s own `sky.gtf` - `2048x2048` DXT1, 12,582,912 B of
payload - that is **1,024 times fewer texels a face**, which is the measured
form of the
maintainer's observation. It is not itself a flat fill: decoded, five of its
six faces carry 445-628 distinct colours in the cyan-teal the Zone palette
works in, and only face 3 (the nadir) is near-flat at 14. Asserted against the
disc by `crates/raceplay/tests/zone_sky_ground_truth.rs`, which races the *same*
circuit in both modes so the mode is the only variable, and leans on the size
rather than on a picture comparison: **no `sky.gtf` on this disc is 64x64**, so
a load that failed to swap cannot satisfy it. **Circuit skies are not all one
size**, which is why the ratio above is quoted against a named circuit rather
than as a disc-wide constant. Every `sky.gtf` on the disc, by payload:

| bytes | shape | environments |
| ---: | --- | --- |
| 262,784 | the odd one out | `zone_3` |
| 3,145,856 | 1024x1024 DXT1, one level | `amphiseum`, `modesto_heights`, `talons_junction`, `tech_de_ra` |
| 12,583,040 | 2048x2048 DXT1, one level | the eight remaining racing circuits, plus `zone_1`, `zone_2`, `zone_4` |
| 16,777,856 | 2048x2048 DXT1 **with mips** | `12_sol_2` |

Same format throughout, two sizes and one mip chain - not, as an earlier
revision of this page had it, one payload in two formats.

**There is exactly one `zonesky.gtf` on the disc.** All seven archives were
listed and counted rather than searched with a silenced error - `DATA00` 3,157
entries, `DATA01` 53, `DATA02` 5,486, `DATA03` 1,163, `DATA04` 54, `DATA05`
151, `DATA06` 1,600 - and `zonesky` matches once, in `DATA02`. The count is
recorded because this thread has already published one census whose every
absolute number had to be retracted for coming off a contaminated extraction
directory.

### Two corrections to the handover thread's premises

- **`zone_3/sky.gtf` is 262,784 B, not 12,583,040.** The claim that all four
  Zone arenas share one cubemap was measured on `zone_1` and `zone_2` (which
  are byte-identical) and `zone_4` (which matches them in size); `zone_3` does
  not. Nothing rests on it any more, but it should not be carried forward.
- **The arenas' own `sky.gtf` are dead art in Zone mode.** The branch above is
  either/or, so `zone_1..zone_4/sky.gtf` are never what a Zone race shows. That
  is the more interesting half of the correction, and it is the reason the
  "all four share one cubemap" measurement never explained the observation.

## The gradient dome: what `Sky_DrawGradientDome` (`0x005ebd58`) draws

**Read 2026-09-04.** It is a dome, not a screen-space quad and not a fog term
- the two forks the previous revision of this page named as open are both
settled. Confidence **82**, capped at 84 per this page's own static-reading
rule.

**Correction to this section's first pass, same day.** The first read of the
disassembly called `fRam008bf4b4`/`fRam008bf4b0`/`fRam008bf4c0` unknown
per-call inputs and described the loop as 18 latitude bands climbing from
horizon toward zenith. Both were wrong. Ghidra's `fRam` naming was misleading:
these are not mystery RAM values, they are **fixed constants in this
function's own TOC**, read directly with `scripts/ps3-toc.py resolve
0x005ebd58 <disp>` (`0x20ec`, `0x20f0`, `0x20f4`, `0x20f8`, `0x20fc`) and
decoded as IEEE-754 floats:

| symbol | TOC slot | value | role |
| --- | --- | ---: | --- |
| scale | `0x008bf4b0` | `0.017453291` | exactly `π/180` - degrees to radians |
| `θ0` | `0x008bf4b4` | `0.0` | the sweep's starting angle, degrees |
| apex ratio (up) | `0x008bf4b8` | `0.75` | `× radius`, the upper apex height |
| apex ratio (down) | `0x008bf4bc` | `-0.75` | `× radius`, one lower apex height |
| step | `0x008bf4c0` | `20.0` | degrees advanced per iteration |

**So this is not a multi-band dome - it is one 18-segment ring swept through a
full 360° (`18 × 20°`), forming a double cone.** The 0.0 starting angle plus
the constant `0.0` written into the base-ring vertices' shared field
(`fmr f26,f23` once, before the loop, never reassigned) both point at a base
ring sitting at a fixed height - call it `Y=0` - tessellated into 18 points
around a full circle in the horizontal plane, not climbing latitude at all.
The eight `_FSin(angle, cursor, sin_or_cos)` calls per iteration are the
sin/cos of two **adjacent azimuth angles** (`θ`, `θ+20°`), i.e. the x/z of two
consecutive base-ring points, not two different latitudes.

Two vertex-emission helpers consume those ring positions plus two apex points
that are computed once and reused across all 18 segments (`f24 = radius×0.75`,
`f25 = radius×-0.75`, and a third, `f20 = -radius`, used once more for a
steeper closing triangle):

- **`FUN_005e9e50`** (decompiled in full) pushes **two triangles from four
  vertex+colour pairs** - `(v2,c2) (v3,c3) (v4,c4)` then `(v5,c5) (v3,c3)
  (v4,c4)` - the standard quad-as-two-triangles shape, into a triangle-strip
  index buffer with its own restart-marker bookkeeping (`0xffffffff` sentinel,
  a `+0x10` per-strip vertex count).
- **`FUN_005ea308`** (decompiled in full) is the same restart-marker logic
  pushing **one triangle from three vertex+colour pairs** - a fan segment
  closing on the shared apex, rather than a quad between two rings.

Each iteration builds four primitive groups per 20° segment: a quad and a
closing triangle toward the **upper** apex (`0.75×radius`), then a quad and a
closing triangle toward the **lower** apex (`-0.75×radius`, with one segment's
triangle reaching all the way to `-radius`). That is a **double cone fanned
from one 18-gon equator ring** - an upper cone whose vertices blend toward one
colour as they approach its apex, and a lower cone held solid in the other -
not a hemisphere built from many latitude bands as the first pass described.

**The two packed colours are the vertex colours, not a per-pixel term.** The
upper-cone primitives take `(param_3, param_3, param_4)` and `(param_4,
param_4, param_4)` - blending toward the caller's second colour as the
vertices approach the upper apex - while the lower-cone primitives take
`(param_3, param_3, param_3)` throughout, uniformly. So one colour
(`param_3`) covers the lower cone solid and the blend toward the other
(`param_4`) happens only going up, GPU-interpolated across each quad the way
vertex colours always are - a genuine two-colour gradient, built as geometry
and colour, not sampled or computed per fragment.

**The submit call confirms it reaches the GPU as ordinary geometry.** The loop
ends with a call to `FUN_005e6870` (861 instructions, calls `Rsx_SetMethod` and
`Rsx_UploadVertexConstants` among 47 callees) - a generic draw-batch submit
shared by several other renderer routines including the already-named
`Environment_UpdateStageBlend`, not something sky-specific. It is deliberately
**not renamed** here: nothing about this pass narrows what it does beyond
"submit the accumulated batch," and every one of its many callers would need
checking before a name could mean anything general.

Renamed `Sky_DrawGradientDome`. **Confidence 82 on the shape** (a double cone
on one azimuth ring, upper half blending, lower half solid) - every number in
the table above is a direct TOC decode, not an inference. **Confidence drops
to inference-level (~65, not separately scored) for "upper = sky, lower =
ground/underside skirt"**: nothing read ties the vertex field the apex heights
write into to world-space up specifically, as opposed to whatever local frame
this draw happens in - it is the conventional reading (a sky gradient's
visible half blends toward its horizon colour going up and a hidden
below-horizon half doesn't need to), not a measured one. **What blocks
drawing this safely**: no blend mode or depth state is set adjacent to the
call site (`0x003ae1bc`) or within the eight instructions either side of it -
whatever state is active was set earlier, upstream of where this pass looked,
and how this double cone composites with the `ZoneSky.gtf` cubemap the same
function already draws (same colour buffer? drawn before or after? additive,
alpha, or opaque?) is unread. Guessing a compositing mode is exactly the kind
of stand-in this project's rules exist to prevent, so the shape is documented
and nothing was drawn on it until the live frame answered the compositing question for the visible directions (next section).

## The dome's colours, and the dome is what the frame shows (2026-11-19, `zone-pair-score`)

**Read this pass** (static, confidence 82 each), from `Scene_PrepareFrame`'s
Zone block `0x003adf58`-`0x003ae1c4` and `Sky_DrawGradientDome`'s first
segment `0x005ebe70`-`0x005ebf68`:

| what | where | value |
| --- | --- | --- |
| the common scale | `lwz r9,-0x60ec(r2)` -> TOC `0x008b72d8` -> `0x007b0e40` | `0x437f0000` = `255.0` |
| horizon inner / outer | `-0x60f0` -> `0x00c81560`, `-0x60e4` -> `0x00c81570` | `Sky horizon colour` (zone-effectsettings-loader.md's slot table) |
| zenith inner / outer | `-0x60e8` -> `0x00c81580`, `-0x60e0` -> `0x00c81590` | `Sky zenith colour` |
| the lerp | `0x003ae0e0`-`0x003ae118` | `inner * w + outer * (1 - w)`, then `fctiwz` (truncation) |
| the packing | `0x003ae13c`-`0x003ae160`, `0x003ae190`-`0x003ae1b4` | `R << 24 \| G << 16 \| B << 8 \| 0xff`: lane 0 is red, the natural order |
| which colour is which | `r5` (`param_3`) = horizon, `r6` (`param_4`) = zenith | the horizon fills the ring and the lower half, the zenith the top |
| the radius | `lfs f1,-0x6098(r2)` -> TOC `0x008b732c` | `0x457a0000` = `4000.0` |
| the band | `stfs` at `0x005ebeb0`/`0x005ebf48` | the quad's lower vertices at `y = 0`, its upper ones at `y = 0.75 r` with the **same** `r sin`, `r cos`: a vertical band, then a flat zenith cap |

So the dome seen from its centre: a direction at elevation `e` meets the band
at height `r tan(e)` and takes `tan(e) / 0.75` of the zenith colour, the cap
above `e = 36.9°` the zenith alone, eye level and below the horizon alone.
The bytes reach the RSX as vertex colours, are interpolated as linear light
and encoded by the target on write.

**The live frame agrees, and shows no cubemap texel.** Confidence 88 that
the dome is drawn, covers every visible direction and carries the horizon
colour at eye level (the static read above plus one RPCS3 frame of the
original). The band's shape stays at the static 82: the frame is flatter
upward than `tan(e) / 0.75` predicts (top-left, about 24 degrees up:
predicted about `(_, 135, 157)`, read `(6, 127, 147)`). The adopted pair
`hd/zone-vineta-k-z5` (Vineta K, Zone start, stage 1 Sub Venom, cold boot,
2026-11-19) shows a sky with no cloud or sea texel of `ZoneSky.gtf` anywhere:
`(4..7, 124..129, 143..154)` over the whole visible sky, bluer upward.
Sub Venom's horizon `(0, 0.2118, 0.2471)` is the bytes `(0, 54, 63)`, which
the sRGB encode displays as `(0, 126, 135)`: green within 1 of the frame at
eye level. So the gradient dome covers every visible direction the cubemap
would show; whether the cubemap is drawn under it at all is still not read,
and the port keeps `ZoneSky.gtf` only as the reflection map.

**Port**: `oag_mesh::mesh::sky_cube::zone_dome` (the band law on the
camera-centred cube, the 18-gon drawn as a circle, at most 1.5 % on a
weight), recoloured per frame by `shade_zone_dome` from the grade's
cross-faded palette (`oag_raceplay::Scene`, written only when the pair
changes). Tests: `sky_cube`'s two unit tests and
`crates/raceplay/tests/zone_sky_ground_truth.rs`. The pair's mean grey moves
53.04 -> 37.40 (rmse 0.322 -> 0.224).

**Open**: ours' sky reads about 17 % brighter than the original's at eye
level (`(7, 149, 174)` against `(6, 127, 147)`), uniformly, which points at
exposure rather than the dome; and the band's flatness above. Detonator's
modes ride the same gate byte and are not wired to the dome.

**2048 and Omega, against their own Zone frames at Flash** (zone 5, HD's
ladder stage 4, whose sky is neutral grey: horizon `0.1255`, zenith `0.6118`):

- **2048 on an HD-lineage circuit: checked, applies, not wired** (confidence
  60). `2048-vita/zone-dlc-vineta-k-z5`'s original draws a flat neutral grey
  sky, `(88, 88, 88)` to `(116, 116, 116)`, no cloud texel, inside the bytes
  HD's Flash pair packs (32 to 156). 2048's port draws its circuit dome.
- **2048 on its own circuit: unresolved.** `2048-vita/zone-altima-z5` shows a
  teal-grey sky, `(87, 138, 132)` to `(124, 134, 138)`; 2048's own table
  authors Flash `Sky.Horizon Colour` `(0.682, 0.753, 0.761)` and
  `Sky.Zenith Colour` `(0.812, 0.725, 0.725)`, not a clean match in either
  domain. 2048's Zone sky draw is unread.
- **Omega: checked, differs.** `omega/zone-vineta-k-z5`'s original (HD mode,
  Flash) shows a bright peach sky, `(255, 255, 171)` at the top centre, not
  HD's Flash grey. Omega's Zone sky source is unread in its executable, and
  Omega's `zone_sky` is `None`, so the dome never draws there.

## What is **not** established

**The gradient's vertex colours.** `Sky horizon colour` and `Sky zenith
colour` *are* consumed, and this page locates the consumer. The four
cross-faded vec4s at `0x00c81560`-`0x00c81590` are loaded together at
`0x003adf58`-`0x003adf78`, inside the same function as `Scene_PrepareFrame`,
under **the same gate byte**:

```text
003ad598  lbz  r0, 0(r17)         ; r17 = 0x00d45f84, unchanged since 003aaea0
003ad59c  cmpwi 7, r0, 0
003ad5a0  bf   30, 0x3adf58       ; Zone: the gradient block
003ad5a4  ...                     ; otherwise: the ordinary path
```

Read from there, with confidence **80** because each step is a direct decode:

- all four are scaled by a common constant from `0x007b0e40` (six components:
  horizon and zenith, each in its inner and outer form);
- inner and outer are then **lerped on the CPU** by one weight `w`, as
  `inner*w + outer*(1 - w)`, `w` fetched from a small indexed record. The
  `1 - w` is read, not assumed: the complement is `fsubs f30, f0, f31` where
  `f0` comes from a GPR round-tripped through the stack (`stw r29,1632(1)` /
  `lfs f0,1632(1)`) and `r29` is set by `lis r29, 16256` at `0x003ad218` -
  `0x3f800000`, exactly `1.0f`. Nothing writes `r29` between there and the
  use on this path, and nothing branches into the range in between, so the
  constant reaches it;
- each result is `fctiwz`'d and packed into a 32-bit word with `0xff` in the
  low byte - two packed colours, one for horizon and one for zenith. **Which
  byte is which channel is deliberately not stated**: the shifts come out
  `<<24`, `<<16`, `<<8` in an order that does not follow the load order, and
  nothing here pins the channel assignment;
- both are passed to `Sky_DrawGradientDome`, which has **exactly one caller in
  the whole image** - this site.

**What that draw does with them is read now** - see the section above. It is
a dome, blended per-vertex from these two colours across the climbing half of
the sweep, with a solid-horizon-colour skirt on the other half. The port
draws it since 2026-11-19 (section above); the shape question a horizon-to-zenith lerp
would have guessed at is answered rather than guessed.

Also not established, and unchanged from
[zone-effectsettings-loader.md](zone-effectsettings-loader.md): what advances
the Zone stage at runtime, and therefore what moves the cross-fade this block
consumes.

## Reproduce

```sh
ELF=data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf
ISO=data/images/hdfury-ps3-eu-dec.iso

python3 scripts/ps3-toc.py attrib 0x007b3ef8      # -> 0x003f3fb0, and only it
python3 scripts/ps3-toc.py u32    0x008b8124      # -> 0x00d45f84
python3 scripts/ps3-toc.py u32    0x008b80b4      # -> 0x00d42c60
llvm-objdump -d --mcpu=pwr6 --start-address=0x3f4030 --stop-address=0x3f4170 $ELF
llvm-objdump -d --mcpu=pwr6 --start-address=0x3f4e24 --stop-address=0x3f4ea0 $ELF
llvm-objdump -d --mcpu=pwr6 --start-address=0x3adf58 --stop-address=0x3ae1d0 $ELF
llvm-objdump -d --mcpu=pwr6 --start-address=0x5ebd58 --stop-address=0x5ec3b4 $ELF  # Sky_DrawGradientDome
llvm-objdump -d --mcpu=pwr6 --start-address=0x5ea308 --stop-address=0x5ea46c $ELF  # FUN_005ea308, the triangle emitter
llvm-objdump -d --mcpu=pwr6 --start-address=0x5e9e50 --stop-address=0x5ea2cc $ELF  # FUN_005e9e50, the quad emitter

python3 scripts/ps3-toc.py toc     0x005ebd58   # -> 0x008bd3c4
python3 scripts/ps3-toc.py resolve 0x005ebd58 0x20ec  # scale, 0x3c8efa34 = pi/180
python3 scripts/ps3-toc.py resolve 0x005ebd58 0x20f0  # theta0, 0x00000000 = 0.0
python3 scripts/ps3-toc.py resolve 0x005ebd58 0x20f4  # apex ratio up, 0x3f400000 = 0.75
python3 scripts/ps3-toc.py resolve 0x005ebd58 0x20f8  # apex ratio down, 0xbf400000 = -0.75
python3 scripts/ps3-toc.py resolve 0x005ebd58 0x20fc  # step, 0x41a00000 = 20.0

python3 scripts/psarc.py list $ISO:PS3_GAME/USRDIR/DATA02.PSARC | grep -i zonesky
python3 scripts/psarc.py cat  $ISO:PS3_GAME/USRDIR/DATA02.PSARC \
    /data/tex/zonesky.gtf | head -c 64 | xxd

OAG_REQUIRE_GAME_DATA=1 cargo nextest run --release -p oag-game \
    --run-ignored all -E 'test(/^zone_sky_ground_truth::/)'
```

Two traps this page tripped over, both already on
`zone-effectsettings-loader.md` and both worth repeating because they cost time
again here:

- **Per-function TOC.** Every displacement above resolves through
  `Environment_LoadRaceScene`'s own TOC (`0x008bd3c4`), not the entry point's.
  `scripts/ps3-toc.py resolve <func> <hexdisp>` parses its displacement as
  **hex**.
- **Confirm a sweep re-finds a positive before believing its negative.** The
  first sweep for consumers of `0x00c81560`-`0x00c81590` returned nothing *and
  also* returned nothing for `zoneOrigin` at `0x00c81550`, a parameter already
  known to be published. The defect was in the sweep, not in the data: these
  addresses are materialised by `lwz rD,disp(r2)` off per-function TOC slots,
  which `scripts/ps3-toc.py`'s own `scan_toc_loads` handles and a hand-rolled
  register tracker did not. With the control re-found, the four sky slots have
  one loader each.

## Names applied

- `Environment_LoadRaceScene` (`0x003f3fb0`, function, 75) - the per-race
  environment loader: it reads `ZoneMode.effectSettings`, writes the mode gate
  byte, and loads either the circuit's `sky.gtf` or `Data/Tex/ZoneSky.gtf`.
  Independently identified as environment setup by two earlier investigations
  (`docs/formats/envsettings.md`, `zone-effectsettings-loader.md`); this page
  adds the mode gate and the sky branch. 75 rather than higher because the
  function is large and only these parts of it have been read.
- `g_ZoneEffectsActive` (`0x00d45f84`, data, 80) - the byte that switches on
  HD's Zone/Detonator scene effects: set for mode ids 6, 13, 14 and 21 and
  cleared otherwise, read by the sky branch here and by `Scene_PrepareFrame`
  twice.
- `Sky_DrawGradientDome` (`0x005ebd58`, function, 82) - builds and submits a
  procedural, vertex-coloured double cone (one 18-segment ring swept through a
  full 360°, fanned to two fixed apexes at `±0.75×radius`) that blends the
  caller's two packed colours toward one apex and holds one colour solid
  toward the other. Exactly one caller, `Scene_PrepareFrame`, passing the two
  colours this page's gradient section decodes.

## See also

- [zone-effectsettings-loader.md](zone-effectsettings-loader.md) - the stage
  table, the parameter publication idiom, and the cross-fade that fills
  `0x00c81560`-`0x00c81590`.
- [zone-shader.md](zone-shader.md) - the surface equation the same gate turns on.
- [mode-manager.md](mode-manager.md) - the mode enum this page's mask indexes.
- [../../../formats/effectsettings.md](../../../formats/effectsettings.md) - the
  file's own keys, `Sky horizon colour` and `Sky zenith colour` among them.
