---
status: measure
titles: [pulse]
updated: 2026-11-02
summary: "Landed: flyby clock, wash, voice-over, SFX fade, ambience, squared effects. Open: tick-0 ~AMB loop, ramp ceiling below slider 100"
categories: [rendering, frontend]
difficulty: medium (30)
---

# The pre-race flyby plays with its panel; its settle and its tail do not

2026-10-01. The maintainer's play report (the original flies the circuit before a race) was
measured and ported: the camera is `start_grid.vex`'s `grid_camera1` animation, not a `Camera`
node, played for `AnimEnd` seconds or until a held Cross, with the world held at its first tick
(so the countdown and every hash are unchanged). Everything measured is in
[race-intro.md](../../docs/gameplay/race-intro.md) and
[the code page](../../docs/ghidra/functions/psp-pulse-usa/race-intro.md); the camera is
`oag_vex::grid_camera` and `oag_raceplay::intro_camera`; `--no-intro` skips it and
`--intro-ticks N --screenshot` photographs a tick of it.

## Open

- ~~The panel floated mid-screen on a taller window~~ **landed 2026-10-21 (`pulse-flyby-panel-anchor`)**:
  top bar flush to the top, bottom bar flush to the bottom, and bars repeat outward on a wider
  window; chosen, not measured. See [race-intro.md](../../docs/gameplay/race-intro.md#the-panel-on-a-window-that-is-not-480272-2026-10-21).
- ~~The track-description panel~~ **landed 2026-10-02**: `InGameTrackDescriptionScreen` read off
  `InGame_Definition.xml`, name = the `PI_Track` id as a table key and paragraph =
  `MSC_TRACK_<nn>` (`RaceManager_Construct`), fade measured per frame (0.7 s linear, a 2 s width
  wipe in, 0.7 s out - not the 30 ticks this said), pointer press skips. See
  [race-intro.md](../../docs/gameplay/race-intro.md#the-track-description-panel). What is left of
  it: the name draws 2-3 % narrower than the original's and the paragraph's wrap width (450) is
  chosen, not measured (line breaks match on `16_Track` and `03_Track`); only Single Race and
  Time Trial were measured; Zone, Eliminator and the others take the same path unwatched.
- ~~**The world does not settle** under the flyby~~ **closed 2026-10-11 (`pulse-flyby-settle`), nothing to
  port**: Pulse's `Ship_UpdateCraft` has no grid-state hover target (HD's `+0x348` clamp has no
  analogue), and the original's craft rises 2.14 from its placement to rest in about 50 ticks while
  ours is placed at rest, so they agree from about 1 s on. The "two units low" was the original's
  pre-settle pose. Only the first second differs (original rises, ours still); see
  [grid-state.md](../../docs/physics/grid-state.md#pulse-has-no-lowered-grid-state-hover-target-2026-10-11-pulse-flyby-settle).
- ~~HD's flyby craft sits 1.84 lower on the original~~ **landed 2026-10-08 (`hd-flyby-hover`)**: the
  original's hover target is `0.75 * min(5.5, entry+0x348)` with `+0x348` a `3.0x` clamp in the grid
  state and a seconds timer from GO, so the craft sits low through the flyby **and the countdown** and
  rises over 2.46 s after GO. Ported as HD `LaunchHover` title data: the spring reads the clamp, written
  per tick from the countdown clock, and the placement height is the lowered spring's rest height, so the
  held world at tick 0 is already low. **This changes the flyby's world-at-tick-0 design for HD only**: the
  placement pose is 1.85 lower than Pulse-style placement, and HD's countdown pose and first 2.5 s of
  racing physics differ from before (the craft rises under the released target). Evidence and the
  ours-against-original table: [hd-ride-height.md](../../docs/physics/hd-ride-height.md), the read:
  [hover-target.md](../../docs/ghidra/functions/ps3-hdfury-eu/hover-target.md). Open: which caller of
  `Craft_SetState(1)` runs at GO (timing measured, caller unnamed); modes that skip the grid branch
  (`0x000f2390`); a respawn's state 0 may re-lower a craft; rivals unmeasured; HD golden hashes that move
  (none found in the affected gate). Pulse's craft update was not read for the same branch.
- ~~The scenery does not animate under the flyby~~ **landed 2026-10-29 (`pulse-flyby-settle`)**:
  `Race::world_clock_seconds` adds `flyby_seconds`; the gantry stays on the race clock (measured:
  it starts at the flyby's end). `scene/frame.rs` is 1,232 lines, under the ceiling, left unsplit.
  Open: clouds still read `World::tick`; no draw alpha during the flyby.
- ~~**The tail**~~ **landed 2026-11-02 (`pulse-flyby-music`)**: the counter's zero starts the
  circuit's `PreRaceDialogue` voice-over on its own voice and the flyby's end stops it
  (`oag_raceplay::intro_camera::IntroCue`, `oag_sound::Audio::start_pre_race_voice`); the `0x28`
  call is an **SFX fade-in** (50 calls at 0.02 from silence, after 20 silent calls), not music -
  [the read](../../docs/ghidra/functions/psp-pulse-usa/race-intro.md#the-music-the-two-counter-calls-and-the-exit-stop-read-2026-11-02-lane-pulse-flyby-music).
  The `0x28` fade **landed 2026-11-02 (`pulse-flyby-sfx`)**: the session runs `sound::race_tick`
  under the flyby, `PreRace::effects_fade_in` silences the Sfx bus on tick 0 and ramps it from tick
  20 ([race-intro.md](../../docs/gameplay/race-intro.md)). **Watched 2026-11-02
  (`pulse-flyby-audio-capture`)**: the fade is exact (silent 20 calls, `+0.02` a call, 50 calls), but
  what it quiets is the circuit ambience, not the engines (the grid's engines are keyed on at tick
  3 at volume 0 and only sound from about tick 1,150), and the gain is the **square** of the fade
  (confidence 85; the whole group volume, slider included, is squared at the commit); see
  [race-intro.md](../../docs/gameplay/race-intro.md#what-the-original-plays-under-the-flyby-watched-2026-11-02-lane-pulse-flyby-audio-capture).
  **Landed 2026-11-02 (`pulse-flyby-ambience`)**: the flyby plays the circuit's placed ambience
  emitters and the craft engines by range, and Pulse's effects bus is `(slider * fade) ** 2`
  (`Title::effects_volume_law`; always in a race, not only the flyby). `03_Track`: first emitter
  at tick 390 (original 393), first audible engine 1,144 (original about 1,150). Not played:
  the original's `~AMB` loop that sounds from its first call (which cue is unread, not an emitter in
  range) and the `~FLYBY_DIST_*` one-shots (no trigger recovered). Open from this: the fade's
  ceiling at a slider below 100 (the original's target is the slider; ours ramps slider times 0-1),
  Speech presumably squared too (not wired), menus outside a race stay linear, Pure inherits the
  square unmeasured.
  Open: no WAV (SDL build has no record action), which cue the one ambience voice is, a circuit
  whose camera opens near the grid, other modes. The
  wash frames (`h0103` to `h0125`) fade faster than the executable's 0.7 s, unresolved whether a
  call is a tick.
- **Demo mode** skips it (`g_game_mode == 2`); a byte at mode object `+0x40` also skips it
  without a held button and is unread. Only Single Race and Time Trial were captured.
- **A fresh menu-walk load on `03_Track` and Metropia reached the intro already in its fade-out
  substate** while RESTART RACE on both played it. Probably the walk's own button presses;
  not chased. `01_Track` (18 s) and `14_Track` (17.97 s) were never watched to their end.
- The field of view (`54.309` vertical degrees) is a measured constant: the leaf's payload is
  identical on all twelve circuits, and no derivation was found.

- **Other titles (2026-10-07)**: Wipeout HD's flyby is ported as `PreRace` Title data (camera loops, a press skips); see
  [race-intro.md](../../docs/gameplay/race-intro.md#other-titles-census-2026-10-07). Open there: HD's `START RACE`
  prompt overlay, HD's own fov/hold/lock/mode file (chosen), Pulse PS2 (same files, watch it on PCSX2 then widen
  `PRE_RACE.on`), 2048 (needs `AnimEnd`-less reading, watch on Vita3K), Pure's `StartSequenceCamera` (code camera,
  unread), HD Zone (`zone_N/start_grid.vex` has only `camera1_group`).

## Next Steps

1. ~~The SFX fade-in at the counter's `0x28`~~ landed; ~~what is audible~~ measured and ~~wired~~
   2026-11-02. Next: identify the tick-0 `~AMB` loop's cue, measure the fade's ceiling at a slider
   below 100 and Speech's law (a capture lane).
2. ~~Pulse's settle~~ closed (see Open). Optional: a view-side rise of the drawn pose over the first ~50
   flyby ticks, if the flyby camera shows the craft then (chosen, not measured).
3. Re-measure one of the short circuits (`01_Track`) through to its end, and a fresh-load
   Zone or Eliminator race.
