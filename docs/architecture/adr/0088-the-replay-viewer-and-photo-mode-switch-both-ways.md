# ADR-0088: The replay viewer and photo mode switch both ways at a tick the player chose

## Status

**Accepted** (2026-11-08, lane `replay-photo-switch`). The decisions the maintainer made on 2026-11-08 are
marked **(M)**. Everything else is **chosen, not measured**. Builds on
[ADR-0087](0087-a-replay-and-photo-mode-share-one-timeline.md), which it does not edit: decision 5 of
that ADR is departed from for photo mode alone (decision 4 below), as its owed note asked.

## Context

ADR-0087 gave the replay viewer (stage 5) and photo mode's rewind (stage 6) one machine, and left the
two screens apart: a player watching a replay could not take a picture of it, and a player in photo mode
who had scrubbed to a moment could not play on from it.

## Decisions

1. **The two switch both ways at the tick the player chose (M).** From the viewer, photo mode opens at the
   tick on screen and the viewer stands still under it; from photo mode, over a live race or a replay,
   playback starts from the tick the mode is scrubbed to (M). **The mechanism is chosen, not measured:** the
   viewer's PHOTO chip, or SELECT, opens the mode; playback is a PLAY row under REWIND in the effects panel
   (on or off, the arrows or a click as every row), run at the speed the race was made at and turning itself
   off at the live tick or the end of the recording.
   **Over a live race PLAY is photo mode's own playback of the race's recording, not a switch into the
   replay viewer.** The mode stays open and its camera and rows stay; no viewer over a live race is built
   (the viewer's `Timeline` is a second race that cannot be put over the one being driven without a
   second snapshot path). A reading of "play starts the replay" as the viewer's screen is the alternative,
   unbuilt, and the maintainer may ask for it.
2. **The scrub is a seek (M).** Over a replay the REWIND row is `Viewer::seek`, its range the whole
   recording, and it stands where the playhead does when the mode opens. Leaving continues the
   replay from where the row stands (M). **Chosen, not measured:** it plays if the viewer was playing when
   the mode opened or PLAY is on, and stays paused otherwise, with the sound rebuilt from that tick. The viewer's own timeline is the only one over a watched
   race; the mode starts no keeper of its own.
3. **A live race still resumes to the bit.** Over a live race PLAY steps the same `Rewind` the scrub
   moves (`Rewind::step`) and `Rewind::end` puts the simulation and its recording back, so there is no
   second snapshot path. `a_live_race_played_from_a_scrub_and_left_hashes_as_it_was` compares every tick
   after the visit with a race that never visited.
4. **The owed departure from ADR-0087 decision 5.** The decision has the probe `Race` moved and the live
   race neither ticked nor written. Photo mode moves the race on screen through its recording and puts it
   back to the bit on leaving, as the viewer moves it (`docs/ui/photo-mode.md`, "Accumulated shots and
   rewind"). The maintainer accepted that on two conditions, both kept: **photo mode offers no
   take-over** (PLAY plays the past and stops at the live tick; only the viewer will take over,
   ADR-0087 decision 9), and **the race resumes exactly where it was**.
5. **The tick that leaves photo mode is still the mode's.** The frame loop read the devices into a
   snapshot before the mode spent its presses, so the leave tick ran `Race::tick` with CIRCLE as an edge:
   the absorb press, and every tick of an autopilot Time Trial after it differed. The leave tick is now a
   mode tick (audio only); `leaving_photo_mode_with_circle_hands_the_race_no_press` taps and holds it.
6. **Over the viewer the shot is one frame.** A viewer has no keyframes of the race it moves, so the blur
   rows are not offered over it (HD, 2048) and the depth of field is the single-frame pass. Not offered in
   POSE VIEW either.

## The binding plan a later lane implements

Not built here; the lane that builds it reads this section as its brief.

- **Shared keys.** An action both screens have has one binding in both: L1 and R1 previous and next craft
  (wrapping, skipping craft out of the race: photo mode's rule, also for one seat of a split recording);
  SQUARE the HUD on or off (the viewer hides the race's HUD with it); CIRCLE leave (photo mode back to
  where it came from, the viewer to the REPLAYS page).
- **CROSS stays per screen: play or pause in the viewer, the shot in photo mode.** The press that
  means "do the thing this screen is for" keeps that meaning, and each wrong guess is harmless (a shot
  taken, a replay paused). Unifying them would give the viewer a second meaning for its main button or
  take the shot off the button both originals measured. The viewer keeps START as an alias for play, and
  photo mode gets PLAY on a button of its own (below), so no one needs CROSS to cross over.
- **L2 and R2 do not alias to L1 and R1 on these screens**
  (`crates/input/src/pad.rs`, `TriggerMode`: in `Airbrakes` a trigger past the threshold also sets its
  shoulder's bit, which is right for the sideshift and wrong for craft cycling). On both screens a trigger
  is a view-side analogue read, taken before `resolve`, whatever the trigger mode: L2 scrubs back and R2
  forward while held, the speed following how far it is pulled; in photo mode a pull past the far
  threshold is PLAY, so photo mode has a key for it. Neither reaches `InputSnapshot`.
- **The right stick is read view-side, never into `InputSnapshot`**, so a recording and its hashes do not
  move. In photo mode the right stick orbits the camera and the left stick pans and zooms (HD's layout,
  `docs/ui/photo-mode.md`, "Not ported"); in the viewer its horizontal axis jogs the playhead.

## Consequences

- PLAY is reachable only through the effects panel until the binding lane lands, a keyboard player's
  PLAY is the panel's row, and the viewer's key for photo mode is SELECT.
- **Found on the way, not fixed:** over a recorded race a rewind whose pre-roll starts from the first
  keyframe (a landing under about 120 ticks) hashed differently from the live run: 101 with `--autopilot`
  and 102 with the throttle held on a Pulse Single Race, with no mod mounted; landings from 131 on matched.
  This lane does not change the seek path. Second, the stage-6 rewind hash test never replays a
  recording while this repository's mods are mounted: the race records nothing then (`recorded_events`
  absent), so its landings fall on whole keyframes. Both are in `docs/ui/photo-mode.md`, "Not done,
  named". The PLAY test mounts no mods and scrubs past the first.
- Over a live race PLAY is silent and plays one tick a frame; the viewer's speeds are not on offer there.
