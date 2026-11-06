---
status: re-first
titles: [pulse]
updated: 2026-11-06
summary: "Pulse Game Options: Multiplayer Tags and Dynamic HUD's Concave/Flat entries are unread, so unwired; HD/Omega ghost rows unchecked"
---

# Pulse Game Options: the entries nobody has read

The census is `docs/ui/menus-original.md`, "Game Options". Ghost Ships Visible, Auto Load Ghost and Dynamic HUD's
default and off entries are wired. What is left needs the executable.

## Open

- **Multiplayer tags** (`OPT_MP_TAGS`, list `Multiplayer Tags`, PSP only). `MPTag_HUD.xml` authors `PlrTag0`-`PlrTag7`
  with no position. Unread: the text, which craft carries a tag, the anchor against the hull, the modes. A row that
  stores a value nothing reads is a defect, so no row exists. Start at `Hud_Update` (`0x0881bf50`) and the string
  `PlrTag0` in `BOOT.BIN`'s xrefs. Split-screen and online are the modes ours could draw it in.
- **Dynamic HUD `FE_CONCAVE` and `FE_FLAT`** (PS2 has `FE_CONCAVE` only). They set bit 1 of `+0xd0` on the HUD's two
  panel groups; what that draws is undecoded. May be the same code as the unread HUD sway (`sway: None` in
  `crates/pulse/src/hud.rs`), worth one read for both.
- **HD ghost and tags rows.** HD's `DATA06` list has `Multiplayer Tags` and `GhostOption`; Omega's own list is unread; Pure
  has no front-end XML (executable-built list, unread); 2048 unread.
- **PS2 Multiplayer Tags and Auto Load Ghost rows.** The PS2 list has neither; the PSP list wins, so both pressings offer
  the ghost rows. Nobody has watched a PS2 save for an auto-load.

## Next Steps

1. Open `psp-pulse-usa` in Ghidra, follow `FUN_08809bc8`'s callers for the `Multiplayer Tags` byte, then its readers.
2. Name what the reads show with a page under `docs/ghidra/functions/psp-pulse-usa/` and rows in `names.tsv`.
3. Only then add the MULTIPLAYER TAGS row beside the ghost rows (`menu.toml`, `Settings`, `GhostOptions`-style Title data).
