# Before the race: the circuit flies itself

**Status: measured on Pulse PSP, 2026-10-01 (Single Race and Time Trial; `16_Track`,
`03_Track`, and Metropia by RESTART RACE; ten capture runs).** The camera, its length, its
cuts, its skip and its hand-over to the countdown are ported. The track-description panel is drawn
over it, with its measured fade (below). The music, the screen wash at its end and the world
settling under it are not (see [Open](#open)).

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
`03_Track`), and the original's scenery animates under the flyby while ours holds its first
frame.

The panel was compared at the sizes a player sees (480x272), `--intro-ticks N --screenshot` against
the original's frames: `03_Track` at tick 300 (identical breaks, bars and positions), `16_Track`
at ticks 30 to 120 and 1000 (the wipe reaching the paragraph's right edge at 2 s) and at 0 to 41
ticks after the flyby ends (`--intro-ticks 1529 --ticks N`, against the original's calls 112, 122
and 132): the bars thin out together, the title and paragraph are cut from the right as the
viewport closes, and the HUD arrives over a panel still a third visible. Scratch:
`data/scratch/pulse-flyby-panel/{montage-enter,montage-exit,cmp-moa,cmp-exit,cmp-bottom}.png`.

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
- **The world does not settle** under the flyby, by choice: ticking it would move every
  hash. The craft is two units low at the grid on `16_Track`.
- **The music**: the flyby has its own (`g_music_player` calls at the counter's 40 and 0,
  `RaceMode_UpdateIntro` substate 1); not listened to.
- **The screen wash** the original starts when the flyby ends (`ScreenFlash` kind 10) is not
  drawn.
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
