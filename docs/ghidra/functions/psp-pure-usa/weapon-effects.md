# Pure's weapon effect triggers: Missile, Mine, Bomb, Disruptor, and the Bomb's tumble axes

**Binary:** `psp-pure-usa` `BOOT.BIN`, image base `0x08804000`, Ghidra `/pure/BOOT-psp-pure-usa.BIN`.
Read 2026-11-03 (lane `pure-weapon-visuals`), static only: decompile and disassembly through the bridge,
string cross-references (`list_strings`, `get_xrefs_to`), constants with `read_memory`. Companion to Pulse's
[missile.md](../psp-pulse-usa/missile.md), [mine.md](../psp-pulse-usa/mine.md) and [plasma.md](plasma.md).
Decompile-level readings: 85 is the ceiling. `jal` operands are image-base-relative (`jal 0x0004df98` is `0x08851f98`).

Every psys spawn is `FUN_08871e1c(object, "<name>", fourcc, matrix-or-null, attach-mode, 0)` on an
allocated `0x1c0`-byte object. The fourcc is the first four characters' packing read straight from the immediate.

| Address | Name | Confidence | What it is |
| --- | --- | ---: | --- |
| `0x0885c884` | `Missile_Init` | 70 | Spawns `WO_MISSILE_HEAD` twice, attached (`mode 1`) to the matrices at `+0xf0` and `+0x130`; fourcc `0x4548494d` (`MIHE`). |
| `0x0885cf30` | `Missile_Update` | 65 | Per-missile flight step (1627 decompiled lines); the bounce arm spawns `WO_MISSILE_BOUNCE` (fourcc `0x4f42494d`, `MIBO`). |
| `0x08852ebc` | `Missile_SpawnExplosion` | 78 | Spawns `WO_MISSILE_EXPLO` (fourcc `0x5845494d`, `MIEX`) at the given point with an identity basis, then `ScreenFlash_Start(.., 0, pos)`. |
| `0x08851f98` | `Mine_SpawnExplosion` | 80 | `WO_MINE_EXPLO` (fourcc `0x5845494d`) at the point, `ScreenFlash_Start(.., 7, pos)`, then stops the mine's looped sound handle (`mine+0x54`) and plays the cue string `MINEEXPL` (`0x08a480a4`). |
| `0x088513cc` | `Mines_Update` | 72 | The mine pool's per-tick update: fuse `+0x50 -= dt`, flag `4` at `<= 0`; removal loop calls `Mine_SpawnExplosion` (call at `0x088515e0`) for each removed mine. |
| `0x0885a2c8` | `DisruptorExplo_Construct` | 78 | Loads `electric_halo2.vex`, spawns `WO_DISRUPTOR_EXPLO` (fourcc `0x58454944`, `DIEX`) at the hit matrix. |
| `0x0885a5bc` | `DisruptorExplo_Update` | 80 | The halo's per-tick update (vtable at `0x08abc6f8`). |
| `0x08859b78` | `DisruptorFx_Construct` | 55 | The on-craft Disruptor effect object (`disruptor_effect.vex`, `disruptor_cockpit.vex`, cue `~DISRUPTORSHIP`); built by `FUN_08924428(ship)` into `ship+0x9f8`. |
| `0x08858710` | `BombBlast_Construct` | 75 | The Bomb blast object: spawns `WO_BOMB_SMOKERING` (fourcc `0x4d534f42`, `BOSM`) at the blast's own matrix `+0x100`. |
| `0x08858ad0` | `BombBlast_Update` | 75 | The Bomb blast per tick (`weapons-gfx.md`, "The Bomb blast is Pulse's pair"). |

(`Disruptor_Init`, `DisruptorPool_Fire`, `DisruptorPool_Update`, `Bomb_Init`, `Bomb_UpdateSpin` already carry rows in
`names.tsv`; their triggers are read below.)

## Missile (item 2): same law as Pulse

- **`WO_MISSILE_HEAD`:** `Missile_Init` (`0x0885c884`, called by the pool's spawn `0x088525bc`/`0x08852808`) spawns the
  psys twice with the attach flag `1` and the matrices at `missile+0xf0` and `missile+0x130`: two anchors that follow the
  missile, as Pulse's two heads do (Pulse missile.md, "plays `WO_MISSILE_HEAD` at two anchors"). Confidence 75.
- **`WO_MISSILE_BOUNCE`:** in `Missile_Update`, after the flight step a ray cast from the old to the new position
  (`FUN_08824a9c`) that returns a track-surface hit whose surface flag at `+0x6c` is `0` increments the bounce counter
  `missile+0x6c`; for `counter < 5` the velocity (`+0xb0`) is reflected and the psys is spawned at the hit with a basis
  built from the hit normal (`FUN_0885c0a0`, a turn by `0x3f85f92d`, 1.0467 rad, when the normal's dot is above `-0.99`).
  The fifth bounce is the end. Pulse: five bounces then the end, the bounce plays `WO_MISSILE_BOUNCE` (missile.md). Confidence 70.
- **`WO_MISSILE_EXPLO`:** `Missile_SpawnExplosion` has two callers, `0x08852b48` (matches a missile by owner and serial
  in 48 slots, sets flags `0x24`, spawns) and `0x088535f4` (the pool update's end-of-flight path, call at `0x08853ac4`), so
  it plays at the end of flight and for a remote/matched missile. The point is an identity basis plus translation.
  Confidence 70 for the trigger, 80 for the spawn.

## Mine (item 4): same law as Pulse

`WO_MINE_EXPLO` is spawned by `Mine_SpawnExplosion` for every mine leaving the pool with flag `4` set (fuse
expiry, or a craft inside the trigger radius: `FUN_08851cbc` sets `4` when a craft is closer than `_DAT_000539ec`).
Pulse: `Mine_SpawnExplosion` plays `WO_MINE_EXPLO` (mine.md). The wired `Looks` for the laid pose need nothing else.
Confidence 78.

## Bomb (item 4)

- **`WO_BOMB_GLOW`:** `Bomb_Init` (`0x088580d8`, call at `0x088582b8`) spawns it with a null matrix and mode `0` as a
  child of the bomb's scene node (`bomb+0x58`: the allocation is linked under that node with `FUN_08893acc`), then plays
  the cue `0x08a48458`. So the glow follows the bomb for its life. The psys object is **not** stored, nothing stops it
  explicitly: it dies with its parent. Fourcc `0x4c474f42` (`BOGL`). Confidence 70.
- **`WO_BOMB_SMOKERING`:** `BombBlast_Construct` (`0x08858710`) spawns it at the blast's `+0x100` matrix (mode `0`), once,
  at construction. The blast is built by `BombPool_Detonate` (`0x0884f214`, call at `0x0884f368`) and `BombPool_Update`
  (call at `0x0884eaec`). Pulse's `WO_BOMB_SMOKERING` call has the same fourcc `BOSM` (mine.md 869-874). Confidence 75.

## Bomb tumble axes (item 5): read, confidence 60

`Bomb_UpdateSpin` (`0x088583d8`) caches two axes in `.bss` on first use and builds two axis-angle turns
(`FUN_08a2e5e4`, a Rodrigues form that assumes a unit axis; `FUN_08a2e7b0` is a rotation about X):

| Turn | Axis | Angle |
| --- | --- | --- |
| 1 | `0x5420` = row 1 of `RotX(bomb+0xd0 = 0.25 rad)` = `(0, cos 0.25, sin 0.25) = (0, 0.9689, 0.2474)` | `fmod(bomb+0xd4 * t, 2 pi)` with `+0xd4 = -4.0` |
| 2 | `0x5450` = `(0, 1, 0)` | `fmod(bomb+0xd8 * t, 2 pi)` with `+0xd8 = 0.5` |

`t` is `bomb+0x64` (the age; not re-checked against the spawn). The two matrices are multiplied (`0x08853d48`), then the
floor alignment and the scale `bomb+0xdc = 0.4` apply (`0x08853db0`). So the tumble is a spin of `-4 rad/s` about a
local axis tilted `0.25 rad` toward `+Z` from `+Y`, plus `0.5 rad/s` about `+Y`; our `-3.5 rad/s` yaw is these two
rates' sum about one axis, so the tilt is the difference. The row-versus-column convention of the matrix, and whether
the local `Y` is the bomb model's up, are unread: **60**.

## Disruptor (item 3)

- **`WO_DISRUPTOR_LAUNCH`:** `DisruptorPool_Fire` (`0x0884fc14`, call at `0x0884fd28`) spawns it (fourcc `0x414c4944`, `DILA`, mode `2`, no
  matrix) only when `*(weapon+0x3c + 0xe0)` (a field of the firing craft's weapon record) is non-zero. Confidence 65.
- **`WO_DISRUPTOR_HEAD`:** `Disruptor_Init` (`0x08859010`, call at `0x08859190`) spawns it attached (`mode 1`) to the projectile's matrix at `+0x60` (fourcc `0x45484944`, `DIHE`) and plays the `DISRUPTOR` cue. Confidence 80.
- **Teardown and `WO_DISRUPTOR_EXPLO` plus `electric_halo2`:** `DisruptorPool_Update` (`0x088506c4`): a bolt whose age `+0x4c` exceeds `10.0`
  or whose flag `4` is set builds a `DisruptorExplo` (`0x100` bytes) at the hit matrix, releases the head psys, plays `DISRUPTOREXPSHP`
  when flag `0x10` is set (a ship was hit) or `DISRUPTOREXPWAL` when `0x20` is (a wall), then removes the bolt. Same `10.0` s
  timeout as the Plasma pool. Confidence 80.
- **The halo (`DisruptorExplo_Update`, `0x0885a5bc`):** `electric_halo2.vex` is drawn camera-facing (the basis comes from
  `FUN_08a32e08` and `FUN_08859aec`, the Plasma halo's builder) at the hit point, its scale eased per 60 Hz tick as
  `cur += (16.0 - cur) * 0.05` from `0` (`+0xdc = 0x41800000`, `+0xe0 = 0`, `+0xe4 = 0x3d4ccccd`), its tint a grey of
  `(1 - age) * 255`, and the object retires when `age >= 1.0`. Confidence 80.
- **`explosion_gaseous.vex` is not the Disruptor's:** its only loader `0x0885a7d8` is called from `Ship_UpdateDestroyed`
  (`0x089258b8`, via `0x089257b0`): it is the wreck explosion (another lane's function set).
- **On-craft effect (open):** `FUN_08924428(ship)` builds `DisruptorFx_Construct` into `ship+0x9f8` (loading
  `disruptor_effect.vex` and `disruptor_cockpit.vex`, eased scale triple `(1.0, 0, 0.1)` at `+0x1a0`, cue `~DISRUPTORSHIP`;
  the cockpit model is shown only when `*(ship+0xc0)+0xe5` is non-zero). Its per-tick update (vtable `0x2b8690`) and
  the `disruptor_cockpit` placement are unread: **address plus recipe**, read the update slot of the vtable at `0x08abc690`,
  confidence 55.

## Wired (2026-11-03, lane `pure-weapons-wire`)

All names resolve and decode on both Pure pressings (disc test `a_pure_race_loads_the_effects_its_weapons_play`).

| Trigger | Wiring | Pure's table |
| --- | --- | --- |
| `WO_MISSILE_HEAD` (twice), `WO_MISSILE_BOUNCE`, `WO_MISSILE_EXPLO`, `WO_MINE_EXPLO`, `WO_BOMB_SMOKERING`, `WO_PLASMA_FLASH` | the engine triggers Pulse already fires (`flare_effect_for`, `ignite_missile_bounces`, `blast_for`) | now `Origin::Measured`, no longer inherited |
| `WO_BOMB_GLOW` | `Trigger::BombGlow`, a riding flare for a laid Bomb (`flare_effect_for`) | measured, Pure only |
| `WO_DISRUPTOR_HEAD` | `Trigger::DisruptorHead`, a riding flare for the bolt | measured, Pure only |
| `WO_DISRUPTOR_EXPLO` and `electric_halo2.vex` | `blast_for` arm for `Weapon::Disruptor`; the halo is `DisruptorHalo` (target 16, rate 0.05, grey `1 - age`, 1.0 s), camera-facing at the bolt's end | measured, Pure only |
| `WO_DISRUPTOR_LAUNCH` | **not wired**: its gate, `*(weapon+0x3c + 0xe0)`, is a field of the firing craft's weapon record that this lane did not map to a table field | no trigger |

The Bomb tumble is wired as read (confidence 60): `-4.0` rad/s about `(0, cos 0.25, sin 0.25)` and `0.5` rad/s about `+Y`,
the tilted turn outermost (the multiply order is unread, **chosen, not measured**), then the `0.4` scale. Ours is render-side
(`scaled_bomb` reads the projectile's age); no simulation state moved.

The halo's texture tracks play at the halo's age, rate 1 (not read on this object: the Bomb blast's reading, applied by
analogy). It draws faint in the headless shot against the wall it hits; no Pure capture of it exists.
Not captured headlessly: the laid Bomb's glow (the shot's bomb had not left the craft).

## Checked against HD and the other titles

HD/Omega's weapon psys are in other binaries; Pulse is the sibling and matches (same triggers, same fourccs for
Mine/Bomb). HD and 2048/Omega: **not checkable** from this binary.
