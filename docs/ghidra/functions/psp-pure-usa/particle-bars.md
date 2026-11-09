# Pure's emitter pool draw: class 6 is Pulse's bar

**Binary:** `pure-psp` `BOOT.BIN` (USA, UCUS-98612), image base `0x08804000`.
**Status:** static read of the disassembly (`/pure/BOOT-psp-pure-usa.BIN`, the VFPU words read as VFPU). No live hit on PPSSPP: Pulse's
twin was hit live every frame on `WO_REPULSER` ([`particle-system.md`](../psp-pulse-usa/particle-system.md), "`ParticleSystem_DrawPoolBars`");
Pure's was not breakpointed. No rename and no `names.tsv` row (the addresses stay `FUN_`, as in [`hull-overlay.md`](hull-overlay.md)).

Pure's `Data\Psys\WO_*.POB` files are Pulse's format, and the draw of an emitter's own particles is the same pool dispatcher.

## The dispatcher, `FUN_088756bc` (Pulse's `ParticleSystem_DrawEmitterPool`, `0x08918bf8`)

Confidence **85** (every step below has its Pulse twin; the jump targets are read off the table).

- Pushes the matrix stack, binds the sprite and applies the blend class (`res+0xc0`, `jal 0x0886ec38`, Pulse's `ParticleSystem_ApplyBlendClass`).
- Dispatches only when the atlas grid is at most 16 cells (`lhu 0xa0 * lhu 0xa2 < 0x11` at `0x08875ab4..0x08875abc`); a larger grid
  draws nothing, as on Pulse.
- Switches on `res+0xb8` (`lw a2, 0xb8(a0)`, bound `< 7` at `0x08875ad0`), jump table at `0x08a498f0` (stored unrelocated, `+ 0x08804000`):

| Index | Table word | Target | Pulse's twin |
| --- | --- | --- | --- |
| 0, 1 | `0x00071b78` | `0x08875b78` calls `0x08871e88` | `FUN_089194d0`, the square draw (not read here) |
| 2 | `0x00071c1c` | `0x08875c1c` calls `0x088742b0` | `ParticleSystem_DrawRolledQuads` (a `sin`/`cos` roll and a quad of `size * aspect` by `size`) |
| 3, 4 | `0x00071e00` | `0x08875e00` | nothing is drawn |
| 5 | `0x00071cc0` | `0x08875cc0` calls **`0x088746c8`** | `ParticleSystem_DrawPoolBars` |
| 6 | `0x00071d64` | `0x08875d64` calls `0x08874bc4` | `FUN_08918160`, the capped bar (not read here) |

Index 5 is class 6 and 6 is class 7, the same indices as Pulse's page.

## The bar, `FUN_088746c8` (Pulse's `ParticleSystem_DrawPoolBars`, `0x08917c7c`)

Read against Pulse's five steps (confidence **85**: the arithmetic is the same, the output format differs):

1. `A = M * (particle + 0x40)`, `B = M * (particle + 0x50)` (`vtfm4.q` at `0x08874888..0x08874890`). Skipped when either `z > -1.0`
   (`lui a3, 0xbf80`, `c.le.s` at `0x088748a0..0x088748bc`); no near fade.
2. `B` becomes the particle's `+0x60` (the last drawn frame's `A`, seeded with this frame's on first draw, bit `4` of `+0x81`) when
   `res+0x20 & 0x3000000 == 0` and the byte at `0x08ab0628` (`lbu t0, -0x7c04(0x29)`) is 0: the same test and the same global address as
   Pulse's.
3. `zm = (A.z + B.z) / 2` (`vadd.s`, `vpfxs [1/2]`), both ends scaled by `zm / z` (`vrcp.s`, `vscl.q`): the same screen points on one
   depth, the **mean**, not the farther depth HD's bar and the template wedge take.
4. `d = normalize(B.xy - A.xy)`, `(0, 1)` at zero length (`vcmp.s EZ`, `vcmovt`); `n = (d.y, -d.x) * size` (`+0x70`, `S700`),
   `a = d * size * aspect` (`res+0x4c8`, `S701`, `vscl.p ... S701` at `0x088749c0`).
5. Four corners `A - a - n`, `A - a + n`, `B + a - n`, `B + a + n` (`vadd.q C430, C400, C410` and the `vsub.q` variant, `C400 = (-a, a)`,
   `C410 = (-n, n)`, `C420 = (A, B)`), colour `+0x74`, the UV words from the atlas table at `+0x10..+0x1c` of the cell
   (`lv.q C720/C730`, `0x0887493c`).

**Where it differs from Pulse: the output.** Pure writes six floating-point vertices of `0x20` bytes per particle (`0xc0`, the buffer is
`0x140 * 0xc0`) and submits `GU_TRIANGLE_STRIP` with vertex type `0x1fe` and `6n - 2` indices (`li a1, 0x1fe`, `0x08874b98`); Pulse packs `s16`
positions at `x8` as type `0x11e` and undoes the scale with a `0.125` matrix. The geometry is the same strip law; only its
encoding differs, and the floats carry no `x8` quantisation (Pulse's grid is `1/8` of a view unit).

## What Pure authors for this draw

`Data\Psys\WO_WEAPON_ABSORB.POB` (27,872 bytes) parses to **three** emitters, not one:

| Emitter | Role | Read |
| --- | --- | --- |
| `WO_WEAPON_ABSORB` | the streak: class 6, life 7 ticks, `20` ticks of `3` per tick, colour mode over-life (the table is purple `(109, 22, 167)` to magenta `(181, 14, 147)`), `res+0x4c8` aspect **2.0**, size keyframed to `0.16128`, resource flag `0x2000000` (the streak's far end stays at the spawn), spawn probability `0.1`, 64x64 sprite | drawn on Pure as the bar of this page |
| `ring1` | a **template** (`template: true`), render mode 2 = **class 3**, additive, 64x64 sprite, size `0.96..1.152`, alpha `0..100`, life 50 ticks, stretch `+0xf0` constant, roll keyframed | played on Pure since the template draw below |
| `glow` | a template, class 3, additive, 64x64 sprite, life 30 ticks, stretch keyframed, roll keyframed | played the same way |

So the earlier reading "one emitter, no `rings` emitter" (this page's predecessors, [`hull-overlay.md`](hull-overlay.md), "Pure's POB",
and `weapon-absorb-pairs.md`) counted the streak alone: the file does author a ring and a glow, and they are played now (next section).

## The template draw is Pulse's (static, `/pure/BOOT-psp-pure-usa.BIN`)

Four functions, each read against its Pulse twin (`/pulse/BOOT-psp-pulse-usa.BIN`). Confidence **85** for the set; the addresses stay `FUN_`
(no rename, no `names.tsv` row), like the rest of this page. Not hit live on PPSSPP.

| Pure | Pulse's twin | What matches |
| --- | --- | --- |
| `FUN_088741d4` | `ParticleSystem_DrawInstanceTree` `0x089177e4` | the child chain at `+0x1ac`/`+0x18`, the `res+0x20 & 0x400000` order flag picking pool-then-templates or templates-then-pool, the template list at `+0x1b0` linked by `+0x78`, the death and sibling instances at `+0x1a4`, `+0x1a8` |
| `FUN_08875138` | `ParticleSystem_DrawParticle` `0x089186bc` | the matrix push and identity, blend class `res+0x878` (`FUN_08872c38`, Pulse's `ParticleSystem_ApplyBlendClass`), the **template's own sprite** bound from the record at `res+0x890` (`FUN_0887edf0`, Pulse's `Texture_BindEmbedded`), the atlas `TexScale`/`TexOffset`, and the dispatch on `0x2b033c[res+0x874] & 0xf0000000`: class 7 capped streak, class 6 streak, class 3 rotated sprite, classes 1 and 2 the inline quad. Class 3 draws only at view `z <= -1.0` (`DAT_08a88500` is `-1.0` on Pulse; Pure compares the literal). The table at `0x08ab433c` reads `0x10000000, 0x20000001, 0x3000001f, 0x4000001d`: class is index + 1, as Pulse's |
| `FUN_08872e50` | `ParticleSystem_DrawRotatedSprite` `0x08916610` | read in full against Pulse's disassembly: half-width `aspect * size`, half-height `size`, the same two trig calls in the same order, the same four corner expressions (`B*w - h*A`, `-h*B - w*A`, ...), the same UV constants and `Gu_DrawArray(4, 0x19f, 4, 0, ...)`; arguments are roll `+0x5c`, size `+0x30`, aspect `+0x64`, colour `+0x34`, position `+0x00` |
| `FUN_0893fc34` | `ParticleSystem_UpdateParticleFields` `0x088f7e64` | size channel (`res+0x10`) times the instance severity (`instance + 0x34` through the handle lookup), colour table `res+0x470` at `age * 255.999` unless `res+0x870 == 2`, alpha (`res+0x1d0`), atlas accumulator (`res+0x2b0`), the roll law (`res+0x390`: mode 3 accumulates the spawn rate, flag `0x20` absolute, the coin bit flips the sign), the aspect channel (`res+0xf0`: `v > 0` stores `1 + v`, else the size is multiplied by `1 - v` and `1 / (1 - v)` is stored), the age step, the class 6 and 7 end copy, and the follow of the owner node `+0x120` |
| `FUN_0893f808` | `ParticleSystem_InitParticleFields` `0x088f79b4` | the same field order, the `16.0` seed of the atlas accumulator, `0x10` in `+0x38`, a final `dt = 0` sample |
| `FUN_0893d848` | `ParticleSystem_InitInstance` `0x088f58a4` | the same gate (`res+0x9a4 != 0`, list head `res+0x9a8`, next record `+0x8cc`), one `0x80`-byte particle per template pushed on `instance+0x1b0`, the `+0x94c` child recursion |

`ring1` and `glow` are both render mode 2, so class 3: the branch that is the same routine on both executables, not the inline quad of classes 1 and 2
(`FUN_08872d18`, unread here). `oag_fx::psys::template` already plays that law (`DrawRotatedSprite` quad, roll, stretch, sprite of its own), so enabling
it for Pure is data, not code (`oag_title::Effects::templates`).

**Not read: the global byte at `0x08a8f604`.** `FUN_08875138` calls `FUN_08874090(res+0x878)` before and `FUN_08874114` after the draw when it is
non-zero. [`quake.md`](quake.md) names the byte as the VR-build gate for `WO_QUAKE_VR`; the effect node's draw (`FUN_08872690`) saves it and clears it for a
node whose flag `+0x140 & 8` is set. The writers are `FUN_08847a54` and `FUN_08848bf8` (the craft code). A normal race is taken to run with it clear;
this was not watched live, and the two helpers (they change depth and blend state) are unread.

**What is still open (below 50).** Where the templates spawn and how often: the pair shows ours drawing them for every one of the eight staggered
nodes of `ABSORB_BURST` and the original showing a glow and a ring mostly in the first half second. And the ground ring: the original's ring is a flat
ellipse on the road around the craft; `ring1` is a camera-facing class-3 quad, so it cannot be that ellipse by itself, and ours shows no ring.
Recipe to settle both: break on `FUN_08875138` (`0x08875138`) on PPSSPP with the Pure state `pure-single-vineta-feisar-race.ppst`, press the absorb, and
log per hit `particle+0x5c` (roll), `+0x30` (size), `+0x64` (aspect), the position `+0x00..0x08` and `res+0x874` (and `*(res+0x890)` for the sprite),
frame by frame over 45 ticks; the count of distinct positions per frame gives the spawn pattern.

## Wired

`oag_title::Effects::pool_bars` lists the effects whose class-6 emitters keep the bar on a source that does not take Pulse's PSP
laws as a whole; Pure lists `WO_WEAPON_ABSORB` (`oag_pure::effects::EFFECTS`). Pulse's PSP is unchanged (it takes every law),
HD, 2048 and Omega list none (their class-6 draw is HD's render-mode 5 far bar, `far_bars`).
`oag_title::Effects::templates` lists the effects whose sprite templates play on a source that is not Pulse's PSP; Pure lists `WO_WEAPON_ABSORB`
(`oag_fx::psys::streak::Kept`). Test: `crates/raceplay/src/tests/pure_absorb_bar.rs`
(`pure_loads_its_absorb_burst_as_the_pool_bar_and_plays_its_templates`, fails with the list dropped).

2048 and Omega: **checked, differs** - their templates are HD's lineage (`ps3-hdfury-eu` `PartRender`), not this PSP routine, and are left muted. HD: not this engine.
