# The particle noise force: modifier type `0x11` (PSP `BOOT.BIN`)

Read 2026-11-25 (`det-emp-look-re`), static: decompile and disassembly of Pulse PSP USA's
`BOOT.BIN`. HD's twin and the effect that made it matter, the Detonator EMP's `SPARKS`, are on
[detonator-emp.md](../ps3-hdfury-eu/detonator-emp.md#the-grey-rows-were-sparks-that-never-moved-2026-11-25).
Ours: `crates/fx/src/psys/turbulence.rs`.

## Where it is applied

`ParticleSystem_UpdateParticles` (`0x088f635c`) walks the emitter's modifier list (`+0x9b4`, node
type at `+0x24`, next at `+0x30`, [pob.md](../../../formats/pob.md)) for every live particle,
before `position += velocity * dt`. The switch: type `3` to `ParticleSystem_ApplyDrag`
(`0x088fa6fc`), `4` to `0x088fa744`, `0xb` to `0x088fa818`, **`0x11` to `Modifier_NoiseForce`
(`0x088fab50`)**, `0x12` to `0x088faab8`, `0x13` to `Modifier_WrapBox` (`0x088fb11c`); `5`-`0x10`
do nothing. Types `4`, `0xb` and `0x12` are unread.

## `Modifier_NoiseForce` (`0x088fab50`), confidence 80

Arguments `(node, instance, particle)`; the node's nine floats are `params`.

1. `p = particle+0x40 (position) * 100` (`vscl.q` by `0x42c80000` at `0x088fac0c`).
2. Three points: `x = p * params[0]`, `y = (p + (135.13, -123.45, 12.39)) * params[1]`,
   `z = (p + (3931.49, 11241.24, -141.89)) * params[2]` (`0x43072148`, `0xc2f6e666`,
   `0x41463d71`; `0x4575b7d7`, `0x462fa4f6`, `0xc30de3d7`).
3. Their doubles (`* 2.0`, `0x088fada4`-`0x088fae20`) are taken **before** the clock is added.
4. When `params[3] != 0` the manager clock (`FUN_088f34f0(DAT_08ab2248)`, the vtable `+0x44`
   call `Modifier_WrapBox` also makes: seconds, measured in [weather.md](weather.md)) is added to
   the `z` of each first point (`0x088fae38`-`0x088fae58`).
5. Each axis whose `params[i]` is not `0` is `Perlin_Noise3(point) + 0.5 * Perlin_Noise3(2 point)`
   (`0x088fae5c`-`0x088faf40`), else `0`.
6. The vector is multiplied by `params[4..8]` (`vmul.q`).
7. `k = max(record+0x9ac, 1)` (an int, `1` on every record seen) and `instance+0xc` (the tick
   count, [particle-system.md](particle-system.md)). With `params[8] < 1` the velocity at
   `particle+0x00` gains `force * dt * k`; otherwise the position does.

## `Perlin_Noise3` (`0x088f2be4`), confidence 88

Ken Perlin's reference `noise3`: each coordinate `+ 4096`, truncated to a cell (`& 0xff`) and a
fraction; the permutation at `0x08aeebac`, the gradients at `0x08aef3b4` (three floats each);
`b = p[p[bx] + by] + bz`, the dot of `g3[b]` with the corner offset, blended by `t^2 (3 - 2t)`
on `x`, then `y`, then `z`. The tables are filled on first use (`DAT_08ab2240`).

## `Perlin_InitTables` (`0x088f2604`), confidence 85

Perlin's reference `init`: for `i < 256`, `p[i] = i`; a one-, two- and three-component gradient
each `((rand() & 0x1ff) - 256) / 256`, the 2D and 3D ones normalised (`0x088f2a20`,
`0x088f2a58`); then `p[i]` swapped with `p[rand() & 0xff]` from the top down; then every table
repeated past 256 (`B + B + 2` entries). So no two runs share a field; ours draws the tables once
from a seeded `Rng` (chosen, not measured).

## Who authors it

HD's census (2026-11-25, every `.pob` in `DATA00`-`DATA06`): 14 emitters, among them the
Detonator EMP's `SPARKS` (`0.3, 0.1, 0.3`, timed, `0.1`), its mine and bomb explosions
(`9, 0.9, 9`, `0.1`), `WO_TRAIL_HITSHIP` (`0.01`, `0.2`), `WO_ROCKET_EXPLO`'s trail,
`WO_PLASMA_FLASH`, `WO_LEACHBEAM_HITSHELL`'s spray and `WO_DAMAGE_MILD`'s sparks. Ours applies it
per effect a title lists (`oag_title::Effects::turbulence`); only the EMP's wave is listed. Pulse's
own effects were not censused, and none is listed.

## Names

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x088fab50` | `Modifier_NoiseForce` | 80 |
| `0x088f2be4` | `Perlin_Noise3` | 88 |
| `0x088f2604` | `Perlin_InitTables` | 85 |

Existing names cited: `ParticleSystem_UpdateParticles` (`0x088f635c`), `ParticleSystem_ApplyDrag`
(`0x088fa6fc`), `Modifier_WrapBox` (`0x088fb11c`).
