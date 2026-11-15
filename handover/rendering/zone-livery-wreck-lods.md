---
status: ready-to-wire
titles: [hd, 2048, omega]
updated: 2026-11-16
summary: "The Zone hull wears the team livery on HD/2048/Omega; the wreck and LODs still read the default zoneship folder"
---

# Zone livery: the wreck and LODs read unswapped

The Zone hull's team livery is wired on HD/Fury (2026-11-16), 2048 and Omega:
`docs/ghidra/functions/ps3-hdfury-eu/zone-craft.md`.

## Open

- **The wreck and the LODs.** The original pushes its `zoneship_zone` -> livery pair
  into a global substitution table (`0x00f868e0` on HD), so every model loaded after it
  reads through it, including `shipwreck.vex` and `ship_lod*.vex`. Ours swaps only the
  hull build (`oag_livery` lib.rs, the `zone_livery::Reads` around the hull);
  `oag_livery::wreck` reads unswapped, so a Zone wreck shows the default skin.
- **The table's reader on HD** was not found. It would be a second site for the law
  (confidence 80 today). Candidates are listed on the evidence page.
- **2048-era craft on 2048/Omega** author no `zone` livery. The original's
  `Zoneship_Faisar` literal applies only when there is no entry, and what a native
  craft's entry holds in Zone is unread.

## Next Steps

1. Route `oag_livery::wreck`'s texture reads through the same `zone_livery::Swap`
   in Zone mode. Test: a Zone wreck build reports the swap (pattern:
   `crates/raceplay/tests/zone_craft_ground_truth.rs`). About 1 hour.
2. On HD, try `0x003df6d8`, `0x003eeb08`, `0x003f0348` and `0x003f3fb0` for a loop over
   `0x00f868e0`'s `+0x70` records.
