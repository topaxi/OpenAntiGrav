# Pointer input: what the mouse and the touchscreen still cannot do

Every screen the front end draws answers a mouse and a finger since
2026-09-14, and each title draws its own cursor - the whole design is on
[menus.md](../../docs/architecture/menus.md) under "A mouse and a finger",
and `assets/cursors/README.md` lists the five SVGs. Verified live on X11
with `xdotool` against Pulse (PSP EU: PRESS START by click, hover-select,
click into OPTIONS, right-click back, QUIT), HD/Fury (strip hover, tab
click, a list row's step arrows both ways, right-click back, QUIT) and
Pure (its own cursor draws over the white language picker; hovering a row
selected the row *below* until `oag_ui::pointer::RowInk` centred the band
on the face's capitals - see menus.md's "A row of plain text gets its band
from the face's ink" - after which the maintainer reported the picker
"feels good now"; the same fix reaches Pure's selection screens through
`menu::Skin::set_row_ink`, unverified live). **The Race Campaign's `Grid
Selection`/`Cell Selection` joined this set the same day**, in the
`campaign-pointer` lane, after this file's own opening line was already
written ahead of them - see `oag_ui_screens::campaign::pointer` and
`docs/ui/campaign-screens.md`.

## Open

- **Pure's selection screens under the ink-centred band, live.** The
  language picker's hover is confirmed; its second tap confirming was
  only exercised in `frontend/tests/pointer.rs`, because confirming there
  persists the language once the menus open. The ship and track rows take the same
  `RowInk` through `menu::Skin` but were only checked by
  `picker/tests.rs`'s `pures_listed_rows_...`. Boot Pure into a race and
  hover its ship list; if a row above the tip lights, `Skin::set_row_ink`
  is being measured off a different atlas than the one `Draw::Text` draws
  those rows with (`rows_face` in `Session::open_menus` today).
- **The frame loop's own composition has no test.** `session/frame.rs`
  gates the menu rows on `prompt_was_open` - the prompt state *before*
  `tick_prompt` - because a `Pointer` is a value nothing consumes, and
  every unit test drives one model at a time (`Confirm::pointer`,
  `Menu::pointer`). A `Session` needs a GPU, so the tick-of-closing case
  (click KEEP, and the row beneath must not fire) is asserted by reading
  the code rather than running it. A headless `--click` (below) would be
  the way to pin it.
- **2048 in PS TV mode.** Its cursor here is invented like the other four.
  `data/FE/Images/cursor.gxt` was checked first and is a 32x16 mark, the
  same shape as HD's strip-underline `cursor.gtf`, not a pointer - but
  whether the Vita build draws a pointer at all under PS TV is unread, and
  `data/FE/Images/TV.gxt` (43,760 bytes) beside it has not been decoded.
  If a real pointer turns up, it replaces `assets/cursors/2048.svg`.
- **Drag-to-scroll is built and unit-tested, not live-tested.** A finger
  drag scrolls a long `Menu` page and pans 2048's campaign map
  (`Pointer::drag`, `oag_game::pointer::Window`), tap is decided on lift,
  and the wheel pans the map; see menus.md "A mouse and a finger". Only
  headless unit tests drove it: no touch can be injected on the Xvfb, and
  the Waydroid/phone pass is the maintainer's. Check on a phone: a drag on
  GRAPHICS scrolls without selecting, a tap on a row still selects, and the
  2048 map pans. The 12 px threshold, the row-per-pitch rule, the 40 px
  wheel detent and the map's wheel step are **chosen, not measured**; no
  fling exists.
- **Headless captures cannot click.** `--press` drives buttons through the
  capture's own tick loop, but the pointer arrives only through winit
  events in `main/app.rs`; a `--click X,Y` would need `capture::run` to
  build a `Pointer` per tick the way it builds an `Input`. The unit tests
  cover every model directly, so this is a verification convenience rather
  than a gap in behaviour.

## Next Steps

1. The Pure live pass above - five minutes with a window.
2. Decode `TV.gxt` and read what 2048's PS TV front end does with a pointer,
   before treating `2048.svg` as final.

## From the HANDOVER.md index (moved 2026-09-25)

every front-end screen answers a mouse and a finger and each title draws its own cursor (`../../docs/architecture/menus.md`, "A mouse and a finger"); left open are a live pass over Pure's language picker, whether 2048 draws a real pointer under PS TV (`TV.gxt` undecoded), drag-to-scroll, and a `--click` for headless captures
