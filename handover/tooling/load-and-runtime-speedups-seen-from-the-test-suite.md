---
status: measure
titles: [all]
updated: 2026-10-13
summary: "Race-load effect of bounded peek, BC7, PSARC inflate and raycast broadphase unmeasured; Deck load runs"
categories: [gameplay, frontend]
---

# Load and runtime speedups seen from the test suite

Opened 2026-10-12 by the `test-trim` lane (maintainer's request): the test-time
work keeps running into costs the game pays too, at load or at runtime, that
barely move the gate on their own. They are collected here rather than lost in a
test-trim report. The test-side rule is in `CLAUDE.md`'s test-budget section;
the measurements are in
[`docs/architecture/workspace-layout.md`](../../docs/architecture/workspace-layout.md).

## Done

- **`oag_assets::Archive::peek` decodes only the head of an LZSS entry**
  (2026-10-12). It used to read and decompress the whole entry to return its
  first bytes. A literal costs nine bits, so `len` bytes never need more than
  `ceil(9 * len / 8) + 1` of the stream; `lzss::decompress` stops exactly at
  the length asked for. Runtime callers: `oag_music::listing`'s RIFF header
  peek over every entry of at least `MIN_SOUNDTRACK_BYTES`,
  `oag_game::boot::movies`, `oag_game::prefetch` (which already skipped
  compressed entries for that reason, and still does since the PS2's hold
  neither container). **Not measured at runtime**: which shipped archive the
  music listing peeks LZSS entries in, if any, is unchecked.

- **The BC7 decoder is a quarter faster per block** (2026-10-12, second
  trim pass), bit-exact on every Omega `.gnf` texel and mip level. Runtime
  reach: `oag_render::ModelTexture::from_gnf` keeps a mip chain on a
  block-aligned base as GPU blocks, but **CPU-decodes every single-level or
  off-grid BC7 texture on every device** (`docs/formats/gnf.md`, "What a
  renderer does with it"), and `decode_bc7_level` is the path for a GPU with no
  block compression. **Not measured in a race load**: how many of an Omega
  race's textures take the CPU path is unchecked.
- **PSARC entries inflate with one reused decompressor** (2026-10-12),
  straight into the entry's buffer, bit-exact on all 130,546 entries of the 40
  PSARCs this project reads. HD's archives read 15-25% faster in a test that
  also hashes every byte, so the inflate itself gained more; Omega and 2048 are
  barely deflated and unchanged. Every HD and HD-PSN race and front-end load
  reads through this path. **Not measured in a race load.**
- **`oag_mesh::mesh::rcs::scene_from` opens its archive once** (2026-10-12),
  not once per texture: HD's Amphiseum went from about 25 s to a few seconds
  through it. Runtime reach: `oag-view`'s PS3 model path (`crates/view`); the
  game's own loader holds its archives already.

- **Every raycast walks one tree over the whole world** (2026-10-13). A
  soup's own BVH landed first (`bd15f779d`), but `CollisionWorld` still asked
  each of a circuit's ~200 colliders in turn, and that loop was most of a
  race's and a speed plan's CPU. `CollisionWorld` now holds one `Bvh` over
  every triangle, numbered so ascending order is the scan's order; same
  answers by construction, and the printed output of twelve sim ground truths
  is identical. Sim tests run about 2x faster in CPU (`ai_roll`'s grid 554 to
  248 CPU-s), and the game ticks the same code. **Not measured in a race load
  or in the game's frame time.** Measurements in
  [`workspace-layout.md`](../../docs/architecture/workspace-layout.md),
  "The collision world's broadphase".

## Open

- **Archive walks that sniff by decompressing everything.** The test suite had
  this shape in a dozen places (`sblk`, `.vex`, `.pob` walks over `WADS2.WAD`);
  `crates/formats/tests/sblk_banks/` and `crates/vex/tests/wad_walk/` now sniff
  a few decompressed bytes first. The runtime loaders read by name or hash, so
  none was found in `src/` (`rg 'looks_like_bank|looks_like_particle_system'
  crates/*/src`), but `crates/tools/src/bin/oag-wad.rs`'s `sounds` command
  still decompresses every blob to find banks. 15 minutes to port the sniff.

## Next Steps

1. Measure the music listing on a PS2 image with `Archive::peek` before and
   after (the commit "Archive::peek decodes only the head of an LZSS entry"),
   and an HD and an Omega race load before and after the BC7 and PSARC changes
   (their commits name them), and record each under "Done". An hour.
2. Port the sniff into `oag-wad sounds`. 15 minutes.
3. Measure a race load and a frame's tick time before and after the world
   broadphase (`24d715ff0`, `658d4654b`). 30 minutes.
