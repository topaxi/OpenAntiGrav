# Before the race: the circuit flies itself

**Status: measured on Pulse PSP, 2026-10-01 (Single Race and Time Trial; `16_Track`,
`03_Track`, and Metropia by RESTART RACE; ten capture runs).** The camera, its length, its
cuts, its skip and its hand-over to the countdown are ported. The track-description panel is drawn
over it, with its measured fade (below), the circuit's voice-over plays under it and its end
starts the screen wash. The world settling under it and the SFX fade-in are not (see
[Open](#open)).

The maintainer's play report: before a race the original flies the circuit and ours went
straight to the grid. The discriminating question was whether the camera starts on the grid
view (flyby falsified for Pulse) or flies something. It **did not fall**, and the answer was
not either candidate in the thread: it is neither the circuit's `Camera` nodes (the destroy
camera's, the post-race director's) nor a rule along the track spline. It is **one authored
camera animation per circuit**, in a file nothing here had read for a camera.

Code behind each row is
[`race-intro.md`](../ghidra/functions/psp-pulse-usa/race-intro.md); the pose reader is
[`oag_vex::grid_camera`](../../crates/vex/src/grid_camera.rs), the clock and the hand-over
[`oag_raceplay::intro_camera`](../../crates/raceplay/src/intro_camera.rs).

## What the original does

Every circuit directory holds a `start_grid.vex`: a `World`, two `Anim Transform` nodes
(`camera1_group`, `grid_camera1`) and one `gridCamera` leaf, a Maya camera scene exported
as-is. `grid_camera1`'s translation and rotation keys (9 to 230 and 112 to 215 of them) are the
flyby. The render view is the leaf's world matrix, **to `3e-4` world units over the whole 25
seconds** once one thing is allowed for: the published pose is built from the animation's local
matrix as of the **previous** frame (against the same frame the median error is 0.74 units,
one frame back it is `5e-5`).

`t` is ticks since the race scene began updating (the first `RaceMode_UpdateIntro` call that
reaches the camera pass).

| `t` | What happens | Seen |
| --- | --- | --- |
| `0` | the front end reads `InGameTrackDescriptionScreen`; the picture is the circuit from `grid_camera1`'s first frame; no HUD; the race clock reads `0.0` and the mode state `0`. A counter (`mode+0x1a04`) starts at 60 | `*` four captures |
| `0 .. ~28` | the animation holds its first frame: the camera node waits `1.0` s of its own clock, which advances two `dt` per tick | `*` three runs: the animation clock first reads non-zero at tick 29 each time (counter-referred), so 28 held ticks |
| `~28 ..` | the animation plays at the tick rate | `*` over 1,521 frames: clock against pose |
| `>= 60` | the flyby **may end**: the counter has reached zero. Before that nothing can end it | `*` counter `59, 58, ... 0` |
| the end | it ends when the animation reaches `AnimEnd` (key units, so `1500` is 25.0 s), **or** when **Cross is held** (`Input_IsHeld(5)`) once the animation has started and the counter is zero | the first: last frame at `24.99` s of `25.0` on two circuits; the second: a 4-frame press ended it on the next frame, and the old "dismissed by a held cross at tick 61" is this |
| `end` | `RaceMode_UpdateIntro` leaves its camera substate. The track-description panel fades out over **42 ticks (0.7 s)** with the chase view behind it (the 30 ticks are the substate's wait, not the fade); **no HUD** until that ends; then the old countdown: **272 ticks** from `end` to the throttle stepping, the same as when a press skips it | `*` substates 2, 3, 4 at `+0`, `+30`, `+31` and `+92`, the countdown at `+272`; HUD absent at +30, present at +60 |

`AnimEnd` is per circuit: `1500` (25.0 s) on nine of the twelve, `1620` (27.0 s) on
`02_Track`, `1080` (18.0 s) on `01_Track` and `1078` on `14_Track`. Only the two 25 s ones were
watched to the end.

The cuts are the translation channel's one-frame key pairs (`360`/`361`, `720`/`721`,
`1080`/`1081` on `16_Track`): the three large jumps the capture saw land on exactly those.

The world **does** tick during all of this: the player's craft hover-settles over about 40
ticks (`y` `-51.74` to `-49.58`, two units) and then sits for the other 25 seconds. The race
clock, the lap clock, the countdown and the gantry do not start until `end`.

**RESTART RACE plays it again** (measured on `16_Track`, `03_Track` and Metropia). A fresh load
from the menu walk of `psp-drive.py` played it on `16_Track` and not on `03_Track` or Metropia:
the first call of the intro that the debugger could reach was already in its fade-out
substate with the counter at zero, so something ended it before the hook was armed. The walk's
own button presses are the likeliest cause and it was not chased.

## The track-description panel

The panel over the flyby is `InGameTrackDescriptionScreen` (`InGame_Definition.xml`): a 480x40
bar at the top and a 480x72 bar at the bottom, each a black `0x9f` fill under a hexagon tile
(`hex_bg.mip`) with a gradient rule on its inner edge, the circuit's name at the top in the
`Menu` face (scale 1.2) and its paragraph at the bottom in the `Default` face. Every number
is the disc's; the strings are looked up, not authored. The name is the circuit's `PI_Track` id
as a table key (`16_Track` is Talon's Junction White) and the paragraph is `MSC_TRACK_<nn>`, from
the format `MSC_TRACK_%.2s` in `RaceManager_Construct`
([code page](../ghidra/functions/psp-pulse-usa/race-intro.md#the-track-description-panel-which-strings-and-how-it-fades)).
A reversed circuit is its own `PI_Track` (`32_Track` is Talon's Junction Black) with its own
paragraph.

Timing, measured per frame on PPSSPP (`16_Track`, 2026-10-02):

| Part | Enters | Leaves |
| --- | --- | --- |
| bars, tile, rules, both texts | alpha rises linearly over 0.7 s from the screen's first frame | falls linearly over 0.7 s (42 ticks) from the tick the flyby ends |
| the texts' viewport | its width opens linearly over 2.0 s, left to right, so the paragraph is **written in across the first two seconds** | closes over 0.7 s, starting from full width whatever it had reached |

So a player sees the title appear letter by letter from the left, the paragraph run in behind it,
and both settle at 2 s; a Cross held at 1 s (the earliest the lock allows) leaves from full width.

## What ours does

| Rule | Where |
| --- | --- |
| the loader reads `<circuit>\start_grid.vex` on Pulse PSP and reports its length and cuts | `race/load/intro.rs` |
| the windowed session begins it when the race scene is handed over and steps it **instead of the world** until it ends: `World::tick` stays `0`, so every golden hash, replay and the 272-tick countdown are exactly as before | `main/session/frame.rs`, `Race::tick_intro` |
| the pose is the animation as of the previous tick, fov `54.309` vertical degrees (measured, both circuits, not derivable from the file: all twelve `gridCamera` payloads are identical but for the aim point) | `oag_vex::grid_camera`, `race::intro_camera` |
| the hold is 28 ticks, the lock 60, the end `AnimEnd` or Cross held | `race::intro_camera::Timeline` |
| the HUD is hidden through it and for 30 ticks after | `Race::hud_shown` |
| in a multi-window race every window that holds a seat plays it full window and switches to its own seats when it ends (view-side, **chosen, not measured**; see [multi-window.md](../rendering/multi-window.md)) | `Race::window_drawn` |
| the panel: read at race load (`race/load/intro.rs`: the circuit's `PI_Track` id off its directory and `Reversed`, the two strings off the language table, `hex_bg.mip`, the `Menu` and `Default` faces), laid out by `oag_ui_screens::track_panel`, drawn by `oag_game::track_panel::Overlay` in `RaceStage::draw_hud` and in `race/capture.rs`. Its clock is `Race::track_panel_progress`: the flyby's ticks, then the world's ticks after it | `track_panel.rs` |
| a pointer press skips the flyby as a held Cross does, the moment the lock lifts (a tap is one tick, the lock is 60, so the request is kept until it can act: **chosen, not measured**; the original has no pointer. Checked in a window: a click 3 s in ended it at tick 161) | `Race::skip_intro`, `session/frame.rs` |
| each one-frame key pair, and the end, count as a cut: the temporal upscaler drops its history, and **the motion blur's previous view becomes the post-cut camera** (it also now does so for the destroy and spectator cameras and a respawn, which smeared before). The blur's snapshot is keyed on `Race::motion_tick`, the world's tick plus the flyby's, because the world sits at tick 0 under it | `Race::camera_cuts`, `scene/motion.rs` |
| the player's own craft is drawn on the grid whatever view they saved | `Race::draws_own_ship` |
| the flyby begins when the race scene is handed over, before the first frame is drawn; a race resumed from the menus does not replay it; ours has no RESTART RACE row (a relaunch builds a new race) | `session/load.rs` |
| `--no-intro` skips it; `--intro-ticks N --screenshot` photographs tick `N` of it | CLI |
| a headless run, a capture and every test drive `Race::tick` from tick 0 and never see it | by construction |

Run in a window (lavapipe under Xvfb, `--race`): it logs `pre-race flyby: begins`, plays the
`16_Track` shots, and logs `ends after 1529 tick(s)` (28 + 1500 + 1) with the HUD back and the
race running; with the X key held from partway through it logs `ends after 1490 tick(s), Cross
held`. `--hold cross` does not skip it in a window: that flag feeds the headless loop only.

Compared with the original at five matched times on `03_Track` (`scripts/psp-flyby.py` frames
against `--intro-ticks` captures, native 480x272): the same buildings, framing and field at all
five, judged by eye (`cmp-all.png`, scratch). What differs: the craft has
settled in the original and sits at its placement pose in ours (4.0 against 1.85 above the track on
`03_Track`), and the original's scenery animates under the flyby while ours held its first
frame (**fixed 2026-10-29, `pulse-flyby-settle`**: the world meshes' clock adds the flyby's seconds, `Race::flyby_seconds`; the gantry's does not, the original starts it at the flyby's end. Talon's Junction at `--intro-ticks 1200` differs in 128,614 pixels against the old build: its junction girders and lamps are elsewhere; Moa Therma only its stand crowds. The clock steps at 60 Hz with no draw alpha during the flyby. Clouds still read `World::tick`: unmeasured. PS2's flyby is not on, `PRE_RACE.on`; it inherits when widened).

The flyby's end starts `ScreenFlash` kind 10 (0.7 s, white, alpha 1 to 0, the executable's keys, additive), wired 2026-10-29. Open: the 2026-10-02 frames fade faster than that (half by two calls, clear by about the 17th); `--intro-ticks 1529 --ticks 17` still shows about half. Whether a call is a tick is unmeasured, so the authored kind stands.

The panel was compared at the sizes a player sees (480x272), `--intro-ticks N --screenshot` against
the original's frames: `03_Track` at tick 300 (identical breaks, bars and positions), `16_Track`
at ticks 30 to 120 and 1000 (the wipe reaching the paragraph's right edge at 2 s) and at 0 to 41
ticks after the flyby ends (`--intro-ticks 1529 --ticks N`, against the original's calls 112, 122
and 132): the bars thin out together, the title and paragraph are cut from the right as the
viewport closes, and the HUD arrives over a panel still a third visible. Scratch:
`{montage-enter,montage-exit,cmp-moa,cmp-exit,cmp-bottom}.png`.

### The panel on a window that is not 480:272 (2026-10-21)

**Chosen, not measured** (the original ran at one aspect, so no confidence score). On a window
taller than the authored shape the top bar (name) is flush to the top edge and the bottom bar
(paragraph) to the bottom edge, each drawn against a 480:272 viewport at that edge. On a wider
window the bars extend to the window's edges: `hex_bg.mip` is a 32x16 tile already drawn as a
repeating `TiledSprite` (16 units wide here), so the black fill and the tile repeat outward in
whole tiles (the hexagon phase holds), each half of the white rule stretches to the new edge, and
the name and paragraph stay where the disc put them. At 480:272 the result is byte-identical to
before. Code: `DrawLists::extend_width`, `frame_bottom` and `Overlay::draw`. Checked with
`--race --intro-ticks 90` under `aspect = "free"` at 1080x1440 and 1024x768 (flush), 2560x1080
(repeating) and 480x272 (unchanged); with the default `psp` aspect the viewport is already the
authored shape and nothing changes. Pulse PSP and PS2 share this one definition; the panel is read
off a PSP disc only, and HD, Omega and 2048 do not use this path.

## Other titles (census 2026-10-07)

Every title but Pure ships a `start_grid.vex` per circuit with a `grid_camera1` animation and a
`gridCamera` leaf; the original **plays** it where it was watched. The title's numbers are
[`oag_title::pre_race::PreRace`](../../crates/title/src/pre_race.rs), each carrying its origin.

| Title / platform | Camera file | Original plays it | Ours |
| --- | --- | --- | --- |
| Pulse PSP | `start_grid.vex`, 17.7 to 27 s | measured (above) | ported |
| Pulse PS2 | same names and reader (`16_Track`: 2,400 B, 25.00 s, 5 cuts); executable carries `%s\start_grid.vex` | **not watched** (PCSX2 run not done) | not wired: `PRE_RACE.on` is PSP only |
| Pure PSP | **none**: `track.vex` hashes resolve on `01_Vineta_K`, `04_Chenghou_Project`, `10_Sebenco_Climb`, `start_grid.vex` and `_reversed` do not; only `Data\Defaults\start_grid_1.vex` (confidence 90 on a hash census, not a full listing). The executable has `be::StartSequenceCamera` and a GridCamera importer but no per-circuit path | not watched; a code camera (`StartSequenceCamera`) may play, unread | none, and no invented path |
| HD / Fury PS3 | `start_grid.vex`, `_reversed`, `_ta`, `_de` per circuit (110 to 200 s, 8 circuits plus Fury's); `zone_N/start_grid.vex` has no `grid_camera1` | **measured**, RPCS3, Talons Junction: after the loading screen a camera flies the grid (craft in their slots, no HUD), cuts between shots, `START RACE` prompt strip at the bottom; a 230 s hold shows it **loops** at the file's 188.33 s (shot 19 repeats shot 1) and never ends by itself; one tap of cross skips it (`docs/rendering/start-gantry.md`) | camera ported, loops, press skips; **prompt not drawn** (below) |
| HD PSN | the same files (same engine) | not watched separately | same rule, `Platform::Ps3` |
| 2048 Vita | `data/art/published/environments/<n>/start_grid.vex`; `grid_camera1` has `LoopEnd` (2300 on `altima`) but mostly no `AnimEnd`, `square` has 3000 (50 s); `defaults/start_grid_1..3` hold eight 5 s `gridCamera`s each (grid slots, not a flyby, unread) | **not watched** | not wired; `GridCamera::read` needs `AnimEnd` |
| Omega PS4 | HD's circuits: HD's files byte-for-byte shape (Vineta K 173.33 s); 2048's: 2048's | **not checkable** here (no PS4 emulator) | not wired |

**Lineage check (2048 against Omega).** *Checked, applies, not wired.* Omega's 2048 circuits ship the
same `start_grid.vex` shape as 2048's; Omega's HD circuits ship HD's. Neither title's flyby was
watched, so neither is turned on (an unwatched title inherits Pulse's law only where the project
has decided it should, and here the end rule and skip differ between Pulse and HD).

**HD's numbers.** Measured: plays after the load, loops, no HUD, one press skips. **Chosen, not
measured**: the field of view (Pulse's `54.309`, the same Maya export, compared by eye against
the original at matched circuit position and found to frame the same buildings), the hold and the lock
(0 ticks), the HUD delay (0), and plain `start_grid.vex` (the executable builds `start_grid_de` and
`start_grid_ta` too; which mode takes which was not watched).

**HD's eight numbered circuits were upside down until 2026-10-07**, and Talons Junction was not.
Those eight (the Pulse lineage: Vineta K, 02, 03, Chenghou, Ubermall, Sebenco, Sol 2, Anulpha)
author the camera's rotation as whole `f32` quaternions, the four HD-original circuits as `s16`
keys. The reader took the four floats as `(x, y, z, w)`; they are `(w, x, y, z)` (measured against
each `gridCamera` leaf's own aim point, `docs/rendering/scenery-animation.md`). The earlier
comparison framed only "the same corridor" on Talons Junction, which is why it passed. Pulse's
`s16` path is untouched: four frames of `03_Track` (ticks 100, 400, 900, 1200) are
byte-identical before and after. Omega: **not checkable** (the extracted `start_grid*.vex` blobs do not open as `.vex`); 2048 not checked. Open: HD's `zone_N` flyby has no
`grid_camera1` at all.

**HD's prompt is open.** `InGame` in `ingame_definition.xml` authors it: two 3 px rules (`y 970`,
`1030`, alpha `0x7f`, slid in from `1.0 s` and `0.5 s`), `FE_CONFIRM_BUTTON` in the `buttons` face
at `(80, 1000)` and `FE_START_RACE` at `(130, 997)` from `1.0 s`, and the `demo_logo_HD` mark at
`(1348, 973)`, in a 1920x1080 grid. The parser already yields those widgets (`oag_ui::screen`); no
overlay draws them for HD yet.

## Open

- **The panel's three small differences** from the original, on `16_Track` and `03_Track` (line
  breaks identical): the name draws 2-3 % narrower than the original's (the original's glyph
  advance is not read; the paragraph's matches); the paragraph's wrap width is **chosen, not
  measured** (450, the viewport less the text's inset both sides; the XML authors none); and the
  leave starts on the tick the flyby ends where the original's starts 1 or 2 ticks after its
  substate changes.
- **The original fades the opening second in from white**; ours does not (measured 2026-11-07,
  frames 6 to 45 of a capture, every circuit).
- **The world does not settle** under the flyby, by choice: ticking it would move every
  hash. Not a visible gap after the first second: the original's craft is placed 2.14 below rest
  and rises to rest by about frame 50, ours is placed at rest, so both sit level from then on
  ([grid-state.md](../physics/grid-state.md#pulse-has-no-lowered-grid-state-hover-target-2026-10-11-pulse-flyby-settle)).
- **The music** (ported 2026-11-02): the flyby has its own - `MusicPlayer_StartPreRaceDialogue`
  starts the circuit's `Data\Sound\PreRaceDialogue\<n>_Track.at3` voice-over as the lock counter
  reaches zero (tick 60 of the flyby), and the flyby's end stops the music player
  ([the call sites](../ghidra/functions/psp-pulse-usa/race-intro.md#the-music-the-two-counter-calls-and-the-exit-stop-read-2026-11-02-lane-pulse-flyby-music)).
  The race song is not touched: it waits for `go`.
- **The sound effects** (ported 2026-11-02, lane `pulse-flyby-sfx`): substate 0 snaps the effects
  fade to silence and the counter's other call (`0x28`, flyby tick 20) starts
  `Audio_StartSfxFadeIn(0.02)`, so the effects are silent for 20 calls and then climb `0.02` a call
  to the SFX Volume fraction, full 50 calls on (tick 70)
  ([the call sites](../ghidra/functions/psp-pulse-usa/race-intro.md#the-music-the-two-counter-calls-and-the-exit-stop-read-2026-11-02-lane-pulse-flyby-music)).
  Ours: `PreRace::effects_fade_in` (Pulse's, measured off the executable) raises
  `IntroCue::SilenceEffects` on tick 0 and `IntroCue::FadeInEffects` on tick 20; `oag_sound::Audio`
  steps the fade once per `Audio::tick`, and `stop_race_sfx` releases it so a flyby quit half-way
  leaves the menus at their slider. The bus gain is the title's `effects_volume_law` applied to
  slider times fade: Pulse's is `Squared` (measured, see below), so the bus reads
  `(slider * fade) ** 2`, through the flyby and for the rest of the race (`hold_race_song` arms
  it with the race); Pure inherits it from Pulse, HD, 2048 and Omega stay `Linear` (their mix is
  authored and `hd.live` owns the buses). The session runs `sound::flyby_tick` under the flyby,
  on a title whose rule has the fade only (`Race::intro_plays_race_sounds`): the circuit's
  **placed ambience emitters** and the craft engines, both by range from the flyby camera's ears,
  through the fade; no cue is taken. The world still does not tick (the cues are output, not
  `World` state). Measured on `03_Track` (`flyby_voice_ground_truth`): the first emitter sounds at
  tick 390 (the original's `NEON_RING` at 393) and the first audible engine at tick 1,144 (the
  original's first non-zero engine volume at about 1,150); the engines hold their voices at gain 0
  before that, as the original's commit volume 0. **Not played:** the original's `~AMB` loop that
  sounds from its first call (conf 70, which cue it is is unread, and it is not among the
  emitters in range), the `~FLYBY_DIST_*` one-shots (a command list's random picks, no trigger
  recovered). A held Cross ending the flyby at tick 60 leaves the ramp at 0.8; it finishes in the
  countdown, as the original's per-frame update does.
  **A call is one tick (60 Hz)**: the lock counter that times the fade is one decrement per
  `RaceMode_UpdateIntro` call and 60 of them are the 60 ticks the lock measured, and
  `Audio_UpdateGroupVolumes` runs once a frame beside it
  ([audio-levels.md](../ghidra/functions/psp-pulse-usa/audio-levels.md)); that the PSP holds 60
  frames a second through the flyby is **chosen, not measured**. Measured at the bus
  (`flyby_voice_ground_truth`): the Sfx bus reads `0` through tick 19, `(0.02 * (n - 19)) ** 2`
  after tick `n`, and full after tick 69.
  **Scope of the square:** always, not only in the flyby. The helper squares every group volume
  and the slider reaches the group as the same fraction (`Audio_SetSfxFadeTarget`), so every
  Pulse race's effects bus is squared (the default slider, 100, is unchanged; a lower slider is
  quieter than before: 50 reads 0.25). Menus outside a race stay linear: leaving a race resets the law (`stop_race_sfx`), launching one
  arms it again; the original's menus are presumably squared too but ours are unmeasured there; Speech is in the same faded group, squared too and wired (see "The Speech group (2) is squared too"). The ramp's
  ceiling at a slider below 100 (the original's fade target is the slider) is unmeasured; ours
  still ramps slider times a 0-to-1 fade.
  **What is audible: watched 2026-11-02**, see "What the original plays under the flyby" below.
- **Other titles, effects fade (checked 2026-11-02)**: *checked, differs.* HD's fly-over sits on the
  `PreRace` mix row ([now-playing.md](../formats/now-playing.md)), which authors its own buses; its
  `PreRace` rule carries no `effects_fade_in` and the fade never touches an HD bus (the Sfx bus is
  left to the authored mix). 2048 and Omega inherit HD's mix; Pulse PS2's flyby is off.
- **The screen wash** the original starts when the flyby ends (`ScreenFlash` kind 10) is drawn
  (0.7 s of white, the executable's).
- **Other titles, music (checked 2026-11-02)**: *checked, differs.* Only Pulse's executable calls
  a music player for a voice-over here. HD's fly-over sits on the `PreRace` mix row with music at
  0 ([now-playing.md](../formats/now-playing.md)); 2048 and Omega inherit HD's mix. Pulse PS2's
  flyby is off; its clips are `PRERACE.WAD` ([ps2-voice.md](../formats/ps2-voice.md)).
- **Demo mode** skips it (`g_game_mode == 2`), and a byte at mode object `+0x40` also does
  without a held button; neither is read further. Only Single Race and Time Trial were
  captured; Zone, Eliminator, Head to Head and Tournament reach the same function through
  their own wrappers and are taken on that.
- **`01_Track` (18 s) and `14_Track` (17.97 s)** were not watched to their end; the length is
  the file's own `AnimEnd`.
- **A reversed circuit** uses the same `start_grid.vex` as its forward one: the directory holds
  only one, and Metropia (`02_Track` reversed) flew `02_Track`'s file to `0.02` units over the
  first 100 frames (a RESTART RACE capture, so about the first 1.7 s of its 27).
- **Why a fresh menu-walk load skipped it on two circuits** (above).
- **The scenery does not animate under the flyby**: our `Scene::render` takes its animation
  clock from `World::tick`, which is held at 0, so the circuit's `Anim Transform`s and
  scrolling textures sit at their first frame through the 25 s (the original's keep running).
  Fixing it is a one-line change in `scene/frame.rs`, which is 1,000 lines of one function and
  has to be split first.

## The opening shot's window panes stipple on Platinum Rush (measured 2026-11-07, lane `pulse-flyby-flicker`)

The maintainer's play report on PSP Pulse: "a flickering texture at the beginning of the flyby on
the left half of the screen" (Platinum Rush White). **Reproduced in ours, not in the original.**

**The surface.** The glass panes of the left dome of the start building, `13_Track`'s first shot
(the camera holds `(55.3, 64.8, 363.4)` for about 28 ticks, then pans slowly). In ours they draw as
a diagonal chequer of pane and wall through each other, the pattern shifting with any change of the
camera. Over ticks 30 to 35 (the camera panning) the left half changes by a mean of 17.1 against
10.9 on the right (960x544), the pane region 59 against 14 on the wall above it; the original's
frames over the same stretch (frames 36 to 69 of a `psp-flyby.py --frames` run) change by about the
same on pane and wall (ratio 1.0 to 1.3), and a pane close-up at frames 100 and 101 is clean.

**Which draws, and the probe.** `oag-game --race --track Data\Environments\13_Track\track.vex
--ticks 0 --intro-ticks 5 --screenshot`, once at `--draw-alpha 0.5` and once at the default `1.0`:
the held shot is the same camera to the last bit and the two frames differ only where depth ties,
left pane region mean `45.72` (0..765 scale), **`0.00` with node 365's panes hidden, or with any two
of nodes 365, 342 and 349 hidden** (a debug hide hook, not kept). The panes are
`rf_spanbuilding_windows_shinemap` batches (`pass_mask 0x3001`, depth bias `0`), drawn by three copies
of the same building: node 365 (6 small batches, layer `0x45`, its pane 10 triangles) and nodes 342
and 349 (full copies, layer `0x4a`, panes 82 and 85 triangles). Every pane triangle centroid of 365 lies within
`1e-5` of the plane of 342's and 349's pane triangles it falls inside: **exactly coplanar surfaces cut in different
triangles, so the interpolated depth differs by a float ulp or two and the winner flips per pixel.**
Not the shine pass: `--no-track-shine` stipples too. PVS, shadows, reconstruction and MSAA flags
change nothing (`--pvs false` is the same bytes; with only the craft's own section row admitted the
shot draws almost nothing, so the flyby is not culled to the grid row). Not temporal jitter: the
camera is bit-held; the blend alone (`between_ticks::blend_view` round-trips through an inverse)
changes the last bit and that is enough to flip the pattern, which is why a live window crawls.

**Why the original is clean (depth state read live).** A GE list of a flyby frame
(`scripts/psp-ge-dump.py`, now with `zscale`/`zcenter`/`minz`/`maxz`): depth range `1001..64946`,
`GREATER` with writes on (a strict test), a 16-bit buffer, projection near `1.2`, far `2000`, so
about `1.2` depth levels per world unit at 250 units. Surfaces closer than a level **tie, and the
first drawn wins**; the same three copies, exactly coplanar, resolve to the first drawn in every
pixel. The batches' own depth bias (`batch+0x29`, [vex.md](../formats/vex.md)) is `0` on all of
them, so it is draw order plus 16-bit ties, not the authored bias.

**Other circuits.** The same probe on the opening shot of every Pulse circuit and its reversed file
(`track_reversed.vex`): `13_Track` 2,076 pixels differ by more than 60 (2,059 of them in the left
half, matching the report), `01_Track` 20, every other circuit 0 to 6. The reversed files gave the
same numbers as the forward ones on the three pairs run (`13`, `16`, `03`), so this probe cannot tell
the two apart: Black is not measured separately. HD, 2048 and Omega: no document names Platinum Rush
(the discs were not searched), and the `.rcsmodel` container layout read in the docs has no
per-batch depth byte (**checked, differs**, by the layout, not by a census of the discs).

**Fixed by the mask below.** The original never draws node 365 in that shot, so the tie never
arises; the tie rule the first pass considered (one 16-bit depth step, first drawn wins) was not
built and is not needed for this surface. The difference in depth precision stays: a circuit
that draws coplanar copies inside one row would stipple in ours and not in the original. None
shows in the opening shots (table below).

## The flyby is masked by the section under its camera (measured 2026-11-07, lane `pulse-flyby-fix`)

**Claim**: Platinum Rush's flyby draws no flickering surface in its opening shot, White and Black.
**True**, because the flyby now draws the section row the original draws.

**What the original draws in the held shot (confidence 92).** A GE list of the shot itself: stop on
`VIEW_PUBLISH` (`0x08882e9c`) of `scripts/psp-flyby.py` at hit 8, the eye reading the held
`(55.31, 64.84, 363.38)`; send `gpu.record.dump` with the CPU stopped, then resume, so the dump is
the next whole frame. That is `scripts/psp-flyby.py --frames N --dump-at 8`; the earlier attempts
asked for the dump by wall clock. White and Black, same result. Matched by vertex count and world box, strip count
being triangles plus two:

| Copy | Group | Its own batches | In the dump |
| --- | --- | --- | --- |
| node 365 | section 32 | pane 10 triangles, box `(95,89,145)..(123,125,181)`; all 6 batches | **none** |
| node 342 | section 27, `LodGroup` 335 child 0 | tile 407 and 434 triangles, pane 82, wall 378, 245, 291 | **none**: the unique boxes are absent, and the strips are 87, 271, 187 and 65 vertices, which are 349's counts |
| node 349 | section 27, `LodGroup` 335 child 1 | tile 440 and 125 triangles, pane 85 | all (prims of 442, 375 and 87 vertices) |

Node 342 and 349 share several boxes, so a box match alone would claim both; the vertex counts and
342's unique batches are what separate them. They are tiers of one `LodGroup`, chosen by distance,
which ours already did. So the only copy to remove was 365, and hiding it alone took the
probe to `0.00` in the first pass.

**Why it is absent: the row (confidence 90).** The render-view publisher `FUN_08882cbc` ends with
`AiTrack_LocatePosition(2000.0, track, record, eye, 0, -1, 0)` and `Camera_PublishSection(record+0x60)`,
which is `view_section` (`0x08ab10a0`), the row `Gfx_GetVisibleSectionMask` hands the draws. It runs
every frame of the flyby with no cursor, so the lookup is global, and the section is the camera's,
not the craft's as in the race. Section 8's row `0x021f0000a8001ff8` holds bit 27 and not bit 32,
so 365's group is out and 349's in.

Ours asked the race's question instead. The camera is searched for near the craft and held to a
half-width test, both of which fail on a flyby, so it read `UNPLACED` and drew every section
(`--pvs false` gave the same bytes, which is what hid this). `Race::flyby_camera_section` now takes
the sample nearest the camera anywhere on the spline, and `Race::visible_set` masks by that one row,
the path the station cameras already use.

**The locate rule against the original, whole flybys** (`0x08ab10a0` read at every publisher hit, `view_section` in
`psp-flyby.py --frames`' log, the camera eye beside it, ours `Spline::nearest` on the same eye):

| Circuit | Rows | Differ | Largest camera to spline distance |
| --- | --- | --- | --- |
| Platinum Rush White (`13_Track`) | 1,523 | 6 | 60.0 |
| Talon's Junction White (`16_Track`) | 1,520 | 10 | 149.5 |

The rows that differ are each a section change: the breakpoint precedes the publish, so the global
lags by a frame, plus the first rows before the camera has a pose (eye at the origin). Sections in
order: Platinum Rush `8, 1, 17, 14, 23` (Black too), Talon's Junction `0, 27, 25, 18, 19, 20, 8`.
`flyby_section_ground_truth` holds both, off the disc, through `Race::tick_intro`.

Not exercised: a miss. `AiTrack_LocatePosition` failing leaves the pre-zeroed record, so the original
would publish section `0`; no camera measured stood more than 150 units off the spline, and ours
has no reach test.

**Acceptance.** The probe of the section above: `45.72` before, `0.00` after, on `track.vex` and on
`track_reversed.vex` (`flyby_pane_ground_truth`, red with the rule off). The opening-shot sweep,
pixels differing by more than 60 between the blends, on the 12 circuit files:

| File | Before | After |
| --- | --- | --- |
| `13_Track` | 2,076 | 0 |
| `01_Track` | 20 | 6 |
| `09_Track` | 6 | 4 |
| `02`, `03`, `04`, `05`, `06`, `07`, `10`, `14`, `16` | 0 to 5 | unchanged |

**Consecutive frames.** Mean change between ticks 30 and 35 (960x544, pane region, then the wall
above it): before `59.5, 56.7, 54.1, 51.8, 64.9` against `14`, after `24.2, 23.8, 24.1, 24.1, 24.3`
against `14`. The camera pans, so a pane with reflections moves more than a plain wall; the
crawling chequer is gone in all six frames.

**The rest of the flyby.** Before and after differ by up to 85,370 pixels (960x544) further in. Read
against the original's frame at the same camera:

| Circuit | Ticks | Against the original |
| --- | --- | --- |
| Talon's Junction | 450, 1250 | after matches; before drew a grey panel and a sign over the pink advert at 1250 |
| De Konstruct Black | 750, 1000, 1250 | after matches; before drew a building across the sky and hover vehicles |
| Platinum Rush | 60, 100 | after matches on the left dome's panes and the walkway's |

Not read against the original: Platinum Rush tick 200 (10,385 pixels) and `03_Track` (up to 4,949
pixels), compared before and after only.

**Other titles.** The path needs `section` nodes. HD, 2048 and Omega draw `.rcsmodel` chunks with a
partition of their own, located by world position, and have no such nodes: **checked, differs**, by
the code path; their flyby's mask is unmeasured. Whether HD authors coplanar copies was not
censused, because the fix takes no tie rule that could reach it. Pure declares no sections. PS2 Pulse
has them, and its flyby is off (`PRE_RACE.on`).

The weather reads the same global (`Weather_Update`); `Race::advance_weather` does not take the
flyby's section, and that is left as it was.

## What the original plays under the flyby (watched 2026-11-02, lane `pulse-flyby-audio-capture`)

**Claim**: the original's flyby plays craft engines under an effects fade, silent for 20 calls
and then `+0.02` a call. **Outcome: the fade is true; "engines" is false.** Under the fade
window the only effect voice that is audible is the circuit's looping ambience; the engines are
keyed on and released inaudible, and the grid's engines only come into earshot about 1,150
ticks in.

**Method.** PPSSPP v1.20.4 (SDL, software renderer, muted, own profile) on the Pulse PSP USA
image, a live race on `03_Track` (Moa Therma White; the id read off `0x08b310b8`), then the
pause menu's RESTART RACE, so the intro plays in full. A WAV dump could **not** be taken: the
SDL build has no "Record audio" binding (the Windows menu's action only), the `DumpAudio` key
did not start a dump, and the instance is a "secondary" one (`Secondary instance 8 - silencing
audio`), so a WAV would have been flat anyway. The envelope was read from the mixer's own state
instead, one breakpoint per pass (v1.20.4 fires only the latest) and every pass anchored on the
first `RaceMode_UpdateIntro` call (`0x08829e6c`, tick 0), the breakpoint then swapped:

| Pass | Breakpoint | Reads |
| --- | --- | --- |
| cues | `Scream_StartSound` (`0x0898f864`) | bank name, cue index, `ra`, the 32-voice table `g_sas_voices` (`0x08b893d0`, `0x6c` stride) |
| fade, envelope | `Audio_UpdateGroupVolumes` (`0x0893ac70`), every call | `mgr+0x90/0xd0/0xd4/0xd8`, the 16 group volumes (`0x08ac3910`), per active voice its group (`+0x1b`) and the SAS commit volume `+0x40/+0x44` (left, right) |
| voice | `MusicPlayer_StartFile` (`0x08938ca8`) | the path argument |
| intro | `RaceMode_UpdateIntro` | one row per call, the tick axis |

**The tick axis.** Tick 0 is the substate-0 call; the first substate-1 call (tick 1) comes
about 12 frames later (the load tail), then one call a frame: tick `k` is at frame `k + 10.5`
(measured: tick 20 at 30.5, tick 60 at 70.5, tick 1528 at 1538.6; spacing 0.43 to 1.1 frames,
mean 1.0). Cue times below are `frame - 10.5`, good to a tick.

**The fade (confidence 92; 6 runs on `03_Track` and one on `16_Track`).** `mgr+0xd4` flips to
1 at call index 20 (counter `0x28`), `mgr+0x90` reads 0 until then, then steps by exactly
`0.02` (every step of every run), reaches 1.0 on the 50th call after, and `+0xd4` clears.
`mgr+0xd8` reads `0.02`, `+0xd0` 1.0. The group table at the intro's start reads `0` for nine
groups (`0 2 3 4 5 10 12 13 14`) and `1024` for `1 6 7 8 9 11 15`; the nine are exactly the ones
`Audio_UpdateGroupVolumes` writes, and they step up together to 1024.

**What the voices sit in.** Engine voices (`SHIP` bank `~ENGINE`, cue 0) are group 0; every
circuit voice (`moather`, `gentrak` banks) is group 3; `SPEECH` is group 2. All three are in
the faded nine, so the fade multiplies engines, ambience and crowd alike (confidence 85: read
off the voice table at about 70 cue starts in 2 runs).

**The law the fade is applied with: the square (confidence 85).** A looping ambience voice
whose commit volume is a constant 1,262 (left plus right) at full reads
`1262 * mgr+0x90 ** 2`: over the 50 ramp calls of three `03_Track` runs the largest deviation
of `volume / volume_full` from `fade ** 2` is `0.0032`, from `fade` it is `0.25` (`0.4` gives
200, `0.8` gives 806, `0.98` gives 1,210). **The mechanism** (decompiled, confidence 80): a
voice's commit volume is recomputed by `FUN_0898c214`, which calls `FUN_089957c0(volume, group)`
for each side; that helper multiplies the voice's volume by the group volume
(`g_audio_group_volumes[group]`, `0..1024`) **squared**, then by a second per-group table
(`0x08ac3990`, also squared when below `0x10000`), and shifts. So every change of a group volume,
the SFX slider's included, acts as its square, not only the fade; whether the slider's own law
matches is unmeasured here (the fade and the slider both reach the group as one fraction, so the
measured square is on their product). Ours: the title's `effects_volume_law` (`Squared` for Pulse) applied to `slider.gain() * fade.gain()`
(`crates/sound/src/sfx_fade.rs`, `bus_gain`; `Volume::gain` is `percent / 100`), so the bus reads
the square, as the original does. Whether the slider's own steady-state law is the square is read off
the helper, not watched below 100.

**The Speech group (2) is squared too (confidence 90; lane `pulse-speech-squared`, 2026-11-02).**
Pulse's options menu has two rows, Music Volume and SFX Volume, so the original has no Speech
slider; `SPEECH` (group 2) is one of the nine groups `Audio_UpdateGroupVolumes` writes, so its
group volume is the SFX fraction. The knob for the measurement is therefore the SFX field: the
settings-apply writes the slider's fraction to `mgr+0x90` and `mgr+0xd0`
(`audio-levels.md`, "The two options-menu sliders"), and each run wrote `mgr+0xd0` at the first
`RaceMode_UpdateIntro` call (the slider's own target field, not the options UI, which was not
driven). Same probe as above (PPSSPP v1.20.4, `03_Track` via RESTART RACE, breakpoint on
`Audio_UpdateGroupVolumes`, every call to tick 1,700), reading the group table, the second table
(`0x08ac3990`) and each group-2 voice's SAS commit volume. The countdown's `SPEECH` 11 `ready`
(frame 1,652 to 1,700) holds two voices steady for the 48 calls after it starts (stereo pair,
left plus right 3,722 at full):

| SFX fraction | group 2 (`/1024`) | `ready` L+R | ratio to full | fraction squared | linear |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1.0 | 1024 | 3722 | 1.0000 | 1.00 | 1.0 |
| 0.5 (2 runs) | 512 | 930 | 0.2499 | 0.25 | 0.5 |
| 0.3 (2 runs) | 307 | 334 | 0.0897 | 0.09 | 0.3 |

Largest deviation from the square 0.0003 (0.2499 and 0.0897 against 0.25 and 0.09); from linear 0.25 and 0.21. The second table's entry 2
reads `65536` at every setting (constant, so the law is the first table's square alone). Four runs
at the two lowered settings plus the full baseline; the nine faded groups all read `fraction * 1024`.
Falsifiers written before the first run: a ratio near 0.5 at 0.5, group 2 not following the
fraction, or an unsteady `ready` commit would each have made the claim false; none occurred.
This also closes the effects page's open item: the slider's own steady-state law is the square
for group 2, read off the same helper as the fade (group 0 and 3 voices move with range, so their
ratio is not a clean law; the group volume itself read `512` and `307`).

**The pre-race voice-over is not in this group.** `3_Track.at3` plays through the music player
(above), outside the fade and outside `FUN_089957c0`, and ours plays it on `Bus::Music`; squaring
the Speech bus never touches it. **Ours**: `Title::speech_volume_law` (Pulse `Squared` measured,
Pure `Squared` inherited from Pulse, HD/2048/Omega `Linear` with their authored mix) set per race by
`hold_race_song` and released to linear on leaving a race, like the effects law. **Wired**
(`Title::speech_follows_effects`: Pulse measured, Pure inherited from Pulse): the original's Speech
has no slider of its own and follows the SFX slider and the flyby fade, so in a race the speech bus
reads `(SFX slider x fade)^2` times our own Speech slider as a plain linear extra (default 1.0,
which changes nothing against the original). Our Speech slider row stays; leaving a race returns
the bus to the menu law (linear, the Speech slider alone). Test:
`oag-sound` `the_speech_bus_follows_the_sfx_slider_and_the_flyby_fade_squared`, and
`oag-game` `arming_a_race_sets_the_titles_speech_source` for the per-title binding. HD, 2048 and Omega: `checked, differs` (an authored mix owns the speech bus; this
helper is Pulse's), not measured on those titles.

**What starts and when (`03_Track`, 4 cue runs and 3 envelope runs, ticks).**

| Tick | Source | What | Audible? |
| ---: | --- | --- | --- |
| about 0 (the 12-frame gap before tick 1) | circuit ambience | `moather` cues 3 `~CITY`, 4 `~AMB`, 5 `~FLYBY_DIST` start (CITY and FLYBY_DIST are command lists with no waveform of their own; one looping voice, group 3, exists from the first call: `~AMB`, confidence 70) | silent to tick 22, then ramps with the fade squared to its full 1,262 at tick 71, steady to tick 100 |
| 2 to 3 | craft engines | seven `SHIP` `~ENGINE` key-ons in one frame, group 0 | **no**: commit volume 0 (every faded group reads 0 at tick 3, so that alone says nothing of range), and the voices are released within a frame or two, which is the range evidence; they are keyed on again as the camera nears the grid from tick 1,151 |
| 60 | voice-over | `3_Track.at3` through the music player (not a SAS voice, not in the fade), 60th substate-1 call at frame 70.5 to 71.0 | yes, full |
| varies per run | circuit ambience | one-shots `~FLYBY_DIST_02..06`, `~CRAFT`, `~craft1` (cues 6 to 10, 12, 16), a command list's random picks, pan 100, volume argument 12 to 351 | yes, from tick 40 on in some runs (inside the ramp), after tick 70 in others |
| 393 | `NEON_RING` loop | `moather` cue 0, in 4 of 4 runs at frame 403.5 to 404.4 | yes |
| 673 | crowd and arrow | `gentrak` 3 `~yellowarrow` and 1 `~crowd`, together, 4 of 4 runs | yes |
| 1033 to 1122 | track loops | `gentrak` 7 `~bighum` and `moather` 11 `~birds` (1033), `gentrak` 1 (1049), 2 (1103, 1122), 6 (1117) | yes |
| 1151 to 1333 | craft engines | `SHIP` `~ENGINE` key-ons every 24 to 29 ticks, group 0 | **first non-zero engine volume at tick about 1,150** (sum 4 to 13), 160 to 380 by tick 1,200 to 1,300, against 3,000 to 4,000 for the ambience |
| 1530 | craft engines | three `SHIP` `~ENGINE` at the flyby's end (tick 1529); sum 283 | yes (the race is about to start) |
| 1621 | `SPEECH` 11 `ready` | the countdown, after the flyby | not the flyby |

`16_Track` (the circuit the menu walk's third Track Select row reached): the same shape, three
group-3 ambience voices from the first call (full level 1,533, the fade-squared ramp again:
`0.4` gives 242), the seven engine key-ons at tick 3 at volume 0, and **no engine above 0 until
tick 1,529** (three voices, 283). Neither circuit's camera starts near the grid: a circuit whose
camera opens within earshot of the engines was **not found** in the two measured (our earlier
eight-circuit census found none within earshot by tick 80 either), so the brief's near-grid
check is open.

No front-end or menu bank (`fronten` or similar) started a cue in the window in 4 cue runs: the only banks are `SHIP`, `moather`, `gentrak` and, after the flyby, `SPEECH`; so there are no UI cues under the flyby.

**Reading it for the port.** The effects fade is real and the right shape, but the sound it
quiets is the ambience, which ours does not play under the flyby (engine idle only); and its
gain law is the square of the group volume. The engines are not audible under the fade at all. Ready to wire (not
done here), written in the handover thread.

**Open.** Whether `~AMB` or another cue is the one voice (a waveform-address compare on
`voice+0x54` is the check); the audible level of the one-shots (their volume argument, 12 to
351, is the cue's, not a commit volume); a real WAV (PPSSPP's Windows menu action, or a build
with the mixer's dump on an unsilenced instance); a circuit whose camera opens near the grid;
Time Trial and the other modes; PS2 (flyby off).
