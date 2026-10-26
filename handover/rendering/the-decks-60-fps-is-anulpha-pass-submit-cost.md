---
status: measure
titles: [pulse, hd]
updated: 2026-10-26
summary: "Deck HD Anulpha Pass, moving race: total 10.1 -> 9.5 ms, fps 75 -> 83 after two submit cuts; about 1 ms more to hold 90"
categories: [rendering, tooling]
---

# The Deck's 60 fps is Anulpha Pass's frame-thread cost, not pacing

2026-10-26 (`fullscreen-pacing`, then `deck-anulpha-submit`). Numbers, recipe and profile:
`docs/rendering/frame-thread-cpu.md` ("HD Anulpha Pass on the Deck").

The maintainer's Steam launch (HD, Anulpha Pass, human-flown, `vsync = "smooth"`,
`frame_limit = "90"`) sat at 60 fps, p99 22.3: frames over 11.1 ms alternate one and two vblanks.
**Measure with `--no-intro`**: without it `--race` sits in the pre-race flyby (ticks 0.06), which
holds 90 and hides the cost. A moving autopilot race reproduces the maintainer's numbers.

Landed: the binding arrays hold only the slots a drawable's draws name (pads bound 269 textures
each), and projectile pool slots share one buffer per pool (6,235 buffers alive to 907; wgpu sizes
every pass's scope to that count). Deck, moving Anulpha: submit 5.33 -> 4.70, total 10.12 -> 9.49,
fps 75.5 -> 82.8, p99 19.8 -> 16.1. Default circuits gained too (HD total 5.66 -> 5.22, Pulse
5.00 -> 4.19).

## Open

- About 1 ms more on Anulpha to hold 90. The fix2 profile: `encode_render_pass` about 36 % of the
  frame thread (per-draw `flush_bindings` 7.8 %, `set_bind_group` merge 6.5 %, `begin_render_pass`
  4.5 %); scene prep: per-craft env-probe writes and sun-occlusion passes (8 craft, 2 passes each).
- About 54 render passes a frame: count them by label (a patched local wgpu-core is the quickest
  way) and merge or skip the empty ones.
- Sun occlusion walks every track draw for each of 8 craft per frame (`within`, a `Vec<&DrawCall>`
  per craft): same set found more cheaply.
- `probes::decode` 1.4 % self: the decoded cache clears whole at 64 entries; count its hit rate.
- Ticks of 1.0 ms in a human-flown race: matches the autopilot's 0.9, so not a separate cost.
- Pulse's post-chain step (any one of SMAA, medium blur or `psp-3000` off removes the same 1 ms of
  submit): look in `oag-post`.

## Next Steps

1. On the Deck under `flock -o ~/.cache/oag/deck.lock`, the recipe in the doc section, two
   interleaved runs per build: `--race --autopilot --no-audio --frame-log --seed 1 --no-intro
   --script 30:quit --track 'Data\Environments\15_Anulpha_Pass\track.vex' --mode single_race`.
2. Profile: prefix `perf record -g --call-graph fp -D 15000 -F 999` with a build carrying
   `-C force-frame-pointers=yes`.
3. Acceptance: fps 90 and p99 near 11.1 ms in that moving run; default circuit and Pulse no worse.
