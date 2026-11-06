---
status: re-first
titles: [pulse]
updated: 2026-11-06
summary: "Pulse Game Options: Concave/Flat warp maths unread; Music playlist row missing; Multiplayer Tags wired, its axis and gate unread; HD unwired"
---

# Pulse Game Options: the entries nobody has read

The census is `docs/ui/menus-original.md`, "Game Options". Ghost Ships Visible, Auto Load Ghost and Dynamic HUD's
default and off entries are wired. What is left needs the executable.

## Open

- **Multiplayer tags** wired (`oag_hud::player_tags`, MULTIPLAYER TAGS row). Still unread: what `FUN_0883e37c` accepts, whether
  the local craft gets a tag, and which axis the 2-unit raise runs along (ours: the craft's up). A lobby name beats `Player N`.
- **Dynamic HUD `FE_CONCAVE` and `FE_FLAT`** (PS2 has `FE_CONCAVE` only). Bit 1 of `+0xd0` on the panel groups is a vertex warp
  mode (readers `Text_Draw`, `Image_Draw`, `Hud_UpdateSpeedBar`); read `FUN_08919c2c` and the `FUN_088cc9f4` flag, then
  offer the entries. The unread HUD sway (`sway: None` in `crates/pulse/src/hud.rs`) is separate code.
- **Music playlist** (`Music Order`: Sequential, Random). Not added in ours (no shuffle in `oag_sound::song_control`).
- **HD ghost and tags rows** (tags: checked, applies, not wired: `HudLayouts::player_tags` is `None`). HD's `DATA06` list has `Multiplayer Tags` and `GhostOption`; Omega's own list is unread; Pure
  has no front-end XML (executable-built list, unread); 2048 unread.
- **PS2** has neither Multiplayer Tags nor Auto Load Ghost; the PSP list wins, so both pressings offer the ghost rows.

## Next Steps

1. Read `FUN_08919c2c` and `FUN_088cc9f4`'s flag argument; offer CONCAVE and FLAT on `DynamicHud`.
2. Read `FUN_0883e37c` and the raise axis; then wire HD's `mptag_hud.xml` if its row should act.
