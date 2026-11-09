# Trail hits and the class-6 streak on Omega

2026-11-09, `hd-trail-hit-draw`. Two findings off `eboot.bin` (EU base), both
static:

1. Omega's CPU particle pusher builds a class-6 streak as Pulse's pool bar in
   view space. This is the CPU twin of HD's `PartRender` SPU job
   ([engine-trail.md](../ps3-hdfury-eu/engine-trail.md), "The class-6 draw law").
2. Omega's original never fires the trail-hit burst. Its CPU port of HD's
   `Trails` SPU job never sets the hit bit the spawn reads.

## `Particle_PushBatchByMode` - `0x01714de0` (confidence 75)

`heat-haze.md` lists it as the "second pusher". It walks an emitter's
particle pool, `switch`es on the emitter record's `+0xb8` render mode, writes
the vertices of every live particle, and appends one batch record. The batch
copies `emitter+0xc84` as its `kColourScale`, and `+0xcac` as its state index
(`Particle_PushQuadBatch`'s same tail).

**Mode 5 (draw class 6), `0x017157c2` onward.** A particle is a record at pool
`+0x20 + index * 0xe0`. The case reads `+0x20` (position `A`), `+0x30` (the
streak's other point `B`), `+0x70` (size), `+0x74` (RGBA bytes) and `+0x78`
(atlas frame). Per particle:

1. Both points go through the view matrix. The particle is skipped unless
   both view `z` are `<= -1.0`.
2. `z = min(zA, zB)`, the farther end. `A' = A_view * (z / zA)` and
   `B' = B_view * (z / zB)`.
3. `d = (B' - A')` with `z`, `w` zeroed; `n = d * rsqrt(|d|^2)` (Newton-refined;
   the constant `_DAT_017fdaf0` when the length is 0).
4. `perp = (n.y, -n.x) * size` and `cap = n * size * emitter+0x4c8`.
5. Four vertices of `(x, y, z, colour, u, v)`, 24 bytes each, all at depth `z`:
   `A' - cap - perp`, `A' - cap + perp`, `B' + cap + perp`, `B' + cap - perp`.
   UVs come from the atlas cell table: `(u1, v1)`, `(u0, v1)`, `(u0, v0)`,
   `(u1, v0)` from the cell's `+0`, `+4`, `+8`, `+0xc` words.

That is Pulse's pool bar (`crates/fx/src/psys/streak.rs`, `StreakDraw::Bar`),
except for the depth: Pulse's uses the mean, this uses the farther end.

## The trail-hit burst never fires (confidence 75)

The spawn chain is intact:

- **`TrailHit_Spawn` (`0x01724f50`, 80)** names `WO_TRAIL_HITSHIP(_RED)`, tags
  `0x494c5254`/`0x524c5254`, and carries the craft's velocity. It is HD's spawn
  ([engine-trail.md](../ps3-hdfury-eu/engine-trail.md), "Omega: checked,
  applies").
- **`Trail_SpawnHitBursts` (`0x016f60e0`, 80)** loops the 16 trail slots (stride
  `0x750`). For a slot whose `+0x6d0` has **bit `0x4`** set, it finds the
  intruder's nearest hull anchor (up to 10, list at craft `+0x81a0`) and gates
  on two craft-state bytes. Then it calls `TrailHit_Spawn` and clears the bit.

Nothing sets that bit. Every instruction in the program with operand
`+ 0x6d0]` was swept (66 matches). The trail-slot writers are:

| writer | what it does to `+0x6d0` |
| --- | --- |
| `FUN_016f20a0` (`0x016f25ca`, trail creation) | `& 0xfff8 \| 0x14000001`: ring length 20, bit 0 |
| `0x012ef5bb`, `0x01304976`, `0x01312979`, `FUN_0172cda0`, `FUN_0172ce60`, `FUN_0172cf50` | `& 0xff00fff8 \| 1`: reset |
| `Trail_UpdateAll` (`0x016f6320`, 65) | sets bit `0x8` and bit `0x10` (from slot table `+0x7cd8`) |
| `Trail_UpdateRing` (`0x016f6e00`, 70) | clears bits 0-1 on restart, **clears bit `0x4`** every frame, writes the ring index |
| `Trail_SpawnHitBursts` | clears bit `0x4` |

On HD the bit is set by the `Trails` SPU job's hit test (engine-trail.md, the
`0x002e3b98` call). `Trail_UpdateRing` is Omega's CPU port of that job: it
rebuilds the ribbon ring (the 1/3 and 2/3 interpolated samples, the
three-segment `sin`/`cos` cross-section, half-float widths, colours). It
receives the per-craft table `Trail_UpdateAll` builds at `+0x7620` (position
matrix and team byte, 0x60 per craft). It uses that table only to pick a colour
set (`param_3 + 0x54`). There is no intersection loop.

2048 has the same gap ([vita-2048-eu-v104/trail-hits.md](../vita-2048-eu-v104/trail-hits.md)).
Omega reads through 2048's code lineage (`docs/formats/omega-status.md`), so the
likeliest history is that the hit test was lost when 2048 ported the SPU job to
the Vita's CPU, and Omega inherited the port. Players report not seeing the
burst on Omega, which agrees.

Why 75 and not higher: the census is an operand sweep. A write through an
aliasing pointer (the `LEA` of `slot+0x6d0` at `0x016f25c2`, `0x016f6152`) would
not show. Both of those aliases were followed: the first only feeds the
creation write and the second is the spawn loop's own clear. Nothing was run
live (shadPS4 has no memory readout).

**This project's wiring.** Omega's race plays the burst only with
`graphics.trail_hits` on, from Omega's own `WO_TRAIL_HITSHIP`
(`oag_title::Effects::trail_hits_native`).

## Names applied

| Address | Name | Confidence |
| --- | --- | --- |
| `0x01714de0` | `Particle_PushBatchByMode` | 75 |
| `0x01724f50` | `TrailHit_Spawn` | 80 |
| `0x016f60e0` | `Trail_SpawnHitBursts` | 80 |
| `0x016f6e00` | `Trail_UpdateRing` | 70 |
| `0x016f6320` | `Trail_UpdateAll` | 65 |
