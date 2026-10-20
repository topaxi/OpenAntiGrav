# Zone's four-corner hover, `Ship_HoverFourCorner`, read whole

Pulse PSP (USA) `BOOT.BIN`, 2026-10-20, lane `zone-hover`. Disassembly of `Ship_HoverFourCorner`
(`0x0884ae90`-`0x0884ba08`) through the Allegrex module, then a live PPSSPP 1.20.4 run. No new
function names: every callee was already named (`Collision_RaycastWorld` `0x08816ac0`,
`Body_AddForceAtPoint` `0x0884d510`, `Body_AddTorqueWorld` `0x0884d604`, `Body_AddForceWorld`
`0x0884d4c8`, `Body_Translate` `0x0884d9f8`). The selector (`Ship_UpdateHover`, game mode 6) and
the bank-to-yaw gain `50.0` are on [zone-rest.md](zone-rest.md).

**How it was watched.** A fresh profile cannot pick Zone and no save on this machine unlocks it,
so as on [zone-rest.md](zone-rest.md) the run is a Time Trial on `16_Track` (Talon's Junction,
Feisar, Venom class, one craft) with `g_game_mode` (`0x08b31048`) written to `6`; `Ship_UpdateHover`
re-tests it every frame. `scripts/psp-zone-hover.py` arms one breakpoint on a callee's entry,
conditional on the return address inside `Ship_HoverFourCorner`, and reads the caller's frame
there; `scripts/psp-zone-hover-check.py` recomputes each output from its recorded inputs in `f32`.
The captures are kept under `data/reference/zone-hover/` (`spring1`, `run2`, `run3`, `run4`
`.jsonl`). A native Zone race was not watched.

## The law, against the racing hover

| | `Ship_HoverTwoPoint` (every other mode) | `Ship_HoverFourCorner` (Zone) | Evidence |
| --- | --- | --- | --- |
| probes | two, `(0, -1.5, +/-6.0) * 0.75` | four, `(+/-2.5, 0, +/-5.0) * 0.75`: **no drop** | `0x0884aeec`-`0x0884b060` build the four from `0x40200000`/`0xc0200000` and `0x40a00000`/`0xc0a00000` with `y = 0`, each scaled by `g_craft_scale` (`0x08ab0e1c`); live, all 340 probe points equal `basis(craft+0x80..0xa0) * offset + craft+0x1a0` to the bit |
| cast | rear derived from front above 50 units/s | all four, every frame | the loop `0x0884b11c`-`0x0884b564` casts `Collision_RaycastWorld` per probe with no speed test |
| ray | probe to `probe - up * craft+0x2f0` | the same | `0x0884b1fc`-`0x0884b238` |
| penetration escape | `up * (1 - h)` under `h < 1` on `Floor` | **none** | after `h` is stored to `craft+0x308 + 4i` (`0x0884b2e0`) the code goes straight to the surface-class test; the loop's only calls are the raycast and `Body_AddForceAtPoint` |
| contact | surface class 1 or 3 | the same | `0x0884b2e4`-`0x0884b338`, `*(*(world)+id*12+0x2454)+0x6c` |
| grounded | `+0.5` a probe | `+0.25` a probe (`0x3e800000` at `0x0884b344`) | live `craft+0x2b0` reads `0.25, 0.5, 0.75, 1.0` across the four calls of a frame |
| spring | `mass * 0.3 * (t - h) * K * (0.75 g' + 0.25) * gravity * (1 + reb * clamp(-0.1 vn, -1, 2)) * (1 - maglock)` | `((((t - h) * K) * gravity) * 0.15) * mass * (1 + 0.8 * clamp(-0.1 vn, -1, 1))` | `0x0884b4a8`-`0x0884b538`: `0x3e19999a` (`0.15`), `0xbdcccccd` (`-0.1`), `f24 = -1.0`, `f22 = 1.0`, `f30 = 0x3f4ccccd` (`0.8`). **No load factor, no rebound or landing rebound, the clamp's top is `1` not `2`, no `(1 - magLockBlend)`** |
| spring direction | craft up | craft up (`craft+0x160`) | `0x0884b51c`-`0x0884b54c`, the scalar finished before it scales `up` |
| contact normal | mean, normalised | `sum * (1.0 / n)`, **not** normalised | `0x0884b6c0` `div.s f12, 1.0, n`, then `vscl.q` at `0x0884b6d8`; live, its length reads `0.99979`-`1.0` |
| alignment torque | `-400 * (up x n)` with the right axis projected out | `-400 * (up x n)`, **nothing projected**: it pitches too | `0x0884b704`-`0x0884b758`: `vcrsp.t`, `vscl` by `0xc3c80000`, then `Body_AddTorqueWorld(body, sp+0x160)` with no `craft+0x170` read between. The decompile drops this call's second argument |
| downforce | `-track_gravity * mass * grounded * (1 - maglock)` along `n` | `-track_gravity * mass` along `n`: **no grounded, no maglock** | `0x0884b7b0`-`0x0884b818`: `mass` (`body+0x374`) times the class block's `+0x100` (`*(craft+0x6c) + class * 0x80`), negated, `vscl` onto `n`, `Body_AddForceWorld` |
| bank-to-yaw | `30 * right.y * (1 - maglock)` | `50 * right.y * (1 - maglock)` | [zone-rest.md](zone-rest.md) |
| trailing snap | none | a 10-unit ray down from the position; `d = dot(pos - hit, hit.normal)`; if `3 < d < 10`, `Body_Translate(n * (3 - d) * 0.25)` | `0x0884b824`-`0x0884b9c0`: `0x41200000` (`10.0`), `0x40400000` (`3.0`), `0x3e800000` (`0.25`), `sp+0x60` (the hit normal) as the axis. Any surface: no class test. It runs every frame, **with or without contact**: the no-contact branch at `0x0884b654` lands on it |

The torque and downforce go straight to the body (`body+0x130`, `body+0x100`) where the racing
law adds them into the craft's accumulators `craft+0x350`, `craft+0x330`; the accumulators are
drained into those same body slots at the end of `Ship_UpdateCraft` ([engine.md](engine.md#the-four-accumulators)),
so the sums are the same.

## Live

| Read | Frames | Result |
| --- | ---: | --- |
| spring, at `Body_AddForceAtPoint` from `0x0884b554` | 340 probe calls (85 frames) | every force equals the formula above from the recorded `t`, `h`, `K = 1.3333334`, `gravity = 85.0`, `mass = 1.0`, point velocity and normal, max error `3.8e-6` on forces of about `20` (one ulp); heights settle at `2.83`-`2.92` on a `4.125` target, so the centre rests about `2.9` above the floor against the racing law's `4.0` |
| torque, at `Body_AddTorqueWorld` from `0x0884b758` | 300 four-contact, 4 two-contact | `-400 * (up x n)` exactly; `dot(torque, right)` reads `-3.80`..`+2.76`, so the pitch component is there |
| downforce, at `Body_AddForceWorld` from `0x0884b818` | 1800 four-contact, 4 two-contact | `-80.0 * 1.0 * n` exactly on both, with `craft+0x2b0 = 0.5` on the two-contact frames: the racing law would have given half |
| snap, at `Body_Translate` from `0x0884b9bc` | 4 | `n * (3 - d) * 0.25` exactly, `d` `3.87`-`4.00` |

The partial-contact and snap frames were made, not found: 1800 frames of driving into walls never
left four contacts. At `Ship_HoverFourCorner`'s entry the capture raised the body and the craft's
copy of its position by `1.0` along up and tilted the basis rows the probes are placed with
(`craft+0x80..0xa0`) by 20 degrees, for that one frame, so the front probes miss and the rear
ones hit. The law under test reads those rows as it always does.

Not watched: the damper clamp's top branch (`-0.1 vn > 1`, a probe closing at over 10 units/s;
the instruction read only), the magnet-lock blend (zero on every frame; its absence is the
instruction read: neither the spring nor the downforce loads `craft+0x280`).

## Confidence

**92** for the probe layout, the spring, the contact normal, the alignment torque, the downforce
and the snap: instruction read plus a bit-exact live recomputation on each. **88** for the
missing escape (instruction read; nothing live can show an absence better than the listing does).
**85** for the damper's top clamp and the missing magnet-lock factor (instruction read only).

## Ported

`oag_physics::hover::Rig` carries each difference as an axis whose default is the racing law;
Pulse's `oag_pulse::race::ZONE_HOVER_RIG` sets them, `oag_title::RaceDefaults::zone_hover_rig`
carries it, and `oag_raceplay::load` flies it when the mode is `Mode::Zone`.

## Also gated on mode 6 in the craft update

`Ship_UpdateCraft` reads `g_game_mode` once (`0x08849ab8`): `lw a0,0xb8(0x08b30f90)`, `xori a0,6`,
`sltu`, and `beq a0,zero` at `0x08849ac4` skips `Ship_UpdateBrakes` (`0x088489d8`) in mode 6 (and
also tests the contact flag at `0x08849acc`, [engine.md](engine.md), step 2). Live: with the mode
written to `6`, a breakpoint on `Ship_UpdateBrakes` was not reached in 3 s, twice; written to `1`, it
was reached 30 times in a row. Confidence **92**.
