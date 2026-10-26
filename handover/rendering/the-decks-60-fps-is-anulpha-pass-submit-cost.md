---
status: measure
titles: [hd]
updated: 2026-10-26
summary: "Deck 60 fps / 8 ms CPU row is HD Anulpha Pass: 5 ms submit in wgpu-core texture tracking of the binding-array group; cut it to hold 90"
categories: [rendering, tooling]
---

# The Deck's 60 fps is Anulpha Pass's submit cost, not pacing

2026-10-26 (`fullscreen-pacing`). Background: `docs/rendering/frame-thread-cpu.md` ("On the Steam
Deck") and `docs/rendering/frame-pacing.md` ("The Steam Deck's 60").

The maintainer's Steam launch (HD, Anulpha Pass, single race, human-flown, `vsync = "smooth"`,
`frame_limit = "90"`, 1280x800) sat at 60 fps, p99 22.3 ms, CPU row 8.8 ms
(`cpu_ms` 15.5 minus `present_ms` 6.7), `frame-phases` ticks 1.0, scene 2.5, submit 5.1,
total 9.4. Frames over 11.1 ms alternate one and two vblanks at 90 Hz: 60 fps, spiky graph.

Native on the same Deck (433c4a0-equivalent build, maintainer's settings copy, autopilot,
`--race --track 'Data\Environments\15_Anulpha_Pass\track.vex' --mode single_race`): submit 5.0
to 5.3 ms, total 7.5 to 7.8, still 90 fps. On `--race`'s default circuit submit is 2.5, total 5.1.
The 6.6 ms present wait is normal under gamescope (also at a flat 90). Fullscreen queue depth
does not matter there (gamescope's WSI keeps three images either way: byte-identical frame logs).

`perf record -g --call-graph fp -F 999` of the frame thread on Anulpha Pass (Deck): the top
self entries are wgpu-core's `TextureUsageScope::merge_single` 5.2 %,
`CommandBufferTextureMemoryActions::register_init_action` 4.3 %,
`TextureInitTracker::check_action` 3.7 %, `BufferUsageScope::clear` 2.6 %,
`UsageScope::merge_bind_group` 2.2 %: per-element work each time a binding-array group is bound
in a pass (`crates/mesh/src/mesh_render/bindless.rs` describes the per-element cost). Anulpha
Pass's track has 593 material slots.

## Open

- Anulpha Pass's submit: count how many times each drawable's group 1 is (re)bound per pass and
  how many elements each carries; fewer rebinds or shorter arrays (one group per pass, split per
  drawable) is the lever. `crates/mesh` was off limits to this lane.
- Ticks of 1.0 ms in a human-flown race against 0.06 to 0.2 under autopilot: what a played race
  adds per tick (weapons, AI, sound cues). Not measured.
- Pulse's post-chain step (any one of SMAA, medium blur or `psp-3000` off removes the same 1 ms
  of submit): look in `oag-post` (intermediate targets, init actions).

## Next Steps

1. On the Deck, under `flock -o ~/.cache/oag/deck.lock`, native on `DISPLAY=:1` with the
   gamescope env (`ENABLE_GAMESCOPE_WSI=1 GAMESCOPE_WAYLAND_DISPLAY=gamescope-0`), private
   `XDG_*`, maintainer settings copy with `pause_on_focus_loss = false`:
   `oag-game <hd image> --race --autopilot --no-audio --frame-log --seed 1 --script 40:quit
   --track 'Data\Environments\15_Anulpha_Pass\track.vex' --mode single_race`; read `submit`
   in `frame-phases` (before: 5.0 to 5.3 ms).
2. Prefix `perf record -g --call-graph fp -D 20000 -F 999 -o F` for the profile.
3. Acceptance: total under about 6 ms on Anulpha Pass, so a played race (plus about 1 ms of
   ticks and 1 ms of scene) stays under 11.1 ms and holds 90.
