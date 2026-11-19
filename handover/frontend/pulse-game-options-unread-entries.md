---
status: re-first
titles: [pulse]
updated: 2026-11-19
summary: "Pulse Game Options: Multiplayer Tags wired, its axis and gate unread; HD unwired (Dynamic HUD warp and Music Order landed)"
---

# Pulse Game Options: the entries nobody has read

The census is `docs/ui/menus-original.md`, "Game Options". Ghost Ships Visible, Auto Load Ghost and Dynamic HUD's
default and off entries are wired. What is left needs the executable.

## Open

- **Multiplayer tags** wired (`oag_hud::player_tags`, MULTIPLAYER TAGS row). Still unread: what `FUN_0883e37c` accepts, whether
  the local craft gets a tag, and which axis the 2-unit raise runs along (ours: the craft's up). A lobby name beats `Player N`.
- ~~**Dynamic HUD `FE_CONCAVE` and `FE_FLAT`**~~ Landed: all four entries on Pulse, `concave` on every title
  (`docs/ghidra/functions/psp-pulse-usa/hud-game-options.md`, "The warp"). Still open: the PS2 screen globals' run-time
  size (`0x00284fe0`), the PSP EU names (hash-transfer from USA), and whether `MPTag_HUD.xml`'s `PlrTag` widgets bend in the
  original (their screen is not the `HUD` group; unread).
- ~~**Music playlist** (`Music Order`: Sequential, Random)~~ Landed as MUSIC ORDER on the AUDIO page, every title (`docs/architecture/music-threads.md`, "Music order").
- **HD ghost and tags rows** (tags: checked, applies, not wired: `HudLayouts::player_tags` is `None`). HD's `DATA06` list has `Multiplayer Tags` and `GhostOption`; Omega's own list is unread; Pure
  has no front-end XML (executable-built list, unread); 2048 unread.
- **PS2** has neither Multiplayer Tags nor Auto Load Ghost; the PSP list wins, so both pressings offer the ghost rows.

## Next Steps

1. Read `FUN_0883e37c` and the raise axis; then wire HD's `mptag_hud.xml` if its row should act.
