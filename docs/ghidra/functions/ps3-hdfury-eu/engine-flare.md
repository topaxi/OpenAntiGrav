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
numbers.** Past everything `oag_formats::rcsmodel::material` used to read, a
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
a reason to share one - `oag_render::mesh::Flame` reads them per craft.

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
`cargo run -p oag-render --example hd_flare_probe` prints those spans.

**The face-on 1.1 survives a race and clamps everywhere else.** A race draws
into `post::hd_bloom::SCENE_FORMAT`, a linear float target, where a fragment
alpha above 1.0 reaches the blend intact; `model_probe`, the asset viewer and
the capture path draw the same models into a gamma target, where it clamps to
1.0 first. So the flame is measurably dimmer in those three than in the game,
by up to 10 % of its own contribution. A property of the target, recorded
rather than worked around.

**Confidence 92 on the layout** - it is validated disc-wide rather than on the
file it was read from: 20,445 parameters across 9,757 materials of 643
`.rcsmodel`s, and every one of them is a finite float of ordinary magnitude,
which a walk that was one field out could not be. Pinned by
`crates/formats/tests/rcsmodel_material_ground_truth.rs`.

**Confidence 88 on the arithmetic**: the instructions are the program's own and
the patch chain is resolved, but the block was read once and the two unnamed
parameters are identified by the slot they patch rather than by a name.

### Three findings that fall out and are deliberately not acted on

- **`SpecScale` (`0x4232e459`) is authored per material instance** - 235 on
  Feisar's hull paint, 500 on its glass. [renderer.md](renderer.md) records the
  specular exponent as "32, the commonest of three round values ... a stand-in,
  now labelled as one", from a sweep of *inline* literals. The exponent is a
  per-instance parameter, and reading it is what would retire that stand-in.
  Nothing reads it yet.
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

**Drawn as of 2026-08-24.** `oag_render::mesh::Flame` reads `Speed` as a sixth
authored number (2.0 on all fourteen craft and on all fourteen boost plumes -
`crates/game/tests/hd_engine_flare_ground_truth.rs` pins it against the disc),
`mesh_render::Scene` carries the clock the way the original binds it - one
value splatted to four lanes - and `mesh.wgsl` takes both of the program's
taps. This project's clock is the race tick over 60, the same one the trackside
texture and node animation already ride, so `--anim-seconds` pins all three at
once and a replay of a tick draws the same frame. On-screen check, headless:
two captures of one tick at phases half a texture repeat apart differ only in
the flame's own pixels (71 of 1,175,040 at tick 1050's dark section, 6 over a
bright one where an additive surface cannot register) - which confirms the
clock reaches the surface and nothing else moved, not that the rate matches the
original. That comparison wants the matched-view harness and a recording.

## Blocks #0 and #2, in one line each

`#0` computes the same rim term, writes a black `H0.xyz` and samples nothing -
a depth or mask variant. `#2` is `#1` with `fogColour` (`0x3dc31258`, already
named on [renderer.md](renderer.md)) and the `float2` still unresolved: the
fogged permutation. Neither changes anything above.

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
`oag_render::mesh::groups::split` reaches the same one by walking the subtrees.
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
while it is above a threshold, a decay after. `oag_render::exhaust`'s recovered
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
`oag_render::exhaust`'s geometry constants stay PSP's. The rest is on
[trail-ribbon.md](../../../rendering/trail-ribbon.md).

## Reproducing this

```sh
just psarc cat data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA02.PSARC \
    /data/materials/ships/engines/flame_test.rcsmaterial > /tmp/flame_test.rcsmaterial
python3 scripts/ps3-microcode.py fp-file /tmp/flame_test.rcsmaterial
python3 scripts/ps3-microcode.py vp-file /tmp/flame_test.rcsmaterial 1
```

and the model side, which needs no decryption either:

```sh
cargo run --release -p oag-render --example hd_flare_probe /data/ships/feisar/engineflare
cargo run --release -p oag-render --example hd_flare_tree  /data/ships/feisar/engineflare
```
