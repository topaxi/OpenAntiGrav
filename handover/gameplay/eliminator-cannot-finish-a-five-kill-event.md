# A parked-player Eliminator finishes at five, now a little faster than the original's 85 s

2026-10-02, from the `pulse-eliminator` lane. Evidence: `docs/gameplay/race-modes.md` ("Who is
credited with a kill, and how this build's Eliminator reaches five"). Guarded by
`crates/game/tests/eliminator_finish_ground_truth.rs` (target 5, seeds 5, 9, 13, 16, one test each,
plus the respawn-keeps-its-place test).

## Open

- **Time to five is 0.9 times the original's** (2026-10-03, `pulse-eliminator-fire`): with
  opponents firing on the original's own law (`oag_ai::weapon_ai`, below), seeds 1 to 240,
  median **75 s**, mean 76 s, 17 to 154 s, 13.6 kills a minute. Not tuned toward 85 s, whose
  provenance is a single number. Before it: median **108 s**, mean 108 s, 17 to 232 s, the field
  scoring 9.1 kills a minute, with empty-slot opponents steering for weapon pads. Before that (main at `64521c88`): median 117 s, 13 to
  225 s, 8.1 kills a minute; seeds 1 to 24 median 135 s. The original: 85 s.
- **Pad steering shipped** (2026-10-03, `pulse-eliminator-pads`, chosen, not measured): an
  Eliminator opponent with an empty slot steers for the next weapon pad (`Field::pad`,
  `PadSeeking`). Paired median shift -6.3 s [-22.1, -2.7] on `16_Track` over 240 seeds; about
  -13 s on `01_Track` and `09_Track` (48 seeds each). Costs some wall contact (10.2 to 10.7
  ticks a craft-minute on `16_Track`, 5.8 to 9.5 on `09_Track`). Steering every craft and a
  longer or leading pull were within noise of it. Sweep table in `docs/gameplay/race-modes.md`.
  Still missed: `16_Track`'s far pad, 19.8 units off the line and outside the corridor.
  Pads off the lap's road (the split circuits' other branch) are skipped. On `05_Track` and
  `07_Track` it is what makes the mode finish: 37 to 46 and 24 to 48 of 48 seeds. Shield at the
  line per lap and at the finish tick unchanged on `16_Track` (56.4 / 45.1 off, 56.3 / 45.2 on).
  **Open: `05_Track` and `07_Track` still take over 200 s** (median 216 s and 207 s): their
  field is pickup-starved (13.9 and 18.6 s median refill even with the steering).
- **The leader tether is catch-up by slowing the front** (`eliminator_pack_scale`, chosen, not
  measured). It is the lever that makes the mode finish (off: 0 of 8) and it is a throttle
  reduction only. **Accepted by the maintainer 2026-10-02** under the AI-obeys-player-physics
  rule (catch-up by slowing the front is fine; extra speed or thrust for the back is not). The respawn fix alone gives 11 of 24 (it finishes the four pinned seeds); the tether lifts it to 22.
  Wider forward-weapon gates were tried and dropped. A chase on equal speed cannot close, and a
  held-Turbo chase cannot happen here (no Turbo on the Eliminator's pads).
- **Beam and Quake kills are now credited** (the original credits all three of Cannon, Beam and
  Quake via `+0x13c`, read statically, not live), and credit needs the shield-emptying blow, so a
  wall death after a standing-shield hit credits nobody. Over seeds 1 to 24: 24 of 24 finish,
  median 118 s (was 22 of 24, about 210 s), against the original's 85 s.
- ~~Wall deaths~~ **not a lever** (2026-10-03): of 393 opponent deaths over seeds 1 to 24, 392 were
  finished by a weapon blow and 1 by a wall; see `docs/gameplay/race-modes.md`.
- **Held weapons are not a lever** (2026-10-03): the original's own mode-8 rule (untargeted
  Mine/Bomb drops, absorb at 0.001 a decision) and a chosen 10 s absorb on top were swept over 240
  seeds; median shifts -4.8 s [-15.7, +4.6] and -2.3 s [-14.5, +8.4], kill rate flat at 8 a
  minute. Neither shipped. The prototype is not in the tree.
- **An Eliminator absorb is a 1 s Shield, not a refusal** (2026-10-03, static, confidence 80):
  ported for the player (`Race::eliminator_absorb`). Not confirmed live: a PPSSPP absorb in an
  Eliminator watching `craft+0x1b8` bit `0x10` and `+0x188` would settle it
  (`scripts/psp-absorb-frames.py` grants and presses circle already).
- **No Turbo exists in this mode**: `WeaponStats_Elimination.xml` gives it zero odds, so a
  held-Turbo chase cannot happen.
- ~~A backward wrap costs a lap in every mode~~ fixed 2026-10-02 (static read of the original's
  crossing count, confidence 92; a live reversed-craft capture on PPSSPP was not taken). The
  player's Eliminator respawn is the same case and is covered.
- **`WeaponAi_DecideFireOrAbsorb`'s fire half is ported** (2026-10-03), for the Rocket, Missile,
  Plasma, Shuriken, LeachBeam and Quake, in every mode, on `WeaponAIstats.xml`'s odds. Read at
  instruction level: the `+0x52` path test is a cone of about 8.5 degrees plus 4 units off the
  shot's own travel, 20 s horizon, no range limit; `+0x44`/`+0x4c` are never written
  (uninitialised heap, taken as 0, chosen); `+0x5c` is dead. Paired median shift -29.5 s
  [-34.9, -22.7]. Shield per lap at the line 56.3 -> 60.1, at the finish tick 45.2 -> 43.7.
  Single Race moves too (forward-weapon spends up 20 to 40 %). See `weapon-ai.md`, "The fire
  half at instruction level", and `race-modes.md`, "Firing on the original's law".
- **The decision cadence is the soft spot** (confidence 65): the read says every call, and the
  call being once a frame is inferred. Decided four times a second instead, the median is 93 s.
- ~~Not looked for: whether HD/Fury, Omega and 2048 ship a `WeaponAIstats.xml`~~ all three do
  (2026-10-03, `fire-law-inherit`, `race-modes.md`), so every title runs Pulse's law on its own
  odds, inherited and unmeasured on HD, Omega and 2048 and on Pure. Their Eliminator times are
  not measured.
- **Ghidra data rename rejected**: `g_eliminator_kill_target` for `0x08b30fb0` (Hungarian-prefix
  check). The docs and `names.tsv` are authoritative for the data name.

## Next Steps

1. ~~Credit beam, Cannon and Quake kills~~ done 2026-10-02.
1b. ~~Leech Beam outlived a kill and drained the respawned craft~~ fixed 2026-10-02 (sweep after: 24 of 24 finish, 63 to 245 s, median 147 s, was 118 s; the tail comes from the beam no longer being OP): `Beam::link_broken` now requires both craft in `CraftState::Racing` (the original's `Ship_State == 1` pair); a broken link never re-forms.
2. ~~Wall deaths~~ done 2026-10-03: 1 of 393 deaths; not a lever.

   Per-step times (parked player, `16_Track`, six game-minutes):
   - baseline, main `64521c88`, seeds 1 to 24: 24 of 24, median 135 s, 33 to 196 s;
   - step 2 shipped (`Race::eliminator_absorb`, player path): seeds 1 to 24 identical, by
     construction (the player is parked);
   - step 2 prototypes, seeds 1 to 240, not shipped: off 117 s (13 to 225), the original's
     mode-8 rule 112 s (13 to 239), plus a chosen 10 s absorb 115 s (13 to 223);
   - steps 1 and 3: no code change.
3. ~~A drop/discard rule for held weapons~~ done 2026-10-03: swept, within noise, not shipped;
   the Eliminator absorb-to-Shield law found on the way is ported for the player.
4. ~~A held-Turbo chase~~ closed 2026-10-03: the Eliminator table authors no Turbo.
4b. ~~Steer the AI for weapon pads~~ done 2026-10-03: empty-slot opponents only, median 117 to
   108 s over 240 seeds. Per-step times (parked player, `16_Track`, seeds 1 to 240): off 117 s
   (13 to 225); empty slot 108 s (17 to 232), shipped; every craft 107 s (17 to 200), not
   shipped (within noise of empty slot, more wall contact).
5. ~~Port the fire half of `WeaponAi_DecideFireOrAbsorb`~~ done 2026-10-03: median 108 -> 75 s
   over seeds 1 to 240 (per-step: old rule 108 s, 17 to 232; original's law 75 s, 17 to 154,
   shipped; the same law decided four times a second 93 s, 25 to 167, not shipped).
5b. Live-count the decision cadence: PPSSPP, a Single Race, hits on `0x088518b4` for one
   opponent against frames. Settles the 75 s against 93 s question above.
5c. Read `+0x44` live (PPSSPP, any race with opponents): if the heap holds non-zero there, the
   forward-weapon fire index gains 2 with nothing close ahead (5x the Eliminator rate).
6. Live-confirm the Eliminator absorb-to-Shield on PPSSPP (see Open).
