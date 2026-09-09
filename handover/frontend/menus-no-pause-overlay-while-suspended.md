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

## The pause overlay did not actually show, and now does (2026-09-09)

**Found and fixed in one pass, on a live run - not read off the source.**
Every prior "landed"/"verified" note above this section was checked against
either an isolated render primitive
(`render::tests::a_translucent_fill_blends_over_whatever_load_kept`) or the
unrelated CONTROLS-page binding prompt; nothing had ever driven a real
`Escape` from a real running race and looked at the frame. No headless
capture path can reach that state: `--race` (`main.rs`'s `cli.race` branch)
always takes the headless `run_race` leg regardless of `--screenshot`, and
`--menu-page` builds a `menu::Menu` from scratch with no `Session` and no
parked race behind it. So this pass drove it live instead - Xvfb, real
keyboard input, a real physical `Escape` key - and the picture on screen was
the ordinary undimmed root menu, not the parked race.

**In-process instrumentation (three sites, `session/draw.rs` and
`menu_stage.rs`) proved the state machine itself was already entirely
correct**: `suspended_race` genuinely `Some`, `frozen_race` genuinely `true`
on every frame, the `PAUSE_OVERLAY` `Draw::Fill` genuinely in the draw list
at a non-degenerate full-space rect, no `Draw::Video`, `LoadOp::Load`
genuinely used. And the presented picture was still pixel-indistinguishable
from the ordinary, un-parked menu.

**The actual cause: a full-screen mark drawn unconditionally, on top of the
overlay.** Dumping every `Draw` in the list found `Draw::Sprite
rect=[0.0, 0.0, 480.0, 272.0]` sitting right after the overlay `Fill`, in
both the frozen and the ordinary case - and it traced to `frame.marks[0]`,
one of Pulse's own `FE_SCREEN` images, read at boot because
`crates/pulse/src/lib.rs`'s `menu_frame: Some(FE_SCREEN)` **is** set.
`docs/architecture/menus.md`'s "Both PSP titles' frames are unread" (still
true when the "the page is drawn inside the disc's own frame" paragraph was
written) is now stale - flagged there, not edited, since that file is
`docs/architecture/`, not this thread's `docs/frontend/` lane. `draw_list`
drew every one of `frame.marks` regardless of `frozen_race`, so this one
full-screen image painted straight over both the overlay and the parked
race underneath it, every frame - the same shape as the `<ScreenClear>` /
`MenuSkin::background` gap already named below, except reachable on Pulse
itself through an ordinary `<Image>` widget, not a `<ScreenClear>`.

**The fix: `Frame::backdrops`** (`crates/game/src/menu/frame.rs`) assembles
the clear/video/marks layer `draw_list` used to build inline, and drops
`clear`/`MenuSkin::background` outright and any mark whose rect covers the
whole screen when `race_behind` - while keeping small structural marks
(Pulse's own top bar and its two footer strips) exactly as before. This
closes the `<ScreenClear>`/`MenuSkin::background` line below too, in the
same fix - the two are the same problem at two different widget types.
`draw_list` takes the new `race_behind: bool`; its three real call sites
(`MenuStage::render`, the page-change snapshot in `session/frame.rs`,
`--menu-page`'s capture path) and eleven test call sites pass it through.
Landed as `bb7ef573`.

Two GPU-free tests in `menu/tests/frame.rs` assert the drop/keep split
directly against fixture `Frame`s. Live-reproduced both before and after
under Xvfb with the recipe below: before, the plain undimmed menu; after,
the dimmed track and HUD clearly visible behind the rows. Full `just` gate
green, including `race_ground_truth::a_lone_craft_gets_round_the_circuits_
it_is_known_to_get_round` re-run under `OAG_REQUIRE_GAME_DATA=1` (this fix
touches no physics or AI).

**Repro recipe**, since none of this project's own test/capture
infrastructure reaches this state - useful for verifying the fix on real
hardware, since this pass only had Xvfb/llvmpipe software rendering to check
it against:

```sh
Xvfb :99 -screen 0 1440x816x24 &
unset WAYLAND_DISPLAY   # winit prefers Wayland when both are set; this
                        # aims the window at Xvfb's X11 instead - see the
                        # "Independently verified on a live capture,
                        # 2026-08-19" paragraph above.
DISPLAY=:99 cargo run -p oag-game --features native-video -- \
    data/images/pulse-psp-eu.chd &
# No window manager runs on Xvfb, so X input focus never lands on the
# window on its own - one explicit XSetInputFocus (raw Xlib, python-xlib
# is enough) before any xdotool key reaches it. Then, all through
# `DISPLAY=:99 xdotool key --window <id> ...`:
#   space              # skip the intro
#   Return             # accept the default language
#   space              # START at the title screen, opens this build's menus
#   Return             # RACE row
#   Down x6, Return     # down to the RACE page's START row, launch it
#   (wait for the loading screen to clear and the race to be visibly running)
#   Escape             # the real physical key, not a mapped abstract button -
#                      # Session::escape's "in a race, it hands the window
#                      # back to the menus" landing
DISPLAY=:99 import -window root shot.png
```

## Open

- The `<ScreenClear>`/`MenuSkin::background` case the fix above also closes
  is now verified fixed only for the `<Image>`-mark shape Pulse actually
  authors. No title this build plays authors a `<ScreenClear>` or a
  `MenuSkin::background` to exercise that half live - `Frame::backdrops`'s
  own unit test covers it directly instead (`backdrops_drops_the_clear_
  behind_a_race_too`), which is why it is listed as covered rather than
  reproduced on a real title.

## Next Steps

None currently open.
