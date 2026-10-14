# ADR-0068: A window holds a list of seats

## Status

Accepted (2026-10-14, lane `multi-window`, at the maintainer's request). Amends
[ADR-0067](0067-a-multi-window-race-is-one-device-and-one-submission-per-window.md), whose
"every window is one person's whole-screen view" it widens; the rest of 0067 (one device, one
submission per window, the lifecycle) stands.

## Context

ADR-0067 put one person in each window. The maintainer wants more: 2 windows of 4 players, 4 and
2, and eventually each player choosing the layout per screen. A window must be able to hold
several seats, cut up inside it exactly as a split screen cuts its views.

## Decision

1. **The model is a list of windows, each `{ seats, cut }`** (`oag_raceplay::WindowViews`, on the
   race's view state beside the split's own cut; `Race::windows`). Slots are dealt to windows in
   order, window 1 first. The race has one camera per seat as it always had a camera per split view
   (`set_viewport_count`); which window draws which slot is presentation only. Nothing reaches the
   simulation or the state hash.
2. **Inside a window the seats are a split screen**: `oag_render::viewport::layout` over the window's
   area with the window's cut, and the split-HUD rules picked by the window's own view count
   (`Race::window_drawn`, `window_cut`, `window_rects`). A window of one seat is the whole-screen
   view of ADR-0067; the scene path for one seat is `Scene::render_view` (recorded into the window's
   encoder), for several `Scene::render_window_views`, which is the split's own loop
   (`render_slots`, shared with `render_viewports`) over the window's slots.
3. **Closing a window hands every seat it held to the AI**, and removes the window from the race's
   layout so the windows after it keep their seats.
4. **The command line is two spellings of the one model**: `--windows N` deals the people across N
   windows as evenly as they divide (`--players` says how many; one each by default), and
   `--window-seats 4,2v` spells the seats and the two-seat cut per window. A layout picker is not
   built; it would write the same list.
5. **A launch whose head count differs from the spelled layout is dealt, not refused**: the join
   screen decides how many people race.

## Rejected

- *One seat per window and N windows for N people only.* The case the maintainer asked for.
- *A per-window `Race`.* The race is one object with one tick; windows only look at it.
- *Deriving the layout from the window sizes.* A 4:3 and a 21:9 window want different counts, and the
  person at the desk decides, not the aspect.

## Consequences

- A window of several seats pays a split's costs in that window: no Pulse bloom, no motion blur, no
  jitter, and HD's and Omega's chains run once over the window after its last view. A window of one
  seat keeps all of them but the blur and jitter (ADR-0067).
- The authored two-player HUD is packed once for the whole race, so a layout mixing counts uses it for
  a two-seat window only when the race was loaded told two views.
- The per-window cut is a `h`/`v` suffix on the command line and nothing else yet.
