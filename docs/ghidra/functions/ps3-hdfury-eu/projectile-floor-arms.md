# HD's projectile floor arms, and its Plasma's fall

Static read, 2026-11-27, of the four flyers' probe and travel-sweep arms on
`/hdfury/EBOOT-ps3-hdfury-eu.elf` (`BCES-00664`), with Omega's and 2048's own updates read
for the same arms. Disassembly with `llvm-objdump --mattr=+altivec` (none of HD's four
decompiles cleanly; `lvlx`/`lvrx` print as `<unknown>` and are decoded by hand), constants
through TOC `0x008ad4d8`. The Pulse laws these are compared with are on
[rocket-visuals.md](../psp-pulse-usa/rocket-visuals.md); what this engine flies is on
[projectile-floor.md](../../../gameplay/projectile-floor.md).

```sh
llvm-objdump -d --mattr=+altivec --start-address=0x123fb0 --stop-address=0x124d90 \
    data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf
python3 scripts/ps3-toc.py toc 0x00123fb0
```

## Answers

| Arm | HD | Pulse | Confidence |
| --- | --- | --- | ---: |
| Rocket, Plasma probe, floor | velocity turned parallel to the floor, class speed | steer to the ride point | 92 (also measured live, [plasma.md](plasma.md#2026-11-25-on-moa-thermas-bank)) |
| Missile probe, floor | velocity turned parallel to the floor | steer to the ride point | 85 |
| Rocket travel sweep, floor | **no arm**: nothing is written | normal adopted, `(next - prev) / dt` raw | 85 |
| Plasma travel sweep, floor | **no arm** | position moves to `hit + normal * ride` | 88 |
| Missile travel sweep, floor | position moves to `hit + normal * 3`, nothing else | the same | 85 |
| Shuriken probe and sweep, floor | `(next - prev) / dt` raw (dead code) | the same | 80 |
| Plasma probe, `0x7f` | `velocity.y -= 50 * dt`, then pinned to the speed ramp | `velocity -= surface * 50 * dt`, not re-pinned | 88 |

## `Rocket_Update` (`0x00123fb0`)

- `next = previous + velocity * dt`. Probe (`bl` at `0x001249dc`) from `next` to
  `next - normal * 6.0` (`0x008aa174`).
- Classes `0`, `4`, `5` explode. `0x7f` (`0x00124d64`) writes nothing. Any other class
  (`0x001249f4`) stores the hit normal as the carried normal and sets
  `next = hit + normal * 3.0` (`0x008aa150`).
- Every probe arm then falls into `0x001240b8`: the forward row takes the velocity, has its
  component along the carried normal removed and is normalised (`0x00124214`..`0x00124240`),
  and the velocity is that row times the class speed times `0.2777`. So the floor arm is the
  parallel turn, and the open-air arm is too ([own-craft-box.md](own-craft-box.md), measured).
- Travel sweep (`bl` at `0x00124444`) from `previous` to `next`. `0x0012444c`..`0x00124468`
  tests classes `0`, `4`, `5` (mask `0x31`) and explodes on them. Anything else jumps to
  `0x00124740`, and nothing between there and the return writes the position, the velocity or
  the carried normal. **A floor across the step has no arm.**

## `Plasma_Update` (`0x0011fc88`)

- Probe (`0x0011fd80`) `6.0` along the carried normal; classes `0`, `4`, `5` destroy.
- `0x7f` (`0x00120328`): `velocity.y += dt * -50.0` (`0x008a9fe4`), one lane replaced by
  `vsel` with the mask at `*(0x008a9fc4) + 0x10`. That is the table and offset the Missile's
  fall uses (`0x0011e46c`, the same pointer `0x00c47730`), whose lane was measured live as `y`
  ([open-air-arms.md](open-air-arms.md#the-missiles-arm)). The table is filled at run time, so the
  lane rests on that measurement.
- Floor (`0x0011fdc0`): `next = hit + normal * 4.0` (`lvlx` of `0x008a9fd8`), normal adopted,
  forward row orthogonalised against it (`0x0011ff30`..`0x0011ff58`).
- Open air orthogonalises the up row against the forward row instead (`0x00120484`..`0x001204b0`),
  so the fallen velocity keeps its direction.
- Every arm reaches `0x0011ffe4`: velocity = the forward row, normalised and multiplied by the
  speed ramp (`0x001200a0`..`0x001202c4`). **The fall is re-pinned to the ramp each update.**
- Travel sweep (`0x001200f0`): mask `0x31` destroys; no other class writes anything.

## `Missile_Update` (`0x0011d288`)

- Probe (`0x0011d3a4`) `6.0`. A wall class (`0`, `4`, `5`) runs no floor arm. `0x7f`
  (`0x0011e440`) is the fall on world Y.
- Floor (`0x0011d420`): normal adopted, `next = hit + normal * 3.0` (`0x008a9ec4`); at
  `0x0011dfc0` the forward row loses its component along the normal, and `0x0011d904` copies it
  to the velocity: the parallel turn.
- Travel sweep (`0x0011d4bc`): `4` destroys, `0` and `5` bounce (`0x0011dde8`), a floor
  (`0x0011d4d8`) writes `next = hit + normal * 3.0` and nothing else.
- **Open, not wired:** the speed step from `0x0011d90c` runs after every arm, the fall
  included, so an unguided HD Missile in open air keeps its ramp speed. This engine pins a
  Missile only on a floor arm and under guidance (confidence 70: the step was read to its
  branch, its post-update velocity not measured).

## `Shuriken_Update` (`0x001592c0`), which never runs

HD builds no Shuriken ([open-air-arms.md](open-air-arms.md#hd-builds-no-shuriken)). Its probe
floor arm (`0x001597d0`) stores the ride point and the hit normal and writes
`(next - previous) * (1 / dt)` to the velocity `+0x130` with no normalise. The travel sweep
dispatches through the jump table at `0x001598c8`: classes `1` and `3` (`0x00159948`) do the
same raw write, class `2` writes nothing, `0`/`5` and `4` go to their wall arms.

## Wired

`oag_weapons::projectile::FloorArms::ParallelToFloor` (HD, Omega, 2048):

- the Rocket's and Plasma's travel sweep ignores a floor class; a craft on the same step is
  still struck. HD tests craft outside the sweep, in each manager's pass: the Plasma through
  `Plasma_CheckShipHit` (`0x00146838`, [plasma.md](plasma.md)), the Rocket through the call at
  `0x0014c5a8` (after `Rocket_Update` at `0x0014c584`, on the rocket's `+0x40` bit `0`) into
  `0x0014bf80`, which reaches `Rocket_HitCraft` at `0x0014c338` (70: the call chain, not the
  corridor or the bit's meaning);
- the Shuriken's two arms write the raw step (this engine's Omega and 2048 draws can still
  hand one out);
- the Plasma's open-air arm re-pins the speed.

`Weapons::plasma_fall` picks the Plasma's axis: Pulse along the carried normal, HD, Omega,
2048 and Pure world Y. Tests: `crates/weapons/src/projectile/tests/floor_arms.rs`.

## Omega and 2048

**Checked on both executables, applies, wired.** Collision classes are renumbered there
(`4`, `5`, `6`, `8` wall, `0xe` open air).

| Arm | Omega (`/omega/eboot-ps4-omega-eu.bin`) | 2048 (`/2048/eboot-vita-2048-eu-v104.elf`) |
| --- | --- | --- |
| Rocket | `Rocket_Update` `0x013691b0` | `Rocket_Update` `0x81204248` |
| Plasma | inlined in `PlasmaManager_Update` `0x0135e140` ([plasma.md](../ps4-omega-eu/plasma.md)) | `Plasma_Update` `0x811fc650` |
| Missile | `Missile_Update` `0x01351930` | `Missile_Update` `0x811f5ef6` |

All three on both: probe `6.0`; floor adopts the normal, rides `3.0` (Plasma `4.0`) and turns
the velocity parallel; the Rocket's and Plasma's travel sweeps act on wall classes only; the
Missile's floor sweep only moves; the Plasma's and Missile's `0xe` arm is `velocity.y -= 50 * dt`
and the Plasma's is re-pinned. Neither title builds a Shuriken manager
([open-air-arms.md](open-air-arms.md#cross-title), census). Confidence **85**, decompile only:
nothing was measured live.

**Read here, not wired** (both applies, not wired):

- **Their Rocket does not fall in open air** (`0xe` writes nothing and the parallel turn runs),
  as HD's; this engine's `rocket_open_air` for both is still Pulse's fall, chosen.
- **Their probes reach `6.0`** for Rocket, Missile and Plasma; `surface_probe_lengths` for both
  is still Pulse's, chosen.

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x00123fb0` | `Rocket_Update` | 88 | the probe, turn and sweep above; the call into the Rocket's trail ([rocket-trail.md](rocket-trail.md)) |
| `0x013691b0` | `Rocket_Update` (Omega) | 85 | `WO_ROCKET_EXPLO_TRACK`, `Error(Rocket::GetSpeed)` |
| `0x01351930` | `Missile_Update` (Omega) | 85 | `WO_MISSILE_BOUNCE`, `MISSILEEXPWALL` |
| `0x81204248` | `Rocket_Update` (2048) | 85 | `WO_ROCKET_EXPLO_TRACK`, `Error(Rocket::GetSpeed)` |
| `0x811fc650` | `Plasma_Update` (2048) | 85 | `Error(Plasma::GetSpeed)`, the `50.0` fall |
| `0x811f5ef6` | `Missile_Update` (2048) | 85 | `WO_MISSILE_BOUNCE`, `MISSILEEXPWALL` |
