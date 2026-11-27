---
status: blocked-maintainer
titles: [pulse]
updated: 2026-11-26
summary: "Detonator backported to Pulse on branch det-pulse-backport (not merged): Zone circuits, Pulse mines, red pads; EMP speed and look need review"
categories: [gameplay]
---

# Detonator on Pulse: a backport kept on its own branch

Pulse ships no Detonator mode. At the maintainer's request (2026-11-25) it was backported on the branch
`det-pulse-backport` (048d70b7a), and the branch is deliberately **not merged into `main`** until the maintainer
has played it. The branch's own commit messages carry the evidence and the frames read.

Try it: check out the branch, then `just play data/images/pulse-psp-usa.chd --race --mode detonator --no-audio`
(or RACEBOX > MODE `DETONATOR`; the PS2 disc works too). L1 fires the EMP, Square the cannon.

## What the branch has

- **Track:** the picked circuit's Zone twin (`zone_track.vex`).
- **Mines and bombs:** `Pulse_Mine.vex` and `Pulse_Bomb.vex`, with Pulse's race-table `Mine`/`Bomb` values.
- **Pads:** every speed pad becomes the circuit's own weapon pad, held at Pulse's red key `(204,102,102)`; it charges the EMP and gives no boost. Pulse's Zone circuits have no weapon pads.
- **Data:** our own layout, values, scoring, HUD layouts and hulls are the official mod `oag.detonator-pulse` (`assets/mods/official/detonator-pulse/fallback/wipeout-pulse/`, ADR-0100).
- **Cannon rounds:** drawn as Pulse's Cannon pickup shots.
- **EMP:** borrows Pulse's Repulsor (`WO_REPULSER` on the front, blue flash, blast and ring at launch).
- **Gauge:** an EMP gauge of six pips cut from Pulse's shield bar art, with a separate PS2 layout.

Not yet:
- no stage grade;
- no score on the HUD;
- no medals;
- no announcer lines.

HD, Omega and 2048 are unchanged (their Detonator tests pass on the branch).

## Open

- **The EMP's speed:** ours is `speed="3"` in `WeaponStats_Detonator.xml`, three times HD's `1`, so the front is
  far ahead within 10 ticks. One line to change if the wave should linger near the craft.
- **The EMP's launch frame** washes most of the screen white for an instant.
- **Merging into `main`:** only after the maintainer has played it.

## Next Steps

1. The maintainer plays the branch and decides the EMP speed and look.
2. Merge `main` into the branch (it falls behind), re-gate, and merge when approved.
3. Then the missing pieces: stage grade, HUD score, medals, announcer lines.
