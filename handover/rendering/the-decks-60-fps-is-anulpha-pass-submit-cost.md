---
status: measure
titles: [pulse, hd]
updated: 2026-10-27
summary: "Deck HD Anulpha Pass holds 90 (fps 74.5 -> 89.9, p99 20.3 -> 11.65, total 10.2 -> 7.8 ms); 3 slow seconds a minute and further submit levers left"
categories: [rendering, tooling]
---

# The Deck's frame thread on HD Anulpha Pass: what is left after 90

2026-10-27 (`deck-anulpha-submit`). Numbers, recipe, profile and the bottleneck explanation:
`docs/rendering/frame-thread-cpu.md` ("HD Anulpha Pass on the Deck").

The Deck now holds 90 on a moving Anulpha race, with the maintainer's settings. In 60 s runs,
3 of 54 one-second windows read 89.x, a single double-vblank frame each.

## Open

- The 3 slow seconds a minute: log a frame's phases when `total` passes 10 ms (a few lines in
  `crates/game/src/main/session/phases.rs`, `end_frame`) and see what the slow ones share.
- Further `submit` levers, from the doc section: post-chain pass count (10 bloom downsample, 7
  motion blur passes a frame), shared 1x1 fallback textures (1,276 of 1,863 live textures),
  a third split point.
- Upstreamable wgpu-core changes (init actions for initialised textures, sparse usage scopes,
  no re-merge of a bound group): a patch to propose upstream, not a fork.
- Ticks of 1.0 ms in a human-flown race: matches the autopilot's 0.9 before the trail-hit cut.
- Pulse's post-chain step (any one of SMAA, medium blur or `psp-3000` off removes the same 1 ms of
  submit): look in `oag-post`.

## Next Steps

1. On the Deck under `flock -o ~/.cache/oag/deck.lock`, the recipe in the doc section, two
   interleaved runs per build: `--race --autopilot --no-audio --frame-log --seed 1 --no-intro
   --script 60:quit --track 'Data\Environments\15_Anulpha_Pass\track.vex' --mode single_race`.
2. Profile: prefix `perf record -g --call-graph fp -D 15000 -F 999` with a build carrying
   `-C force-frame-pointers=yes`; `OAG_SPLIT_FINISH=0` for the single-encoder A/B.
3. Acceptance for anything further: no one-second window under 90 in a 60 s run; default circuit
   and Pulse no worse.
