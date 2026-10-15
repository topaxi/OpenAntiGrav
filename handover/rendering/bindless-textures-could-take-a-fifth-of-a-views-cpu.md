---
categories: [rendering]
---

# Bindless textures could take up to a fifth of a view's CPU, on native only

2026-10-15, drive lead, from `encoder-finish`'s ceiling measurement. Not started; this thread
is the plan, written so the lane can be picked up without re-measuring the ceiling. Related:
[wgpu-levers-for-per-view-cpu-none-used-yet.md](wgpu-levers-for-per-view-cpu-none-used-yet.md)
(this is its lever 4) and
[split-screen-cpu-is-the-drivers-encoder-finish-now.md](split-screen-cpu-is-the-drivers-encoder-finish-now.md).

## The ceiling

`encoder-finish` built a timing-only hack that never re-switches bind group 1 (the
per-draw texture group; the picture is wrong, the timing is the point). NVIDIA laptop dGPU,
pinned P-cores, quiet machine, the second view of a 2h split:

| | `encoder.finish` | `render_viewports` CPU, 2h |
| --- | --- | --- |
| Pulse | 216 -> 96 us | 1,605 -> 1,297 us |
| HD | 273 -> 98 us | 2,283 -> 1,814 us |
| Pure | 133 -> 39 us | 680 -> 420 us |

So per-draw texture bind-group switches are 55-65% of `finish` and 20-25% of a view's CPU.
That is the most any texture-array or bindless scheme can win; a real one wins less (it still
writes an index per draw). Skipping the 11 group-2 sets instead moved `finish` by only 3-4 us,
so group 2 is not the cost. Measured after `encoder-finish`'s same-texture batch merge (HD
1,447 -> 969 draw calls a view), so the ceiling is over what is left, not over the old count.

## Why the cheap routes are closed

- **Sorting draws by texture** changes pixels: coplanar surfaces resolve by draw order (the
  2026-10-03 coplanar-tie note), so the order is part of the picture.
- **`texture_2d_array`** needs one size and one format per array; the discs ship mixed sizes
  and formats per circuit.
- **An atlas** breaks repeat wrap, which track and scenery UVs rely on.

## The route that is open, and what it costs

`wgpu::Features::TEXTURE_BINDING_ARRAY` plus
`SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING`: one `binding_array` of
textures (and samplers) per drawable or per scene, bound once, with each draw passing its
index (an instance-index slot, or a push constant where `PUSH_CONSTANTS` is available).

- **Native only.** WebGPU has no binding arrays, and phones may not expose the feature, so the
  per-draw group-1 path stays as the fallback: two mechanisms, selected by the device's
  features at load. The Steam Deck (Vulkan, RADV) is expected to support it - unchecked.
- **Every title's mesh shader changes.** Group 1 has 5 bindings including a per-material
  sampler, so the samplers go into an array too (or are deduplicated to a small fixed set).
- The GL backend trial (`gl-backend`, 2026-10-15) added a GL variant of the mesh shader; a
  bindless variant must not multiply that into a third copy - shared WESL modules, one
  selection point.

## Open

- Whether the Deck's RADV exposes both features under wgpu 30 (check `adapter.features()` on
  the Deck before building anything).
- The real win against the ceiling: build it for HD's circuit first (most batches), measure
  `finish` and the view's CPU, pinned and interleaved, before converting other drawables.
- Whether the per-material sampler set is small enough to deduplicate (count distinct sampler
  descriptors per title).

## Next Steps

1. On the Deck: log `adapter.features()` for `TEXTURE_BINDING_ARRAY` and the non-uniform
   indexing feature, and the binding-array limits.
2. Census the distinct sampler descriptors per title.
3. HD circuit drawable only, behind the feature, fallback unchanged; pictures byte-identical on
   both paths; numbers against the ceiling above.
