---
status: re-first
titles: [hd, 2048, omega]
updated: 2026-11-07
summary: "Wired: HD, Fury, 2048 and Omega never stamp a pad in Eliminator. Left: HD's stamp function, 2048 campaign Eliminator, Omega live"
---

# Eliminator weapon pads: what is wired and what is only inferred

**Wired, 2026-11-07 (`pad-nodisarm`).** `Weapons::pads_stamp_in_eliminator` (`false` for HD, Fury,
2048, Omega) is resolved once in `crates/raceplay/src/load/global.rs` into the race's
`weapon_pad_refresh`; Pulse keeps `elimination_refresh_time`. Evidence and per-title confidences:
[`docs/gameplay/pickups.md`](../../docs/gameplay/pickups.md#eliminator-and-the-pads-per-title).

## Open

- HD's own stamp function is not located (the `.opd`/TOC resolution hides its readers of the
  `refresh_time` table). Confidence 75 for HD rests on an RPCS3 run, not on a read.
- Omega live is not checkable here (shadPS4 has no memory readout; the native `wipeout_pc` port's
  drivers are untracked): `scripts/rpcs3-hd-weapon-pad-poll.py`'s loop against that process's
  `/proc/<pid>/mem`, reading the pad object's `+0x140`.
- 2048's claim for campaign Eliminator events is inferred (their mode word is a CRC; they reach the
  no-stamp path through the mode object's `+400` in `{1, 4}`); a Vita3K run would settle it.
- HD's "grant straight away" is inferred from 2048 and Omega, not seen.
- One frame of draw order on Pulse: ours greys the stamp tick, the original greys the tick after.

## Next Steps

1. Locate HD's stamp site (search the Eliminator-family mode test beside the `refresh_time` read).
2. Vita3K run of a 2048 campaign Eliminator event, command on the 2048 `weapon-pads.md`.
