# A struck craft's hull sparks, measured against the original, locator by locator

2026-09-30, Pulse (PSP, USA `BOOT.BIN`). The port's `WO_SHIP_COLL_SPARK_DAMAGE`
on a craft a weapon has hit read **1.6x weaker** overall and **2.6x** on later
hits than the original. This page is the per-locator measurement that settled
why: three causes, two of them ours, one of them the comparison itself. Code:
`oag_raceplay::hit_sparks`, `oag_fx::psys::template`. The trigger and the
law it fires on are in
[`shield.md`](../ghidra/functions/psp-pulse-usa/shield.md), "`Ship_Damage`'s
weapon branch".

## Method

Same craft, same grid slot, same moment on both sides: a Single Race, the player
stationary after GO, `WO_SHIP_COLL_SPARK_DAMAGE` at severity 2.4 (Cannon-tagged
hit, `amount 2.0`).

- **Original, on PPSSPP v1.20.4.** The hit is posted into the player's
  pending-damage channel (`craft+0x120`, `+0x138 = 3`) at a
  `Weapons_DispatchFire` stop. **The locator is forced** by writing the player's
  locator list: `fx_count` (`entity+0xca8`) to `1` and `fx[0]` (`+0xc80`) to the
  chosen node, so `Ship_Damage`'s two random picks are both index 0 and
  `ShipCollisionFx_Trigger` runs once, on that node. A screenshot is taken at
  every frame after the post. Locator numbers are the list order: 0 and 1 the
  wing roots, 2 and 3 the nose, 4 and 5 the tail, and ours are the same indices
  (the world positions agree to the craft scale, `0.75`).
- **Ours**, a scratch hook (not committed; text in
  `scratch-hooks.patch`) that calls
  `cannon::direct_hit` and `throw_hit_sparks` at a named tick with the picks
  forced the same way, `--race --mode single_race --ticks N --screenshot`.
- **Metric.** Struck frame minus the *same run's* unstruck frame (the craft is
  stationary), Rec. 709 luminance, mean over the hull box
  (`x 280..700`, `y 250..544` of 960x544), the HUD rectangles masked. Sums over
  the eight frames that hold the flash: original `k4..k11`, ours the first eight
  after the throw.
- **Controls, to keep the camera out of it.** `Camera_ArmShake`
  (`0x08878750`) is overwritten with `jr ra` on the original and skipped on
  ours: the numbers do not move (the shake is worth under 1 unit of mean
  luminance). `Bloom_Draw` (`0x089075c0`) likewise: the original's spark
  luminance is unchanged with the bloom gone (within 2 units), and ours with
  `bloom = false` moves by 1. Neither is a cause.

**What would have falsified it.** A gap that survived matching the camera view,
the pick sequence and the particle ages would have put the cause in how a
sprite is blended or sampled. It did not survive; the remaining difference is
the opposite sign and small.

## The three causes

| Step | Sum, original / ours, locators 0-3 | Why |
| --- | --- | --- |
| The earlier comparison | 1.8-2.4x | the original was in `OPT_CLOSE`, the port's default is `OPT_FAR`: the ship drew 195 px wide against 140 |
| Same view (`OPT_FAR`) | 1.43-1.54x | a template's first draw was one tick late |
| First draw at age 0 | 1.26-1.35x | the quad was square |
| Stretch and roll played | **0.89-0.93x** | ours is now 7-11 % *brighter* |

1. **Different camera views were compared.** A fresh PPSSPP profile
   here cycled `OPT_CLOSE` -> `OPT_FAR` -> `OPT_INT` with SELECT, and the profile
   that had been copied from the earlier lane sat on `OPT_CLOSE`; the port's
   setting was `far` (the project's default until 2026-10-01; the default is now
   `close`, which is what a fresh original profile starts on - see
   [camera.md](../ghidra/functions/psp-pulse-usa/camera.md#the-default-view-is-opt_close-measured-2026-10-01)
   - so a comparison against an `OPT_FAR` capture passes `--camera-view far`).
   With the craft 1.4x larger on screen, the same world-space sprites cover 2x
   the pixels. Every measurement on this page has the original on `OPT_FAR`
   (press SELECT twice from `OPT_CLOSE`).
2. **A template particle was drawn one tick too old.** The original's first
   draw of `shazam` is half-size `9.36` and of `glow` `0.75`; the port's was
   `9.36` (the second) and `3.18`. `ParticleSystem_UpdateParticleFields`
   samples the channels *before* taking the tick off the particle's life, and a
   particle made in the game logic is updated once in the frame it is born, so
   its first draw is age 0. `psys` now skips the first tick's ageing for a
   template particle (`Particle::fresh`).
3. **The sprite is not square.** `+0xf0` of the template record, which the
   parser had set aside as "constant, unread", is the quad's aspect: `0.5` on
   `shazam`, `0.7` on `glow`, read live as `1.500` and `1.700`, and the
   `glow`'s keyed roll channel turns it `2 pi` downward over its life
   (`6.28, 5.60, 4.81, 4.13, 3.35, 2.65, 1.91, 1.24`, live). Played as
   `ParticleSystem_DrawRotatedSprite`'s quad, half-height `size`, half-width
   `aspect * size`, turned by the roll. Law and evidence:
   [`particle-system.md`](../ghidra/functions/psp-pulse-usa/particle-system.md),
   "The per-tick field update"; format: [`pob.md`](../formats/pob.md), "The
   sprite templates".

**Confidence.** The stretch and roll law and the first-draw-at-age-0 rule are
85 (`ParticleSystem_UpdateParticleFields` read in full, and every field matched
live on four effects: the collision sparks, and the Mine's `BANG`, which tests
the `1 + v` law on the one keyed stretch on the disc - aspects `2.130, 5.272,
8.468, 11.623, 14.763` at ticks 0..4 against the channel's own keys, pinned in
`pob_initial_particles_ground_truth`). The 0.89-0.93 is a measurement of the
port against the original on one race, one craft, one camera view.

**The earlier look comparisons are retracted for the same reason.** The Plasma,
Mine, Quake and collision-spark "re-shot against the original" frames of the
previous lane were taken on a profile sitting on `OPT_CLOSE` against the port's
`far`. Shapes survive that (an aspect does not depend on camera distance), which
is how the Mine's `BANG` bars were checked above; washes and the `0.6 s` flash
brightnesses do not. The Mine's and Plasma's whole-frame brightness against the
original is **not** re-measured: ours reads whiter than the original's frames
around the detonation, before and after this change.

The smoke, the fountain and the embers were never the gap: with every
`ParticleSystem_DrawParticle` call skipped on the original (which leaves exactly
those layers) the struck-minus-control luminance is 1-10 against 20-55 for the
whole effect, and ours without its templates is of the same size. About nine
tenths of the original's flash is the two template sprites.

## Result

Per locator. The original was captured on three boots of the same race for
locators 0-3 (sums repeated to within 1 unit, `L0` 230 and 231, `L2` 92 and
94); locators 4 and 5 were clean once, in the second of two runs (**seen once**;
the first, in a run with a shorter gap between hits, drew nothing for locator 4
and a negative difference for locator 5 late on, unexplained - the locator's
previous instance or cooldown is the suspect):

| Locator | Original | Ours | Original / ours |
| --- | ---: | ---: | ---: |
| 0 | 230 | 253 | 0.91 |
| 1 | 252 | 270 | 0.93 |
| 2 | 92 | 103 | 0.89 |
| 3 | 97 | 108 | 0.90 |
| 4 | 318 | 351 | 0.91 |
| 5 | 321 | 358 | 0.90 |

The frame-by-frame shape agrees too: original `L0` `45 49 53 43 18 6`, ours
`46 52 56 45 22 11`.

**The later hits.** Six hits six frames apart, one locator each, the same six
locators on both sides (3, 2, 1, 0, 4, 5): original / ours per hit
`0.95 0.82 0.92 0.88 0.87 0.81`. The "2.6x weaker later hits" was the two sides
throwing their random picks at different locators, plus the three causes above;
with the picks forced the same, ours is never the weaker one. Frames:
`seq-cmp.png` (top the original, bottom ours).

**What is left.** Ours reads 7-11 % brighter on a single hit and up to 20 % on
the sixth, where the flashes overlap and the framebuffer saturates. Not
decoded. The per-tick draw law above is measured on *templates*. **An emitter's
own particles are played the same night they were read**: a different routine
(`ParticleSystem_DrawRolledQuads`) with a different law - roll always a rate, the
aspect a constant `+0x4c8`, the quad the turned unit square scaled in screen
`x`. The collision sparks' smoke root (random `0..0.105` rad per tick, a random
start and a coin) now turns; the struck craft's numbers above were taken
**before** that, and the smoke was never the gap (see the paragraph before
"Result"), so they are not re-measured here. Confidence 80 on the law, from the
instructions alone, **not** yet measured live.
