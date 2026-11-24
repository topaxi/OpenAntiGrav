---
status: re-first
titles: [pulse, pure]
updated: 2026-11-24
summary: "Pulse's campaign cursor rules are measured (race, leave, reboot, Grid Selection); only Head2Head QUIT, other modes and a few cursor cases are unread"
---

# The campaign cursor memory: what is left

Settled 2026-11-24 (`docs/ui/campaign-screens.md`, "The original's rules, read off the running game"): on Pulse PSP the
cursor survives a race and back (QUIT RACE and a finished race), survives leaving to `Main Menu`, and `Grid Selection`
reopens on the grid and page left; a **reboot resets both** to `Grid 1` / the default cell. Ours matches all but the
reboot, where `[[campaign_cursor]]` persisting is the maintainer's ask and stays ours.

## Open

- **Head2Head QUIT RACE**: the 2026-10-08 reading that landed on the default cell was not reproduced (that grid needs
  far more progress); Speed Lap, Zone and Tournament cells were not walked. Time Trial and Single Race keep the cell.
- **Cursor cases never measured on the original**: a cell whose lock glyph a medal cleared (the screen's predicate is the
  glyph, not the `Locked` byte), a slot the new grid lacks, and HD/Fury's cursor (chosen from Pulse's law). The
  `PI_Grid` `Locked` byte (`+0xa0`) read 1 after a reboot while `grid1` still opened: the grid's real unlock test is
  untraced.
- Pure's and 2048's reboot behaviour is unread; theirs is ours, not measured.

## Next Steps

1. A save-state-free Head2Head walk once a profile has grid 4 open (`psp-postrace.py`'s Autopilot arming plus `g_race_laps` = 1 gets a race finished in about 2 minutes).
