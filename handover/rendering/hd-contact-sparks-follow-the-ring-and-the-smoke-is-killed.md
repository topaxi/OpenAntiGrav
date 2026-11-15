---
status: ready-to-wire
titles: [hd, 2048, omega]
updated: 2026-11-15
summary: "HD: craft-to-craft contact sparks, the spark's own strength and reach, three damage-smoke corrections: read, not wired; a frame pair owed"
size: M (55)
difficulty: medium (60)
---

# HD's contact sparks follow the contact ring, and its damage smoke is killed on a state change

2026-11-15, lane `hd-impact`. The lane's first claim (the collision **sound**) is wired; this
thread is what the same reading found about the **visuals** and did not wire. Evidence:
[ship-collision-fx.md](../../docs/ghidra/functions/ps3-hdfury-eu/ship-collision-fx.md)
("2026-11-15: who owns the looping spark, and kind 0 is not a no-op"),
[particle-triggers.md](../../docs/ghidra/functions/ps3-hdfury-eu/particle-triggers.md)
("2026-11-15: the damage smoke re-read, and a film of a hit"),
[collision-cues.md](../../docs/ghidra/functions/ps3-hdfury-eu/collision-cues.md) (the ring
walk both hang off).

## Open

Each row: what the original does, the address, what this engine does, and what wiring takes.
Nothing below needs more reverse engineering unless the row says so.

| # | The original | Address | This engine | To wire |
| --- | --- | --- | --- | --- |
| 1 | A craft-to-craft contact record restarts each craft's attached `WO_SHIP_COLL_SPARK_DAMAGE` at the locator nearest the contact point, exactly as a wall record does | ring walk `0x000ec378`, `Ship_SpawnContactFx` `0x000e6fd8`, trigger kind 0 `0x002d9bd0` | HD: sparks for a wall contact only (`contact_fx.rs`, attached mode); `Race::throw_contact_spark` finds no anchors on HD and returns | feed the pair contact (`resolve_craft_pairs`, `contact.point`) to the seat's `ContactFx` as a contact alongside the wall's; the pairs resolve after the craft step, so either carry the point to the next tick or run the seat's contact step after the pairs |
| 2 | The spark stops the first frame with no contact record that asked for it | `0x000ebf8c`..`0x000ebfb8`, the byte `ship + 0x7d80` | the same rule for the wall (`evaluated.wall.resolved.is_none()`) | the rule must count a pair contact too, or row 1's spark is cut the tick it starts |
| 3 | Strength: scale block slot 5 = `min(speed * 0.0125, 1) + min(impulse * 0.0125, 1)`, zero under `0.1`; the instance carries the craft's velocity over 60 | `0x002d9c7c`..`0x002d9d00`, `0x002d9c20`..`0x002d9c40` | `oag_fx::sparks::severity(impact_speed)`, Pulse's `2x + 0.4` law, no carried velocity | a per-title strength law beside the existing one; **what slot 5 scales in the instance is not read** (`0x002846d0` recomputes `+0x64..+0x7c` from the emitter record, [engine-trail.md](../../docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md)): read that before choosing where the number goes |
| 4 | Every craft a local view can see within 100 units of its eye throws the spark, opponents included; never under a Shield | `0x000ec378`..`0x000ec4a4` (distance squared under `10000`) | the seat's own craft only | per-craft attached instances for opponents (the stage's riding slots), gated on the distance to the eye |
| 5 | A record with no friction throws `WO_SHIP_COLL_SPARK_NODAMAGE` (kind 2), a one-shot | `0x000e7730`, kind 2 at `0x002d9d18` | `Trigger::CollisionSparkNoDamage` exists and no HD caller reaches it (no anchors) | **RE first**: which HD records carry no friction (the cage, the floor) is not read |
| 6 | Damage smoke: a hit that changes the state (MILD, MODERATE, CRITICAL) kills the previous state's effects outright, particles included | `0x002a1994` on, `0x0027f7b0(.., 1)`, `0x00285868` | `damage_fx.rs` lets the old state's smoke finish | keep the `Playing` handles per craft and `psys::Stage::kill` them when the state changes |
| 7 | Damage smoke carries the craft's velocity over 60 | `0x002a1c18`..`0x002a1c4c` | no velocity | `oag_fx::psys::carried` (`carry`, `play_carried`) on the riding instance |
| 8 | Damage smoke anchor: the node's origin plus `1.5` of its third row (`2.0` once dead), minus `1.9` of the second row under a view flag | `0x002a0268` | the craft's origin | **read which node `object + 0x38` is first** (if it is the craft's root, row 2 is forward); then one line in `throw_damage_smoke` and `advance_damage_smoke` |
| 9 | An RPCS3-against-ours pair of a weapon hit: one film is taken (table on particle-triggers.md), ours was not rendered | `scripts/rpcs3-hd-missile-film.py`, `scripts/hd-missile-film-pair.py --film DIR --out OURS` | - | render ours from a fresh film and judge rows 1, 6 and 7 on it; the film is game data, kept under `data/`, not tracked |

Leads read in passing, not followed:

- `Ship_SetState` (`0x000e1e20`) state 4 plays `~SHIP_ONFIRE` on the craft's emitter (handle at
  `ship + 0x7a30`) after `0x000d96c8` (the `~SHIP_DESTROYED` function); state 5 calls
  `ShipDamageFx_Update`, the death plume's caller. The wreck sequence, sound and effect, is
  unread on HD.
- What tells kind 0 from kind 3 after `ShipCollisionFx_Trigger` (the word at `locator + 0xe0`).

## Lineage

- **2048: checked, the files apply, the callers not re-read.** Its executable names the same
  effects (particle-triggers.md, "Omega, 2048, and the other titles") and carries HD's contact
  sound law term for term (collision-cues.md, "Lineage"), so the ring walk is there; its
  `Ship_SpawnContactFx` twin was not located. Not wired.
- **Omega: checked, same shape, not re-read.**
  [ps4-omega-eu/ship-collision-fx.md](../../docs/ghidra/functions/ps4-omega-eu/ship-collision-fx.md)
  reads the trigger's kinds 0 and 3 as one shared branch, as HD's are, and calls it a no-op;
  on HD that branch is the attached spark. Whether Omega's does the same needs the stack
  arguments read (`0x01723ae0`). Not wired.

## Next Steps

1. Row 9 first: `scripts/emu-env.sh`, restore
   `hd-racebox-talons-venom-weapons-grid-rpcs3-20574`, run the film, render ours with
   `hd-missile-film-pair.py`, and look at 1.0 s to 2.0 s (the hulls touching) and 0.5 s to
   0.75 s (the struck rival alone).
2. Rows 1 and 2 together (one change, one test: a pair contact on HD starts the seat's attached
   spark and the next contact-free tick stops it).
3. Rows 6 and 7 (small, `damage_fx.rs` alone), then row 8 after its one read.
4. Rows 3 and 4 after the slot-5 read; row 5 after its read.
