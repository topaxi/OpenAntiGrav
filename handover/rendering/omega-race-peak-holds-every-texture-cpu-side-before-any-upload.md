# The Omega race peak no longer holds every texture on the CPU before any is uploaded

2026-10-02. `lane/omega-memory` took Tech De Ra's peak RSS from 9,605 to 3,360
MiB (debug build) by passing `.gnf` BC7 chains through as blocks. `omega-texture-stream`
then took it to **792 MiB** (3,417 MiB measured on the same tree before the
change) by uploading each texture as it is decoded: `race::TextureSink`
carries the device into `race::load`, `oag_render::mesh_render::TextureSinkScope`
catches every texture the `.rcsmodel` build decodes, and a `Model` holds a
`Texels::Uploaded` view where it held the blocks. RSS after the uploads fell from
2,650 to 560 MiB; the still is byte-identical, and so are HD's and 2048's with
their peaks at 769 to 667 and 701 to 349 MiB. Numbers, method and caveats are in
`docs/formats/omega-status.md`, "What one race costs in memory".

## Open

- **Three routes load without a sink and keep every texture on the CPU.** The
  windowed `--race` loads before its window, and so its device, exists
  (`headless::run_race` hands the loaded circuit to the app); `--trace-out` and
  `--dry-run` build no scene from a device; the 2048 front-end capture
  (`capture.rs`, `race::load_event`) opens its device after the load. None is a
  player's route except the windowed `--race`, which is a developer one.
- **HD's `sky.gtf` is still decoded on the CPU** (24 MiB kept in the loader's
  texture line): the sky cube path builds its textures without going through the
  decode sites the sink is wired into. Vita `.gxt` textures go through the Omega
  site and are streamed.
- **The flush rule is chosen, not measured.** `texture_sink::FLUSH_EVERY` (a submit
  and a non-blocking poll every 64 MiB uploaded) was never compared against no
  flush. The peak above is with it.
- **The GPU side is not in RSS on a discrete adapter** and is on a software or
  integrated one. The craft liveries are 8192-square BC7 (85 MiB with the chain,
  three teams in this race); that is the disc's data, and this change does not
  shrink the GPU column.
- **`environments2048` circuits report 200 to 690 unresolved draws each**, the same
  on `main`. Not checked whether they are materials naming the 846 single-level
  `.gnf` files the disc ships (99 in `mall`, 71 in `tower`, 59 `shared`).
- One `.gnf` per `amphiseum` and `modesto_heights` is refused as blocks and still
  decodes to RGBA8 (the loader line counts it, does not name it). Single-level and
  off-grid are 0 on every circuit.
- HD's 5 pitched `.gtf` files and Vita `.gxt` textures stay RGBA8 on the way in,
  and are uploaded as such now.

## Next Steps

1. Name the refused `.gnf` on `amphiseum` and `modesto_heights`: a `refused` path
   in the loader line, or one probe over `Texture::block_levels`'s error.
2. Check whether the `environments2048` unresolved draws are the single-level
   textures (one `textures(path)` probe on `mall`).
3. Route HD's `sky.gtf` through the sink, or note why the sky cube cannot.
4. Optionally open the 2048 front-end capture's device before its load so that
   route streams too.
