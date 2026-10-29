---
status: ready-to-wire
titles: [pure, 2048, pulse]
updated: 2026-10-29
summary: "Campaign cursor memory persists on Pulse, HD, Omega; Pure, 2048 and the original's reboot and race rules are open"
---

# The campaign cursor memory: what it does not reach

Landed 2026-10-29 (`docs/ui/campaign-screens.md`, "Where the campaign reopens"):
`[[campaign_cursor]]` in `records.toml` keeps the grid and the cell slot per
campaign for every title on `CampaignStage` (Pulse, HD/Fury, Omega).

## Open

- **Pure** reopens `pure_chain` on the raced event in a session but keeps
  nothing across a relaunch; same row, keyed by the chain's event or class.
- **2048**'s campaign is the touch event map with no `CampaignStage`; its last
  event is not remembered.
- **Other frame-thread saves**: race results, loyalty and `escape.rs` still call
  `records::save` inline; only the campaign cursor and difficulty use
  `records::save_in_background`.
- **Original, unread**: whether Pulse's cursor survives a reboot (boot 1 ended on
  the default), a race (the 2026-10-08 QUIT RACE reading saw the default cell)
  and whether `Grid Selection`'s tile index survives leaving the campaign
  (`GridSelection_PopulateGrids` `0x088de380` resets `+0x114`; `+0xe0` untraced).

## Next Steps

1. Pure: file the chosen event under `campaign_cursor` (15 minutes to wire, 30 with a drive test).
2. Route the remaining `records::save` calls through `save_in_background` (45 minutes; the lock already serialises them).
3. Trace `+0xe0` in `GridSelection_OnEnter`'s callers to settle the Grid Selection index (an hour in Ghidra).
