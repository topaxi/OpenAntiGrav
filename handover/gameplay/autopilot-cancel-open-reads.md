# The Autopilot's cancel: what is still unread

**Wired (2026-10-26, `autopilot-wire`).** Absorb cancels a running Autopilot, fire does
nothing, a pad grants nothing mid-stint, per title through
`oag_title::weapons::Weapons::autopilot_absorb_cancels` and `autopilot_pad_grants`.
What is left is reading, not wiring. Evidence and the wired law:
[autopilot.md, "Absorb cancels it, fire does not, and no corner holds it"](../../docs/ghidra/functions/psp-pulse-usa/autopilot.md#absorb-cancels-it-fire-does-not-and-no-corner-holds-it-2026-10-26-autopilot-re).

## Open

- **2048's and Omega's own Autopilot update and cancel are unread.** Both run HD's law,
  `Origin::InheritedFrom`. The 2048 `disengaging` cue is passed at `0x8120ed52..0x8120ed5e`
  (v1.04) inside an undelimited function.
- **Pure's absence of a cancel is static only** (70). A PPSSPP run of
  `scripts/psp-autopilot-cancel.py`, adapted to Pure's offsets (running byte `+0x154`,
  timer `+0x15c`), would settle it. Pure's and HD's pad grants and their fire path under a
  running stint are unread and run Pulse's, inherited.
- **The `disengaging` call on a cancel is inferred** (85, static). A breakpoint on the cue
  string's caller during a live absorb with more than a second left would settle whether
  the original speaks; ours raises `Cue::Disengaging` there.
- **The original does not ask a weapon pad at all while the slot is full**
  (`Ship_TestWeaponPads` asks only with an empty slot); `Race::test_weapon_pads` still
  stamps a full craft's pad, so the cooldown differs. Ours kept as it was.
- **`docs/gameplay/ai.md`'s per-class opponent Autopilot table was measured with pad grants
  mid-stint**, which are now gone. Not re-run.

## Next Steps

1. Read 2048's and Omega's update and cancel when a 2048 or Omega lane passes by.
2. Run the Pure cancel script on PPSSPP when a capture lane has the Pure disc up.
