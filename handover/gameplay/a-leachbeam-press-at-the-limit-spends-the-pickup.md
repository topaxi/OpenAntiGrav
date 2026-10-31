---
status: ready-to-wire
titles: [pulse, hd, 2048, omega]
updated: 2026-10-31
summary: "LeachBeam press at the limit spends the pickup on every title (ours keeps it); 2048/Omega allow 8 beams; the Quake press stays armed"
size: M (40)
difficulty: medium (40)
---

# A LeachBeam press at the limit spends the pickup; 2048 and Omega allow eight beams

Evidence: `docs/ghidra/functions/ps3-hdfury-eu/weapon-instance-limits.md` (the table for all four
binaries), `docs/ghidra/functions/ps4-omega-eu/weapon-instance-limits.md`,
`docs/ghidra/functions/vita-2048-eu-v104/weapon-instance-limits.md`, and the 2026-10-31 section
of `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.

**HD matches Pulse**: one Quake and one LeachBeam per race, shared by every craft. So the
"earlier titles adopt HD's more permissive rule" change is **not** warranted, and no Pulse or HD
pool is needed. What the read did find, in order of how sure it is:

| # | Law | Titles | Ours now | Confidence |
| --- | --- | --- | --- | ---: |
| 1 | A LeachBeam press while the limit is reached **spends the pickup and fires nothing** | all four | keeps the pickup | 88 Pulse, 85 HD/2048/Omega |
| 2 | LeachBeams live at once: **8** | 2048, Omega | 1 (inherited from HD) | 85 |
| 3 | A Quake press while a wave is live keeps the pickup **and stays armed**: the next wave fires by itself once the first ends | all four | keeps the pickup, drops the press | 65 (static, two binaries) |

## Open

1. **Spend the LeachBeam pickup on a press at the limit** (player). `crates/raceplay/src/weapons.rs`,
   the `Weapon::LeachBeam` arm of the pickup spend: the `if self.sim.world.leach_beam.is_some()
   { return; }` guard runs before the spend. Move it after the spend, so the slot empties and
   nothing fires: no beam and no fire cue. The press-time switches play nothing for a LeachBeam
   (Pulse `Ship_FireHeldWeapon` case 10 and HD's case 10 at `0x000ea6f4` only bump stats
   counters), and HD's LeachBeam cue (`0x0012d820(2)`) sits inside `LeachBeamManager_Fire`'s armed
   branch. One unread event, `FUN_006778d8(..., 0x1d)` after the HD call in
   `WeaponManager_DispatchFire`, runs on every press, the spent one included. Fix the comment there, which claims the original keeps the
   pickup. Test: a race with a beam up, the player presses with a LeachBeam held, and the pickup is
   gone with `leach_beam` unchanged. It must fail if the guard goes back above the spend.
2. **The opponents' choice, revisited.** `fire_opponent_leach_beam`
   (`crates/raceplay/src/field/opponent_weapons.rs`) holds the pickup while a beam is up and fires
   only with a lock, both labelled chosen. The lock-only reasoning is that an unlocked beam fizzles
   but still holds the race's only pool entry until it retires, locking the player out too. On
   Pulse and HD (capacity 1) that still holds, and the hold-while-busy stays a chosen behaviour
   (an opponent that does not press is not a law; the original AI's own LeachBeam decision is
   unread). Revisit both once 2048 and Omega get the pool of eight (step 3): there a fizzled beam
   holds one entry of eight and the argument mostly goes away.
3. **A pool of eight on every title that ships a LeachBeam (maintainer, 2026-10-31: "8 everywhere").**
   2048 and Omega measure 8; Pulse (PSP and PS2) and HD/Fury measure 1 and take 8 as a **chosen,
   intended divergence**: label their `leach_beams` value chosen in `docs/gameplay/weapon-lineage.md`
   with the reason (the later titles' rule), keeping the measured 1 recorded beside it. Pure ships no
   LeachBeam and stays absent. The determinism references for Pulse and HD runs may then move only
   where a second beam is fired; regenerate them in a separate commit naming this change, never to
   make a test pass.
   - `oag_gameplay::World::leach_beam: Option<Beam>` becomes a fixed pool (`[Option<Beam>; 8]` or
     an array plus a live count). The fire arms the next free entry while `live < limit`, and the
     retire swap-removes like the originals' `LeachBeamManager_Update`.
   - The limit stays per title data, not a `title.name` branch (ADR-0058): a new
     `oag_title::weapons::Weapons` field, e.g. `leach_beams: u8`: 8 on every title that has the
     weapon (2048, Omega measured; Pulse, HD chosen against a measured 1), Pure absent.
   - A new `Weapons` field needs its "Checked axes" row in `docs/gameplay/weapon-lineage.md`
     (`just check-weapon-lineage`).
   - The World grows by seven `Option<Beam>`, so `the_world_snapshot_does_not_grow`
     (`crates/gameplay/src/world.rs`) fires. Raise its ceiling on purpose, in the same commit, saying
     why.
   - Every reader of `world.leach_beam` loops over the pool: the drain, the HUD sight, the strip and
     ball draw (`oag_raceplay::leach_strip`), the cues, the replay and the net state hash. A run on
     any title with never more than one beam live must not move.
4. **The Quake press stays armed (measure first, or wire at 65).** Ours drops a press made while a
   wave is up. The originals leave the request bit set, so the next wave fires the update after
   the first wave's last span ends. Wiring it means a per-slot "Quake requested" flag that the fire
   retries every tick (one bool per craft, also a snapshot change). The live check that raises it to
   85 or more is in the HD page's "Open" (RPCS3; on Pulse, PPSSPP: re-press with a wave live, read
   `craft+0x1b8 & 8` and the wave's age). Doing the live check first is recommended.

## Next Steps

1. Step 1 and its test, about 30 minutes; `just gate-affected` selects `oag-raceplay`.
2. Step 2: a comment and, if (b), one line; 15 minutes.
3. Step 3: 2 to 3 hours with the snapshot and the readers (confidence 40 on the estimate: the
   reader count is a guess from `rg leach_beam`).
4. Step 4: 1 hour for the RPCS3 check, then 1 hour to wire.
