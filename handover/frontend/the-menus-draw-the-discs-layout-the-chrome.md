# The menus draw the disc's layout; the footer's ticker and prompts are still unbuilt

**2026-08-10, and it closes the standing "only `pulse_text.fnt` is wired to the menus" item.** `Data.wad` carries `MainMenu_Definition.xml` and 16 sibling GUI files - `menu.rs`'s comment that "the disc's own menu layout has not been read" was simply out of date. Read, then measured against a PPSSPP capture: rows at x=50, pitch 28, seven of them, in the `menu` font role, `TextColor` for an unselected row and a brightening toward white for the selected one. Our capture now measures identical to the original's - glyph tops 40.0, heights 11.5, pitch 28.0, left edge 50.0. Everything is on [menus-original.md](../../docs/ui/menus-original.md) and [fe-menu-definitions.md](../../docs/formats/fe-menu-definitions.md). **Two things are known and unbuilt.** (1) The selected row's highlight **pulses** - two frames of the same still menu measured different peaks - and its period and depth were not measured, so it is drawn flat; the same discipline, and the same warning, **do not measure it off our own build**. (2) The **easing curve is invented** (confidence 30): the capture shows only that the motion accelerates. `Tween::eased` is the one function to change if anyone reads the real curve out of the executable. **Two traps cost time here**: `just drive menu` walks straight past `Main Menu` into a live race, and the original enters an attract demo after ~120 s idle, which silently invalidates a capture left sitting.

**2026-08-25: the top bar and the footer's two strips are built - closing this thread's own first item.** `topbarleft`/`center`/`right` was a guess made before `Skin.xml` was actually read; on the disc it is **one** `<Image>` (`x=20 y=0 w=224 h=24`), and the footer is two more (`x=12 y=236` and `x=12 y=249`, both `w=454 h=14`, each its own `U`/`V` sub-rect) - all three patches of one shared `Data\FE\Images\pulse_assets.mip`, sitting three anonymous `<Screen>` levels down inside `Top FE Screen->FE Screen`. That nesting is why they were missing rather than wrong: `Screens::collect_widgets` recursed into a `<Viewport>` and an `<Animation>` but not into a bare grouping `<Screen>`, so all three were silently dropped with no error and no picture. Fixed in `crates/game/src/screen.rs`; `crates/game/src/menu/frame.rs` also gained the `U`/`V` sub-rect reading `frontend/draw.rs` already had, which the footer's two strips need (they share a texture, so without it both would draw the sheet's own top-left corner). The authored black `TitleColor` draws now too - `Skin::title_color` used to have only two answers (this build's substitute, or a frame's own ink), and Pulse's marks carry no tint of their own, so it would have put white text on the bar; it now checks the disc's declared colour first. See [menus-original.md](../../docs/ui/menus-original.md)'s own section and `crates/game/tests/pulse_frame_ground_truth.rs`, which pins all three widgets against the disc two ways - the raw XML and `read_frame`'s own output.

**Still unbuilt, and now the whole of what "the footer" means: the ticker text and the button-prompt line.** The two strips drawn are backdrop art only; the moving/localised parts on top of them - a `<TextInfo>` (the tag and the scrolling news text) and a `<NavigationController>`'s button prompts - are elements nothing in `oag_game::screen` recognises, the same gap already named for Wipeout HD's own trial-build text and button prompts on `FE Screen`.

## Open

- The footer's own ticker text (`<TextInfo>`) and button-prompt line (`<NavigationController>`) are not built - `oag_game::screen` recognises neither element yet.
- The easing curve is invented (confidence 30) - the capture only shows that motion accelerates. ~~The selected row's highlight pulse period and depth were never measured~~ - measured 2026-09-05, see below.

**2026-09-05, from the HD side, and it narrows both of the two items above
without closing either.** A census of Wipeout HD's entire front-end XML (772
entries, all seven archives) found that **no easing curve is authored anywhere**:
its 122 `<Key>` elements carry `Time`, `X` and `Y` and nothing else, and the
shape of a motion is authored by adding a key rather than by naming a curve, so
interpolation is hard-coded in the executable. It also found that **no pulsing
or oscillating highlight is authored** - no `blink`/`flash`/`cycle`/`sine`/
`period`/`phase` attribute exists at all, `<Key>` cannot key a colour, and
`pulse="true"` is real but never appears on a menu `<Entry>`.

**Neither finding is Pulse's answer.** HD is a different title on a different
engine, and these items were measured on a PPSSPP capture of Pulse's own menus;
both stay open exactly as written. What HD's data does establish is that
*looking in the front-end XML* is unlikely to be the route for either one here
either - Pulse's own `Skin.xml`/`*_Definition.xml` would need the same census
before assuming it, but the executable is where HD's answer lives, and that is
worth knowing before spending a session in the wrong file. See
[hd-frontend.md](../../docs/formats/hd-frontend.md#three-questions-this-closes).

**2026-09-05: the highlight pulse's period and depth are measured, and closes
this thread's second item - Pulse's own XML census, and the executable read,
both done.** A whole-GUI census confirmed the same shape HD's own audit found:
`pulse="true"` exists (on `BOOT_PRESS_START` and equivalent blinking prompts,
never on `MainMenu_Definition.xml`'s own rows) but is a bare boolean with no
numeric period anywhere, and `delay` turned out to be a reveal-offset timer
shared with `<Animation>`/`<TextInfo>`, not a pulse period either - see
[fe-menu-definitions.md](../../docs/formats/fe-menu-definitions.md). So the
period had to come off the running original, the same way the page-transition
curve did: a frame-accurate PPSSPP capture (breaking on `Gfx_PresentFrame`,
150 consecutive presented frames of an uncontaminated still `Main Menu`) found
an exact, repeating 33-presented-frame period across four cycles (1.1 s at
30 Hz) and a peak of exactly `0xFFFFFFFF` on three of them, with a flat
control column (an unselected row) proving it is the row's own animation.
Built in `oag_game::menu::Skin::selected`, keyed off a new
`oag_title::MenuSkin::selected_pulse_period_secs` field, `Some(1.1)` on Pulse
only. Full method and numbers in
[menus-original.md](../../docs/ui/menus-original.md#the-highlights-pulse---period-and-depth-measured-2026-09-05).
**The curve's shape between the two endpoints is still invented** (a raised
cosine, marked the same way `Tween::eased` marks its own) - the capture found
eleven unevenly-spaced discrete levels, enough to rule out a linear ramp, not
enough to name the real curve.

**The easing curve itself (this thread's third item) was not attempted this
session, and here is why, for whoever picks it up next.** `names.tsv` has
zero front-end/menu functions on `psp-pulse-usa` - the closest anchor is
`Xml_AttributeAsFloat` (`0x0895379c`), and its callers are unnamed `FUN_*`
territory with nothing to recognise a tween by. Direct `jal` calls do survive
the missing-relocations problem (unlike data loads), so a call-graph walk from
there is still the right *route* - it just did not fit in the same session as
the period/depth measurement above, and starting it without a plan for how to
recognise the right function once found would have been an open-ended Ghidra
session on a guess. `Tween::eased` is untouched.

## Next Steps

- Read what `<TextInfo>` and `<NavigationController>` author (Pulse's footer, HD's trial-build text) and decide which parts belong on a retail build before drawing either.
- Read the real easing curve out of the executable and update `Tween::eased`. Start from `Xml_AttributeAsFloat` (`0x0895379c`)'s callers by call structure, not by name search - nothing menu-shaped is named yet.
