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
4. **Quarter is available when the extent's ceiling gathers at least 540 lines**
   (`BlurResolution::for_lines`, `QUARTER_MIN_LINES`), so a ceiling of 2160 render lines or more,
   960x540 at 4K, the size half gathers at 1080p. Under that ceiling the gather is half, at the
   player's strength. Nothing is written, so a profile's `quarter` under such a ceiling keeps
   saying `quarter` on the row and draws `half`.
5. **Quarter stays drawn while the extent falls beneath it.** The rung exists to shed before
   pixels, so a fall must not draw a dearer gather. The drawn gather is a function of the rung's
   depth and the ceiling, never of the extent now. **Gathers under 540 lines do occur, once the
   extent has fallen**: 513 lines at the first step of a 2160-line ceiling, 270 at the default
   `minimum_resolution` of 50 %, 135 at the lowest row, 25 %. The rule is about what a player's
   machine is offered when it can hold the ceiling, and 513 lines is not meaningfully coarser than
   540. Nobody has looked at the 270-line case; it is the first thing to look at on the Deck.
6. **At quarter the effective strength is capped at `low`** (`BlurResolution::shutter_cap`). It is
   a cap on the shutter where the chain is drawn (`Scene::encode_motion_blur`, the one place every
   path meets), so it never raises a lower strength and never reaches a setting. Under the
   controller it eases across `CAP_EASE_SECONDS`, 0.25 s, **chosen**, in and out. With no
   controller running (a capture, a bench, a held profile) it applies at once.
7. **A restore is never read off a gather the cap is moving under.** The run does not count, and
   the ratio is not learned, until the cap has settled: a quarter gather drawn at `high` is not
   the gather a settled one is.

8. **The strength follows the frame rate** (`drs::ramp`, 2026-11-06, the maintainer's decision):
   the drawn shutter is the player's strength, under the cap above, times one factor, 1 at 60 fps
   and above, falling linearly in fps to 0 at 30. Both ends are **chosen, not measured**. It is
   driven by smoothed frame time (the median of seven frames, then an average of 1/32 a frame), in
   a window, during a running race, with or without a target. At 0 the chain and the velocity it
   reads are not drawn. It returns on the rung's law: the frame predicted within 0.95 of the 30 fps
   period with the chain's last measured cost added, for 3 seconds, doubling on a relapse.

   **The cap stays a cap rather than becoming a multiplier.** A multiplier needs the player's
   strength in `Limits`, and `chosen * (low / chosen)` is not `low` to the bit, which a paused
   frame must be. `Controller::blur_shutter` folds both into the one number the chain reads.

Compare and capture runs use `--motion-blur off`, or the profile's own gather with no controller,
so they are unchanged. Split screen draws no blur.

## Alternatives

- **Judge the 540 lines on the current extent.** It never gathers under 540 lines, but a fall from
  a 2160-line ceiling draws half again at the first step: the blur gets dearer as the extent falls,
  which is backwards for a rung that exists to shed before pixels, and the extent falls further
  than it needed to. Built first and measured: Pulse 4.67 to 5.03 MP and 3 to 5 flips against 5.77
  MP and 2.
- **A ramp on factor steps (`low`, `medium`, `high`).** Rejected by the maintainer: it does not
  refer to them, and steps would hit the frame rate they are judged on.
- **A second threshold table for the step.** Rejected: the margins are derived from the deadband,
  not tuned, and nothing measured asks for different ones.
- **Dropping the composite's depth test at quarter, or a wider footprint.** Not tried: crops at the
  cap show no background crossing a silhouette, only the stepped fringe under Consequences.

## Consequences

- **It pays inside and below its band.** On the iGPU at 4K, target 30, quarter does not fit at the
  ceiling, and the extent falls with quarter still drawn: Pulse 5.77 MP at a median 1836 lines
  against 4.67 to 5.03 MP at 1620 on the current-extent reading, Pure 5.99 against 4.67, same fps,
  2 flips against 3 to 5. The numbers are in the half-gather page. A closed-loop test holds the
  flips at 2 under the same load (`quarter_stays_drawn_as_the_extent_falls_and_settles`), and at 4
  across 40 race weights and three swings (`no_race_weight_at_a_2160_line_ceiling_goes_round`).
- **Strength is not free, so the ramp is a feedback loop**, and a target at 30 is a blur that is
  never drawn on a machine that needs the target: on the iGPU at 4K, Pulse, target 30, the chain
  went off once and the race ran 8.1 MP at 47.8 fps against 5.77 MP at 15.6 to 30 with the chain.
  The freed cost goes to pixels, the chain's measured cost keeps the return out, and a player who
  chose `high` sees none. Whether none at 30 is right is the maintainer's to judge on the Deck.
- A rate just above 30 pays the chain's whole cost for a smear too short to see.
- Most of the saving is the strength cap (5.3 of 8.5 ms on both titles): a player gives up the
  smear they chose, and a player already on `low` saves 15 % rather than 33 %.
- `Limits` carries no strength: the cap eases from `high`, the most anyone draws, so a player on
  `medium` sees it move for less than the full ramp and one on `low` not at all.
- The ramp means the first 0.25 s at quarter draws a longer smear than `low`, where the bleed the
  cap exists for could show. Judged on stills only, not in motion.
- The Steam Deck at 1280x800 and at its default 4K budget (1906x1080) never reaches 2160 lines, so
  the step is inert there; only a native 4K opt-in sees it.
- One more target and one more bind group: 960x540 of `Rgba16Float`, about 2 MB at 4K.
- A dark surface's own smear steps at the 4 px texel: a faint 8 px band under a thin wing on HD,
  finer at `low`+`half`. It holds still and takes looking for at native size.
- The strength a player sees drops at quarter. That is the step's price and part of what it saves.
