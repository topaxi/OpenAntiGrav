---
status: ready-to-wire
titles: [hd, 2048]
updated: 2026-11-24
summary: "Photo mode defect: the first speed blur shot of a visit has a stronger zoom streak than later shots of one frozen scene; later ones are the reference."
categories: [tooling]
size: S (35)
difficulty: medium (45)
---

# The first speed blur shot of a visit has another zoom history

2026-11-24. A photo mode defect found while making `--from-png` draw a speed blur shot's zoom history. Two saved shots of
one frozen scene, with the same controls, in one visit to the mode, should be the same picture. They are not: on HD (Anulpha
Pass, tick 702, 1440x816, speed blur 100, HUD on) the first shot differs from the second by **3.5 mean** (left and right edge
strips 3.6 and 6.8), the first with the stronger streak. **Later shots are the reference**: `--from-png` draws the ring with its
history settled (`race_capture/accumulate.rs`, 40 ticks of its own) and matches them.

## Evidence

- The settled redraw against a later shot (edge strips, mean absolute difference, limit 1.5): 1.10 and 1.38, against 2.11 and 4.10
  with no history; against the first shot of the visit: 4.22 and 7.43 (whole picture 4.30), against 3.35 and 9.15 with no history.
- Settling depth does not matter from about 4 ticks on (200 ticks equals 40 bit for bit), and a plain scene copy of a fresh ring (3.35
  and 9.15) does not match the first shot either.
- It is what a visit's first shot does, not speed blur: with a plain shot taken first, the speed blur shot after it matches the settled
  redraw (1.05 and 1.81).
- The first shot's frame reallocates the targets while it is pending (`photo_draws_natively`). How that moves the ring's history buffers
  (`oag_post::hd_zoom::Host`, `last_tick`) is unread.
- `a_speed_blur_shot_is_drawn_again_with_its_zoom_history` asserts the second shot and prints the first.

## Open

- What the first shot's history is, and so what the preview showed the player.
- Whether the mode's own tick (`Photo::age`, `speed_blur_pulse`) and the redraw's counter differ in a way that matters.

## Next Steps

1. Read the first shot's frame sequence in the session (the pending frame, the frame that draws it) with the ring's history buffers
   dumped on both sides, about 2 hours, runnable now.
2. Make the first shot carry the settled history, or say why it must not; then assert the first shot in the ground truth too.
3. Check on 2048 (`Accumulation::WIPEOUT_2048`); the ring is HD's, borrowed.
