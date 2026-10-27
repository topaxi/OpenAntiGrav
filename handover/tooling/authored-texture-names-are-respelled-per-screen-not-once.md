---
status: ready-to-wire
titles: [pulse, hd, 2048, omega]
updated: 2026-10-27
summary: "Front-end texture src respelling (.mip/.gtf/.gxt) and hand lists of extra textures per screen; one resolver over what the XML names instead"
categories: [frontend]
size: M (30)
difficulty: medium (35)
kind: debt
---

# Authored texture names are respelled per screen, not once

2026-10-27, from the lead's asset-link sweep (a read-only scout listed every asset-name literal in non-test
Rust and checked it against what the disc's data authors). This is the one cluster where the disc's own
data names the link and our code repeats it by hand.

The front-end XML names its textures in an older spelling than the archive holds, and each place that
loads such a texture fixes the spelling its own way:

- `oag_game::boot::sprites` (`vita_texture_name`): 2048's XML says `.gtf`, the package holds `.gxt`;
  tried after the authored name fails. Generic.
- `oag_hd::campaign::HEX_TEXTURES`: five `(.mip, .gtf)` pairs, because HD's `CellMode_Definition.xml`
  spells hex textures `.mip` (PSP era) and no archive carries a `.mip` by that stem
  (`docs/ui/campaign-screens.md`, "The archive's `.mip` naming is dead"). The sprite sheet is keyed by the
  widget's exact `src`, so the blob is read as `.gtf` and shelved under the `.mip` name.
- `oag_game::campaign::HEX_TEXTURES`: Pulse's two hex textures, a hand list because they are not in
  `Skin.xml`'s sheet.
- `oag_hd::campaign::OTHER_TEXTURES`, `oag_hd::endrace::EXTRA_TEXTURES`: more hand lists of textures a
  screen's widgets name.
- `oag_hd::hud`: the same respell in the other direction for the HUD.

Every texture in those lists is already named by a widget's `image.src` in the screen's XML. The lists
restate what the XML authors, and a new screen, DLC or mod needs a code edit to show a texture its XML
already names.

## Open

- Whether one resolver (authored name first, then the title's known extension respells: `.mip` -> `.gtf`,
  `.gtf` -> `.gxt`, and HD's HUD direction) covers every case, or some screen deliberately leaves an image
  unresolved (`oag_ui_screens::endrace::hd` says it leaves the ship-badge column and medal art unresolved
  on purpose: keep that decision, it is about drawing, not loading).
- Whether walking a screen's widgets for `image.src` at load is cheap enough at boot (it is a few dozen
  names per screen).

## Next Steps

1. One `resolve_texture(archives, authored_name, title)` beside `vita_texture_name`, with the respell rules
   as `Title` data (ADR-0058), and its unit tests.
2. The campaign and end-screen loaders collect `image.src` from the screen's own widgets and load those
   through it; delete `HEX_TEXTURES` (both), `OTHER_TEXTURES`, `EXTRA_TEXTURES` where the XML covers them.
3. Prove no picture changes: the HD campaign, cell, grid and end-race compare pairs and the Pulse campaign
   pairs before/after, byte-identical or within noise.
