# Load time was repeated work; what is left

The 2026-10-07 profile cut the HD race load from 7.9 s to 1.9 s (15.2 s to 3.5 s
on a Steam Deck) and the PS2's from 2.3 s to 1.1 s by removing repeated work (PSARC path index, a per-load
read memo, the gantry clock's per-vertex sampling, per-material texture
decodes, a single-threaded CHD read). Every number, the method and the
negative results are in
[`load-time.md`](../../docs/architecture/load-time.md). No SIMD was written:
nothing left in the profile is a loop it would pay for.

## Open

- **Inflate is still 42% of HD's load and 36% of 2048's**, now of distinct
  data. `zlib-rs` measured -9.3% (HD) and -6.7% (2048), byte-identical over
  30,102 entries, and was not taken under this lane's 10% bar for a new
  dependency in `oag-formats`. Block-parallel inflate (PSARC blocks are
  independent 64 KiB streams) was not tried: it would add load threads on top
  of the CHD split, unmeasured on the Deck.
- **HD's race music start decodes all fifteen MP3s whole to measure them**
  (`oag_music::measure`), 14% of the headless HD load; on the player's path
  it is the `race-music` thread. The function's own doc names the fix: a
  first-block read on `oag_assets::psarc::Archive`.
- **2048's PVRTC decode and `.gxt` twiddle, Omega's GNF untile**: 28% and 12%
  of their loads, one texture at a time. A parallel decode needs the reads
  (serial, `&mut Archives`) split from the decodes, and the texture sink's
  uploads kept on the loading thread in order.
- **A read memo hit copies its bytes** (about 3% of HD), because
  `oag_mesh::mesh::rcs::Textures` returns an owned `Vec<u8>`.
- **The player's path (`--measure-race-load`) was only run under lavapipe**,
  where frame times mean nothing, and per-frame CPU was not profiled on a
  presenting GPU. The Deck's headless loads are measured (HD 15.2 to 3.5 s,
  `load-time.md`); its windowed loading screen is not.

## Next Steps

- Run `--measure-race-load 2` on the Deck (it presents on a real GPU) for
  Pulse PS2 and HD Fury and record the named spans in `load-time.md`.
- If a load-time budget is ever set, `zlib-rs` is the next cut to take; the
  identity test that proved it is described in `load-time.md`.
