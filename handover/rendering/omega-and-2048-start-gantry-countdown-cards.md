---
status: re-first
titles: [omega, 2048]
updated: 2026-11-10
summary: "Omega and 2048 play 321Go_2048.vex as a card via HD's pass; the HD board waits on material curves, the phase on a renderer"
categories: [rendering]
---

# Omega's and 2048's start gantry countdown cards

2026-11-10, `omega-gantry`. Landed: the `.gxt`/`.gnf` spellings, HD's card pass as both titles' `Title::adverts`
(inherited), and `321Go_2048.vex` playing on every native circuit that authors a `billboard8` surface, with
`Clock::assembled_at_release` over the authored `GO` frame (73 ticks, chosen). Evidence and numbers:
[start-gantry.md](../../docs/rendering/start-gantry.md), "Omega and 2048".

## Open

- **The HD-lineage board** (`321Go_StartFinish.vex`, on the 16 HD-ported 2048 circuits and Omega's HD circuits) is withheld
  with a loader line: its digits are material curves in a container `oag_rcs::rcsmodel::psp2` does not parse
  (`omega-low-hanging-from-hd.md` in `handover/tooling/`, item 4). Once read, add it to `Adverts::gantry_boards` (or drop the
  field), then the Zone, Zone Battle and detonator boards through `zone_gantry`.
- **The digits' phase is chosen.** shadPS4 draws this card's face blank white through the whole countdown, so only the board's
  place is measured. A frame that draws the card is needed: Vita3K for 2048 (`scripts/vita3k-drive.py`), nothing known for Omega.
- **FINAL LAP and the chequered flag** are never played: the clock has no `laps`, so it holds the `A·G·R·C` logo state at
  `CLOCK_LIMIT`. HD's `HD_LAP_WINDOWS` land on the 2048 file's own 9.333 and 12.333 s beats (start-gantry.md, three-beat section).
- **2048's `park`** ships no `billboard8.gxt` in the base package; the v1.04 patch is not mounted (`oag_2048::open`), Omega's
  copy ships one.
- The wipe bars between digits (a white rectangle over `2` and `1` at the changeover) rest on the positional node-to-submesh
  mapping (confidence 75, start-gantry.md); unverified against a drawn original.

## Next Steps

1. Read the PS4/Vita material curves (tooling thread, item 4).
2. A Vita3K countdown capture of 2048's Altima to pin the clock and the wipe bars.
