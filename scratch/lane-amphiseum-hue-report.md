# lane-hd-amphiseum-hue: per-region re-measurement after the maintainer's "both, varies by area" answer

2026-09-17, continuation of `lane-hd-track-lighting` (merged into main as
`016ef0c6`, then this follow-up spun up as its own worktree/branch by the
lead - `oag-lane-lighting` was already reaped by the time I went to
continue, so this session picked up in the fresh `oag-lane-hue` worktree
instead; no work was lost except a handful of uncommitted script edits from
right before the switch, redone here).

## What the lead asked

The maintainer, asked directly whether Amphiseum reads brighter or darker
and whether the complaint is brightness or colour: **"Both, and it varies
by area."** Three asks: (1) measure per region, report sign per region,
say how much the frame aggregate moves with framing; (2) get Amphiseum's
region boxes right, verified by `--dump-regions`; (3) carry hue/saturation
per region, not just luma.

## What I did

Rather than hand-picking new semantic boxes (wall panel, gate, signage -
every attempt at that so far has turned out to sample the wrong thing on
this circuit), added `--tiles ROWSxCOLS` to `hd-frame-compare.py`: a plain
box-free grid, so there's no claim about content, only about position.
Ran `--tiles 4x6` on the same 0 km/h grid pose from the prior session,
bloom on and off, on both Amphiseum and (for contrast) Talon's Junction.

## Headline result

- **Amphiseum's gap is vertical.** Top half of the frame (ceiling, arch
  beams, crowd stands) reads brighter in our render: px-weighted
  ref-ours = **-0.121**. Bottom half (barrier chevrons, floor near camera)
  reads darker: **+0.100**. That's a **0.22 luma swing from vertical
  framing alone** - bigger than the 0.053 bloom-driven swing found last
  session. Holds with bloom off too (same split, cells move by 1-2
  hundredths). The two halves' weighted sums reproduce the previously-
  reported whole-frame "-0.045" number exactly - it was never a separate
  measurement, just this same split averaged away.
- **Talon's Junction's tile grid, run for contrast, is close to uniform**:
  20 of 22 valid cells "ours darker", hue gaps under 20 degrees almost
  everywhere. Confirms the existing "brightness-only, global" reading in
  concrete per-cell form rather than inference from one aggregate.
- **Hue: reference varies by row, ours does not.** Reference reads warm
  (39-86 deg) at the ceiling, cool (155-198 deg) at the floor - two real
  material colours. Our render holds one blue-violet family (217-305 deg)
  everywhere. Reads as "doesn't reproduce two materials' colour
  difference" rather than a uniform tint - a lit-material/lightmap gap
  correlated with vertical position (ceiling vs. floor materials), not a
  scene-wide colour cast.

## What I did NOT do

- No new semantic region boxes (wall panel / gate / signage) - the tile
  grid answered the "per region, correct boxes" ask without needing them,
  and hand-picking more boxes felt like repeating the same mistake
  (`sky` samples ceiling, `road surface` samples a wall) rather than
  fixing it. `--dump-regions` still works for the existing named boxes;
  a labelled tile overlay image was not built (row/column labels are
  printed as text, e.g. `r0c0`, not drawn on a saved PNG).
- No `.rcsmaterial` mapping of which materials draw into which row
  (`hd-material-probe.py` territory, flagged as the natural next step).
- No `mesh.wgsl`/`mesh/`/`emissive.rs`/`sky_cube.rs` changes - still
  read-only this session, per lane scope.

## Gate

No `.rs` changed (script + docs only again). Ran `just check-docs` only,
same reasoning as last session - the `just` gate has zero coverage of
`scripts/*.py`. Real verification: 4 successful `hd-frame-compare.py`
runs producing the tile tables above, checked by hand against a labelled
grid overlay of the reference frame (`scratch/amphiseum-tile-grid-overlay.png`,
this worktree, not committed).

## Commit

`d3718b48` on `lane/hd-amphiseum-hue` - renderer.md + handover thread +
`hd-frame-compare.py`'s `--tiles` addition.

## Still open

- Map tile rows to `.rcsmaterial`s.
- Whether Amphiseum's bloom chain over-contributes vs. the original's
  (flagged last session, not reached this one either).
- `track_surface`/`prelitScale` lead, still Talon's-Junction-only evidence.
