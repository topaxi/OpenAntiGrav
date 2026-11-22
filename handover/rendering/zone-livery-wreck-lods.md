---
status: re-first
titles: [hd, 2048, omega]
updated: 2026-11-22
summary: "The Zone wreck is built through the livery swap (HD, 2048); open: the table reader on HD, Omega's wreck folder, native 2048 craft"
---

# Zone livery: the wreck and LODs read unswapped

The Zone hull's team livery is wired on HD/Fury (2026-11-16), 2048 and Omega:
`docs/ghidra/functions/ps3-hdfury-eu/zone-craft.md`.

## Open

- **The table's reader on HD** was not found. It would be a second site for the law
  (confidence 80 today). Candidates are listed on the evidence page.
- **2048-era craft on 2048/Omega** author no `zone` livery. The original's
  `Zoneship_Faisar` literal applies only when there is no entry, and what a native
  craft's entry holds in Zone is unread.
- **Omega's Zone wreck** asks for `zoneship_zone\...` while the hull's key is
  `zoneship_team`; whether the original reads it through another pair is unread.
- The wreck is built through the swap (`oag_livery::wreck::load`) but a Zone race does not
  draw it until the modes 6 and 14 sequence lands.

## Next Steps

1. On HD, try `0x003df6d8`, `0x003eeb08`, `0x003f0348` and `0x003f3fb0` for a loop over
   `0x00f868e0`'s `+0x70` records.
