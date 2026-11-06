---
status: re-first
titles: [pulse]
updated: 2026-11-06
summary: "Pulse Game Options: Multiplayer Tags law read, draw not built; Concave/Flat warp maths unread; Music playlist row missing; HD rows unchecked"
---

# Pulse Game Options: the entries nobody has read

The census is `docs/ui/menus-original.md`, "Game Options". Ghost Ships Visible, Auto Load Ghost and Dynamic HUD's
default and off entries are wired. What is left needs the executable.

## Open

- **Multiplayer tags** (`OPT_MP_TAGS`, PSP only). The law is read (`docs/ghidra/functions/psp-pulse-usa/hud-game-options.md`,
  confidence 80): name 2 units above each craft in view, alpha `min(fade-in at 1.2/s, 1 up to 20 units, 0 at 60)`, the option
  forces alpha 0. Not built: needs text anchored on a projected world point in `oag-hud` and a name for every craft in
  a split-screen race. A row that does nothing is a defect, so none exists. Unread: what `FUN_0883e37c` accepts, and whether
  the local craft gets a tag.
- **Dynamic HUD `FE_CONCAVE` and `FE_FLAT`** (PS2 has `FE_CONCAVE` only). Bit 1 of `+0xd0` on the panel groups is a vertex warp
  mode (readers `Text_Draw`, `Image_Draw`, `Hud_UpdateSpeedBar`); read `FUN_08919c2c` and the `FUN_088cc9f4` flag, then
  offer the entries. The unread HUD sway (`sway: None` in `crates/pulse/src/hud.rs`) is separate code.
- **Music playlist** (`Music Order`: Sequential, Random). Not added in ours (no shuffle in `oag_sound::song_control`).
- **HD ghost and tags rows.** HD's `DATA06` list has `Multiplayer Tags` and `GhostOption`; Omega's own list is unread; Pure
  has no front-end XML (executable-built list, unread); 2048 unread.
- **PS2** has neither Multiplayer Tags nor Auto Load Ghost; the PSP list wins, so both pressings offer the ghost rows.

## Next Steps

1. Read `FUN_08919c2c` and `FUN_088cc9f4`'s flag argument; offer CONCAVE and FLAT on `DynamicHud`.
2. Build the projected-text tag draw in `oag-hud` and give split-screen seats names; then add the MULTIPLAYER TAGS row beside
   the ghost rows (`menu.toml`, a `GhostOptions`-style Title field, `Settings`, the projected draw gated by the byte).
