---
status: re-first
titles: [2048, omega, pure]
updated: 2026-10-26
summary: "2048's and Omega's Autopilot cancel unread (inherit HD); Pure's absence of a cancel is static only"
---

# The Autopilot's cancel and expiry: what is still unread

**Wired (2026-10-26, `autopilot-wire`).** Absorb cancels a running Autopilot, fire does nothing,
a pad grants nothing mid-stint, per title through
`oag_title::weapons::Weapons::autopilot_absorb_cancels` and `autopilot_pad_grants`. Evidence and the
wired law:
[autopilot.md, "Absorb cancels it, fire does not, and no corner holds it"](../../docs/ghidra/functions/psp-pulse-usa/autopilot.md#absorb-cancels-it-fire-does-not-and-no-corner-holds-it-2026-10-26-autopilot-re).

**Expiry measured (2026-10-26, `autopilot-expiry`).** No title holds or eases the Autopilot when its
timer runs out in a corner, so nothing was wired: Pulse and HD watched live, Pure live for the timer
(76 expiries, two boots), 2048 static. The pasted "extended in a turn zone" claim is refuted
([autopilot.md](../../docs/ghidra/functions/psp-pulse-usa/autopilot.md#expiry-mid-corner-and-the-pasted-extended-in-a-turn-zone-claim-2026-10-26-autopilot-expiry)).
The `disengaging` call on a cancel with time left is watched live on Pulse (it fires), which closes
the old "inferred" read.

## Open

- **2048's cancel and Omega's own update and cancel are unread.** Both run HD's law,
  `Origin::InheritedFrom`. 2048's update is read now
  ([autopilot.md](../../docs/ghidra/functions/vita-2048-eu-v104/autopilot.md), static, 80) and has no
  hold; a Vita3K run would raise it, and needs a race state that is not in `data/saves/`.
- **Pure's absence of a cancel is static only** (70). The 2026-10-26 PPSSPP probe (CIRCLE, TRIANGLE,
  SQUARE against a running stint) changed nothing, but Pure's race weapons switch was not found, so
  the buttons may never have reached a pickup path: no positive control. Finding that byte (Pulse's is
  `0x08b30fa8`, the race settings' `+0x18`) is what makes
  `scripts/psp-pure-autopilot-expiry.py --policy buttons` decisive. Pure's and HD's pad grants and
  their fire path under a running stint are unread and run Pulse's, inherited.
- **HD's `disengaging` position against the hand-back is read, not watched** (80): an RPCS3
  breakpoint needs the interpreter. Pulse's is watched (the call is taken the frame the timer crosses
  `1.0`, 0.984 s of timer before the release).
- **The original does not ask a weapon pad at all while the slot is full**
  (`Ship_TestWeaponPads` asks only with an empty slot); `Race::test_weapon_pads` still
  stamps a full craft's pad, so the cooldown differs. Ours kept as it was.
- **`docs/gameplay/ai.md`'s per-class opponent Autopilot table was measured with pad grants
  mid-stint**, which are now gone. Not re-run.

## Next Steps

1. Read 2048's cancel and Omega's update and cancel when a 2048 or Omega lane passes by.
2. Find Pure's race weapons switch, then run
   `scripts/psp-pure-autopilot-expiry.py --policy buttons` with it set.
