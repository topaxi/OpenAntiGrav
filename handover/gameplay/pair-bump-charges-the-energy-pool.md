---
status: ready-to-wire
titles: [pulse, pure, hd]
updated: 2026-11-11
summary: "An unshielded pair bump costs each hull 0.035 * |p| of pool (halved weapons off); ours charges nothing: wire it, hashes move"
size: S (70)
difficulty: easy (75)
---

# A craft-to-craft bump charges the energy pool; ours charges nothing

Measured live on Pulse PSP (2026-11-11, lane `pair-bump-damage`); the evidence, the method and the table are
[contact-response.md](../../docs/ghidra/functions/psp-pulse-usa/contact-response.md), "Measured 2026-11-11: an
unshielded craft-to-craft bump charges the energy pool". Confidence 92.

## The law to wire

For each ring record of a pair contact, on **each** of the two hulls that is racing (`CraftState::Racing`) and
holds no Shield (`shield_pickup_timer == 0`): `Ship_Damage(0.035 * |p|)` with `source = 1`, where `|p|` is
`oag_physics::pair::PairContact::impulse` (bit-equal to the resolver's `j`, checked on four contacts). The pool falls
by that amount, halved with weapons off (`NO_WEAPONS_DAMAGE_SCALE`). Both constants already exist in
`oag_physics::damage` (`CONTACT_DAMAGE_SCALE`, `NO_WEAPONS_DAMAGE_SCALE`); the shared `subtract` path carries the
state gate (1 and 3), clamp, 20 % edge and the destroyed transition.

## Open

- **Where to call it.** `Race::resolve_craft_pairs` (`crates/raceplay/src/field.rs`) already has both hulls, the
  `PairContact` and the racing/shielded split used for `pair_spoil` and `shield_hit`. `damage::apply_contact` takes a
  `WallResponse`, so either build the pair path beside it (`contact_damage(impulse, rules)` then `subtract`, no
  regeneration call: the damage-off regeneration already ran for the tick) or give `subtract` a public pair entry.
  `apply_weapon` is wrong for this: it is the weapon branch (`source == 2`) and would also pass its sparks.
- **Determinism and the hash.** `ShipState::shield` is hashed. Every race with a craft-to-craft bump moves its
  hash, so `race_ground_truth` (and any race-level reference) moves: quote it before and after, and regenerate in
  a separate commit naming this behaviour change (never edit a golden to pass). The wire lands in
  `oag_raceplay`, and `oag-physics`' own determinism test does not call `pair::resolve`, so its reference should
  not move; if it does, that is a bug to find, not a reference to regenerate.
- **The extra records.** The original appends two or more ring records per hull on most contact ticks (the second
  is a variable share of the first, up to over 200 %); one charge per pair per tick undercounts the energy by
  about 6 % over the logged ticks (all records over first records, 1.061). Their origin is unread (a second manifold
  contact or a second resolver pass; the recipe is on the doc page). The first record alone is the wire.
- **AI hulls pay too.** A rival can be wrecked by a bump. That follows from the shared path and needs no flag;
  check the AI lane's expectations (`an-ace-craft-should-lap-every-circuit-without-touching-a-wall.md`) before
  the references are regenerated.
- **Unshielded only.** A shielded hull takes the shell bulge instead (already wired), not the charge.
- **Pure and HD inherit.** Pure's twin loop calls its `Ship_Damage` twin with the same literals (static read);
  the pair friction value is only tested for `> 0`. HD's pair path is unread; inherit until measured.

## Next Steps

1. Wire in `resolve_craft_pairs` as above, with a test in `tests::shield_hits`' neighbourhood: an unshielded pair
   bump lowers both pools by `0.035 * impulse` (halved with weapons off), a shielded one lowers neither.
2. `just gate-affected` plus `test-data-affected`; quote `race_ground_truth` before and after; regenerate the
   moved references in a separate commit.
3. Update `docs/gameplay/` damage pages and `docs/overview/status.md`'s craft-collision cell through the lead's
   regeneration, not here.
