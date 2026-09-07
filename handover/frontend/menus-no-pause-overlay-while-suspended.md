# Menus: no pause overlay while a race is suspended

The shell navigates and every row is live ([menus.md](../../docs/architecture/menus.md)),
including rebinding - see below.

**Escaping a race now suspends it rather than discarding it, as long as it has
not finished.** `Session::open_menus` parks the outgoing `Stage::Race` in the
new `Session::suspended_race` instead of letting the stage swap drop it, and
backing all the way out of the menus (`MenuEvent::Closed` at the root page,
with nothing behind it in the tree) reaches `Session::resume_race` in place of
quitting - the fourth landing of `escape`'s own "one rule, three places it
lands" rule, documented on `Session::escape` itself. A race that *has*
finished is still discarded on the way to the results table: there is no
track behind that screen to resume into. See
`crates/game/src/main/session/menus.rs`.

**Rebinding landed** (this thread's original open item, formerly titled "the
one thing that does not work"): `oag_input::bindings::Bindings` replaces the
hardcoded `oag_input::keys::map_key` match with a permutation over its closed
eighteen-key candidate set, persisted complete in `[controls] bindings`, and
`Session::maybe_begin_binding` / `crate::rebind::decide` (in
`crates/game/src/main/`) wire a `binding` row's confirm press into a raw-key
capture. Full account, including the three design choices (table location,
conflict-is-a-steal, unknown-entry fallback) and what it deliberately does
not cover - the original's own `Control_Type = "custom"` action-to-button
mapping - in `docs/architecture/menus.md`'s new "Rebinding" section.

**The rough edge closed (2026-09-07): confirming a `binding` row now says a
capture is open.** `Session::draw` resolves `OAG_BINDING_CAPTURE_PROMPT` off
`Session::awaiting_binding` and hands it to `MenuStage::render` as one more
parameter - the same seam `bound_keys` and `frozen_race` already cross - and
that draws it last, over everything, with `oag_game::prompt::message_draw`: a
small third shape alongside `Keyboard` and `Confirm`, drawing a status line
with no input model of its own since the raw key that resolves a capture is
decided upstream of the whole crate. Landed in `main/` exactly as this
thread's own Next Step named: `menu_stage.rs` and `prompt.rs` both gained a
few lines, `crates/game/src/menu.rs` (the one actually at its
`scripts/check-file-size.py` `BASELINE` ceiling) was not touched at all. See
`docs/architecture/menus.md`'s Rebinding section for the full account.

**The pause overlay landed (2026-09-05).** Backing into the menus over a
parked race now renders that race's own scene into the frame - the same
`RaceStage::render` call `Stage::Race` itself makes, from inside
`Session::draw` - and `MenuStage::render` draws over the resolved result
with `LoadOp::Load` and a translucent `Draw::Fill` instead of its usual
black clear, leaving the disc's own looping backdrop out for as long as a
race is parked. See `docs/architecture/menus.md`'s "and backing into the
menus..." paragraph for the full account and `crates/game/src/main/menu_stage.rs`'s
`PAUSE_OVERLAY` for the tint, which is chosen rather than authored - neither
PSP title's front-end XML defines a pause screen, and the one HUD string
that names pausing (`IG_PAUSE_QUIT`) is drawn nowhere. Verified headlessly:
`render::tests::a_translucent_fill_blends_over_whatever_load_kept` in
`crates/game/src/render/tests.rs` pins the new `render_with(LoadOp::Load,
..)` + translucent-fill mechanism directly - confirmed sensitive by
temporarily swapping in `LoadOp::Clear` and watching it fail. This also
pushed `session/frame.rs` past the 1,000-line size-gate ceiling, split into
`Session::draw` in the new `session/draw.rs`.

One gap in what landed is real but invisible on both titles this build
actually plays:

- **A title whose frame authors a `<ScreenClear>`, or whose `MenuSkin`
  carries a `background` (only Pure's does, per `Skin::background`'s own
  doc), still hides the parked race outright rather than dimming it.**
  `menu::draw_list` puts that clear in as an opaque `Draw::Fill` ahead of
  everything else regardless of `frozen_race`, and nothing in the pause
  overlay's own code taught it to skip that layer. Both PSP titles' frames
  are unread and neither authors a `MenuSkin::background`, so this cannot
  fire today - it would need a title whose frame *is* read (HD's is, per
  menus.md's "the page is drawn inside the disc's own frame") to actually
  suspend a race and show it.

**Verified headlessly, both states (2026-09-07).** `--menu-page controls
--screenshot` for the capture-closed page and `--menu-page controls
--menu-prompt binding --screenshot` for capture-open - the second flag's new
`"binding"` arm in `crate::capture::menu_page::prompt_draws` calls the exact
`message_draw` function the live path does, off the first `binding` row the
CONTROLS page opens on (`THRUST`, so the captured shot reads "PRESS A KEY FOR
CROSS - ESCAPE CANCELS"). Both screenshots are real pictures, not black: the
scrim dims every row legibly and the two-line prompt is centred and readable
over it, including the BRAKE row's now-three-key value
(`BACKSPACE / Z / Y`, off `Y` joining `Z` on Circle) neither overlapping the
label column nor running off the value column's own right edge.
`cargo nextest run -p oag-game` (1064 tests) and the full `just` gate both
pass; nothing here is a game-data test, so `just test-data` was not run.

## Open

- The pause overlay does not suppress a title's own `<ScreenClear>` /
  `MenuSkin::background` fill, so a title that authors one would show an
  opaque menu background over the parked race rather than a dimmed picture
  of it

## Next Steps

None currently open. The `<ScreenClear>`/`MenuSkin::background` gap above has
no actionable next step of its own yet: it cannot fire on either title this
build plays today - reproducing it needs a live run against a title whose
`MenuSkin` carries a `background` or whose frame authors a `<ScreenClear>`,
which neither Pulse nor Pure does. Revisit if a third title's own menus are
wired up and turn out to author either.
