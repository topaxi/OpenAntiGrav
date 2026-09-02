# FSR 1's default is open

`oag_render::post::fsr1` transliterates AMD's MIT `ffx_fsr1.h`; the route for FSR 3.1 is settled in [ADR-0012](../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md). **Do not** read a menu capture taken with `--upscaler` as evidence either way: the flag changes the UPSCALER row's own text, so the two images differ for an unrelated reason - that nearly produced a false conclusion.

## Phase 0 landed, and it changed this thread's question

**2026-09-02.** The shared UI-compositing restructure is built and green
([ADR-0036](../docs/architecture/adr/0036-ui-composites-at-presentation-resolution.md),
[ADR-0038](../docs/architecture/adr/0038-a-stage-with-no-scene-draws-at-presentation-resolution.md)).
`docs/overview/modern-features.md`'s prerequisite row *a scene without UI in
it* is **Done**; camera jitter is the last FSR 3.1 prerequisite left.

**There are exactly three paths that draw a frame, and no upscaler touches UI
content in any of them.** The windowed loop (`main/session/frame.rs`),
`race/capture.rs`, and `capture.rs`'s front-end path - `main/headless.rs` only
builds a `race::Presented` for the second, so it is not a fourth; checked
2026-09-02. The HUD and the scoreboard composite into the presentation target
after the resolve; the menus, the front end, the launcher and the loading
screen skip the scaled target entirely.

**This dissolves one half of the doubt rather than deferring it.** The thread
was opened on two unmeasured risks: paletted 480x272-era raster and
coverage-atlas glyphs, which a sharpener rings on, and high-frequency scene
effects. The first is now structurally out of reach of every sharpener - there
is no menu frame an upscaler can ever touch, so there is nothing left to
compare and no capture that would say anything. **An earlier version of this
file said the opposite** - that the menus "only become measurable once Phase 0's
last slice moves them", and that ADR-0036 made the doubt *narrower, not better*.
That was written while the HUD had moved and the menus had not. The last slice
moved them out of the upscaler's path, not into a measurable one, and the
sentence is retracted here rather than left to send a reader after a risk that
cannot exist.

**What is left is the scene half, and it is genuinely unmeasured.** The one
comparison behind the current preference was taken at 50 % on one frame of one
track, and FSR 1 won clearly - but see the stationary-capture trap below for
what that frame did not contain.

## Open

- **Whether the default should move to `fsr1` at all.** This is now the whole
  thread. It rests on one comparison, at 50 % on one frame of one track, on
  scene content that contained no effects.
- **A plain `--race` capture is quiet on the content a sharpener is worst on, and that is how the one comparison so far was taken.** A stationary capture plays no particle effect, fires no weapon and raises no shield, so the frame it produces has none of the high-frequency additive content RCAS rings hardest on - the spark burst, the rocket, the shield shell. Measured from the other end on 2026-09-02: the `psys` vertex path costs 199 bytes in a stationary `--race` capture and 1.99 MB under `--autopilot --give rocket --press square`, four orders of magnitude, because the stationary one never reaches it at all. The same flags are what put the effects in front of an upscaler.
- **The HUD is not part of what is being compared any more**, and neither are
  the menus. A capture diff that includes either is measuring the compositing
  change, not the upscaler.
- **`--presented` still does nothing on the front-end path.** `capture.rs` has
  no `Framebuffer`, so a front-end capture gets no brightness/gamma grade and
  no aspect bars. This used to be described as the blocker on FSR 1 reaching
  the front end; since ADR-0038 it is not that at all - an upscaler is never
  going to run there - it is a missing grade and missing bars, which is a much
  smaller piece of work and no longer on this thread's critical path.
- **The user's own play-testing is the better oracle here** and has been named
  as available. A byte-diff of two stills answers "are they different"; whether
  the sharpening is an improvement on a moving race is a judgement a still
  cannot carry.

## Next Steps

- **Re-run the comparison on a race frame that actually has effects in it** -
  `--autopilot --give rocket --press square`, per the trap above - at 50 %
  render scale, `--upscaler off` against `--upscaler fsr1`. That single sample
  is now the whole of what the default flip is held to; there is no menu half
  waiting behind it.
- **Then ask for a play-test** rather than settling it on the stills alone.
- Optional, and not blocking: give `capture.rs`'s front-end path the grade and
  the aspect bars so `--presented` means the same thing on both capture routes.
