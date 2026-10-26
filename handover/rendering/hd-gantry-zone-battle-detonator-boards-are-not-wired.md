---
status: ready-to-wire
titles: [hd]
updated: 2026-10-12
summary: "Zone Battle and Detonator need Modes before their start boards draw; Zone release 210/190 tick question unmeasured"
categories: [rendering, gameplay]
---

# HD's Zone Battle and Detonator start boards are measured and not drawn

2026-10-08, `hd-gantry`: slot 8 is a 512 x 256 card. **2026-10-12, `hd-zone-board`: a Zone race now shows
its own board** (`321Go_Zone.vex`, `DATA00`'s copy, the loop icon assembling over 210 ticks): the blank
panel was the Zone grade replacing the quad's albedo, not the model. See `docs/rendering/start-gantry.md`,
"Wipeout HD: the Zone board". Zone Battle and Detonator are what is left.

## Open

- **`oag_race::Mode` has Zone only.** Zone Battle (`GetMode()` 13, and 21 on the PS4) and Detonator (14)
  need modes before their boards can be chosen; the rule is `oag_title::adverts::ZoneGantry`, which is a
  `Mode::Zone` rule today and would become a table by mode id. Files: `321Go_HD_Zone_Battle.vex`,
  `321Go_HD_Detonator.vex`; both are loaded as a card the same way, and each probably needs its own check
  of which `DATA0N` copy is live (the game's own load order is last-wins, `data00` last).
- **The Zone board's post-release behaviour is chosen, not measured**: the timeline runs on and is held at
  `CLOCK_LIMIT`, so the pieces fade to the arrow by about 6 s in. Nothing was captured after the release.
- **The 45-tick mismatch**: the asset's keys run about 250 frames, the thread's reading of the capture is
  "starts about 3.5 s (210 ticks) before the release, whole about 20 ticks before it". Needs an RPCS3
  frame sequence at the release, which this session could not get (RADV cannot present on Xvfb, the NVIDIA
  ICD freezes the guest on the grid).
- **How the original keeps the board in a Zone race**: the Zone variant of the quad's material
  (`sign_emissive_glow`, block #7) samples no albedo. Ours switches the grade off for that quad (chosen,
  not measured, `ZONE_EXEMPT_RATE`). If the original selects the variant per chunk, the same per-material
  rule would also stop the blanket grade reaching the sky, pads and signs, which `zone.wesl` calls an
  approximation.
- HD's own code site (which function fills the name) is unread; the only references to the four names
  are the hash registration loop at `0x003f1300`.

## Next Steps

1. Capture the Zone release on a host where RPCS3 runs (`rpcs3-drive.py countdown --nav "Single
   Player=right,right,right,right,right,cross"`) and compare: the loop's brightness, the 210/190 tick
   question, what the board does after `GO`.
2. When Zone Battle and Detonator gain `Mode`s, add them to the chooser and repeat the copy check.
