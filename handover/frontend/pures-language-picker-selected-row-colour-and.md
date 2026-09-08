# Pure's language picker: the selected row's colour is a placeholder, and its highlight sits high

## Open

Two follow-ups left after fixing the picker's real blank-row-while-selected
bug (the selected row's `Draw::Text` was hardcoded to opaque white, which
Pure's white picker backdrop swallowed completely - fixed by drawing the
selected row in the same colour as every unselected row, since the highlight
`Fill` is what marks selection).

### 1. A measured "selected" colour already exists and is not wired in

`crates/pure/src/frontend.rs`'s `MENU_SKIN` constant carries
`selected: Some(0xFF16AED1)` - measured off a real `pure-psp-usa.chd`
capture under PPSSPP (`SINGLE PLAYER` selected against `MULTIPLAYER`/
`PROFILE`/`DOWNLOAD` not, on `Main Menu`), confidence 65. It is **darker and
more saturated** than the unselected `TextColor` (`0xFF88D6E8`, which
matches what the language picker's own `Menu` XML widget authors and what
this fix now draws every row in) - the opposite direction from Pulse's own
"brightens toward white" `selected()` behaviour in
`crates/game/src/menu/skin.rs`.

`crates/game/src/frontend/draw.rs`'s `draw_language_selection` has no way to
reach that table: `Frontend` (`crates/game/src/frontend.rs`) carries no
`oag_title::MenuSkin` reference at all - it only holds the parsed disc XML
(`Screens`), not the title's separately-measured skin constants. Threading
one through means adding a field to `Frontend`, updating every constructor
(`Frontend::new`, `Frontend::booting`) and every call site
(`crate::boot::load`, `crate::remix`, `crate::race::load`,
`crate::race::hud`, and every test fixture in
`crate::frontend::tests::{pure, frontend, ...}`), which is why this pass
used the safe fallback (row's own colour, unselected or not) instead of
guessing under a deadline.

**Next step**: give `Frontend` an `Option<&'static oag_title::MenuSkin>` (or
just the two fields the picker needs, `text`/`selected`, if the full type is
overkill here), thread it from `boot::load_shell`/`assemble` the same way
`languages`/`strings` already are, and have `draw_language_selection` prefer
`skin.selected` when selected and fall back to the XML's own `Menu` colour
when the title carries none (Pulse's own picker never showed the invisible-
text bug because its backdrop isn't white, but confirm it still reads right
once this lands - Pulse's `selected()` pulses toward white, which needs the
same `pulse_elapsed` clock `crate::menu::Skin` tracks, not just a static
colour swap).

### 2. The highlight sits noticeably higher than the row it marks

Now that the selected row's text is visible at all, a second, pre-existing
defect is visible with it: the highlight `Fill`'s rect
(`[menu_x - 6.0, y - 4.0, 220.0, row - 4.0]`, `crates/game/src/frontend/
draw.rs`) is shorter than the row's own glyph height and offset upward
enough that it visibly overlaps the row *above* the selected one as much as
the selected row itself. E.g. selecting row 1 (`Français`) draws the cyan
band mostly over `English` (row 0)'s lower half, with `Français`'s own
glyphs sitting mostly below the band. Screenshotted,
`--press down --ticks 2` on `pure-psp-usa.chd`,
`/tmp/oag-shots/fixed_row1_ctx.png` (not committed, per ADR-0006).

The `- 4.0` on both `y` and `row` in that rect is an unmeasured guess from
whoever first wrote this loop (`git blame`: `918f78215`, 2026-08-18) -
`row - 4.0` also being the same "too short to hold the glyph" instrument
this bug's own investigation used to prove the underlying colour was the
real defect. It was invisible before this pass because white-on-white hid
both problems at once; fixing the colour did not fix the geometry.

This is also very likely the same root cause as the already-open
`MenuSkin::first_row_y`/row-pitch gap in `docs/formats/pure-status.md` -
worth checking together rather than as two separate defects, since both are
"a row's vertical geometry on this screen is not actually measured."

## Next Steps

- Thread `oag_title::MenuSkin` (or just its two colour fields) into
  `Frontend` and have `draw_language_selection` prefer
  `MENU_SKIN.selected` over the row's own colour when selected, with
  Pulse's own pulse-toward-white behaviour intact for its title.
- Measure the real highlight `Fill` rect against a fresh PPSSPP capture of
  `Language Selection` (position and height, not just colour) rather than
  reusing the unmeasured `-4.0` guess - fold in whatever
  `docs/formats/pure-status.md`'s `first_row_y`/row-pitch gap turns up,
  since they may be one finding.
- Re-verify both titles' pickers (all rows selected in turn, headless
  screenshots) once either lands.
