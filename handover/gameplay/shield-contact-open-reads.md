---
status: measure
titles: [pulse, pure, hd]
updated: 2026-11-11
summary: "Pair bump and LeachBeam linger bulge a Shield shell; open: unshielded pair-bump charge, LeachBeam linger drain, HD reads"
---

# Shield shell: what the craft-to-craft wiring left open

The shell reaction for a craft-to-craft contact and an absorbed weapon hit is wired and tested
(`tests::shield_hits`); the read is [contact-response.md](../../docs/ghidra/functions/psp-pulse-usa/contact-response.md), "A craft-to-craft contact bulges a running Shield's shell".

## Open

- **Does an unshielded pair bump charge the energy pool?** The contact loop's non-shield branch calls
  `Ship_Damage(|p| * 0.05 * 0.7)` for any positive-friction record, and a pair record has friction `0.02`.
  The 2026-11-03 "arms nothing" diff dumped the craft struct and body, not the entity at `+0x88`. Ours charges
  nothing for a pair bump. Settle with a pool read across a staged unshielded rear-end hit; if it charges,
  it is a simulation change (`oag_physics`, hashes move, `race_ground_truth` quoted before and after).
- **The LeachBeam's linger drain on HD, 2048 and Omega, and their Quake's shielded skip.** Pulse is read and wired
  (the shell bulges on the break tick and linger of a link on a Shield-holding target; a Quake never does,
  [cannon-quake-leachbeam.md](../../docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md#2026-11-11-a-shielded-targets-shell-under-a-quake-and-a-leachbeam-shield-weapon-reactions)).
  HD's per-trip beam reports nothing here (`Beam::shell_swallows`); read HD's `UpdatePool` tail for the same
  fall-through and whether its Quake handler skips a shielded craft.
- **The linger drain itself (simulation change).** The same fall-through keeps `LeachBeam_Drain` running on an
  unshielded target and keeps crediting the owner's `+0x128` repair through the 0.5 s linger, gated by
  `LeachBeam_PulseStrength` (`age - last_pulse` in `[0, 1]`, re-armed at the ribbon cursor wrap); ours stops at
  the break. Moves hull energy: `race_ground_truth` quoted before and after, and the pulse cadence wants a live
  `instance + 0x138` log first.
- **Pure's pair friction value** (collider friction for a racing craft) is unread; Pulse's `0.02` is inherited.

## Next Steps

1. PPSSPP: stage the 2026-11-03 rear-end hit and read `entity + 0x88` before and after (Pulse).
2. HD: open its `UpdatePool` equivalent and Quake handler for the two open reads above.
