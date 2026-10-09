# PS2 front-end layout is hardcoded to 480x272

**Closed 2026-08-09, not the way this row proposed.** `SCREEN` did not become per-source; `frontend::Space` carries a grid **and** the display aspect (24% apart on the PS2, more than one `(f32, f32)` holds), and the rects/pillarboxes route through it. `SCREEN`'s readers are where 480x272 is right whatever the disc: the loading screen, the HUD bounds check, `race::AUTHORED_ASPECT`. **That 24% was 7% until 2026-08-23**: the display aspect was 4:3 off the television and it is 480/272 - the port stretched the PSP's art by `(640/480, 448/272)`, as `FUN_001e9370` does. `docs/ps2/aspect-ratio.md` also retires the movies' declared 4:3. **The `widthlimited` half closed 2026-08-26** in `7d7cb2c3` ("wrap widthlimited front-end text, fixing BOOT_LEGAL"): `screen.rs` resolves the attribute against its enclosing `Viewport`'s width and `render/text.rs` does the wrapping, pinned by `screen/tests.rs`'s `a_widthlimited_text_wraps_against_its_viewports_width`.

**The face axis this row's own "Next Steps" asked for has existed since**
`oag_title::MenuSkin::title_font`, `oag_ui::font::Atlas`,
`crates/game/src/boot/fonts.rs`'s `load_title_font`/`face_atlas_slot` and
`crates/game/tests/font_roles_ground_truth.rs` - `crates/game/src/font.rs`,
the file this row named, no longer exists at all. HD's chrome title was
flipped onto its own `Title` role 2026-09-13; **Pure's was checked and
flipped 2026-09-25** - `Skin.xml`'s `Main Menu` screen authors
`font="Title"` on both pressings, resolving to the same `.fnt` `Default`
already uses, so the fix is correctness with no visible change
(`--menu-page main --screenshot` renders pixel-identical before and after).
See `docs/formats/hd-frontend.md`'s "The title's own font role" section for
both.

**2026-09-27: the PS2 approximation this row's title names is retired for the
row list, and the subtitle overlap it caused is fixed.** `oag_ui::menu::Skin`
used to draw a PS2 menu by scaling `oag_pulse::frontend::MENU_SKIN` - itself
read off the PSP's `Data.wad` - by the PS2's `640/448` grid ratio, which is
exactly the approximation `oag_display::space::Space`'s own doc already
flagged as covering only 30 of 43 shared coordinates. Reading the PS2's own
`WADS2.WAD` (`Data\Plugins\PI001\GUI\MainMenu_Definition.xml`, `Skin.xml`)
directly finds the row list is one of the 13 it does not cover:
`<Menu>` authors `y="53"` (the scaled PSP number, `32 * 448/272 = 52.7`, is
close by coincidence), `helptext0` sits `y="75"` - 22 below its row, not the
PSP's 18 scaled to 29.6 - and the same file's own `<!-- +41 for each line
-->` comment states the row pitch outright, against the scaled
approximation's 33.9. The 22-versus-29.6 miss, combined with a row pitch
scaled too small to absorb it, is exactly what put the `RACE CAMPAIGN`
subtitle (`The story mode: work your way up the tournament grid`) on top of
`RACEBOX`/`COURSE` below it - confirmed live: a fresh PCSX2 capture of the
real `pulse-ps2-eu.chd` Main Menu (`SCES-54748`, `just pcsx2-boot`) shows the
same subtitle sitting cleanly above `SPLIT SCREEN VERTICAL` with no overlap.

Fixed by `oag_title::FrontEnd::menu_ps2` and `oag_pulse::frontend::PS2_MENU_SKIN`
(new, read off `WADS2.WAD`), preferred over the scaled `MENU_SKIN` on a PS2
source in `crates/game/src/boot.rs`'s `load_shell` - the one place
`Shell::menu_skin` is resolved, so every reader (`--menu-page`, the live menu
stage, end-race, campaign) picks it up with no changes of its own. PSP output
is unchanged: every `--menu-page` PNG across all twelve pages is byte-identical
before and after. See `crates/pulse/src/frontend.rs`'s own doc on
`PS2_MENU_SKIN` for the full authored/derived table and confidence per field.

**2026-09-27: the PS2 title-font question is settled, not just leaned on -
do not flip `PS2_MENU_SKIN::title_font`.** A second, independent PCSX2 walk
(fresh boot, `SCES-54748`, English) pixel-measured both glyph heights in the
same capture rather than eyeballing them: cropping `MAIN MENU`'s title-bar
text and gridding it 8x found its cap height spans y=13 to y=27 (14 px);
the same treatment on `RACE CAMPAIGN` (the selected row, white) and `SPLIT
SCREEN VERTICAL` (an unselected row, teal) both span 14 px too - three
glyphs, two different draw states, one measured height. If the title drew
in the disc's `Title` role (`Pulse_14.fnt`, 17px) at any scale that did not
happen to net out to exactly the `Default` row face's rendered size, this
would show as a visibly taller title; it does not. This is the capture the
project's own rule asks for before flipping a font, and it says don't -
closing what 2026-09-25 and the first 2026-09-27 pass could each only lean
on. See `measure-title.png` and
`measure-row.png` (the gridded crops) and `back2.png` (the source capture).

A same-image x-position check on the two glyphs' left edges (`M` of `MAIN`
at x=63, `R` of `RACE` at x=68, both against `menu_x`/`title_x`'s shared
authored `66`) came back within 5 px of each other - consistent with the
one shared x offset the XML already gives both, not evidence of a second
one. The y position is less clean: calibrating a px-per-authored-unit scale
off the `SPLIT SCREEN VERTICAL` -> `SPLIT SCREEN HORIZONTAL` gap (47 px for
the authored 41-unit pitch, itself a second, independent corroboration of
that pitch beyond `Main Menu`'s own `helptext` run) and projecting from
`first_row_y=53`'s measured cap-top (68 px) predicts the title's cap-top at
`title_y=9` should land near y=18; it measures at y=13, a 5 px (~4 authored
unit) residual. That is plausibly font-metrics bearing between two
different typefaces rather than a real coordinate error - the same
measurement approach that settled the font question outright can't settle
this one at this capture resolution - so `title_x`/`title_y`/`title_scale`
stay open below rather than promoted, but with data now instead of an
unmeasured gap.

## Open

- `PS2_MENU_SKIN::row_extra_leading` (`17.0`) is corroborated on `Main
  Menu`'s own `helptext0..6` step alone. `first_row_y` (`53.0`) is
  corroborated on three screens (`Main Menu`, `Racebox`, `Additional`/
  Options), but none of the other two carries a second `helptext` run to
  re-check the `41` pitch against - unlike the PSP's matching constant
  (`6`), corroborated on four screens across two faces before this crate
  trusted it. Lower confidence than the rest of `PS2_MENU_SKIN`'s table
  until a second PS2 screen with `helptext` rows turns up (`Single Player`'s
  PS2 name, if it has one, is the natural next check - it is the PSP
  screen that measured the `small`-face pitch).
- `title_x`/`title_y`/`title_scale` (`66.0`, `9.0`, `0.8`) are authored,
  straight off `WADS2.WAD`'s `FEGlobals`. **2026-09-27**: a same-image pixel
  check (above) found the x offset consistent with the authored `66` within
  ~5 px of measurement noise, and the y offset within a 5 px (~4 authored
  unit) residual of a row-pitch-calibrated prediction - not a confirmed
  match the way the row list is, but not a contradiction either. Settling it
  fully needs a cleaner anchor than glyph cap-tops across two typefaces at
  682x512 - a widget whose box (not just its text) is visible in the
  capture, or a higher internal resolution.
- Every other Pulse menu page's PS2 XML (`race`/`Racebox` confirmed sharing
  `Main Menu`'s row-list numbers; `remix`/`display`/`graphics`/`audio`/
  `controls`/`pilots` not read at all) is unread. Nine of the eleven pages
  rendered clean under `PS2_MENU_SKIN` in the 2026-09-27 sweeps
  (a scratch directory, not kept, `ps2-walk/`), but "clean" here
  means "no overlap `first_row_y`/`row_extra_leading` would produce," not
  "confirmed against that screen's own XML" the way `Main Menu` now is.
  `records` is the exception worth a look: the PS2 disc's own `RACE
  RECORDS` is a two-tier flow (`Profile` -> `Race Records` -> `Single Race
  Records`, the last carrying a `Track`/`Speed Class` picker and a
  `Lap`/`Time`/`Tag`/`Team` podium table) where this build's single
  `records` page lists `Mode`/`Track` then every speed class's best time
  flat - a real structural difference from the disc, but a menu-content
  reorganisation this project already makes throughout (`OPTIONS`'s own
  four-screen split versus this build's six flat pages is the same shape of
  difference), not a PS2-specific layout bug. See
  `pcsx2-race-records.png`,
  `pcsx2-single-race-records.png` against `ours-records.png`.
- **A real, non-layout gap found in the same walk**: the PS2's own
  `RACEBOX` screen (`--menu-page race`'s equivalent) authors five rows -
  `RACE TYPE`, `SPEED CLASS`, `WEAPONS`, `AI DIFFICULTY`, `KILLS` - and this
  build's `race` page in `assets/ui/menu.toml` offers three (`MODE`,
  `SPEED CLASS`, `AI DIFFICULTY`); `WEAPONS` and `KILLS` (the Eliminator
  kill-limit setting) have no row and no backing setting anywhere in this
  crate (`rg` for `weapons_enabled|eliminations|kill_limit` across
  `crates/` finds only `Mode::weapons_enabled`, a per-mode default with no
  player override, and no kill-limit field at all). This is shared with the
  PSP build (same `menu.toml`), not PS2-specific, and it is a settings +
  UI feature addition rather than a position/font fix, so it is out of this
  lane's scope - documented here since this walk is what found it. See
  `racebox1.png` (the disc's row
  list) against `ours-race.png` (this build's).

## Next Steps

- Read a second PS2 screen's `helptext` step (or find the PS2's equivalent
  of `Single Player`/`Cell Setup`, the two PSP screens that corroborated the
  `small`-face pitch) to move `row_extra_leading`'s confidence up from
  single-source. **2026-09-27**: `RACEBOX` itself (this walk's own capture,
  `racebox1.png`) turned out to author no per-row subtitle at all, so it
  cannot be that second screen - the PS2's equivalent of `Single Player`
  remains unfound.
- `title_x`/`title_y`/`title_scale`'s residual (see Open) needs a cleaner
  anchor than two typefaces' cap-tops at 682x512 to close - a widget
  bounding box, or a capture at a higher internal resolution than PCSX2's
  GS buffer gives on this title.
- ~~The `WEAPONS` row gap above~~ **landed 2026-09-30** (`race.weapons`, with `KILLS`'s
  `race.kill_target` the same day): the disc's own `Weapons` list, editable in a single race and
  pinned to the mode's own answer elsewhere - see `docs/formats/race-setup.md`.
