# The Autopilot's cancel is on absorb: ready to wire

**Ready to wire (2026-10-26, `autopilot-re`).** The RE is done. Evidence, addresses and
confidence:
[autopilot.md, "Absorb cancels it, fire does not, and no corner holds it"](../../docs/ghidra/functions/psp-pulse-usa/autopilot.md#absorb-cancels-it-fire-does-not-and-no-corner-holds-it-2026-10-26-autopilot-re).

## The law (Pulse PSP, measured live on PPSSPP)

1. **Absorb (CIRCLE) cancels a running Autopilot**, with or without a pickup held, in
   every mode. The timer goes to `0.0` on the press tick and the craft is handed back on
   the next. A held pickup is neither paid out nor spent by that press. Confidence 95.
2. **Fire (SQUARE) does nothing** under a running Autopilot. The player's fire byte never
   reaches the fire path while the driver flies, because only the absorb byte is passed
   through into the driver's control record. Confidence 92.
3. **A held Autopilot cannot be fired into a running one.** The press is refused and the
   pickup stays held. Confidence 90.
4. **No pickup is granted from a weapon pad while the Autopilot runs.** Confidence 80,
   static only.
5. **There is no corner hold.** The countdown runs on the frame's `dt` and releases on
   expiry wherever the craft is. 152 of 152 live expiries released, the sharpest at about
   290 degrees of heading a second. Confidence 90. **Do not add a hold.** The only case
   that keeps the craft flown at zero is the finished racer, which ours already has
   (`standing.finished()` in `Race::flown_for_the_player`).

HD/Fury has the same cancel (static, 80). Pure has no cancel at all (static, 70), so
Pure's `Title` must not get it. 2048 and Omega were not checked: they inherit HD's,
per the rule for unmeasured 2048 rules. Inheriting is chosen, not measured.

## Why ours never cancels in play

`Race::spend_pickup` (`crates/raceplay/src/weapons.rs`) returns on an empty slot
before it reaches its cancel, and the cancel is on SQUARE. `Autopilot_Fire` spends the
slot, so under a running Autopilot the slot is empty and neither button does anything.
A held pickup only appears through ours granting from a pad mid-stint (item 4), and
then SQUARE cancels, which is the wrong button. Measured with a scratch test on
`main` at 65973a658: armed through the pickup, then CIRCLE and SQUARE with the slot
empty, `flown_for_the_player` stayed `true` after both.

## Files to change

- `crates/raceplay/src/weapons.rs`, `Race::spend_pickup`:
  - Move the cancel to the absorb press and put it **before** the empty-slot return.
  - Under a running Autopilot an absorb only cancels: no pay-out, no pickup spent, and
    no `eliminator_absorb` (with an empty slot the original's Eliminator branch has
    nothing to request).
  - A fire press under a running Autopilot does nothing at all.
  - Rewrite the fire arm's comment. It cites `FUN_08844ec4` as the fire path; that is
    `Ship_AbsorbHeldPickup`.
  - In the Autopilot arm, refuse a held Autopilot while one runs (keep the pickup). The
    "Re-arming is allowed here" comment is wrong: `Ship_FireHeldWeapon` refuses it at
    `0x08844b44`.
- `crates/raceplay/src/pads.rs`, `Race::test_weapon_pads`: no grant while
  `autopilot_timer > 0.0`. The original still calls `WeaponPads_TestCraft` with an
  empty slot and gates only the grant request (`0x0883f0c0`). Keep ours' refresh stamp
  and skip only the grant.
- `crates/raceplay/src/eliminator.rs`: drop "One edge is not ported" from
  `eliminator_absorb`'s doc. That edge is now item 1.
- Gate the law per title (`oag_title`, ADR-0058): Pulse PSP/PS2 and HD measured, Pure
  absent, 2048 and Omega inherited from HD. Use a `Title` field with provenance, not a
  title-name branch.
- `docs/gameplay/weapon-lineage.md`: move the "Autopilot cancel and release" row's cells
  for ours.

## Tests, red today and green after

In `crates/raceplay/src/tests/autopilot.rs`, beside its fixtures:

- `absorbing_under_autopilot_cancels_it_with_an_empty_slot`: arm through the pickup, then
  CROSS, then CROSS|CIRCLE, and `!flown_for_the_player(0)`. Red: ours stays flown.
- `absorbing_under_autopilot_keeps_a_held_pickup`: a Rocket written into the slot, then
  CIRCLE. The Autopilot is cancelled, the Rocket is still held and energy is unchanged.
  Red: ours pays out the Rocket and keeps flying.
- `firing_under_autopilot_does_nothing`: a Rocket written into the slot, then SQUARE.
  Still flown, Rocket still held, no projectile. Red: ours cancels.
- `a_held_autopilot_cannot_be_fired_into_a_running_one`: an Autopilot written into the
  slot mid-stint, then SQUARE. The timer keeps counting down from where it was and the
  pickup is still held. Red: ours cancels.
- `no_pad_grant_while_the_autopilot_runs`: run over an enveloping pad mid-stint and the
  slot stays empty. Red: ours grants. This may sit better in the pads tests.
- Replace `firing_under_autopilot_cancels_it_and_keeps_the_pickup`. It pins the wrong
  law.
- `crates/raceplay/src/tests/cues.rs` around line 192: the comment says "`spend_pickup`'s
  fire arm cancels". Reword it; the fixture's timing does not depend on it.
- A Pure test: CIRCLE under a running Autopilot does not cancel.

The regression gate to quote before and after is `cargo nextest run -p oag-raceplay
autopilot`.

## Open

- 2048's and Omega's own Autopilot update and cancel are unread. The 2048 `disengaging`
  cue is passed at `0x8120ed52..0x8120ed5e` (v1.04) inside an undelimited function.
- Pure's absence of a cancel is static only. A PPSSPP run of
  `scripts/psp-autopilot-cancel.py`, adapted to Pure's offsets (running byte `+0x154`,
  timer `+0x15c`), would settle it.

## Next Steps

1. Wire items 1 to 4 in `spend_pickup` and `test_weapon_pads` behind a `Title` field, with
   the five tests above red first.
2. Read 2048's and Omega's update and cancel when a 2048 or Omega lane passes by.
