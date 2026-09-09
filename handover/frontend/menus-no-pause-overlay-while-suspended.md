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

- **A player who actually pauses does not see the dimmed race - they see the
  ordinary undimmed menu backdrop, exactly as if nothing were parked at all
  (2026-09-09).** This contradicts the "landed 2026-09-05" claim above: that
  entry's own "Verified headlessly" evidence is `render::tests::
  a_translucent_fill_blends_over_whatever_load_kept`, which pins the
  `LoadOp::Load` + translucent-`Draw::Fill` **primitive** in isolation, and
  the `--menu-page --menu-prompt binding` screenshots, which are the
  *rebinding* prompt, not this overlay - **nothing in this project's test
  suite or prior verification actually drove a live `Escape` from a running
  race and looked at the frame**, because no headless capture path can reach
  it: `--race` (`main.rs`, `cli.race` branch) always takes the headless
  `run_race` leg regardless of `--screenshot`, and `--menu-page` builds a
  `menu::Menu` from scratch with no `Session` and no parked race behind it
  (`crate::capture::menu_page::menu_page` calls `menu::draw_list` directly,
  never `MenuStage::render`, so `frozen_race` cannot be exercised that way
  either). The only way to reach `Session::escape` while `Stage::Race` holds
  something is a real windowed run with real input.

  **Reproduced live, with a GPU window, real keyboard input and in-process
  instrumentation** (worktree `worktree-pause`, 2026-09-09) - recipe below.
  Instrumented three sites across the two files this thread owns and none
  outside them:

  - `session/draw.rs`, where `has_scene` is computed: confirmed
    `suspended_race.is_some()=true`, `stage_is_menu=true`, `has_scene=true`,
    on every one of dozens of consecutive frames after a real `Escape` from a
    running race, and `reconstruction=Off` on **both** the live-race and the
    parked-menu frames (ruling out an FSR3-to-FSR1 `resolve_scene` fallback
    rung as the cause - it is off on this run regardless of stage).
  - `menu_stage.rs`, where `frozen_race` arrives: confirmed `true`, every
    frame, and `overlay_rect(...)=[0.0, 0.0, 480.0, 272.0]` against
    `viewport=(0.0, 0.0, 1440.0, 816.0)` - the full skin space, not a
    degenerate or off-screen rect.
  - `menu_stage.rs`, right before `render_with`: confirmed the draw list
    (`list.len()=10`) carries the `PAUSE_OVERLAY`-coloured `Draw::Fill`
    (`has_pause_fill=true`), carries **no** `Draw::Video` (`has_video=false`,
    so the disc backdrop movie is correctly left out of the list), and
    `load_is_load=true` (`LoadOp::Load`, not a clear).

  **Every value this thread's own code computes is exactly what the design
  doc says it should be, on every single frame, and the presented picture
  still does not show it.** The picture that actually reaches the window is
  pixel-indistinguishable from the ordinary, undimmed, un-parked root menu
  (`compare -metric AE` on an earlier pass of this same repro:  202 of
  1,175,040 pixels differ, i.e. a different point in the same looping movie,
  not a different picture) - meaning either `resolve_scene` is not actually
  carrying the parked race's scene into the presentation target this frame,
  or something between that and `Framebuffer::composite` is showing stale
  content from before the race ever launched, despite `render_with` being
  called with the right list and the right `LoadOp` against that same
  target. That is downstream of every file this thread owns:
  `Framebuffer::resolve_scene`/`composite` live in `crates/game/src/
  upscale.rs`, and the scene draw itself is `race::Scene::render` in
  `crates/game/src/race.rs` - both out of this thread's lane (`race.rs`/
  `race/` explicitly so; `upscale.rs` is unclaimed but is rendering-pipeline
  code, not menu/session code). Per this thread's own brief: reported here
  rather than fixed.

  **`docs/architecture/menus.md`'s "And backing into the menus over a parked
  race now draws it, dimmed..." paragraph (around line 679) is currently
  wrong** for the same reason - it was written against the isolated render
  test, not a live run. Out of this thread's lane (`docs/architecture/`, not
  `docs/frontend/`) - flagged, not edited.

  **Repro recipe**, since none of this project's own test/capture
  infrastructure reaches this state:

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

  Screenshots and the full instrumented log from the pass above are at
  `/home/topaxi/oag-scratch/1-race-running.png`,
  `/home/topaxi/oag-scratch/2-post-escape.png` and
  `/home/topaxi/oag-scratch/app.log` (grep `DEBUG`) - not committed, since
  `handover/` may not be linked into from outside itself and these are not
  reproducible build artifacts anyway.

- The pause overlay does not suppress a title's own `<ScreenClear>` /
  `MenuSkin::background` fill, so a title that authors one would show an
  opaque menu background over the parked race rather than a dimmed picture
  of it. Still real, still moot on both titles this build plays - unchanged
  from the entry below.

## Next Steps

- **Find why the parked race's resolved scene does not reach the
  presentation target a real `Escape` composites from**, even though every
  value this thread's own code (`session/draw.rs`, `menu_stage.rs`) computes
  for that frame is correct - see the instrumentation above. The search
  starts in `Framebuffer::resolve_scene`/`composite` (`crates/game/src/
  upscale.rs`) and `race::Scene::render` (`crates/game/src/race.rs`), neither
  of which this thread owns. Whoever picks this up should re-run the repro
  recipe above with a GPU and a real display (this sandbox's Xvfb/llvmpipe
  path proved the *state machine* correct but cannot itself rule out a
  software-renderer artifact in the compositor step - the coordinator should
  verify on real hardware before assuming the same fault reproduces there).
- The `<ScreenClear>`/`MenuSkin::background` gap two entries up still has no
  actionable next step: unreachable on Pulse or Pure, revisit if a third
  title's menus get wired up.
