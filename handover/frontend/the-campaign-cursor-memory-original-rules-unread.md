---
status: re-first
titles: [pulse, pure]
updated: 2026-10-29
summary: "Campaign cursor memory persists on Pulse, HD, Omega, Pure and 2048; the original's reboot and race rules are unread"
---

# The campaign cursor memory: what is left

Landed 2026-10-29 (`docs/ui/campaign-screens.md`, "Where the campaign reopens"):
`[[campaign_cursor]]` in `records.toml` keeps the grid and the cell slot per
campaign for every title on `CampaignStage` (Pulse, HD/Fury, Omega); Pure's chain
and 2048's event map file the event started in the same row. Every session save
(race results, loyalty, `escape.rs`, the cursor) is `records::save_in_background`.

## Open

- **Original, unread**: whether Pulse's cursor survives a reboot (boot 1 ended on
  the default), a race (the 2026-10-08 QUIT RACE reading saw the default cell)
  and whether `Grid Selection`'s tile index survives leaving the campaign
  (`GridSelection_PopulateGrids` `0x088de380` resets `+0x114`; `+0xe0` untraced).
  Pure's and 2048's reboot behaviour is unread too; theirs is ours, not measured.

## Next Steps

1. Trace `+0xe0` in `GridSelection_OnEnter`'s callers to settle the Grid Selection index (an hour in Ghidra).
