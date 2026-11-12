# A circuit's vertex data in more than one buffer (2026-11-12, `big-vertex-buffer`)

Omega's Rockway Stadium (`arena`) and Unity Square (`square`) in WipEout 2048 mode could not
be raced: the race load panicked with wgpu "Buffer size 308016200 (arena) / 310295700 (square)
is greater than the maximum buffer size (268435456)" on the one `vertices` buffer. Omega
re-cooks 2048's meshes about 3.5 times heavier (Altima 726,699 triangles in the frame against
201,323), and wgpu's default `max_buffer_size` is 256 MiB, which is also what a Steam Deck, a
phone and WebGPU report. Both circuits now load and draw on such a device.

## What a circuit's vertex data is now

`oag_mesh::mesh_render::Geometry` holds a model's vertex buffers: **one** buffer for every
circuit that fits `device.limits().max_buffer_size` (all of them but those two), cut into
several for one that does not.

- **The cut is a vertex range, placed where no draw straddles it.**
  `chunks::VertexChunks::plan` gives each draw's lowest and highest vertex, forbids every cut
  inside one, and takes the furthest allowed cut under the ceiling. A cut the planner cannot
  place is `Unsplittable`, which the load reports by name; it is not drawn wrong.
- **The index buffer, the draw ranges and every per-draw table are unchanged.** A draw indexes
  the model's vertices by absolute number; the chunk's buffer starts at its first vertex, so the
  draw is issued with a *negative base vertex* (`-first vertex of the chunk`).
- **A model that fits is drawn exactly as before**: one buffer, bound once by
  `Geometry::cursor`, base vertex 0, no per-draw lookup. A cut model looks its draw's chunk up
  (a binary search over the model's draw starts) and rebinds the buffer only when the chunk
  changes.
- **Every writer goes through `Geometry::write`**, which splits a write at the chunk
  boundaries: the whole-model rewrites (tint, UV transform, overlay, shine), the flaps, the
  pads' recolour, the Quake ripple, the streamed texture coordinates
  (`Geometry::write_texcoords`; the stream is chunked the same way, since a base vertex applies
  to every bound vertex buffer), and the deferred upload (`Geometry::defer`).
- **Every drawer takes the same cursor**: the race's solid, cutout, blended and stamp lists,
  the overlay and shine lists, the shadow depth map, the self-shadow map, the sun-occlusion
  map, and the viewer, preview and countdown draws.

The limit is read from the device, not assumed: a device that grants more than 256 MiB gets one
buffer. **This engine does not raise `max_buffer_size` above wgpu's default** (it could, to the
adapter's), so that the cut path is the path every machine takes for these two circuits and
stays exercised.

## Census: the largest vertex buffer a circuit needs

`crates/mesh/tests/vertex_buffer_census_ground_truth.rs` builds every circuit it can find with
no textures (a vertex count does not depend on them) and prints the table. Vertex stride is
`size_of::<GpuVertex>()` = 100 bytes. MiB of vertex data in one buffer, then the largest
index buffer a circuit needs (4 bytes an index; the prepass's reversed copy is the same size).

| Title | Circuits measured | Largest vertex buffer | Which | Index buffer, largest |
| --- | --- | --- | --- | --- |
| Pulse (PSP) | 12 of its circuits, found by name | 19.1 MiB (200,682 vertices) | `09_Track` | 2.2 MiB |
| Pure (PSP) | 5 of its circuits, found by name | 17.0 MiB | `03_Modesto_Heights` | 2.0 MiB |
| Wipeout HD / Fury | all 16 with a sibling `.rcsmodel` | 112.7 MiB (1,181,248 vertices) | `01_vineta_k` | 11.2 MiB (`zone_4`) |
| Wipeout 2048 | all 14 (10 and 4 DLC1) | 91.0 MiB (954,520 vertices) | `subway` | 7.4 MiB |
| Omega Collection | all 26 with a `track*.rcsmodel` | **295.9 MiB** (3,102,957 vertices) | `environments2048\square` | 38.1 MiB (`arena`) |

**Headroom under 256 MiB**: Pulse and Pure 13x, HD 2.3x, 2048 2.8x. Omega is the only title
with circuits over, and its next four are within 7 to 13 percent of the ceiling, so a few
percent more geometry would cut them too (the cut path handles that):

| Omega circuit | Vertex data | Buffers on a 256 MiB device |
| --- | --- | --- |
| `environments2048\square` | 295.9 MiB | 2 (largest 255.7 MiB) |
| `environments2048\arena` | 293.7 MiB | 2 (largest 255.6 MiB) |
| `environments2048\park` | 239.1 MiB | 1 |
| `environments2048\subway` | 236.2 MiB | 1 |
| `environments2048\tower` | 228.0 MiB | 1 |
| `environments\tech_de_ra` | 223.5 MiB | 1 |

The Pulse and Pure rows are the circuits whose directory name the probe found, not all of
them (a WAD names its entries by hash); both are PSP-sized, so the headroom is not in doubt.

**The other buffers a circuit allocates**, checked for the same limit: the index buffer (largest
38.1 MiB, Omega `arena`) and its reversed copy, the streamed texture coordinates (8 of the
vertex's 100 bytes, so a chunk of them is never larger than a chunk of vertices), the shine
pass's own sub-model (a subset of the track's draws, so never larger), the per-draw and
per-view uniform blocks and the animation tables (tens of kilobytes) and the instance pools.
None comes within a factor of 6 of 256 MiB. The load refuses an index buffer over the ceiling
with a message naming the model, which no shipped circuit reaches.

## Evidence

- **The panic, reproduced then gone.** `oag_mesh` test
  `omega_arena_and_square_build_on_a_device_with_a_256_mib_buffer_ceiling` builds both circuits
  through `mesh_render::build_with` on a device holding exactly 256 MiB: with the planner
  replaced by one whole buffer it fails with `Buffer size 310295700 is greater than the maximum
  buffer size (268435456)`.
- **Cut and uncut draw the same pixels.** Headless `--race --screenshot --ticks 120` of both
  circuits on the default device (two buffers) and on a device raised to the adapter's limit
  (one buffer; a throwaway env switch, not kept): byte-identical PNGs, with the shadow tier
  `--shadows mapped` as well as without. An ordinary circuit (Omega Vineta K, HD Vineta K,
  2048-mode Altima) is byte-identical before and after the change.
- **`raceplay::tests::vertex_chunks`**: a drawable whose strips sit in several buffers draws
  every pixel a one-buffer drawable does, through the main draw, a whole-model write across the
  cuts, and the overlay list; setting the base vertex to 0 in `Cursor` fails all three.
- **`oag_mesh` `deferred`**: a cut model parks the bytes of every buffer for the drain.

## Not done, not checked

- **GL and WebGL2** may lack `DownlevelFlags::BASE_VERTEX`; a cut model there draws with a
  validation error rather than wrong, and GL is best effort. No shipped title but Omega's two
  circuits cuts, and no GL device was available to try.
- **The sun-occlusion and shadow casters of a cut track are covered by the pixel identity
  above (arena and square, `--shadows mapped`), not by a unit test of their own.**
- The arena frame shows a black patch of road around the craft. It is in the single-buffer
  control frame too, so it is not the cut; its cause is not examined here.
- **2048 and Omega:** ported. The reader is generic over the model; 2048's own circuits all fit
  (largest `subway`, 91.0 MiB) and draw as before.
