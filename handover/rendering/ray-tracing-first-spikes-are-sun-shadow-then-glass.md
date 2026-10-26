---
categories: [rendering]
---

# Ray tracing: a traced sun shadow built, glass reflections next - PARKED

**Parked by the maintainer on 2026-10-26 until further notice.** Branch `raytrace-spikes` only, not merged to main.
Evidence, numbers and the design are in [docs/rendering/ray-tracing.md](../../docs/rendering/ray-tracing.md); this
file is what is left open when work resumes.

wgpu 30.0.1 exposes `EXPERIMENTAL_RAY_QUERY` on Vulkan, DX12 and Metal (not GL, not WebGPU), and this machine's RADV
reports it on both the 7800 XT and the Raphael iGPU. The feature needs an `unsafe` token under a workspace that denies
`unsafe_code` (ADR-0072).

## What is built

The structures (BLAS per model, animated node and PVS section; TLAS per frame), `--ray`, `--ray-debug`,
`graphics.shadows = traced`, and `graphics.traced_quality` (`medium`: one ray, faded by the caster's distance;
`high`: three probes, then eight rays across a 2.5-degree sun in a penumbra). The TLAS holds the finest `LodGroup`
tier only, and only the PVS sections the raster shows (`VisibleSet::around`, the union over a split's seats), so a
circuit's far-LOD road copy no longer shadows the detailed road. `TRACED_START` 0.12.

## Open

- **Cost.** RX 7800 XT, single race, GPU median: Pulse `medium` 8.0 ms, `high` 20.3 ms; HD 3.7 and 9.3 ms. The
  maintainer finds `medium` smooth and `high` heavy. NVIDIA (traversal in hardware) is unmeasured and the obvious
  next number; the next design if `high` stays heavy is two rays with a spatial or temporal denoise. On the Deck the
  first single-ray `traced` was +11 ms p50 over `mapped` on HD (57 ms p99): the Deck keeps `mapped`.
- **Pulse paints no cast shadows, so traced scenery shadows contradict its art.** A fit over every Pulse circuit's
  baked vertex colour against occlusion toward any direction found no painted shadow (difference at most +0.07 where a
  shadow would paint about 0.25, 12 base-disc circuits, confidence 75). The maintainer chose on 2026-10-26 to keep the
  still scenery casting under `traced` on Pulse anyway ("more shadows than the original"), and to keep the sun on the
  hull axis (`-oag_pulse::shadow::WORLD_AXIS`, what `original` drops craft shadows along).
- **Pulse does author lights.** Every circuit has 1 to 3 `DirectionalLight` nodes, loaded as `HullLights` for the
  hull (`scene-light.md`). The brightest one fits the painted lighting on Moa Therma (3 degrees off the best fit, R²
  0.65, where the hull axis gives 0.085) and circuit 04 (6 degrees, R² 0.56), weakly on 02 and 16, and not at all on
  01, 05, 06, 07, 09, 10, 13 and the DLC 08/11/12/15; 14 is lit from above; 15 authors none. Switching the sun to
  the key light was offered and declined; it would point craft shadows away from `original`'s.
- **Unplaced viewpoints instance every section** (a respawn in flight, a craft off the line, a posed capture at
  `--ticks 0`), so the far-LOD patches return for those frames, worse at 0.12 than the old 0.6. Keeping the last
  placed mask, or `--pose` locating `Ship::segment`, closes it. Capture posed frames with `--ticks 1 --pose-hold`.
- **A split's union of masks can re-admit a swap pair.** One shared TLAS per frame; per-view TLASes are not built.
- **Rendered on Moa Therma White only.** Pulse circuit 01 (25 swap pairs) is the cheap next check; 02 and 05 also
  have mask-exclusive overlaps that `SwapConflicts` misses.
- **HD, 2048 and Omega are not section-filtered**: their PVS is per chunk (983 to 1,258 chunks, more than
  `MAX_INSTANCES`) with no swap relation, so their circuit BLAS is whole as before. Omega's and 2048's `traced`
  frames were not judged; Pure's sun is unchecked.
- **Switching to `traced` in the menu mid-session draws `mapped`** until the next launch (the device's features are
  fixed when it is made); the only sign is a log line. A menu hint such as `TRACED (RESTART)`, or greying the value by
  adapter, is not done.
- An acceleration structure's size is unmeasured (needs a hal build-sizes query, one more scoped `unsafe`);
  compaction is not used. Cutout fences cast solid. Hull flaps and wrecks are in the structures at rest pose.
- **Side finding for main**: `crates/mesh/src/mesh.rs` leaves PSP `.vex` normals in node-local space for the 0 to 5 %
  of vertices under a non-identity node. Nothing traced reads normals; whatever on main lights by those normals would.
- **The maintainer's question about red and orange lights "reflecting" in a Pulse tunnel** was answered as most likely
  Pulse's own track shine pass (`oag_render::shine`), not rays; not confirmed at a position.
- The probe examples from the investigations are on branches `pulse-sun` (`pulse_sun_dump.rs`,
  `pulse_sun_occlusion.rs`) and `hover-layer` (section overlap sweeps), kept unmerged.
- Merging to main needs the ADR re-read and a decision on the `unsafe` exception.

## Next Steps

1. Close the unplaced-viewpoint fallback (last placed mask), then render Pulse circuit 01.
2. Measure `traced` on an NVIDIA GPU (the notebook) and the Galaxy S24.
3. If `high` stays heavy: two rays with a denoise.
4. Glass reflections (HD Vineta K, Talon's Junction floor), after the original's own glass paths are done.
