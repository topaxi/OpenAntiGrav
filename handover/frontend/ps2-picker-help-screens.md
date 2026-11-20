---
status: re-first
titles: [pulse]
updated: 2026-11-20
summary: "Track Help and Team Help are drawn; Team Help's per-team paragraph, Pre Race Music Select, Tournament Help and the split screens' Help are not"
difficulty: medium (45)
---

# What is left of the pickers' Help screens

`Track Help` and `Team Help` are drawn on both pressings, opened on `Roles::help` and closed on
cross, triangle, circle or a click (`docs/ui/back-and-help.md`, "The pickers' Help overlays").
Photo mode, the replay viewer, the disc chooser and the key hints follow the resolved Back.

## Open

- **Team Help's second paragraph** (`Team Ship Help`, authored `MSC_TEAMDES_ASS`) draws Assegai's text for
  every team. The team catalogue carries `helpText="MSC_TEAMDES_<team>"` per team and the widget is named in
  `TeamSelection`'s string block (`docs/formats/race-setup.md`), so the original probably swaps it per team;
  the setter was not traced (55). Read it in Ghidra (`TeamSelection` update on the PSP), then set the text from
  the selected `Entry`.
- **`Pre Race Music Select`** (`Music Redirect`, `forward="square"`) is a `MusicList`, not a Help overlay:
  it needs its own widget and the playlist behind it. The footer's `Music playlist` is drawn and does nothing.
- **`Tournament Help`** has no `Tournament C` screen model to open from.
- **The split screens' Help** (`Team HelpSplit`, `Track HelpSpilt`, the PS2's `Single Player` and `Split Screen*`
  pages) is not read: `Layout::read_split` builds no `Layout::help`.
- **The 3D preview is not drawn under an open Help**: it is a pass over the finished picture, so a dimmed
  craft or circuit under the overlay needs a second 2D pass above it.

## Next Steps

1. Trace `Team Ship Help`'s setter and wire the per-team paragraph.
2. Read the split screens' Help in `Layout::read_split`.
3. `Pre Race Music Select`: the `MusicList` widget, on `square`.
