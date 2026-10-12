# A destroyed craft and craft-against-craft contact

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse, PSP, UCUS-98712) |
| **Subsystem** | rigid body step, ship state machine |
| **Related** | [`contact-response.md`](contact-response.md) (`Body_StepWorld`, `Body_ResolveContactPair`), [`collision.md`](collision.md) (`Collision_BoxAgainstBox`, "Settled 2026-09-07"), [`shield.md`](shield.md) ("State 6 measured", "State 8 measured"), [`ship-wreck-model.md`](ship-wreck-model.md) |

Lane `wreck-faithful`, 2026-10-12. Question from the maintainer: should a destroyed craft collide with
the craft still racing? Answered by a live PPSSPP measurement and the instruction that explains it.

## The law

**A craft takes part in craft-against-craft contact in every state except the original's state 6 (and 7).**
So a craft that is exploding (state 4) and a wreck that has just landed (state 5, `1.5` s) are solid; a
single race's wreck, from the moment state 6 begins, is driven through. In the Eliminator, whose
destroyed craft goes 4, 5, 8 and back to 1 (never 6), the craft is solid the whole way.

Confidence **90**: a measurement in three modes and the instructions agree.

## The mechanism: one byte on the box collider

| Address | What | Evidence |
| --- | --- | --- |
| `0x0884ef30` `Body_ResolveContactPair` | one caller, `Body_StepWorld` (`0x0884f70c`), per narrowphase contact between two bodies | `get_function_callers`; the caller has no state test of its own |
| `0x08817054`, `0x08817060` in `Collision_BoxAgainstBox` | `lbu t3, 0x68(t0)` and `lbu t3, 0x68(t2)`: the pair is refused unless **both** colliders' byte `+0x68` is non-zero | disassembly; `collision.md` records the same gate |
| `0x08818b9c` in `0x08818b78` | `sb 1, 0x68(a0)` - the collider initialiser, called from `Collider_InitBox` | `sb` search of the whole image: the only writer of `1` |
| `0x088445d0` in `Ship_SetState` | case 6: `lw a0,0x794(s0); lw a0,0x3bc(a0); sb zero,0x68(a0)` | disassembly |
| `0x088445f8` in `Ship_SetState` | case 7: the same store | disassembly |

`entity+0x794` is the craft's body, `body+0x3bc` its box collider. Cases 4 (`0x0884430c`), 5
(`0x08844548`) and 8 (`0x088446ec`) do not touch the byte. **The byte is read nowhere else in the
collision code** (`lbu`/`lb` search for `0x68(`: the two `Collision_BoxAgainstBox` reads; the other
hits are other structures: `FUN_0888c408`, `FUN_0892b638`, `FUN_08831104`, `SoundEmitter_*`, `Wad_*`).
So only craft-against-craft contact skips a wreck. The box-against-mesh pass the circuit's floor and
walls go through does not read it, which is why a wreck still lands and comes to rest (`shield.md`,
"State 6 measured": speed `0.0` from frame 410, never moved).

Nothing writes the byte back to `1` after the initialiser, so a craft that reaches state 6 or 7 stays
non-solid until the process ends. State 6 is not left in a single race ("State 6 measured"); state 7's
callers are `FUN_08847f54` only and were not exercised.

## Measured on PPSSPP v1.20.4, software renderer, Pulse USA

Method (`scripts/psp-wreck-contact.py`, `scripts/psp-wreck-phases.py`): in a live race an opponent
(grid slot 3 or 4, `entity+0x368 == 2`) is moved by calling `Body_SetPosition` (`0x0884d840`) from a
`Ship_UpdateCraft` stop onto the stationary player (offset `(1, 0, 1)`), its velocity zeroed, and
`Body_ResolveContactPair` is breakpointed. A stop counts when its two body arguments are exactly that
pair; every stop also records the target's `entity+0x8C`, `entity+0x874` and `collider+0x68`. Only one
execution breakpoint fires at a time on this PPSSPP, so the pair breakpoint and the
`Ship_UpdateCraft` one are never armed together. Other pairs (two grid-mates overlapping) keep
resolving through every leg, so the breakpoint was alive.

| Mode | Target's state | `collider+0x68` | Stops for the exact pair |
| --- | --- | ---: | ---: |
| Single Race | 1 (control) | 1 | 22 of 30 (run 4); 6 of 6 (run 2) |
| Single Race | 4 (`+0x874` 0.417) | 1 | **4 of 4** |
| Single Race | 5 (`+0x874` 1.5) | 1 | **4 of 4** |
| Single Race | 6 (`+0x874` -0.85) | **0** | **0** (30 stops, all other pairs; and 24 in another run) |
| Eliminator (`g_game_mode` 8) | 1 (control) | 1 | 4 of 4 |
| Eliminator | 4 | 1 | 4 of 4 |
| Eliminator | 5 | 1 | 4 of 4 |
| Eliminator | 8 (`+0x874` 0.8) | 1 | **4 of 4** |

The byte's row-by-row trace in the Eliminator (frames after `Ship_Damage(1e6, ...)`): `1` in 4, 5 and 8,
`1` after the 8 to 1 edge; the Single Race trace reads `1` through state 5 and `0` from the first
state 6 row. The 8 to 1 edge also answers the persistence question for the Eliminator: `entity+0x860`
goes `0x1621 -> 0x421` (destroyed bit `0x1000` cleared) and `live_is_wreck` goes false, so **the respawned craft
is the ordinary hull again and no wreck hull is left behind** (`Ship_SetState` case 1 calls
`Ship_SelectHullModel`, `0x0884417c`). Confidence **90** (one run, both edges read per frame).

Caveats, stated rather than hidden: the wreck is injected by `Ship_Damage` from a debugger stop, not a
weapon; one track (not Talon's Junction) and one profile. **The method had to be repaired twice**:
two early runs (a stop cap of 6 and a 2 s wait per stop) read 0 for the control and for states 4 and 5
alike, because the pair stops of other craft used the cap up or the wait ran out; they were discarded, not
averaged in. The runs behind the table use a cap of 40 stops, an 8 s wait and stop after four stops for the
exact pair. In the last Single Race run the control leg (a zero-velocity teleport about 20 frames after the
start, with the target still in its start state 3) read 0, while states 4 and 5 read 4 of 4 in the same run; the
moving-craft controls (22 of 30, 6 of 6) and the Eliminator's (4 of 4) fired. The method also cannot say
whether a wall hit differs: a resting wreck reached no single-body contact.

## At the flag, and in the standings

`FUN_088418e0` promotes a craft to state 2 only when `Ship_State(entity) == 1`
([`race-finish.md`](race-finish.md)), and `Race_FinishAllCrafts` never runs in Single Race (same page).
So a state-6 wreck is not touched at the flag: it stays state 6, the wreck stays the live model and it
stays non-solid. Confidence **75**, derived from the two cited readings and the 13 s live run in
`shield.md`; **not measured to the flag** (no race was run to its end).

## Port

`oag_physics::damage::solid_to_craft` and `ELIMINATED_SOLID_DURATION` (`1.5`),
`oag_raceplay::Race::resolve_craft_pairs`. Ours folds the original's states 4 and 5 into
`Destroyed` and `Eliminated`; `Eliminated` now counts `state_timer` down from `1.5` and the pair pass
skips a craft whose timer has run out, except in the Eliminator. Tests that fail when the law is
dropped: `damage::tests::a_wreck_is_solid_to_craft_for_state_fives_dwell_and_not_after`,
`tests::wreck_contact::*`. Every title inherits Pulse's rule because no other title has a measured one
of its own; the Pure static check below agrees.

Not changed, and not measured: whether a weapon strikes, a raycast shadows or the AI avoids a wreck.
The original's collider byte is not what those use (the byte is read only by the pair test), so they are
separate questions.

## Other titles

| Title | Result |
| --- | --- |
| Pure (PSP) | **checked, applies** (static, confidence 75): `FUN_08824fbc` reads `0x68(collider)` of both sides at `0x08824fe4`/`0x08824ff0` - offset `0x28` into the function, as in Pulse - and `Ship_SetState`'s twin `FUN_08928824` writes `0` to `0x68(*(entity+0x8b4 -> +0x3fc))` in two cases. See [`../psp-pure-usa/wreck-collision.md`](../psp-pure-usa/wreck-collision.md). Not run live. |
| HD (PS3) | **checked, structure present, writer not found**: `Collision_ProcessPairs` (`0x00038428`) tests a per-body byte at `+0x70` before it queries a collider's neighbours and `+0x78 & 1` per entry. Which function clears it for a destroyed craft was not located (`Craft_SetState`, `0x000f0630`, is the hover state and clears nothing). Next: xrefs to `+0x70` stores from the ship's own state setter. Ours inherits Pulse's rule. |
| 2048 (Vita), Omega (PS4) | **not checked** in this lane: the collider layout differs (ARM, x86-64) and no equivalent of `Collision_BoxAgainstBox` was identified. Next: find the pair dispatch by its 15-axis SAT and read its enable test. Ours inherits Pulse's rule. |

## Open

- The meaning of state 7 (`FUN_08847f54` is its only caller) and whether any mode reaches it.
- A wreck made by a weapon or a wall rather than `Ship_Damage` from a debugger stop.
- The same measurement on the EU disc and on a second circuit.
