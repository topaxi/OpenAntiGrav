---
status: re-first
titles: [pulse]
updated: 2026-11-20
summary: "Pulse Game Options: Multiplayer Tags read and wired (gate, axis, local craft); HD tags unwired (Dynamic HUD warp and Music Order landed)"
---

# Pulse Game Options: the entries nobody has read

The census is `docs/ui/menus-original.md`, "Game Options". Ghost Ships Visible, Auto Load Ghost and Dynamic HUD's
default and off entries are wired. What is left needs the executable.

## Open

- ~~**Multiplayer tags**~~ Read and wired (`docs/ghidra/functions/psp-pulse-usa/hud-game-options.md`, "Multiplayer Tags"):
  the gate `FUN_0883e37c` is the track-section row, the raise is world Y, the local craft has no tag. Still open: whether a
  multiplayer race builds AI (kind 2) craft, which the original would tag and ours does not; and the Concave warp on the tags.
- ~~**Dynamic HUD `FE_CONCAVE` and `FE_FLAT`**~~ Landed: all four entries on Pulse, `concave` on every title
  (`docs/ghidra/functions/psp-pulse-usa/hud-game-options.md`, "The warp"). Still open: the PS2 screen globals' run-time
  size (`0x00284fe0`), the PSP EU names (hash-transfer from USA), and whether `MPTag_HUD.xml`'s `PlrTag` widgets bend in the
  original (their screen is not the `HUD` group; unread).
- ~~**Music playlist** (`Music Order`: Sequential, Random)~~ Landed as MUSIC ORDER on the AUDIO page, every title (`docs/architecture/music-threads.md`, "Music order").
- **HD ghost and tags rows** (tags: the option is read by HD's `Hud_Update`; checked, applies, not wired: `HudLayouts::player_tags` is `None`, `mptag_hud.xml` unread, and HD's chunk-cell visibility is not Pulse's section row). HD's `DATA06` list has `Multiplayer Tags` and `GhostOption`; Omega's own list is unread; Pure
  has no front-end XML (executable-built list, unread); 2048 unread.
- **PS2** has neither Multiplayer Tags nor Auto Load Ghost; the PSP list wins, so both pressings offer the ghost rows.

## Next Steps

1. HD: read the tag update behind `Hud_Update`'s `Multiplayer Tags` read (`PlrTag%d`, `mptag_hud.xml`, its gate), then wire `HudLayouts::player_tags`.
