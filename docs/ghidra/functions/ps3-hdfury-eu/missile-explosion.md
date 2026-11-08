# The Missile's explosion object: lifetime, the sixteen streaks, the bloom boost

Read 2026-11-08 (`hd-missile-explo-re`) with capstone (PPC64 big-endian) over
the raw bytes of the decrypted `EBOOT.elf` (`ps3-hdfury-eu`, TOC
`r2 = 0x008ad4d8`), because Ghidra truncates these functions at AltiVec.
Static only; the live numbers quoted as corroboration are the 2026-10-07
`hd-weapon-fx` polls ([weapons.md](weapons.md), "the Missile's explosion pool,
read statically and live").

This page supersedes three claims of the 2026-10-07/08 sections of
[weapons.md](weapons.md); see "Corrections" at the end.

## The class

Sixteen objects per `MissileManager` (pointers at `manager + 0xcc + 4 i`,
count `+0x10c`), vtable `0x00864b38`. The vtable's own slots are generic
except slot 5 (`MissileExplosion_Draw`, `0x00155420`) and the two destructors
(slots 12 and 13, `0x00155d00`/`0x00155c10`); **the per-frame update is not a
virtual**: the manager calls it directly.

| Offset | Field | Written by |
| --- | --- | --- |
| `+0xe0` | byte, alive | `Start` `= 1` (`0x001558f4`), `Reset` `= 0` (`0x001548f8`) |
| `+0xf0 + 0x40 v` | the placement matrix per viewport `v` (`v < GameState + 0xe4`, two at most) | `Start` |
| `+0x170` | age, seconds | `Update` (`0x00154aec`), `Reset` `= 0` |
| `+0x174` | the saved `Bloom adaption boost`, or `-1.0` when this entry does not own it | `Start`, `Reset`, constructor |
| `+0x178` | the boost's value when the object was built | constructor (`0x00154ec4`) |
| `+0x17c` | `(1 - p)^2`, the point light's term | `Update` (`0x00154b38`), `Reset` `= 1.0` |
| `+0x180` | `2 (1 - p)`, the streak jitter's amplitude | `Update` (`0x00154b34`), `Reset` `= 2.0` |
| `+0x184` | the model (`HD_missile_explosion`) | constructor |
| `+0x190 + 0x50 i`, `i < 16` | the streak entries (below) | `Start`, `Update` |
| `+0x690` | the environment settings block (`EnvSettings_GetOrCreate`, `0x003a9520`) | constructor (`0x00154ee4`) |

The tuning block is `0x008c1b54` (through `TOC[-0x2270]`): `K0 = 2.0`,
`K1 = 1.0` (the lifetime, seconds), `K2 = 6.0` (the boost during a blast).

## 1. Lifetime: what retires an entry

**`MissileExplosion_Update` (`0x00154a78`, `this`, `f1 = dt`), confidence 85.**
Its only caller is the manager's update `0x00143580`, at `0x00143650`, in the
pool loop `0x00143638..0x00143678`:

```text
for i in 0..count:                       (count = manager + 0x10c)
    Update(pool[i], dt)
    if pool[i].alive == 0:               (lbz +0xe0, 0x00143660)
        count -= 1                       (stw r11, 0x10c(r26) at 0x00143610)
        swap pool[i] with pool[count]; i -= 1
```

`Update`, in order:

```text
if !alive: return 1
age += dt                                (0x00154adc, stored 0x00154aec)
if age > K1 (1.0): Reset(this); return   (fcmpu ... bgt 0x00154ce4 -> bl 0x00154858)
p = age / K1
SetTime(model, p)                        (AnimNode_UpdateTransformTree 0x002c1b30)
+0x180 = (1 - p) * K0                    (2 (1 - p))
+0x17c = (1 - p)^2
for each of the 16 streaks: see 2
if +0x174 != -1: env.bloom_adaption_boost = K2 + p * (+0x174 - K2)   (see 4)
```

`Reset` (`MissileExplosion_Reset`, `0x00154858`, confidence 85): age `= 0`,
both viewport matrices to identity, alive `= 0`, `+0x180 = K0`,
`+0x17c = 1.0`, `SetTime(model, 0)` and the model's draw bit cleared, and if
`+0x174 > -1` the boost is put back to `+0x178` and `+0x174 = -1`.

So an entry lives while `age <= 1.0` s, by the frame `dt`, and is freed on
the first update after its age passes one second. The live polls agree: the
pool count fell 0.98 s and 0.96 s after it rose on two boots, `+0x170` ran
linear at 1/s, `+0x17c = (1 - p)^2` and `+0x180 = 2 (1 - p)` at all 25
samples.

**The model's clock is this age.** `SetTime(model, p)` writes the
`MeshImporter` node's `+0xc0` ([plasma.md](plasma.md), "The code side"), which
the object's constructor `0x00154cf0` binds by pointer to both `UV_offset` and
`Shockwave_scalar` (`0x00154e18..0x00154e84`). With `K1 = 1.0` the normalised
time and the age in seconds are the same number.

## 2. The sixteen entries at `+0x190`: one `FatLine` streak each

**Confidence 80** for the law (one function read end to end, and the
streak's own element class already read in
[missile-ribbon.md](missile-ribbon.md)); not seen on a frame.

### The entry

| Entry offset | Field | Start |
| --- | --- | --- |
| `+0x00..+0x3f` | a 4 x 4 matrix: rows 0, 1, 2 the basis, row 3 the position | `Start`'s argument matrix, then rotated |
| `+0x40` | a `FatLine` from `RibbonEffects` pool `+0x5c`, or `0` | acquired in `Start` |
| `+0x44` | speed, `U(7, 8)` | `0x0015567c` |
| `+0x48` | node-life scale, `U(0.5, 1)` | `0x00155690` |

`U(a, b)` is `Random_RangeFloat` (`0x0028c660`, `a + (b - a) * next / scale`,
confidence 80). `Start` (`0x00155640..0x00155710`) copies its argument matrix
into each entry, draws the two numbers above, then three angles
`U(-2 pi, 2 pi)` (`TOC[-0x2220]`/`[-0x221c]` = `-6.28319`/`6.28319`) and
rotates the entry's basis by them, about its row 1, then row 0, then row 2
(`0x00677688` -> `0x006ca6b0`: builds a rotation about the given axis and
multiplies the three basis rows, the position untouched; confidence 75).
So the sixteen streaks start at the blast's centre with uniformly random
directions.

**The streak element.** `Start` (`0x00155a88..0x00155b1c`) reads
`RaceManager + 0x2dcc` (`RibbonEffects`), takes pool `+0x5c` (count `+0x5c`,
elements `+0x60`, flags `+0x64`, stride `0x60`), and for each entry takes the
first free flag, writes `3` and stores the element at `+0x40`; with no free
flag it stores `0`. Pool `+0x5c` has **20** elements
([rocket-trail.md](rocket-trail.md), "The manager"), so a second blast inside
the first one's streak life gets at most four.

### Per update

`Update` (`0x00154b40..0x00154c68`), per entry, with `a` the age:

```text
s = sin(15 a), c = cos(15 a)             (0x00676fe8 -> 0x0042fbf0 with r4 = 0 / 1;
                                          15.0 = TOC[-0x224c])
if entry.fatline: FatLine_PushChains(entry.fatline, entry.matrix, f1 = entry.scale)
pos += row1 * entry.speed
pos += row0 * (s + 2 (1 - p) * U(-1, 1))
pos += row2 * (c + 2 (1 - p) * U(-1, 1))
```

`0x0042fbf0` is the library `sinf`/`cosf` pair (a `2/pi` quadrant split,
`0.63662` and `-pi/2` at `0x008b99b4`/`0x008b99bc`, the second argument
offsetting the quadrant), confidence 85.

**The step is per update call, not scaled by `dt`.** At one update per 60 Hz
frame that is 420 to 480 units/s along the entry's row 1, with a unit-radius
`sin`/`cos` swirl in the other two axes (the same phase for all sixteen, each
in its own basis) and a random jitter shrinking from 2 to 0 over the life. The
60 Hz is assumed (the manager's update rate was not measured).

### How a streak is drawn

Not by `MissileExplosion_Draw`. The element is a `FatLine`
([missile-ribbon.md](missile-ribbon.md)); `RibbonEffects_UpdatePools` ticks it
and `RibbonEffects_Render` draws it with the other pool-1 lines under
`blend(SRC_ALPHA, ONE)`. Pool `+0x5c`'s config, read from the ELF at
`0x00921050` (confidence 90):

| Config | Value | Effect |
| --- | ---: | --- |
| `+0x10` width at birth | 0.0 | |
| `+0x14` width at death | 2.0 | each node widens 0 to 2 as it ages |
| `+0x18` lifetime | 1.2 s | `node.life = entry.scale * 1.2`, so 0.6 to 1.2 s |
| `+0x1c` | 0.2 | the texture's `u` per world unit |
| `+0x20` texture | `Data/Tex/ThickLine_bighalo.gtf` (`0x0079c258`) | |
| `+0x24` chains | 1 | |
| `+0x28` node capacity | 350 | |
| `+0x2c` phase step | 0.06 | |
| `+0x30` helix radius | 0.6 | |
| `+0x34` opacity ramp | off | `alpha = 1 - t` |

One node is laid per update at the entry's position plus `row1 * 0.6` turned
about `row2` by the element's running phase, and stays where it was laid. The
streaks therefore outlive the entry: the last nodes are laid just before the
one-second retirement and live up to 1.2 s more, so the burst's lines fade by
about 2.2 s. The entry's retirement does not free the element; the pool frees
it when its last node dies.

## 3. Which node draws which material

**Confidence 90**: read off the files, and three of the four chunks matched
to the original's own draws.

`HD_missile_explosion.vex` (`DATA02`) is `world / Root` with four keyed
`Anim Transform` children, each holding one `Mesh` node whose payload `+0x30`
names a chunk of `HD_missile_explosion.rcsmodel`. The chunk's `+0x20` picks
the material (`oag_rcs::rcsmodel::Mesh::material`). Read with a throwaway
`oag-rcs` test over `vex::nodes` and `rcsmodel::Model::parse` (not kept):

| Node | Mesh node | Chunk (hash) | Indices | Material |
| --- | --- | --- | ---: | --- |
| `sphere` | `sphereShape` | 0 (`532d1720`) | 4,320 | `hd_missile_explosion_core_glow` |
| `bloom` | `bloomShape` | 1 (`37dbf9be`) | 10,080 | `hd_missile_explosion_lightrays_glow` |
| `rays` | `raysShape` | 2 (`2cbe4fd5`) | 2,700 | `hd_missile_explosion_lightrays_glow` |
| `shockwave` | `shockwaveShape` | 3 (`fdfed0de`) | 3,072 | `hd_missile_explosion_shockwaves_glow` |

Four nodes, three materials: **`bloom` draws the rays' Fresnel-shell program,
not the core's.** The earlier wording "core: the sphere and bloom"
(`weapons.md`, `hd-missile-blast`, and `oag_mesh`'s `MISSILE_CORE` doc) is
wrong about `bloom`; the engine's draws are right anyway, because the slot
bits are earned per chunk from the chunk's own material.

**The live match.** `hd-whiteout`'s `run1 hit0` frame (`weapons.md`,
"`hd-whiteout`", point 3) listed three draws carrying the age in `c[464].x`,
with index counts 224, 96 and 140. Those dumps kept only the last 256-index
batch of each `0x1824` method (corrected the same day,
`docs/reverse-engineering/rpcs3-capture.md`, "Index batches"), and
`4320 mod 256 = 224`, `10080 mod 256 = 96`, `2700 mod 256 = 140`. So those
three were `sphere`, `bloom` and `rays`; `shockwave`'s `3072` is a whole
number of batches and would have read 256.

**Blend, state and parameters**, each already read and cited here only:

- Blend: HD authors `SRC_ALPHA, ONE` on all three materials
  (`oag_rcs::rcsmodel::psp2::lineage_blend`, re-derived from the disc by
  `hd_lineage_blend_ground_truth.rs`).
- State on the original's frame: additive `SrcAlpha/One`, depth test `LEQUAL`,
  depth mask off, culling off, alpha test off (`weapons.md`, `hd-whiteout`
  point 3, confidence 75).
- Programs (`weapons.md`, `hd-missile-blast`, confidence 70):
  `core_glow` `rgb = tex(u, v + 0.5 U).rgb * colour.rgb`, `a = tex.a * colour.a`;
  `lightrays_glow` `rim = sat(1 - N.V)`, `f = sat(0.9 - 0.9 rim^5)^5`,
  `out = (f tex(u, U).rgb, tex.a)`; `shockwaves_glow` the Bomb ring's program
  with `c = 0.45 S - (v + 0.05 n)` and `a = colour2.a * sum(tex(c, c).rgb) * (1 - S)`.
- `U` (`UV_offset`) and `S` (`Shockwave_scalar`) are the same four bytes,
  the `MeshImporter` node's time, which `Update` sets to `p` every frame
  (section 1; `MissileExplosion_Construct` binds both by pointer).
- Node scales: the `Anim Transform` keys, `sphere`/`bloom`/`rays` 1 to 18 over
  60 frames, `shockwave` pre-scaled `0.0977` to 44 ([plasma.md](plasma.md)).

## 4. The bloom boost: a blast raises `Bloom adaption boost` to 6 and lets it back

**Confidence 80** for the law (three functions agree: `Start`, `Update`,
`Reset`; the live `+0x174 = 5.0` while alive and `-1.0` when free matches the
owning branch on Talon's Junction, which authors 5). **Its effect on a frame
is not measured.**

`+0x690` is the environment-settings singleton (`0x006772e8` ->
`EnvSettings_GetOrCreate` `0x003a9520`), and its `+0x53c` is
`HDR and Bloom.Bloom adaption boost` (the registrar's key at `TOC[-0x6340]`
-> `0x007b0770`, stored at `0x003a9d08`; engine default 15.0 at
`0x003a9880`, the circuit's file overrides it).

```text
construct:  +0x178 = env.boost;  +0x174 = -1                         (0x00154ec4, 0x00154e94)
Start:      +0x174 = env.boost
            if env.boost == +0x178: env.boost = K2 (6.0)              (0x0015590c..0x0015592c)
            else:                   +0x174 = -1   (another blast owns it)
Update:     if +0x174 != -1: env.boost = 6 + p (+0x174 - 6)           (0x00154c6c..0x00154c88)
Reset:      if +0x174 > -1:  env.boost = +0x178; +0x174 = -1          (0x00154930..0x0015494c)
```

**It is live.** The post chain reads the field every frame at `0x003b6230`
(`EnvSettings_GetOrCreate` then `lfs f13, 0x53c(r3)`), multiplies by `0.25`
(`0x008b743c`) and the adapted luminance, clamps at 1 (`fsel`) and writes
`(1 - min(adapted * boost * 0.25, 1)) * contribution` into the gate's
constant ([renderer.md](renderer.md), the gate's `contribution.y`). So on a
circuit that authors 5, a blast **lowers** the bloom gate's fade term by up
to a fifth for its second; on one that authors more than 6 (Sol 2 authors
15), it raises it.

**Conflict, open.** `hd-whiteout` reported the gate and composite programs'
patched constants identical in its `base`, `hit0` and `hit1` dumps. With
this law the gate's fade should differ at `hit0` (boost 5.92) unless
`adapted * boost * 0.25 >= 1` in both frames or the constant compared was not
the fade. Not resolved; the next read is the gate program's patched
`contribution.y` in a base and a hit dump of the same boot.

**The Bomb does the same.** `NormalBombBlast` code reads the same field at
`0x001518f8` against its own saved copy (`+0x2d8`, `+0x2d4`) and loads
`0x40c00000` (6.0) beside it; not read further here.

## The pieces together

What one Missile detonation is, for wiring:

1. An entry is taken per viewport the hit point is visible in, while the
   pool holds fewer than 16.
2. For 1.0 s (retired on the first update with `age > 1`): the model at the
   hit, its four nodes at `p = age`; the point light `(500, 200, 50)`,
   range `150 (1 - p)^2`, exponent `7 (1 - p)^2 + 1.5`
   ([weapons.md](weapons.md), "Weapon point lights"); the bloom boost at
   `6 + p (authored - 6)`.
3. Sixteen `ThickLine_bighalo` streaks from the centre in random directions,
   one node per frame for that second, each node living `1.2 U(0.5, 1)` s and
   widening 0 to 2, additive; all from the shared 20-element pool.

## 2048 and Omega

Census, the code not read on either.

- **2048** (`PCSF00007` v1.04 `eboot.elf`): carries `HD_missile_explosion.vex`
  / `.rcsmodel`, `Shockwave_scalar` (three times, as HD) and
  `data/Tex/ThickLine_bighalo.gxt`. **Checked, applies, not wired** for
  sections 1 to 3: the same model, the same parameter name and the same streak
  texture; whether its `Update` keeps `K` = `(2, 1, 6)` was not read. Section 4
  **checked, differs**: 2048 has no `Bloom adaption boost` key (its bloom
  vocabulary is `Bloom.Bloom Factor`, `Bloom.Bloom Gate`,
  `Bloom.Luminance Factor`), so the override cannot be the same field.
- **Omega** (`omega-eu` and `omega-eu-patch` `eboot.bin`): carries
  `HD_missile_explosion.vex`/`.rcsmodel` and `data/Tex/ThickLine_bighalo.gnf`,
  and its `MissileManager_Construct` (`0x013574d0`) builds a pool with an
  explosion object ([../ps4-omega-eu/weapons.md](../ps4-omega-eu/weapons.md)).
  **Checked, applies, not wired** for sections 1 to 3 on the same terms. No
  `Shockwave_scalar` string (its programs bind by another route, unread).
  Section 4 **checked, differs**: no `Bloom adaption boost` key either (its
  vocabulary is `Bloom.Bloom Gate`, `Bloom.Bright-pass *`, `BloomHDR.*`).

## Corrections to earlier sections

Of [weapons.md](weapons.md)'s 2026-10-07 (`hd-weapon-fx`) and 2026-10-08
(`hd-missile-blast`) entries, left as written there:

- "Which function advances `p` and frees the entry was not found ... it runs
  through a computed store": it is `MissileExplosion_Update`
  (`0x00154a78`), called directly from `0x00143650`; the count's decrement is
  the plain `stw r11, 0x10c(r26)` at `0x00143610`, a swap-with-last.
- "There is no age clock in the object ... the node clock runs on the render
  tree's own tick": the age is `+0x170`, and `Update` sets the node clock to it.
- "`+0x17c` ... never stored by this class's own functions": stored at
  `0x00154908` (`Reset`) and `0x00154b38` (`Update`).
- "The 16 entries ... read by nothing ... confidence 45": read by `Update`
  every frame; they are the streaks (section 2).
- "core: the sphere and bloom": `bloom` is a `lightrays_glow` chunk (section 3).
- [plasma.md](plasma.md) calls `0x00154cf0` `MissileManager_Construct`; it is
  the explosion object's constructor, called in a 16-iteration loop
  (`0x00141d2c`, `cmpwi r24, 0x3c`) of `MissileManager_Construct`'s twin
  `0x00141a88` and at `0x00142114` in `MissileManager_Construct`
  (`0x00141e70`) itself.

## Named here

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x00154a78` | `MissileExplosion_Update` | 85 |
| `0x00154858` | `MissileExplosion_Reset` | 85 |
| `0x00155568` | `MissileExplosion_Start` | 80 |
| `0x00154cf0` | `MissileExplosion_Construct` | 80 |
| `0x00143580` | `MissileManager_Update` | 75 |
| `0x0028c660` | `Random_RangeFloat` | 80 |

No Ghidra rename was applied (the decompiler truncates these functions); this
page is authoritative (ADR-0005).
