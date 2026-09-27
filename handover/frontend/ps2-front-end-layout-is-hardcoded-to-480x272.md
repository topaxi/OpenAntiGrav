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

## Open

- Pulse's screen title (both PSP pressings and the PS2 port) still draws in
  the row (`menu`) face at title scale rather than the disc's own `Title`
  role (`Pulse_14.fnt`, 17px against `Default`'s 13px `pulse_text.fnt`) -
  confirmed unchanged by a fresh `--menu-page main` render of both PSP disc
  and the PS2 port, 2026-09-25 (`OPENANTIGRAV` draws in the same face as
  `RACE CAMPAIGN` below it on both). **2026-09-27**: the first real PS2
  capture of this screen (PCSX2, `SCES-54748`) is inconclusive rather than
  confirming - `MAIN MENU`'s glyphs in the title bar read as the same face
  as `RACE CAMPAIGN` below it, not a visibly taller one, which is a lean
  against flipping `PS2_MENU_SKIN::title_font` rather than the capture this
  project's own rule needs to flip it. See
  `data/scratch/drive-2026-09-27/ps2-fe/pcsx2-main-raw11.png`.
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
  straight off `WADS2.WAD`'s `FEGlobals`, but not independently corroborated
  against a pixel measurement of the PCSX2 capture the way the row list was -
  the capture's own resolution (682x512, not evenly divisible into 640x448)
  and this build's own presented output (1440x816, `480/272` display aspect)
  do not share a common scale factor without recalibration. Worth a proper
  measurement before treating the title bar position as settled the way the
  row list now is.
- Every other Pulse menu page's PS2 XML (`race`/`Racebox` confirmed sharing
  `Main Menu`'s row-list numbers; `remix`/`records`/`display`/`graphics`/
  `audio`/`controls`/`pilots` not read at all) is unread. All eight rendered
  clean under `PS2_MENU_SKIN` in a 2026-09-27 sweep
  (`data/scratch/drive-2026-09-27/ps2-fe/`), but "clean" here means "no
  overlap `first_row_y`/`row_extra_leading` would produce," not "confirmed
  against that screen's own XML" the way `Main Menu` now is.

## Next Steps

- This is a genuinely different case from Pure's: Pulse's `Title` role
  resolves to a *different* file than `Default`, so flipping
  `oag_pulse::frontend::MENU_SKIN::title_font` (PSP) or
  `PS2_MENU_SKIN::title_font` (PS2) to `Some("Title")` would be a real
  visual change, not a no-op one. `docs/ui/menus-original.md`'s Layout
  table is already capture-verified at confidence 95 against the current
  (unflipped) PSP rendering, and the 2026-09-27 PS2 capture above leans
  against rather than for flipping the PS2 one. Whoever takes this next
  needs a PSP capture (PPSSPP, the way `hds-strip-tabs-have-no-shape-and-no-scene-behind-them.md`
  did for HD on RPCS3) checking the title specifically, and a second look at
  the PS2 capture already taken, before flipping either - see
  `oag_title::MenuSkin::title_font`'s own doc for the widget already found.
- Read a second PS2 screen's `helptext` step (or find the PS2's equivalent
  of `Single Player`/`Cell Setup`, the two PSP screens that corroborated the
  `small`-face pitch) to move `row_extra_leading`'s confidence up from
  single-source.
- A pixel measurement of `title_x`/`title_y`/`title_scale` against a PCSX2
  capture, once the two images' scales are reconciled (see Open above).
