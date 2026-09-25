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

## Open

- Pulse's screen title (both PSP pressings and the PS2 port) still draws in
  the row (`menu`) face at title scale rather than the disc's own `Title`
  role (`Pulse_14.fnt`, 17px against `Default`'s 13px `pulse_text.fnt`) -
  confirmed unchanged by a fresh `--menu-page main` render of both PSP disc
  and the PS2 port, 2026-09-25 (`OPENANTIGRAV` draws in the same face as
  `RACE CAMPAIGN` below it on both).

## Next Steps

- This is a genuinely different case from Pure's: Pulse's `Title` role
  resolves to a *different* file than `Default`, so flipping
  `oag_pulse::frontend::MENU_SKIN::title_font` to `Some("Title")` would be a
  real visual change, not a no-op one. `docs/ui/menus-original.md`'s Layout
  table is already capture-verified at confidence 95 against the current
  (unflipped) rendering, and no capture has yet checked whether the taller
  face is what the real console's title actually shows. Whoever takes this
  needs a fresh PSP/PS2 capture (PPSSPP/PCSX2, the way
  `hds-strip-tabs-have-no-shape-and-no-scene-behind-them.md` did for HD on
  RPCS3) checking the title specifically, before flipping it - see
  `oag_title::MenuSkin::title_font`'s own doc for the widget already found.
