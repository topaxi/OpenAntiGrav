# HD: the Missile's and Shuriken's open-air arms, and why HD throws no Shuriken

Carried over from [own-craft-box.md](own-craft-box.md), which read only the Rocket's `0x7f` arm.
`/hdfury/EBOOT-ps3-hdfury-eu.elf` (`BCES-00664`), read 2026-11-26 with `llvm-objdump` (the Ghidra
GUI has no function at either update) and measured on RPCS3.

**Answers.**

- **The Missile falls** on `0x7f`, on world Y: `velocity.y -= 50 * dt`. It does not turn like the
  Rocket. Pulse's law, which is what this engine already flies. Confidence **92**: static, and 23
  live updates over three emulator launches.
- **HD never builds a Shuriken**, in any mode: nothing constructs `ShurikenManager`. Confidence
  **88**: static, and the manager pointer read null live twice.
- The Shuriken's dead `Shuriken_Update` reads as a **fall** too, with a probe of **`12.0`** (static
  only, **82**; it cannot be measured because it never runs).

## The Missile's arm

`Missile_Update` (`0x0011d288`) casts its probe at `0x0011d3a4`, reach `6.0` (the float at TOC
`-0x361c`, `0x008a9ebc`, the value measured live before). The return is tested for `0x7f` at
`0x0011d418`, which branches to `0x0011e440`:

```text
0011e444  lvx v12, 0, r24            ; r24 = this + 0xb0, the velocity
0011e448  lfs f0, -0x35dc(r2)        ; 0x008a9efc = -50.0
0011e460  stvewx v12 / lfsx f13      ; velocity.y
0011e468  fmadds f0, f31, f0, f13    ; f31 = dt:  y = y + dt * -50
0011e488  stvx v8, 0, r24            ; only y replaced (vsel)
0011e48c  b 0x0011d494               ; the travel sweep
```

No normalise, and the carried normal `this + 0xd0` is not read. The travel sweep
(`0x0011d4bc`) runs from the previous position `+0xc0` to a point computed with the old velocity, so
the fall moves the missile from the next update.

## Measured live (RPCS3)

Falsifier written before the capture: after a `0x7f` probe, the velocity at the same update's travel
sweep is the parallel turn on the carried normal, or is unchanged, or is any other vector than the
fall.

**Method.** The own-box recipe on
`hd-racebox-talons-venom-weapons-grid-rpcs3-20574`, `--interpreter`, craft parked. The Missile's
velocity `+0xb0` and normal `+0xd0` are read at its probe call (return `0x0011d3a8`) and at its
sweep call (return `0x0011d4c0`), `dt` taken as sweep length over speed. A Missile fired on the
grid probes onto the floor, so each one was **lifted once** (its position `+0xa0` raised 25 or 40
units at its first `Missile_Update`) to put it in open air; everything after that is the game's own.

```sh
scripts/emu-restore-state.sh <lane> data/saves/hd-fury/hd-racebox-talons-venom-weapons-grid-rpcs3-20574.SAVESTAT.zst --interpreter
OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-own-box.py capture \
    --log-dir <dir> --weapon missile --fires 2 --lift 40 --calls 20 --quiet 10
python3 scripts/rpcs3-hd-own-box.py fall <dir>/missile.json ...   # the table below
```

| Launch of the emulator | Lift | `0x7f` updates compared |
| --- | ---: | ---: |
| 1 | 40 | 12 |
| 2 (the GDB stub timed out early) | 25 | 1 |
| 3 | 25 | 10 |

| Hypothesis for the velocity at the sweep | Error over 23 updates |
| --- | ---: |
| fall on world Y, `y - 50 * dt` | largest **0.000004** |
| fall along the carried normal | largest 0.0145 |
| unchanged | smallest 0.83 |
| parallel turn on the carried normal (the Rocket's arm) | smallest 1.07 |

The grid's carried normal is `(0.0032, 1.0000, -0.0081)`, about half a degree off world up. That is
what separates the first two rows. Four records (velocity before, velocity at the sweep, `dt`):

| Launch | Before | After | `dt` |
| --- | --- | --- | ---: |
| 1 | (42.200, 0.447, -1.447) | (42.200, -1.219, -1.447) | 1/30 |
| 1 | (64.488, -2.733, -2.211) | (64.488, -3.567, -2.211) | 1/60 |
| 3 | (42.219, 0.459, -0.703) | (42.219, -1.208, -0.703) | 1/30 |
| 3 | (64.517, -2.716, -1.075) | (64.517, -3.550, -1.075) | 1/60 |

Wired: nothing to change. `oag_weapons::projectile` already drops an HD Missile on world Y; the
record above is now pinned
(`projectile/tests/hd_box.rs`, `an_hd_missile_in_open_air_falls_on_world_y_as_recorded`).

## HD builds no Shuriken

- `WeaponManager_DispatchFire` (`0x0012b2c0`) sends slot flag `0x20000` (slot state `12`,
  `Weapon_FireFromSlot`'s jump table at `0x0012d9b8`) to `ShurikenManager_Fire` (`0x0014cbb0`),
  the one caller of `Shuriken_Init` found there, with the manager at `this + 0x80`.
- Every path of both copies of `WeaponManager_Construct` ends by storing `0` to `+0x7c` and `+0x80`
  (`0x0012c924`/`0x0012c928`, and `0x0012d3f0`/`0x0012d3f4` in the second copy).
- `ShurikenManager_Construct` (`0x0014cdf8`, and its twin `0x0014d6b8`) has no caller: no `bl` or
  `b` reaches it, and no word in the image holds its descriptor (`0x008777c0`, `0x00877810`) outside
  the descriptor table itself.
- Nor is it inlined elsewhere: the manager's vtable (`0x008649a8`, TOC `-0x2494`) is loaded only by
  the two constructors and by three functions that write it back into a `this` they were handed
  (`0x0014ca50`, `0x0014e2d8`, `0x0014e448`, destructor-shaped, no allocation); the
  `ShurikenManager.cpp` tag slot (TOC `-0x2470`) only by the constructors; and the `Shuriken`
  object constructor `0x00158d18` is called only from them (`0x0014cfac`, `0x0014d86c`;
  `0x00158e98` by nothing).
- Live, `*(WeaponManager + 0x80)` read `0` on two restores of the weapons grid.

This explains two earlier observations: no object with `Shuriken.cpp`'s vtable in a race
([plasma.md](plasma.md), "The objects, read live"), and slot states `11` and up crashing the game
([rpcs3-capture.md](../../../reverse-engineering/rpcs3-capture.md), "Giving the player a weapon";
state `12` would call through the null manager).

**The pickup grant drops it.** HD's `weaponstats_Elimination.xml` still gives the Shuriken and the
Repulser (`+0x7c`, zeroed by the same store, read `0` live) nonzero odds, and both pickup draws test
a per-mode weapon mask that holds neither ([pickup-draw.md](pickup-draw.md), 88). Why the table
weighs them is not read.

## The Shuriken's update, which never runs

`Shuriken_Update` (`0x001592c0`) casts its probe at `0x00159400` with the reach at TOC `-0x2094`
(`0x008ab444` = `12.0`). `0x7f` (tested at `0x001597c8`) branches to `0x001598f8`:
`velocity +0x130 .y += dt * [0x008ab460 = -50.0]`, then the travel sweep at `0x0015989c`. The other
arm (`0x001597d0`) writes Pulse's raw `(ride point - start) / dt`, not the parallel turn
([projectile-floor-arms.md](projectile-floor-arms.md#shuriken_update-0x001592c0-which-never-runs)).

## Names

| Address | Name | Confidence | Evidence |
| --- | --- | ---: | --- |
| `0x0011d288` | `Missile_Update` | 90 | the probe, fall and sweep above; its calls watched live |
| `0x001592c0` | `Shuriken_Update` | 75 | the same shape over `+0x130`/`+0x140`/`+0x150`, the fields `Shuriken_Init` fills ([projectile-launch-normal.md](projectile-launch-normal.md)); never runs |
| `0x0014cbb0` | `ShurikenManager_Fire` | 75 | reached from `WeaponManager_DispatchFire`'s `0x20000` bit; calls `Shuriken_Init` |
| `0x0014cdf8` | `ShurikenManager_Construct` | 80 | builds 16 `Shuriken` objects with the `Shuriken.cpp` constructor `0x00158d18` |

## Cross-title

- **2048: checked, applies, not wired** for "no Shuriken is built" (confidence 55, a census, not a
  read of its weapon manager). The 1.04 executable's code references 25 of its 29
  `Backend/Weapons/*.cpp` tags through `movw`/`movt`; the four it does not are `Shuriken.cpp`,
  `ShurikenManager.cpp`, `Repulser.cpp` and `RepulserManager.cpp`.
- **Omega: checked, applies, not wired** (confidence 60, a census). The 1.07 executable carries a
  `Backend\Weapons\*Manager.cpp` tag for every manager 2048's code references (Bomb, Cannon, EMP,
  LeachBeam, LightBarrier, Mine, Missile, Plasma, Quake, Rocket) and none for the Shuriken's or the
  Repulser's. It also lacks `Bomb.cpp` and `BaseMine.cpp` while throwing bombs, so a missing class
  tag alone proves nothing there. Shuriken names for the stats parser, effects, cues and rumble are
  still in it.
- The Missile's open-air arm on 2048 and Omega: not read; both keep Pulse's fall, as HD does.
