---
status: parked
titles: [all]
updated: 2026-11-02
summary: "History comments (dates, maintainer, lane names) remain in about 36 crates; cleanup parked by the maintainer after oag-raceplay, oag-title and oag-game"
size: L (40)
difficulty: easy (60)
kind: debt
---

# History comments left after oag-raceplay, oag-title and oag-game

CLAUDE.md's rule "Code comments say what and why, never when or who" landed on 2026-11-02 with a
ratchet, `just check-comment-history` (`scripts/check-comment-history.py`), whose per-crate
`BASELINE` may only shrink. `oag-raceplay` went from 188 to 0 the same day; `oag-title` (84) and
`oag-game` (76) are the last crates cleaned before the maintainer parked the rest (2026-11-02).

## Open

- The remaining rows of `BASELINE` in `scripts/check-comment-history.py`, largest first after
  `oag-game`: `oag-fx` 62, `oag-hd` 57, then the rest. New comments cannot add to them.
- The general compact pass at the oag-compact calibration was barely applied on `oag-raceplay`
  (40,422 to 40,333 comment lines), and about 25 undated "used to" / "until" phrases there are not
  counted by the ratchet.

## Next Steps

1. When the maintainer reopens it: one oag-compact member per crate, largest row first; verify
   every dropped dated or measured fact is on the cited doc page before cutting (add it there if
   not); the lead checks code is byte-identical with `strip_rs_comments.py` before merging.
