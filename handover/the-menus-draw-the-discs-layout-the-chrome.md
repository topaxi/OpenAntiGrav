# The menus draw the disc's layout; the chrome around them is unbuilt

**2026-08-10, and it closes the standing "only `pulse_text.fnt` is wired to the menus" item.** `Data.wad` carries `MainMenu_Definition.xml` and 16 sibling GUI files - `menu.rs`'s comment that "the disc's own menu layout has not been read" was simply out of date. Read, then measured against a PPSSPP capture: rows at x=50, pitch 28, seven of them, in the `menu` font role, `TextColor` for an unselected row and a brightening toward white for the selected one. Our capture now measures identical to the original's - glyph tops 40.0, heights 11.5, pitch 28.0, left edge 50.0. Everything is on [menus-original.md](../docs/ui/menus-original.md) and [fe-menu-definitions.md](../docs/formats/fe-menu-definitions.md). **Three things are known and unbuilt.** (1) The light angled **top bar** (`topbarleft`/`center`/`right`) and the **footer** (tag block, scrolling ticker, button prompts) - until the bar exists the authored black `TitleColor` cannot be used, so this build substitutes its own title colour, the same substitution the picker already makes. (2) The selected row's highlight **pulses** - two frames of the same still menu measured different peaks - and its period and depth were not measured, so it is drawn flat; the same discipline, and the same warning, **do not measure it off our own build**. (3) The **easing curve is invented** (confidence 30): the capture shows only that the motion accelerates. `Tween::eased` is the one function to change if anyone reads the real curve out of the executable. **Two traps cost time here**: `just drive menu` walks straight past `Main Menu` into a live race, and the original enters an attract demo after ~120 s idle, which silently invalidates a capture left sitting.

## Open

- Top bar and footer chrome (tag block, ticker, button prompts) are not built, so the authored black `TitleColor` cannot be used yet.
- The selected row's highlight pulse period and depth were never measured; it is drawn flat.
- The easing curve is invented (confidence 30) - the capture only shows that motion accelerates.

## Next Steps

- Build the top bar and footer so the authored `TitleColor` can replace the current substitution.
- Measure the highlight pulse's period and depth from the original - not off our own build.
- Read the real easing curve out of the executable and update `Tween::eased`.
