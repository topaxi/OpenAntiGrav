# A parked-player Eliminator finishes at five, slowly; the remaining gap is the original's 85 s

2026-10-02, from the `pulse-eliminator` lane. Evidence: `docs/gameplay/race-modes.md` ("Who is
credited with a kill, and how this build's Eliminator reaches five"). Guarded by
`crates/game/tests/eliminator_finish_ground_truth.rs` (target 5, seeds 5, 9, 13, 16, one test each,
plus the respawn-keeps-its-place test).

## Open

- **Time to five is about 1.4 times the original's**: 65 to 325 s, median 118 s, over seeds 1 to 24
  on `16_Track`, player parked, against 85 s; 24 of 24 finish within six game-minutes (after the
  beam and quake credit).
- **The leader tether is catch-up by slowing the front** (`eliminator_pack_scale`, chosen, not
  measured). It is the lever that makes the mode finish (off: 0 of 8) and it is a throttle
  reduction only. **Accepted by the maintainer 2026-10-02** under the AI-obeys-player-physics
  rule (catch-up by slowing the front is fine; extra speed or thrust for the back is not). The respawn fix alone gives 11 of 24 (it finishes the four pinned seeds); the tether lifts it to 22.
  Wider forward-weapon gates were tried and dropped. A chase on equal speed cannot close; **a
  held-Turbo chase (Turbo is lawful speed) was not tried**.
- **Beam and Quake kills are now credited** (the original credits all three of Cannon, Beam and
  Quake via `+0x13c`, read statically, not live), and credit needs the shield-emptying blow, so a
  wall death after a standing-shield hit credits nobody. Over seeds 1 to 24: 24 of 24 finish,
  median 118 s (was 22 of 24, about 210 s), against the original's 85 s. Wall deaths
  and why opponents scrape lethally here are still not investigated.
- **Held weapons**: Mine, Bomb, Cannon and Leech Beam together were held for about a quarter of all
  craft-ticks and are rarely usable by a leader or a tail; Eliminator refuses absorbing so those
  craft stop collecting pickups. A discard or a drop rule would free the slot (chosen, would need a label).
- ~~A backward wrap costs a lap in every mode~~ fixed 2026-10-02 (static read of the original's
  crossing count, confidence 92; a live reversed-craft capture on PPSSPP was not taken). The
  player's Eliminator respawn is the same case and is covered.
- **`WeaponAi_DecideFireOrAbsorb` is not ported.** Its `+0x52` predicted-path test and the fields `+0x44`, `+0x4c`, `+0x5c` are unread.
- **Ghidra data rename rejected**: `g_eliminator_kill_target` for `0x08b30fb0` (Hungarian-prefix
  check). The docs and `names.tsv` are authoritative for the data name.

## Next Steps

1. ~~Credit beam, Cannon and Quake kills~~ done 2026-10-02.
1b. ~~Leech Beam outlived a kill and drained the respawned craft~~ fixed 2026-10-02: `Beam::link_broken` now requires both craft in `CraftState::Racing` (the original's `Ship_State == 1` pair); a broken link never re-forms.
2. Wall deaths: log where opponents die with no weapon hit and whether the shield was already low.
3. A drop/discard rule for held Mine, Bomb, Cannon and Beam in Eliminator, labelled chosen.
4. A held-Turbo chase: keep Turbo until a craft is 100 to 400 units ahead, then fire it.
5. Port the WeaponAi decision only if the above leaves the finish time far from 85 s.
