# Before the race: the circuit flies itself

**Status: measured on Pulse PSP, 2026-10-01 (Single Race and Time Trial; `16_Track`,
`03_Track`, and Metropia by RESTART RACE; ten capture runs).** The camera, its length, its
cuts, its skip and its hand-over to the countdown are ported. The track-description panel drawn
over it, the music, the screen wash at its end and the world settling under it are not (see
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
[`oag_game::race::intro_camera`](../../crates/game/src/race/intro_camera.rs).

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
| `0 .. ~28` | the animation holds its first frame: the camera node waits `1.0` s of its own clock, which advances two `dt` per tick | measured once at 28 (the animation clock `0.0834` s five ticks after it left zero, the counter at 57 at tick 3) |
| `~28 ..` | the animation plays at the tick rate | `*` over 1,521 frames: clock against pose |
| `>= 60` | the flyby **may end**: the counter has reached zero. Before that nothing can end it | `*` counter `59, 58, ... 0` |
| the end | it ends when the animation reaches `AnimEnd` (key units, so `1500` is 25.0 s), **or** when **Cross is held** (`Input_IsHeld(5)`) once the animation has started and the counter is zero | the first: last frame at `24.99` s of `25.0` on two circuits; the second: a 4-frame press ended it on the next frame, and the old "dismissed by a held cross at tick 61" is this |
| `end` | `RaceMode_UpdateIntro` leaves its camera substate. The track-description panel fades out over **30 ticks** (substate 2) with the chase view behind it; **no HUD** until that ends; then the old countdown: **272 ticks** from `end` to the throttle stepping, the same as when a press skips it | `*` substates 2, 3, 4 at `+0`, `+30`, `+31` and `+92`, the countdown at `+272`; HUD absent at +30, present at +60 |

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

## What ours does

| Rule | Where |
| --- | --- |
| the loader reads `<circuit>\start_grid.vex` on Pulse PSP and reports its length and cuts | `race/load/intro.rs` |
| the windowed session begins it when the race scene is handed over and steps it **instead of the world** until it ends: `World::tick` stays `0`, so every golden hash, replay and the 272-tick countdown are exactly as before | `main/session/frame.rs`, `Race::tick_intro` |
| the pose is the animation as of the previous tick, fov `54.309` vertical degrees (measured, both circuits, not derivable from the file: all twelve `gridCamera` payloads are identical but for the aim point) | `oag_vex::grid_camera`, `race::intro_camera` |
| the hold is 28 ticks, the lock 60, the end `AnimEnd` or Cross held | `race::intro_camera::Timeline` |
| the HUD is hidden through it and for 30 ticks after | `Race::hud_shown` |
| each one-frame key pair, and the end, count as a cut, so the temporal upscaler drops its history | `Race::camera_cuts` |
| `--no-intro` skips it; `--intro-ticks N --screenshot` photographs tick `N` of it | CLI |
| a headless run, a capture and every test drive `Race::tick` from tick 0 and never see it | by construction |

Run in a window (lavapipe under Xvfb, `--race`): it logs `pre-race flyby: begins`, plays the
`16_Track` shots, and logs `ends after 1529 tick(s)` (28 + 1500 + 1) with the HUD back and the
race running; with the X key held from partway through it logs `ends after 1490 tick(s), Cross
held`. `--hold cross` does not skip it in a window: that flag feeds the headless loop only.

Compared with the original at five matched times on `03_Track` (`scripts/psp-flyby.py` frames
against `--intro-ticks` captures, native 480x272): the same buildings, the same framing and
the same field at all five, to the pixel of scenery detail. The craft is the one visible
difference: the original's has settled, ours is at its placement pose (4.0 against 1.85 above
the track on `03_Track`).

## Open

- **The track-description panel** (circuit name, a paragraph, a fade) is not drawn. Its
  text is a string-table entry per circuit that nothing here has located; it is a front-end
  screen and ships with the pointer rules a new screen carries.
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
- **The first-frame hold's 28 ticks** was measured once; the read says 30.
