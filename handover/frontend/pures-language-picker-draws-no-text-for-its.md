# Pure's language picker draws no text for its own selected first row

Found while confirming a fix to Pure's string-table and font loading (closed
this pass - see `docs/formats/pure-status.md`'s
`#the-language-plugin-id-space-is-pures-own-not-pulses` section), by
screenshotting `Language Selection` more than once rather than trusting the
first frame. It is a different, unrelated bug in the same screen.

## Open

`Language Selection`'s row list is `English, Français, Deutsch, Español,
Italiano`. Whichever row is index 0 - always `English` on this disc, since
the list order never changes - draws **no text at all** while it is the
selected row, and every other row draws correctly while selected.

Reproduced four ways, `--dry-run`/`--screenshot` against
`pure-psp-usa.chd`, headless (no window opened, no black-frame problem -
every PNG below has real content):

- `--ticks 0`, no press: row 0 (English) selected, blank. Confirmed not a
  fade-in artifact - `--ticks 1` and `--ticks 3` with no press show the same
  blank row, well past any one-frame transition.
- `--press down --ticks 4`: row 1 (Français) selected, text renders fine.
- `--press down,down,down,down --ticks 8`: row 3 (Español) selected, text
  renders fine; row 4 (Italiano) partly clipped by the highlight rect's own
  height, a probably-unrelated row-pitch question already open in
  `docs/formats/pure-status.md` (`MenuSkin::first_row_y` is measured,
  row *pitch* is not).
- `--press down,down,down,down,down --ticks 10`: the cursor wraps from
  Italiano back to English (row 0) - blank again. Settles that this is tied
  to row 0 (or to the language "English" itself; the two are
  indistinguishable on this disc, since English is always index 0), not to
  "the very first frame after the screen opens."

**Pixel-level, not eyeballed**: cropping the row-0 region in the `--ticks 0`
capture and histogramming it (`magick ... histogram:info:`) finds exactly two
colours - the highlight fill and the white page background, with ordinary
antialiasing between them. No third colour anywhere in the region, which
rules out a contrast problem (pale text on a pale highlight would still show
as a *third*, blended colour) - the glyphs are not drawn, not merely
invisible. The same crop over a *correctly*-rendering selected row (Français)
finds a clearly distinct ink colour, `~(123,217,238)`, that row 0 never
shows.

Screenshots (not committed, per ADR-0006):
`/tmp/oag-shots/pure_lang_picker_t0.png`, `_t1.png`, `_t3.png` (all blank
row 0), `_down.png` (row 1 selected, fine), `_down4.png` (row 3 selected,
fine), `_wrap.png` (wrapped back to row 0, blank again).

## Hypothesis, not yet a finding

`crates/game/src/frontend/draw.rs`, the `DisplayLanguages`/`Menu` loop
(currently around line 753-776), draws each row's highlight `Fill` then a
`Draw::Text` in the same iteration, with the selected row's text colour
hard-coded to `[1.0, 1.0, 1.0, 1.0]` (opaque white) rather than the row's own
measured colour. That is consistent with *a* text-invisibility bug in
general, but does not by itself explain why it only misses row 0: rows 1 and
3 are also `selected` under the identical branch and both render visible,
non-white ink (`~(123,217,238)`, matched above) while selected - so whatever
sets that visible colour for rows 1/3 either does not apply to row 0, or row
0's `Draw::Text` call is never reached/never receives a non-empty string in
the first place. Both are guesses; neither is verified. No confidence score
- below the RE workflow's threshold for one, per `CLAUDE.md`.

## Next Steps

- Add a `println!`/report line (or a unit test against `PickerFrame`/whatever
  builds this `Draw::Text` list directly, without a screenshot) that dumps
  the exact string and colour queued for row 0 when it is selected, and
  compare against row 1 selected - settles "empty string" against "string
  present, colour/paint wrong" against "draw call never queued" in one step,
  cheaper than another screenshot pass.
- If the string is present and non-empty: read what differs about `index ==
  0` specifically in the loop at `crates/game/src/frontend/draw.rs`'s
  `DisplayLanguages` handling - check for anything above it in the same
  function keying off `self.selected == 0` or `index == 0` that could emit a
  second, overriding draw command (a title-adjacent widget, a "current
  language" indicator, anything sharing row 0's rect).
- Try the same probe on `pure-psp-eu.chd` and on Pulse's own language picker
  (same draw path, `crates/game/src/frontend/draw.rs` is shared, not
  per-title) - if Pulse's row 0 (also its own first language) shows the same
  blank, this is a shared-code bug rather than a Pure-only one, which changes
  where the fix belongs.
- Not the same bug as the `first_row_y`/row-pitch gap already open in
  `docs/formats/pure-status.md` - keep the two separate; that one is about
  *position*, this one is about a *missing draw*, and Italiano's clipping in
  the `_down4.png` capture is likely that one, not this one.
