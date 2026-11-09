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
| `ring1` | a **template** (`template: true`), billboard, additive, 64x64 sprite, size `0.96..1.152`, alpha `0..100` | muted on Pure: the template draw is unread, `mute_templates` runs for every source but Pulse's PSP |
| `glow` | a template, billboard, additive, 64x64 sprite | muted the same way |

So the earlier reading "one emitter, no `rings` emitter" (this page's predecessors, [`hull-overlay.md`](hull-overlay.md), "Pure's POB",
and `weapon-absorb-pairs.md`) counted the streak alone: the file does author a ring and a glow. They are the likely source of the
ground ring and the bright bloom at the nose and wingtips the original draws and ours does not. Not wired: Pure's template draw
(`ParticleSystem_DrawParticle`'s counterpart, the caller of the template draw) is unread, so what they draw, and where they spawn, is open.

## Wired

`oag_title::Effects::pool_bars` lists the effects whose class-6 emitters keep the bar on a source that does not take Pulse's PSP
laws as a whole; Pure lists `WO_WEAPON_ABSORB` (`oag_pure::effects::EFFECTS`). Pulse's PSP is unchanged (it takes every law),
HD, 2048 and Omega list none (their class-6 draw is HD's render-mode 5 far bar, `far_bars`).
