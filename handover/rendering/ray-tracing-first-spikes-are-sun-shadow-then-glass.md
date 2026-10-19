---
categories: [rendering]
---

# Ray tracing: first spikes are a traced sun shadow, then glass reflections

2026-10-19, `raytrace-spikes`, on its own branch (not merged). Evidence and ranking:
[docs/rendering/ray-tracing.md](../../docs/rendering/ray-tracing.md). Only the read-only probe
`crates/gpu/examples/ray_query_probe.rs` was written. Every cost number on that page is an estimate.

wgpu 30.0.1 exposes `EXPERIMENTAL_RAY_QUERY` on Vulkan, DX12 and Metal (not GL, not WebGPU), and this machine's RADV
reports it on both the 7800 XT and the Raphael iGPU. `shadows.md`'s "effectively Vulkan-only" is out of date. The
feature needs an `unsafe` token under a workspace that denies `unsafe_code`.

## Open

Built on the branch 2026-10-19 (lane A and spike 1, numbers in the docs page): the structures, `--ray-debug`, `--ray`,
`graphics.shadows = traced`, ADR-0072 for the one scoped `unsafe`.

- `traced` costs +1.0 ms p50 and a +5 ms p99 over `mapped` on the 7800 XT and about +11 ms p50 (57 ms p99) on the Deck.
  Open views look worst; testing that, and whether a shorter `TRACED_REACH` or a half-resolution ray with a bilateral
  upsample fixes the tail, is the next thing to try.
- An acceleration structure's size is unmeasured (no safe wgpu query; needs a hal `get_acceleration_structure_build_sizes`
  call, one more scoped `unsafe`). Compaction (`ALLOW_COMPACTION`) is not used.
- 2048 was not run (PFS-encrypted `.pkg`); Omega builds but its `traced` frames were not judged. Pure's sun is unchecked.
- The notebook (Intel, NVIDIA) and the Galaxy S24 have not run it.
- The menu offers `traced` everywhere and falls back to `mapped` with a log line; greying it by adapter, and applying on
  restart with the RENDERER row's note, are not done. Cutout fences cast solid. Hull flaps and wrecks are not in the
  structures (hulls at rest pose).
- Branch only: not merged to main. Merging needs the ADR re-read and a decision on the `unsafe` exception.

## Next Steps

1. Look at the traced tail: time a view looking at open sky against a tunnel, then try a shorter reach or a lower
   resolution ray.
2. Glass reflections (HD Vineta K, Talon's Junction floor), after the original's own glass paths are done.
3. The notebook and APK probes.
