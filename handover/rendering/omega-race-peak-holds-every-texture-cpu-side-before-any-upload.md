# The Omega race peak holds every texture on the CPU before any is uploaded

2026-09-30. `lane/omega-memory` took Tech De Ra's peak RSS from 9,605 to 3,360
MiB (debug build) by passing `.gnf` BC7 chains through as blocks and dropping
each `Model`'s CPU copies after upload; the per-category numbers are in
`docs/formats/omega-status.md`, "What one race costs in memory". What is left
is structural.

## Open

- **The peak is all textures at once.** Each model's textures are decoded
  when its `Model` is built, and `Scene::new` builds no `Drawable` until every
  model exists, so 2.3 GiB of track blocks, 0.5 GiB of craft and 0.13 GiB of
  sky sit on the CPU together. `Model::release_texels` frees them one drawable
  at a time afterwards; it cannot lower the peak.
- **glibc keeps what was freed.** After the uploads RSS is 2,585 MiB; with
  `MALLOC_MMAP_THRESHOLD_=1048576` it is 1,167 MiB. The freed 1-32 MiB
  buffers sit in arenas below the mmap threshold. Not changed: a `mallopt` or
  `malloc_trim` call is platform FFI in a crate that has none.
- The craft liveries are 8192-square BC7 (85 MiB with the chain, three teams
  in this race). That is the disc's data, not a decode artefact.
- Single-level `.gnf` textures (no authored chain) still decode to RGBA8 and get
  a synthesised chain. None appears among Tech De Ra's 512; how many the other
  circuits have is not counted.
- HD's 5 pitched `.gtf` files and Vita `.gxt` textures also stay RGBA8. Vita's
  race is 637 MiB, so it is not where the bytes are.

## Next Steps

1. Upload a texture as it is decoded: give the `rcs::build` texture callback a
   sink that uploads through the scene's device, so a `Model` never holds the
   blocks. About 2 hours of plumbing through `race::load`; the peak should
   fall to roughly the steady state.
2. Count single-level `.gnf` textures per circuit (one line in the loader
   report, `ModelTexture::from_gnf` knows when it falls back).
