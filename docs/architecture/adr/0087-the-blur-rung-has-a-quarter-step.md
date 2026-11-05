# ADR-0087: The blur rung has a quarter step

## Status

**Proposed (2026-11-05)**, for the maintainer to accept or change once the Steam Deck runs in
`docs/rendering/motion-blur-half-rung.md` ("The quarter step") are in. Extends the order of
ADR-0075, which stands: everything it decided holds for each step.

## Context

At 3840x2160 on a Raphael iGPU the half gather's chain is still about 26 ms of a race's median,
and the extent falls next. The reconstruct is the part of that chain that scales with the gather,
so a coarser gather has something left to save. The risk is the silhouette: the half gather keeps
a hull sharp because its depth-aware upsample picks each side of an edge from a 2 px texel, and a
4 px texel has fewer samples on a thin edge. The maintainer agreed the shape of the step before it
was built. The numbers are in [the half-gather page](../../rendering/motion-blur-half-rung.md),
"The quarter step".

## Decision

1. **`BlurResolution` gains `quarter`**, one texel per 4x4 pixels, on the same reduced pipeline as
   `half` with the divisor in the uniform. Both reduced targets exist at every size, so a flip
   allocates nothing.
2. **The rung sheds full, half, quarter before the extent falls**, and restores one step at a
   time, last, at the extent's ceiling. The player's `motion_blur_resolution` is still the
   ceiling: at `half` only the second step exists, at `quarter` the rung is inert.
3. **One table.** Each step keeps its own measured ratio; the deadband, the patience, the relapse
   window and the backoff are ADR-0075's, shared by the steps. A shed taken before any restore
   was tried reads the step above's ratio on its way down, the only reading it will get.
4. **Quarter is drawn only at 540 gather lines or more**, on the current render extent, not the
   window (`BlurResolution::for_lines`, `QUARTER_MIN_LINES`). That is 2160 render lines, 960x540
   at 4K, the size half gathers at 1080p. Below it the gather is half, at the player's strength.
   Nothing is written, so a profile's `quarter` is a ceiling the extent may not reach: the row
   keeps saying `quarter`, and the frame draws `half`.
5. **The drawn gather is a function of the rung's depth and the extent.** The rung keeps its depth
   when the extent falls, so a fall from a 2160-line ceiling draws half again at the first step
   down, and a climb back to the ceiling draws quarter again.
6. **At quarter the effective strength is capped at `low`** (`BlurResolution::shutter_cap`). It is
   a cap on the shutter where the chain is drawn (`Scene::encode_motion_blur`, the one place every
   path meets), so it never raises a lower strength and never reaches a setting. Under the
   controller it eases across `CAP_EASE_SECONDS`, 0.25 s, **chosen**, in and out. With no
   controller running (a capture, a bench, a held profile) it applies at once.
7. **A restore is never read off a gather the cap is moving under.** The run does not count, and
   the ratio is not learned, until the cap has settled: a quarter gather drawn at `high` is not
   the gather a settled one is.

Compare and capture runs use `--motion-blur off`, or the profile's own gather with no controller,
so they are unchanged. Split screen draws no blur.

## Alternatives

- **Judge the 540 lines on the extent's ceiling.** It keeps quarter on the way down, so there is no
  cost step at the first fall, but it gathers under 540 lines at every lower extent, 513 at the
  first step of a 2160-line ceiling, which is what the rule exists to refuse.
- **A second threshold table for the step.** Rejected: the margins are derived from the deadband,
  not tuned, and nothing measured asks for different ones.
- **Dropping the composite's depth test at quarter, or a wider footprint.** Not needed: crops at
  the cap show no bleed (see the page).

## Consequences

- With a 2160-line ceiling the fall off quarter is a cliff. The extent's first step down draws
  half again, so the blur's cost rises as the extent falls, and the extent may fall a step or two further
  than half alone would have (not visible above run-to-run spread in the sessions). It cannot go round: a fall from the ceiling means quarter read over 0.95
  there, so half does too, and the rise back is refused until half fits at 0.80. A closed-loop
  test holds this at 3 flips across 40 race weights and three swings
  (`no_race_weight_at_a_2160_line_ceiling_goes_round`).
- **It pays only in a band**: where half does not fit at the ceiling and quarter does. On the iGPU
  at 4K, target 30, quarter does not fit, so a race lands where it did without the step, with two
  to four more flips. Inside the band the gain is modelled, not measured; the Deck's HD at native
  4K is the first real case.
- Most of the saving is the strength cap (5.3 of 8.5 ms on both titles): a player gives up the
  smear they chose, and a player already on `low` saves 15 % rather than 33 %.
- For the same reason a machine can sit below the ceiling on half when quarter at the ceiling
  would just fit, until the scene eases: the rise reads the half gather it draws. It is the cheap
  direction and only ever one race stretch long.
- `Limits` carries no strength: the cap eases from `high`, the most anyone draws, so a player on
  `medium` sees it move for less than the full ramp and one on `low` not at all.
- The ramp means the first 0.25 s at quarter draws a longer smear than `low`, where the bleed the
  cap exists for could show. Judged on stills only, not in motion.
- The Steam Deck at 1280x800 and at its default 4K budget (1906x1080) never reaches 2160 lines, so
  the step is inert there; only a native 4K opt-in sees it.
- One more target and one more bind group: 960x540 of `Rgba16Float`, about 2 MB at 4K.
- The strength a player sees drops at quarter. That is the step's price and part of what it saves.
