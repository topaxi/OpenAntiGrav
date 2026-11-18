---
status: ready-to-wire
titles: [hd, 2048, omega]
updated: 2026-11-18
summary: "HD: frictionless spark (row 5), smoke anchor (row 8) and a frame pair (row 9) open; rows 1-4, 6, 7 landed"
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
| 1 (LANDED (`contact_fx.rs`, `PairContact`; test `a_pair_contact_starts_and_ends_the_attached_spark`)) | A craft-to-craft contact record restarts each craft's attached `WO_SHIP_COLL_SPARK_DAMAGE` at the locator nearest the contact point, exactly as a wall record does | ring walk `0x000ec378`, `Ship_SpawnContactFx` `0x000e6fd8`, trigger kind 0 `0x002d9bd0` | HD: sparks for a wall contact only (`contact_fx.rs`, attached mode); `Race::throw_contact_spark` finds no anchors on HD and returns | feed the pair contact (`resolve_craft_pairs`, `contact.point`) to the seat's `ContactFx` as a contact alongside the wall's; the pairs resolve after the craft step, so either carry the point to the next tick or run the seat's contact step after the pairs |
| 2 (LANDED with row 1) | The spark stops the first frame with no contact record that asked for it | `0x000ebf8c`..`0x000ebfb8`, the byte `ship + 0x7d80` | the same rule for the wall (`evaluated.wall.resolved.is_none()`) | the rule must count a pair contact too, or row 1's spark is cut the tick it starts |
| 3 (LANDED (`sparks::attached_strength`, `contact_fx.rs`; tests `the_seats_attached_spark_plays_at_the_attached_strength`, `the_attached_strength_is_cut_under_a_tenth_and_both_terms_saturate`)) | Strength: scale block slot 5 = `min(speed * 0.0125, 1) + min(impulse * 0.0125, 1)`, zero under `0.1`; the instance carries the craft's velocity over 60 | `0x002d9c7c`..`0x002d9d00`, `0x002d9c20`..`0x002d9c40` | `oag_fx::sparks::severity(impact_speed)`, Pulse's `2x + 0.4` law, no carried velocity | a per-title strength law beside the existing one; **what slot 5 scales in the instance is not read** (`0x002846d0` recomputes `+0x64..+0x7c` from the emitter record, [engine-trail.md](../../docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md)): read that before choosing where the number goes |
| 4 (LANDED (`field_sparks.rs`; tests `an_opponent_pair_contact_within_a_hundred_units_of_the_view_throws_the_spark`, `an_opponent_wall_contact_throws_the_spark_inside_a_hundred_units_only`, `a_shielded_opponent_throws_no_spark`)) | Every craft a local view can see within 100 units of its eye throws the spark, opponents included; never under a Shield | `0x000ec378`..`0x000ec4a4` (distance squared under `10000`) | the seat's own craft only | per-craft attached instances for opponents (the stage's riding slots), gated on the distance to the eye |
| 5 | A record with no friction throws `WO_SHIP_COLL_SPARK_NODAMAGE` (kind 2), a one-shot | `0x000e7730`, kind 2 at `0x002d9d18` | `Trigger::CollisionSparkNoDamage` exists and no HD caller reaches it (no anchors) | **RE first**: which HD records carry no friction (the cage, the floor) is not read |
| 6 (LANDED (`damage_fx.rs`; test `a_state_change_kills_the_previous_smoke_with_its_particles`)) | Damage smoke: a hit that changes the state (MILD, MODERATE, CRITICAL) kills the previous state's effects outright, particles included | `0x002a1994` on, `0x0027f7b0(.., 1)`, `0x00285868` | `damage_fx.rs` lets the old state's smoke finish | keep the `Playing` handles per craft and `psys::Stage::kill` them when the state changes |
| 7 (LANDED (`Stage::carry`; test `the_damage_smoke_carries_the_crafts_velocity`)) | Damage smoke carries the craft's velocity over 60 | `0x002a1c18`..`0x002a1c4c` | no velocity | `oag_fx::psys::carried` (`carry`, `play_carried`) on the riding instance |
| 8 | Damage smoke anchor: the engine flare node's origin plus `1.5` of that node's Z row (`2.0` once dead), minus `1.9` of its Y row under a view flag | `0x002a0268`; the object is the `EngineFlare` at `craft + 0x5f70` | the craft's origin | take the flare node's matrix where the engine flare is placed today and offset along its Z; **which way that Z points along the hull is unsettled** (engine-trail.md), so check the result against a frame before keeping it |
| 9 | An RPCS3-against-ours pair of a weapon hit on a craft: the original's film is taken (table on particle-triggers.md); ours rendered with `hd-missile-film-pair.py` is not a pair for the craft (the tool matches the camera to the blast, and the struck craft is a few pixels away) | `scripts/rpcs3-hd-missile-film.py`; `scripts/rpcs3-hd-weapon.py` and `scripts/hd-weapon-pair.py` take per-frame craft poses | - | take the film with every craft's pose (the film's `aNNNN.json` already has them), render ours posed on the struck rival with `--pose-hold-ticks` long enough for the smoke to be on screen, then judge rows 1, 6 and 7 |

Leads read in passing, not followed:

- `Ship_SetState` (`0x000e1e20`) state 4 plays `~SHIP_ONFIRE` on the craft's emitter (handle at
  `ship + 0x7a30`) after `0x000d96c8` (the `~SHIP_DESTROYED` function); state 5 calls
  `ShipDamageFx_Update`, the death plume's caller. The wreck sequence, sound and effect, is
  unread on HD.
- What tells kind 0 from kind 3 after `ShipCollisionFx_Trigger` (the word at `locator + 0xe0`).

## Lineage

- **Rows 3 and 4, 2048 and Omega: checked, applies, not wired.** Both ship `WO_SHIP_COLL_SPARK_DAMAGE`
  looping like HD's, so the strength law and the reach rule would hold; the callers are the same
  unread twins as above.
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

Rows 1 to 4, 6 and 7 landed 2026-11-17 (attached mode only, so Pulse's burst sparks and every
non-HD title are unchanged; the 2048 and Omega callers are still not wired).

1. Row 9 first: `scripts/emu-env.sh`, restore
   `hd-racebox-talons-venom-weapons-grid-rpcs3-20574`, run the film, render ours **posed**
   from the film's craft poses (not with `hd-missile-film-pair.py`, see the row), and look at
   1.0 s to 2.0 s (the hulls touching) and 0.5 s to 0.75 s (the struck rival alone).
2. Row 8 against a frame (`damage_fx.rs`).
3. Row 5 after its read. Slot 5 of rows 3 and 4 is mapped onto the particle system's
   severity (`scale`), **chosen, not measured** (what `0x002846d0` does with slot 5 is still
   unread); the opponents' spark is one riding `Stage` instance per craft.
