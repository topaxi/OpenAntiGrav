---
status: measure
titles: [hd, 2048]
updated: 2026-11-24
summary: "`--from-png` of a speed blur photo matches the visit's later shots, not its first, whose zoom-ring history is not the settled one."
categories: [tooling]
size: S (35)
difficulty: medium (45)
---

# The first speed blur shot of a visit has another zoom history

2026-11-24. `--from-png` draws a speed blur shot with the ring's history settled (40 ticks of its own at the shot's
moment, `race_capture/accumulate.rs`). On HD (Anulpha Pass, tick 702) that matches a shot taken after another in the
same visit to the mode (left and right edge strips 1.10 and 1.38 of mean absolute difference, against 2.11 and 4.10 with no
history; `a_speed_blur_shot_is_drawn_again_with_its_zoom_history`), but not the first shot of the visit (4.22 and 7.43, against
3.35 and 9.15 with no history). Two shots of one scene in one visit differ by 3.5 on average, with the first the stronger streak.

## Open

- **What the first shot's history is.** Not the settled one at any depth (1 to 200 ticks give 4.2 and 7.4 from 4 on), and not
  the plain scene copy of a freshly allocated ring (3.35 and 9.15). The shot frame reallocates the targets while it is pending
  (`photo_draws_natively`); how that moves the ring's history buffers (`oag_post::hd_zoom::Host`, `last_tick`) is unread.
- **Whether the age keys matter.** The mode's own tick counts `Photo::age` and the history steps once per tick it is drawn under
  (`speed_blur_pulse`); a redraw uses its own counter.

## Next Steps

1. Read the first shot's frame sequence in the session (the pending frame, the frame that draws it) with the ring's history
   buffers dumped, and give `FixedPicture` the same steps.
2. Check on 2048 (`Accumulation::WIPEOUT_2048`); the ring is HD's, borrowed.
