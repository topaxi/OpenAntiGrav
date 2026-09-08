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

## Confirmed 2026-09-08: it is row-0-plus-selected, and it is not the shared draw loop

Two things settled by fresh captures (`--until "Language Selection" --hold
start --press down[,down...] --screenshot`, `pure-psp-usa.chd`), before this
thread's own "Next Steps" instrumentation was reached:

- **Row 0 is blank only while it is *also* selected - not "whichever row is
  selected".** Selecting row 1 (`--press down`) leaves row 0 (`English`,
  unselected) rendering fine *and* row 1 (`Français`, selected) rendering
  fine. Selecting row 2 (`--press down,down`) leaves rows 0 and 1 fine
  unselected and row 2 fine selected. Only row 0 selected (`--ticks 0`, no
  press, or wrapped back via five downs) is blank. This was checked because a
  second read of the original repro proposed "the selected row in general is
  blank" as an alternative - it is not; the original characterization in
  `## Open` above is the one the evidence supports.
- **Pulse's own picker does not reproduce it, and its row 0 is a different
  language.** Pulse's list order is `Français, Deutsch, Español, Italiano,
  English` - French at index 0, English last - and French renders fine while
  selected at index 0 (`pulse-psp-usa.chd`, same `--ticks 0` capture shape).
  So this is not "index 0 breaks in the shared draw loop"
  (`crates/game/src/frontend/draw.rs`, `DisplayLanguages`/`Menu`, the same
  code both titles run): the loop treats every index identically and Pulse's
  index 0 is fine. What differs between the two discs at row 0 is the
  *language object* - Pure's is `English`/`PI000`, Pulse's is
  `French`/`PI008` - which points at something about Pure's own `PI000`
  entry rather than at position in the list. Confirmed by reading the loop
  itself too: no `index == 0` or `self.selected == 0` branch exists anywhere
  in `frontend.rs` or `frontend/draw.rs`.

**A live lead, not yet checked**: `docs/formats/pure-status.md` recently
corrected a claim that `PI000` inlines its strings - the correction being
that *none* of Pure's five languages name an external `entries.xml`, on
either pressing. Whatever is special about `PI000` in the string/plugin
read path is worth checking against this draw bug before assuming they are
unrelated: "the selected row draws no glyphs" and "this language's own text
resolves differently" could be the same defect seen from the string side and
the draw side. The instrumentation `## Next Steps` already asks for - dump
the queued `Draw::Text` for row 0 selected against row 1 selected - is still
the fastest way to tell "row 0 is given no string" apart from "row 0 is
given a string that fails to rasterise", and it would show which of the two
this is in one step.

Screenshots (not committed, per ADR-0006): `/tmp/oag-shots/pure_row0.png`,
`pure_row1.png`, `pure_row2.png` (Pure, rows 0/1/2 selected in turn),
`/tmp/oag-shots/pulse_lang_row0.png` (Pulse, its own row 0 selected, fine).

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

- **Settled 2026-09-08, so skip straight to instrumenting the string, not the
  loop**: it is not a shared-code index-0 bug (Pulse's own row 0 is fine, a
  different language), so reading `frontend/draw.rs`'s loop for an
  `index == 0` branch is a dead end - there isn't one, checked directly.
- Add a `println!`/report line (or a unit test against `PickerFrame`/whatever
  builds this `Draw::Text` list directly, without a screenshot) that dumps
  the exact string and colour queued for row 0 when it is selected, and
  compare against row 1 selected - settles "empty string" against "string
  present, colour/paint wrong" against "draw call never queued" in one step,
  cheaper than another screenshot pass. Given the loop reading above, "empty
  string" is now the better-supported guess - check `Language::native_name`
  for Pure's `PI000` entry specifically before assuming the draw side at all.
- **Check alongside `PI000`'s string/plugin read path**, not only the draw
  path - see the "A live lead, not yet checked" note above. If `PI000`
  resolves its own name or its string table differently from every other
  Pure language (the same plugin `pure-status.md` already flagged for a
  different, corrected claim), that is a stronger candidate than anything in
  the draw loop, which reads uniform across rows.
- Try the same probe on `pure-psp-eu.chd` once the cause is narrowed further -
  not done yet, since the USA pressing already settled "row 0 plus selected,
  language-specific" without it.
- Not the same bug as the `first_row_y`/row-pitch gap already open in
  `docs/formats/pure-status.md` - keep the two separate; that one is about
  *position*, this one is about a *missing draw*, and Italiano's clipping in
  the `_down4.png` capture is likely that one, not this one.
