---
categories: [gameplay]
---

# A struck craft throws its own hull sparks; they still read weaker than the original's

2026-09-24. The user saw it in the original: a craft taking Cannon rounds
shows sparks and smoke. Ours showed nothing on the struck craft.

**The trigger is recovered, measured and built.** `Ship_Damage`
(`0x088439ac`), on its weapon branch, throws `WO_SHIP_COLL_SPARK_DAMAGE` on
one or two random `Ship Collision Fx` locators of the victim:

- severity 2.4;
- at most once per locator per 0.8 s;
- `WO_SHIP_SPARK_DAMAGE_LEACHBEAM` instead for a LeachBeam hit.

The weapon's own path spawns nothing on a craft. PPSSPP stops at
`ra 0x08844050` on every posted hit, and the smoke is per hit, not a
shield state.

Evidence:

- [shield.md](../../docs/ghidra/functions/psp-pulse-usa/shield.md), "`Ship_Damage`'s weapon branch throws the hit sparks".
- [contact-response.md](../../docs/ghidra/functions/psp-pulse-usa/contact-response.md): corrects the old "death burst only" reading.
- Code: `oag_game::race::hit_sparks` and `oag_gameplay::projectile::WeaponHit`.

Frames are in `data/scratch/hit-sparks/` (gitignored):

- `cannon-orig-vs-ours.png` and `cannon-orig-vs-ours-zoom.png`. The top row is PPSSPP k=0..80, the bottom row ours at ticks 300..380.
- On both sides the hit is posted into the player's pending-damage channel every 6 frames. It is not a fired Cannon.
- On PPSSPP the post goes through `craft+0x120`/`+0x138 = 3`. Ours uses a scratch capture hook calling `cannon::direct_hit` then `throw_hit_sparks`. The hook was never committed; its text is in `scratch-struck-hook.patch`.
- `cap-lowshield/` is the no-hit, 8-energy control.

## Open

- **Ours still reads much weaker than the original, and the draw-side
  pieces did not close it.** The fx-brightness thread's streak UVs and atlas
  advance landed on 2026-09-24 (psys-draw lane,
  [particle-system.md](../../docs/ghidra/functions/psp-pulse-usa/particle-system.md),
  "Streaks, atlas advance and the screen flash"). Re-shot with the same
  hook and ticks: `data/scratch/psys-draw/struck-main-vs-streak.png` shows
  the original, then main, then the streaks.
  - `DrawStreak` (class 6) is a **wedge**. It has a sprite-sized head at
    the particle and a sliver back to its spawn point, sampling the peaky
    `orange_glow2` sprite. So the long yellow rays became orange heads with
    faint tails. That is dimmer and shorter than both main and the
    original's long orange rays.
  - **Wall scrapes changed too**: every Pulse PSP collision spark is the
    same effect. `data/scratch/psys-draw/scrape-main-vs-final.png` and
    `scrape-zoom.png` show main's small white glints becoming small orange
    embers. They are no spikier than before. No original scrape capture
    exists to judge against, so whether the wedge stays on by default is
    the lead's call. The read itself is at instruction level.
  - None of the four emitters authors an atlas grid, so the frame advance
    changes nothing here.
  - The original's white-to-orange bloom hugging the hull is still not
    reproduced. Candidates, none read:
    - the original's view-space build (a common depth, not ours);
    - the light strip behind the craft in the original's shot;
    - `FUN_0883e37c`'s gate;
    - `CockpitHitFx_Arm_q`'s overlay.
  - **The next step is a matched capture, not a tune.** Put the original
    craft on the grid (not over the light strip) and take a struck/unstruck
    diff at the same spot, so the background stops confounding the
    comparison.
  - The smoke is emitted and matches the file. A dump at tick 310 shows 8
    alpha-over puffs 3-9 units across. They are grey to brown
    (`0.22`-`0.49`), alpha up to `0.78`, so they read as dark smoke over the
    dark start grid.
- **Wall sparks and hit sparks keep separate cooldowns.** The original's
  0.8 s gate lives on the locator's `ShipCollisionFx` instance, so the two
  share it. Ours keeps two separate gates, and wall sparks remain the
  player's alone. Chosen.
- **`FUN_0883e37c`**, the display-mask gate in front of the spawn, is unread.
  Ours draws hit sparks on every craft.
- **`CockpitHitFx_Arm_q` (`0x088eeaf8`) is not drawn.** It is a cockpit-view
  (`craft+0x6d`) overlay armed on each weapon hit to the player: 0.6 s linear
  decay, jittered in eighths. What it draws is unread. `Camera_ArmShake` on
  weapon hits (`Ship_Damage`, same block) is not ported either.
- **HD:**
  - The Cannon sparks from the weapon side. `Cannon_ApplyCraftHit`
    (`0x0010f730`) calls `Ship_DispatchCollisionFx(..., 1)`, which spawns
    `WO_SHIP_SPARK_DAMAGE_WEAPON` at the nearest locator; see
    [ship-collision-fx.md](../../docs/ghidra/functions/ps3-hdfury-eu/ship-collision-fx.md).
  - Not built. HD's own `Ship_Damage` is unread, and
    `WO_SHIP_SPARK_DAMAGE_WEAPON` has not been checked against the disc
    inventory.
  - HD also names `WO_DAMAGE_MILD`/`_MODERATE`/`_CRITICAL`, a likely
    shield-state smoke that Pulse lacks. Its consumer is unread.
- **Pure** is unasked and unwired: its `Ship_Damage` is unread.

## Next Steps

- Capture the original's struck craft on the start grid, off the light
  strip, struck and unstruck at the same spot. Compare against
  `data/scratch/psys-draw/streak/struck/` at k=8-16.
- Find HD's `Ship_Damage` (the `uWeaponDamageReceived` telemetry string is
  a lead) and read whether it sparks the hull. Then read the `WO_DAMAGE_*`
  consumer through the TOC-displacement search.

## From the HANDOVER.md index (moved 2026-09-25)

2026-09-24: `Ship_Damage`'s weapon branch throws `WO_SHIP_COLL_SPARK_DAMAGE` (LeachBeam: its own variant) on one or two random hull locators per landed hit, severity 2.4, 0.8 s per locator; built as `race::hit_sparks`, Pulse only. The streak strips and atlas advance landed and did not close the bloom gap (rays became orange wedge heads); next is a matched struck/unstruck capture off the light strip. HD's Cannon sparks are read, not built.
