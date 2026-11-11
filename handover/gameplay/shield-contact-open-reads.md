---
status: measure
titles: [pulse, pure, hd]
updated: 2026-11-11
summary: "Pair bump and LeachBeam linger bulge a Shield shell; open: LeachBeam linger drain, HD reads, Pure pair friction"
---

# Shield shell: what the craft-to-craft wiring left open

The shell reaction for a craft-to-craft contact and an absorbed weapon hit is wired and tested
(`tests::shield_hits`); the read is [contact-response.md](../../docs/ghidra/functions/psp-pulse-usa/contact-response.md), "A craft-to-craft contact bulges a running Shield's shell".

## Open

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

1. HD: open its `UpdatePool` equivalent and Quake handler for the two open reads above.
2. The unshielded pair-bump charge is measured and has its own ready-to-wire thread
   (`pair-bump-charges-the-energy-pool.md`).
