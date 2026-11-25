---
status: ready-to-wire
titles: [hd, 2048, omega]
updated: 2026-11-25
summary: "HD: rows 1-8 landed (slot 5 = particle count wired); floor spark rides (wired); open: a hull-contact film, the posed pair, a CRITICAL frame"
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
| 1 (LANDED (`contact_fx.rs`, `PairContact`; test `a_pair_contact_starts_and_ends_the_attached_spark`)) | A craft-to-craft contact record restarts each craft's attached `WO_SHIP_COLL_SPARK_DAMAGE` at the locator nearest the contact point, exactly as a wall record does | ring walk `0x000ec378`, `Ship_SpawnContactFx` `0x000e6fd8`, trigger kind 0 `0x002d9bd0` | HD: sparks for a wall contact only (`contact_fx.rs`, attached mode); `Stage::throw_contact_spark` finds no anchors on HD and returns | feed the pair contact (`resolve_craft_pairs`, `contact.point`) to the seat's `ContactFx` as a contact alongside the wall's; the pairs resolve after the craft step, so either carry the point to the next tick or run the seat's contact step after the pairs |
| 2 (LANDED with row 1) | The spark stops the first frame with no contact record that asked for it | `0x000ebf8c`..`0x000ebfb8`, the byte `ship + 0x7d80` | the same rule for the wall (`evaluated.wall.resolved.is_none()`) | the rule must count a pair contact too, or row 1's spark is cut the tick it starts |
| 3 (LANDED (`sparks::attached_strength`, `contact_fx.rs`; tests `the_seats_attached_spark_plays_at_the_attached_strength`, `the_attached_strength_is_cut_under_a_tenth_and_both_terms_saturate`)) | Strength: scale block slot 5 = `min(speed * 0.0125, 1) + min(impulse * 0.0125, 1)`, zero under `0.1`; the instance carries the craft's velocity over 60 | `0x002d9c7c`..`0x002d9d00`, `0x002d9c20`..`0x002d9c40` | `oag_fx::sparks::severity(impact_speed)`, Pulse's `2x + 0.4` law, no carried velocity | a per-title strength law beside the existing one; slot 5 is the emission **count** multiple (read 2026-11-25, ship-collision-fx.md, "slot 5 is the particle count"; now wired as `ignite_counting`, the severity stays 1.0) |
| 4 (LANDED (`field_sparks.rs`; tests `an_opponent_pair_contact_within_a_hundred_units_of_the_view_throws_the_spark`, `an_opponent_wall_contact_throws_the_spark_inside_a_hundred_units_only`, `a_shielded_opponent_throws_no_spark`)) | Every craft a local view can see within 100 units of its eye throws the spark, opponents included; never under a Shield | `0x000ec378`..`0x000ec4a4` (distance squared under `10000`) | the seat's own craft only | per-craft attached instances for opponents (the stage's riding slots), gated on the distance to the eye |
| 5 (LANDED (`floor_sparks.rs`; tests `a_floor_record_throws_one_spark_per_record_every_tick`, `a_craft_in_a_floor_sparks_and_in_a_wall_does_not`, `a_floor_only_frame_keeps_the_attached_spark_alive`, `the_kind_follows_the_records_friction_and_the_crafts_state`, ground truth `floor_sparks_ground_truth`); the locator ride read live and wired 2026-11-25, the cockpit-view flag read: none in ours) | A frictionless record throws `WO_SHIP_COLL_SPARK_NODAMAGE` (kind 2, `_ZONE` under the mode mask), a one-shot: a new unattached instance per record, a child of the nearest locator, no strength and no carried velocity, its file's own lifetime. Frictionless (read 2026-11-23, [collision-cues.md](../../docs/ghidra/functions/ps3-hdfury-eu/collision-cues.md), "which records carry no friction"): `Floor Collision`, `Mag Floor Collision` and `Reset Collision` (material 2 and 4, friction `g_ShipFloorFriction`, never written, 0; confidence 78) material 5 unread (a debris lead at `0x0010c958`). A wreck's `(0, 1)` is 0 but never recorded: the compound is not tested against a craft's box. Walls, `collision_trackwall` and live craft keep friction `0.035` and the attached spark. The cage builds no collider. Kind is `friction <= 0 ? 2 : 0`, then 3 when not racing; the byte `ship + 0x7d80` is set for every kind, so a frictionless frame keeps a running attached spark alive. The flag read as the cockpit view (`ship + 0x5f42`, inferred) stops every kind; one spark per record, for the first view that passes | `0x000e734c`..`0x000e7414`, `0x000e7730`; kind 2 `0x002d9d18`..`0x002d9e94`; materials `0x002eaba8`; table `0x000be068` | one `Trigger::CollisionSparkNoDamage` per record, from the nearest locator, where the title has dispatch locators | a floor, mag-floor or reset contact record on HD throws one `CollisionSparkNoDamage` at the nearest locator per record per frame (not per view), no shield drain, without stopping the attached spark that frame. The spark rides its locator (read live 2026-11-25, confidence 88, wired: `floor_sparks.rs`, riding instances). Open: how often HD's box meets a floor at all |
| 6 (LANDED (`damage_fx.rs`; test `a_state_change_kills_the_previous_smoke_with_its_particles`)) | Damage smoke: a hit that changes the state (MILD, MODERATE, CRITICAL) kills the previous state's effects outright, particles included | `0x002a1994` on, `0x0027f7b0(.., 1)`, `0x00285868` | `damage_fx.rs` lets the old state's smoke finish | keep the `Playing` handles per craft and `psys::Stage::kill` them when the state changes |
| 7 (LANDED (`Stage::carry`; test `the_damage_smoke_carries_the_crafts_velocity`)) | Damage smoke carries the craft's velocity over 60 | `0x002a1c18`..`0x002a1c4c` | no velocity | `oag_fx::psys::carried` (`carry`, `play_carried`) on the riding instance |
| 8 (LANDED (`damage_fx::anchor`; tests `the_anchor_is_the_flare_nodes_origin_plus_one_and_a_half_of_its_z_row`, `the_smoke_leaves_the_flare_node_and_follows_it`); the `1.9` term read 2026-11-25: the player's own look-back view only, none in ours; open: the Z sign against a CRITICAL frame) | Damage smoke anchor: the engine flare node's origin plus `1.5` of that node's Z row (`2.0` once dead), minus `1.9` of its Y row under a view flag | `0x002a0268`; the object is the `EngineFlare` at `craft + 0x5f70` | the flare locator plus `1.5` of its Z (toward the nose, into the engine bay); the film's frames cannot separate the sign (particle-triggers.md, "The smoke's anchor against a frame") | `ship + 0x5f43` is the player's look-back view (R1 held reads `01 01` at `+0x5f42..43`), the global is the camera object `*0x009878c0` (destroy camera sets `+0x34 & 6`): a rival never takes the `1.9`. A CRITICAL rival frame needs a hit that leaves the rival alive under 40 (a Missile on a poked rival destroyed it at 30, 50 and 54) |
| 9 (film 2026-11-25: the hulls stay 11 units apart, no touching window; posed pair open) | An RPCS3-against-ours pair of a weapon hit on a craft: the original's film is taken (table on particle-triggers.md); ours rendered with `hd-missile-film-pair.py` is not a pair for the craft (the tool matches the camera to the blast, and the struck craft is a few pixels away) | `scripts/rpcs3-hd-missile-film.py`; `scripts/rpcs3-hd-weapon.py` and `scripts/hd-weapon-pair.py` take per-frame craft poses | - | take the film with every craft's pose (the film's `aNNNN.json` already has them), render ours posed on the struck rival with `--pose-hold-ticks` long enough for the smoke to be on screen, then judge rows 1, 6 and 7 |

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

- **Row 5, 2048 and Omega: ported by title data, not run.** Both carry the `WeaponSpark` entry that gates the locators, so a floor record throws it there as on HD; no 2048 or Omega race was run for it. 2048's `Collision_PairFriction` (`0x811a1870`) reads a never-written float for its floor-class materials and Omega folds it to `0`; both give `(0, 1)` `0.035`; their material 1 is not read. Their contact-fx callers are not re-read.

## Next Steps

Rows 1 to 8 landed (2026-11-17 and 2026-11-25). What is open, in order:

1. **Row 9, a contact film then the posed pair.** `rpcs3-hd-missile-film.py --back 3` holds the
   player a hull width behind the rival (the 12-unit film never touches: particle-triggers.md,
   "A second Missile film"); then render ours with `--field-from` (every craft at the film's pose),
   `--force-hit`, `--camera-pose` from the film's pushbuffer camera (frames with a camera only) and
   score a box over the struck rival. Ours cannot throw the attached spark or the damage smoke in
   a posed capture (no ticks run), so the pair needs a scenario that ticks.
2. **A CRITICAL rival frame**: `rpcs3-hd-missile-film.py --rival-shield S` destroyed the rival at
   S = 30, 50, 54 and left it MODERATE at 75; try the shield written after the hit, or a weaker
   weapon.
3. **Lineage**: locate 2048's `Ship_SpawnContactFx` twin and read Omega's stack arguments at
   `0x01723ae0`; the slot 5 count consumer on both.
