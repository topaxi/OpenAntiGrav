---
status: measure
titles: [pulse, pure]
updated: 2026-11-11
summary: "Pair bump now bulges a running Shield's shell; open: does an unshielded pair bump charge the pool, and a Quake or LeachBeam on a shielded craft"
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
- **A Quake wave or LeachBeam link on a shielded craft raises no shell reaction.** Ours skips a running
  Shield before `Ship_Damage` (`quake.rs:283`, `leach_beam.rs:326`), so nothing reports `absorbed`; the
  original's pending-damage drain calls `ShipShield_Hit` for any posted amount. The Quake's own damage path
  (does it post to `craft + 0x120`?) is unread. `advance_quake`'s direct `view.shield[slot].hit()` skips the
  `note_shield_hit` journal; make it go through `Race::shield_hit` when this is wired.
- **Pure's pair friction value** (collider friction for a racing craft) is unread; Pulse's `0.02` is inherited.

## Next Steps

1. PPSSPP: stage the 2026-11-03 rear-end hit and read `entity + 0x88` before and after (Pulse).
2. Read the Quake handler's damage post in Ghidra, then route a shielded Quake hit through `absorbed`.
