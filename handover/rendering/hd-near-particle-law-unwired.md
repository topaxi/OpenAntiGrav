---
status: ready-to-wire
titles: [hd, omega, 2048]
updated: 2026-11-25
summary: "HD's near-camera particle law (cull under 1, fade to 4, size cap +0xc80) is recovered, not wired: ours draws 2.6x the fill at a near Rocket hit"
size: M (60)
difficulty: medium (65)
---

# HD's near-camera particle law is recovered and not wired

A Rocket hit near the camera costs ours 52.7 screens of blended particle fill at the worst frame
(`docs/rendering/particle-fill.md`). The original draws the same hit with about 20: its
billboard builder culls, fades and caps particles by view depth
(`docs/ghidra/functions/ps3-hdfury-eu/particle-render.md`, confidence 88 to 94, read in HD's SPU
job, in Omega's CPU twin, and on live RPCS3 vertices). It has **no** reduced-resolution particle
buffer: the question "does the PS3 draw particles at half resolution" is answered no
(confidence 90).

## Open

### The ranked plan

Gains are GPU scene time on an RX 7800 XT at load 47 to 60, median of six interleaved pairs,
from a throwaway patch (not kept; particle-fill.md describes it). Deck numbers are estimates:
nothing was run there.

| # | Change | Faithful? | 4K worst frame | 1080p worst frame | Deck | Effort | Confidence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | **Wire the near law**: cull under depth 1, fade `(depth - 1) / 3` to depth 4, size `min(size, depth * cap)` | **Yes**, the original's own rule, and a fidelity fix: ours whites the screen out where the original shows a bounded cloud | 9.65 to 5.28 ms (**-45 %**); window mean -16 % | 3.29 to 3.15 ms (-4 %) | fill at the peak -61 % (52.7 to 20.4 screens); expect the hit's frame-time spike roughly halved at 4K (50) | M | 85 that the law is right, 85 on the 4K gain |
| 2 | **HD's width throttle as the `original` value of dynamic resolution**: scene width `1 - level / 96` of the output (1920 to 1300 of 1920), one level a frame, narrower at 16.1 ms of GPU frame or a missed vblank, wider under 15.6 ms | **Yes on HD**, it is `Render_FinishFrame`'s controller. An option, off unless chosen: our default is a fixed scale | up to -32 % of every scene pixel under sustained load; a 9 frame blast sheds at most 9 % | the same ratios | where it matters most: a Deck at 4K is over budget all race, so it would sit near the floor (-32 % of scene fill) | M | 60 on the gain, 90 on the law |
| 3 | **Particles into a half-resolution target, composited** | **No.** The disc authors a per-distance resolution mode (`+0xc8c`, thresholds 17 and 35) and HD computes it, but its flush ignores it. Opt-in setting, default `original` (full resolution) | after 1: about 20 screens of fill become about 5 plus one full-screen composite; estimate 5.3 to about 3 ms (40) | small | the largest single cut of what is left (40) | L | 40 on the numbers; edges where smoke meets hulls need a depth-aware upsample |

Do 1 first: it is the only one that is both free of a setting and a picture fix. 2 and 3 are
independent of it and of each other.

### Wiring item 1

- `oag_pob::Emitter` gains the cap: `f32` at `+0xc80`, on the titles whose record reaches it (HD,
  2048, Omega; `record.len()` is the rest of the file, so gate on the title's record length, not
  on the slice). The PSP and PS2 records end at `0x9b8`: no cap there.
- `oag_fx::psys` builds billboards from `right` and `up` only (`layers.rs`, `extend_layer`,
  `extend_shaded`; `distort.rs`). The law needs each particle's **view depth**, so the frame
  build needs the view matrix's z row (or eye and forward) per view. Split screen builds per
  view already.
- Apply to render modes 1 to 4 (our billboards and rolled quads), every blend class, the
  distortion quads included (Omega's builder is the one routine for all of them). Not to the
  streak classes: they have their own depth rule and read no cap.
- The cap acts on the particle's size before the aspect (`+0x4c8`) widens it.
- The fade multiplies the alpha the pipeline already carries. `drop_dead_quads` then removes
  nothing new (the fade reaches 0 only at depth 1, where the cull has already dropped the quad).
- Titles: HD measured. Omega the same code (static), wire it with HD and say so. 2048 inherits
  through Omega (project rule) until its own builder is read. Pulse and Pure: not read, leave
  as they are.
- Tests that fail if the law is dropped: the cap at a depth where `size / depth` exceeds it and
  one where it does not; the fade at depths 1, 2.5, 4; the cull at 0.99; a disc-backed check
  that `WO_ROCKET_EXPLO`'s root reads cap 0.5 and `SMOKE_BLUCLD` 1.0.
- Regression gate: the reproduction in particle-fill.md, coverage or `gpu_scene_us` at tick 775
  before and after, and the state hash unchanged (the law is drawing only).
- Every HD and Omega pair with a near particle moves. Re-render the weapon and blast pairs.

### Owed measurements

- **A matched original frame of a blast at close range.** The live capture that proved the law
  had its blasts 70 to 350 units away, so the proof is on the player's own near effects. A pair
  with the camera inside a CraftBlast is the picture check for item 1: fire at `GO` with a rival
  directly ahead and teleport forward (`rpcs3-hd-weapon.py --teleport-back` with a negative
  distance), or use the missile film's pose.
- **The Deck** at 3840x2160, 1920x1080 and 1280x800 (vsync off, no frame limit): the
  reproduction's `gpu_scene_us` over ticks 760 to 789, before and after item 1. Still not run.
  The fragment `discard` of particle-fill.md's "Measured and dropped" belongs to the same visit.
- **About 1.7 ms of a hit frame at 1080p that is not fill.** With and without the law, ticks 770
  to 774 cost 3.26 ms against 0.75 ms before the hit, and at that size it does not move when the
  covered area halves. Unprofiled: leave the particle draws, the blast's light and the flash out
  in turn at 1920x1080.
- **2048's quad builder** (does it carry the law, and with which constants) and **Omega's
  passes 8 to 10 by target size** (is one of them a reduced particle target). Both are reads.
- **The PSP builds' builder**: whether Pulse and Pure have a near rule at all.

## Next Steps

1. Wire item 1 as above (an `oag-wire` lane; the RE is done).
2. Shoot the close-range pair and judge item 1's picture against it.
3. Run the Deck numbers before and after.
4. Put items 2 and 3 to the maintainer as settings once item 1's numbers are in.
