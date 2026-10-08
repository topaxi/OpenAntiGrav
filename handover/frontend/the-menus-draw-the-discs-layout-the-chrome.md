# The menus draw the disc's layout and its footer ticker; the page-transition easing curve is still invented

**2026-08-10, and it closes the standing "only `pulse_text.fnt` is wired to the menus" item.** `Data.wad` carries `MainMenu_Definition.xml` and 16 sibling GUI files - `menu.rs`'s comment that "the disc's own menu layout has not been read" was simply out of date. Read, then measured against a PPSSPP capture: rows at x=50, pitch 28, seven of them, in the `menu` font role, `TextColor` for an unselected row and a brightening toward white for the selected one. Our capture now measures identical to the original's - glyph tops 40.0, heights 11.5, pitch 28.0, left edge 50.0. Everything is on [menus-original.md](../../docs/ui/menus-original.md) and [fe-menu-definitions.md](../../docs/formats/fe-menu-definitions.md). **Two things are known and unbuilt.** (1) The selected row's highlight **pulses** - two frames of the same still menu measured different peaks - and its period and depth were not measured, so it is drawn flat; the same discipline, and the same warning, **do not measure it off our own build**. (2) The **easing curve is invented** (confidence 30): the capture shows only that the motion accelerates. `Tween::eased` is the one function to change if anyone reads the real curve out of the executable. **Two traps cost time here**: `just drive menu` walks straight past `Main Menu` into a live race, and the original enters an attract demo after ~120 s idle, which silently invalidates a capture left sitting.

**2026-08-25: the top bar and the footer's two strips are built - closing this thread's own first item.** `topbarleft`/`center`/`right` was a guess made before `Skin.xml` was actually read; on the disc it is **one** `<Image>` (`x=20 y=0 w=224 h=24`), and the footer is two more (`x=12 y=236` and `x=12 y=249`, both `w=454 h=14`, each its own `U`/`V` sub-rect) - all three patches of one shared `Data\FE\Images\pulse_assets.mip`, sitting three anonymous `<Screen>` levels down inside `Top FE Screen->FE Screen`. That nesting is why they were missing rather than wrong: `Screens::collect_widgets` recursed into a `<Viewport>` and an `<Animation>` but not into a bare grouping `<Screen>`, so all three were silently dropped with no error and no picture. Fixed in `crates/game/src/screen.rs`; `crates/game/src/menu/frame.rs` also gained the `U`/`V` sub-rect reading `frontend/draw.rs` already had, which the footer's two strips need (they share a texture, so without it both would draw the sheet's own top-left corner). The authored black `TitleColor` draws now too - `Skin::title_color` used to have only two answers (this build's substitute, or a frame's own ink), and Pulse's marks carry no tint of their own, so it would have put white text on the bar; it now checks the disc's declared colour first. See [menus-original.md](../../docs/ui/menus-original.md)'s own section and `crates/game/tests/pulse_frame_ground_truth.rs`, which pins all three widgets against the disc two ways - the raw XML and `read_frame`'s own output.

**Still unbuilt, until 2026-09-25: the ticker text and the button-prompt line.** The two strips drawn are backdrop art only; the moving/localised parts on top of them - a `<TextInfo>` (the tag and the scrolling news text) and a `<NavigationController>`'s button prompts - are elements nothing in `oag_game::screen` recognises, the same gap already named for Wipeout HD's own trial-build text and button prompts on `FE Screen`.

**2026-09-25: the button-prompt line is built, on every ordinary menu page, both PSP titles' shared root plus Wipeout HD/Fury's own - the ticker still is not.** `oag_ui_screens::campaign::footer::NavigationLegend` (already reading Pulse's `Skin.xml` for `Cell Selection`, see `the-campaign-grid-draws-and-does-not-launch.md`) is now read once at boot (`boot::screens::read_nav_legend`, off the same raw XML `fury::load` already parses) and carried on `Shell`/`Boot` for both the live session and `--menu-page`. `MenuStage::render` draws it on every ordinary page: `Confirm` unconditionally, `Back` only past the tree's own root (`Menu::depth() > 1`) - **chosen** for this build's own tree (no disc screen to read a gate off), but corroborated rather than guessed: Wipeout HD/Fury's own `NavigationButtons` bitmask, read across its whole front-end XML, and two real RPCS3 captures already in this repo (`data/reference/hd-main-menu-screenshot/`, `data/reference/hd-settings-screenshot/`) both show exactly that shape on the real title - see [menus.md](../../docs/architecture/menus.md#a-mouse-and-a-finger) for the census. Wipeout HD/Fury's own icon half (`font="buttons"`, `ps_buttons.fnt`) draws nothing - this build loads no atlas for that face and would otherwise risk showing the raw codepoint (a Greek letter) instead of the disc's own glyph; the resolved word (`FE_CONFIRM`/`FE_BACK`) still draws. The ticker is unaffected by any of this and stays exactly as unbuilt as it was.

**2026-09-27, `pulse-ticker` lane: the ordinary menus' own footer ticker is built, closing this thread's first `Open` item.** `MenuStage` now carries a `TickerLayout` (`Shell`/`Boot`'s own `ticker` field, read once at boot the same way `nav_legend` is, off the identical `Skin.xml`'s `TextInfoIsAlwaysLast` viewport `screens::read_ticker` mirrors `read_nav_legend` for) and a free-running `ticker_elapsed` clock (`MenuStage::tick`, never reset on a page change - the same choice `CampaignStage::ticker_elapsed` already made and for the same reason). The rotation itself moved rather than duplicated: `CampaignStage::ticker_tips`'s own `TKR_NO*` logic is now `oag_game::records::ticker_tips(strings, store)`, one function three callers agree through - `MenuStage` (`Session::draw`), `CampaignStage` and `capture::menu_page`'s `--menu-page` still - so a live session and a screenshot of it can never show two different tips for one save file. New `crates/game/src/main/menu_stage/footer.rs` holds the draw call and the marquee/ticker clip trade-off (`resolve_clip`, "marquee wins" - documented there, not re-litigated here). Verified against both real PSP pressings, `crates/game/tests/pulse_menu_ticker_ground_truth.rs`: the viewport `[85, 235, 370, 32]` still matches `docs/ui/campaign-screens.md`'s 2026-09-21 measurement, and a fresh save's `TKR_NOTOURN`/`TKR_NOHH` still draw. Screenshotted on PSP and PS2 (`data/scratch/drive-2026-09-27/ticker/`) at player size, both platforms' own viewports, scrolling confirmed via a temporary probe (see `docs/ui/campaign-screens.md`'s own 2026-09-27 section for the full account) - **and that same account is also where a real bug this lane introduced, then fixed, is written down**: `--menu-page main`'s still capture drew the ticker unclipped at first (the same defect class `Cell Selection`'s own ticker had on 2026-09-21), overrunning the frame on PSP and the bar on PS2, fixed by threading the same `Renderer::render`-level clip capture never had a slot for before.

## Open
- The easing curve is invented (confidence 30) - the capture only shows that motion accelerates. ~~The selected row's highlight pulse period and depth were never measured~~ - measured 2026-09-05, see below. **2026-09-28: the per-widget fade mechanism it was suspected to share is found and is linear, but a live-measured discrepancy (see below) argues it is not the same curve as the page-level zoom - still open, still invented.**
- **The ticker overdraws its own footer strip on a settings page whose focused row's value is both overflowing and mid-marquee** - `footer::resolve_clip`'s own "marquee wins" choice (not re-litigated by this lane, but the ticker's own overdraw it causes was not previously named as an open item): `Renderer::render`'s clip has one slot, and a focused `OPTIONS` row's overflowing value keeps that slot for as long as the row stays selected and overflowing, not just one frame - the ticker draws unclipped past its own bar for the whole time. Fixing it means either skipping the ticker outright while the marquee holds the slot, or giving `Renderer::render` a second clip slot; neither was attempted this pass.
- **The live session's own ordinary-menu ticker (clock, `resolve_clip`, the `frozen_race` index shift) has never been seen running in a real window** - only code-read and cross-checked against `capture::menu_page`'s still path (which the 2026-09-27 pass did screenshot, both edges, on real PSP and PS2 discs). An attempted live verification under Xvfb (`:92`, separate from another member's `:78`) got real input handling (boot skip, hand-off to `MenuStage`) but no readable pixels - `import`/X11 grabs came back solid black regardless of on-screen content, and forcing `LIBGL_ALWAYS_SOFTWARE=1`/`WGPU_BACKEND=gl` did not change the renderer's own Vulkan choice. No compositor (`picom`/`xcompmgr`) is installed on this machine to work around it. Whoever next has a working interactive Xvfb setup (the same gap `the-campaign-grid-draws-and-does-not-launch.md`'s 2026-09-21 pass named for the campaign screens) should confirm the live picture directly.

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
session, and here is why, for whoever picks it up next.** At the time,
`names.tsv` had zero front-end/menu functions on `psp-pulse-usa` - the closest
anchor was `Xml_AttributeAsFloat` (`0x0895379c`), and its callers were unnamed
`FUN_*` territory with nothing to recognise a tween by. That anchor point is
now stale: `psp-pulse-usa` has front-end/menu-adjacent names as of 2026-09-08
(`TrackSelection`/`TeamSelection`'s screen classes,
[`race-box-screens.md`](../../docs/ghidra/functions/psp-pulse-usa/race-box-screens.md)),
though none of them is `Tween`-related specifically. **The relocation defect
that made a call-graph walk the only viable route is also fully resolved** -
the 2026-09-07 patch fixed data loads too, not just `jal` calls, so
`get_xrefs_to`/`get_function_callers` should work directly on both now rather
than needing the `search_instructions` workaround. `Tween::eased` is
untouched.

**2026-09-28, `pulse-easing` lane: the per-widget fade mechanism is found and
confirmed linear, but it is not this curve, and `Tween::eased` stays
untouched.** Intersecting `lwc1 ...,0x68(a0)` with `lwc1 ...,0x6c(a0)` across
the whole binary (the two transition-duration offsets `Widget_CreateFromElement`
already reads) turns up exactly one function, now named
`Widget_UpdateTransitionFraction` (`0x0888d8e4`) - the base Widget class's
shared per-frame fade tick, called from ~30 different widget kinds' own
`Update`. Both its enable and disable branches are a plain `elapsed / duration`
ramp, no curve at all; confirmed live too, breaking on it across several
thousand hits (`data/scratch/pulse-easing/`): the fraction it writes sat
pinned at exactly `0.0` or `1.0` on every steady-state sample, and `dt` varied
by at most 21 microseconds around `1/30` s, consistent with a genuinely
measured per-frame clock rather than a fixed step. Full account and the two
sibling functions it feeds (a subtree-max "is anything still fading" check
and the destroy-when-settled garbage collector built on it) are in
[race-box-screens.md](../../docs/ghidra/functions/psp-pulse-usa/race-box-screens.md#the-per-widget-fade-is-confirmed-linear-2026-09-28).

**Why this doesn't close the item**: `menus-original.md`'s own measurement -
the outgoing page's alpha at `0.55` eleven presented frames into the transition
- does not match a raw linear disable ramp off this mechanism, which predicts
`0.267` at that frame. Two live-capture attempts to catch the transition's
first frame directly (breaking on `StateMachine_TransitionTo`, `0x0889123c`,
before pressing, so no frame is lost to round-trip latency) did not land a
clean run this pass - an attract-mode idle timer and a stray extra button
press each derailed one attempt, and the session ran out before a third try.
No function that reads or writes a widget's own *scale* was found either.
`Tween::eased`'s `t*t` is left exactly as it was: swapping it for `t` would
trade one unproven curve for another, since linear is now confirmed for a
*sibling* mechanism (the per-widget fade), not for the page-level zoom this
curve actually draws.

## Next Steps

- **Land a clean live capture of the Main Menu -> Grid Selection transition's
  first 3-4 presented frames**, reading each outgoing widget's own `+0x64`
  (`Widget_UpdateTransitionFraction`'s fraction) and `+0xbe` alongside a
  screenshot, to settle whether the alpha discrepancy above comes from a
  delayed branch switch (enable-branch hold before `+0xbe` flips) or from a
  genuinely different, still-unlocated curve. `data/scratch/pulse-easing/capture_transition_v3.py`
  has the trigger-breakpoint scaffolding; it needs the press sent via `dbg.send`
  with no reply-wait (per that file's own notes) and a screenshot step added.
- **Find where scale is computed**, if anywhere separate from the alpha
  fraction - no candidate function was located this pass. `Widget_CreateFromElement`
  reads no scale attribute at all, so it is not a per-widget field the same
  way the fade durations are.
- If neither turns up more, `Tween::eased` stays as documented - invented,
  confidence 30 - and the next lead is whoever finds `+0xbe`'s setter.

## Open, from the 2026-10-08 side-by-side (ranked list in docs/ui/menus-original.md)

- Title-face top bar: Pulse needs `Title` (`Pulse_14.fnt`) and `Default` at once, a third renderer face slot beyond `face_atlas_slot`; then flip `oag_pulse::frontend::MENU_SKIN::title_font`.
- Mixed-case `Default` labels on Track/Ship Select and the pickers' `Confirm`/`Back` and ticker footer.
- Page transition on the campaign and picker pages, and the outgoing-page zoom; fit the easing from the 30-frame burst (`scripts/psp-menu-transition-burst.py`).
- Racebox settings page shape: rules between rows, `<` `>` selector, value beside the label.
- Profile tag in the footer's left strip; `Confirm` label scale against the glyph.
