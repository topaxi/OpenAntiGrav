# Load time: where it goes and what was cut

Where a race load and a boot spend their time, measured on 2026-10-07, and the
five changes that came out of it. Engineering, not reverse engineering: every
number is a wall-clock or `perf` measurement of this build, and every
threshold is **chosen, not measured**.

The headline is that none of it was SIMD. The profile was dominated by work
done more than once: the same entry inflated thousands of times, a path lookup
that scanned and allocated, a texture decoded once per material that names it,
an animation sampled once per vertex, and a 40 MiB disc read decompressed on
one core. Removing repeated work cut the HD race load by a factor of four; no
loop was hot enough, once that was gone, for vector code to pay for itself.

## Results

Release build (`debug = 1`), the development desktop (24 hardware threads,
Linux), `--no-audio`. **This is the headless `--screenshot` path**: process
start to the first race frame written as a PNG
(`--race --screenshot <png> --ticks 1`), or to the boot's first frame
(`--screenshot <png> --ticks 1`). It is not the player's path, which loads on
the `race-load` thread and builds on `race-build` behind a loading screen (see
[Load-to-race transition](race-load-transition.md)); it runs the same
`race::load`, so a cut here is a cut there, but it also pays the capture's own
GPU work and starts the race music on the main thread. Median of three runs;
the load average was 5 to 12 throughout (a desktop in use, no other builds).

### Race load

| Source | Circuit | Before | After | Peak RSS before / after |
| --- | --- | ---: | ---: | ---: |
| Pulse PSP (`pulse-psp-eu.chd`) | `16_Track` | 0.900 s | **0.818 s** | 258 / 286 MiB |
| Pulse PSP | `04_Track` | 0.908 s | **0.820 s** | 259 / 288 MiB |
| Pulse PS2 (`pulse-ps2-eu.chd`) | default | 2.260 s | **1.096 s** | 270 / 311 MiB |
| Pure (`pure-psp-eu.chd`) | default | 0.880 s | **0.777 s** | 275 / 310 MiB |
| HD Fury (`hdfury-ps3-eu-dec.iso`) | Vineta K | 7.940 s | **1.925 s** | 678 / 733 MiB |
| HD PSN (`hd-psn-eu`) | Vineta K | 6.467 s | **1.538 s** | 683 / 754 MiB |
| 2048 (`PCSF00007`) | Altima | 2.945 s | **1.695 s** | 339 / 382 MiB |
| Omega (`data/extracted/ps4`) | Tech De Ra | 4.116 s | **2.111 s** | 988 / 1,032 MiB |

Spread within each triple was under 5% except where a run caught the desktop
(the worst: HD PSN after, 1.518-1.737 s).

### Boot to first frame

| Source | First screen | Before | After |
| --- | --- | ---: | ---: |
| Pulse PSP | `LogoFMV` | 0.303 s | 0.324 s |
| Pulse PS2 | `LogoFMV` | 1.323 s | **0.631 s** |
| Pure | `Language Selection` | 0.380 s | 0.421 s |
| HD Fury | `Studio Logo` | 0.274 s | 0.235 s |
| HD PSN | `Studio Logo` | 0.166 s | 0.161 s |
| 2048 | `Boot Studio Logo` | 0.483 s | **0.288 s** |
| Omega | `Language Selection` | 0.721 s | **0.651 s** |

Boot was never the problem: every title but the PS2 reaches its first frame in
well under a second, and the PSP and Pure rows moved by the machine's own
noise (the "after" set ran at a load average of 11). The PS2 row is the CHD
split below; 2048's and Omega's are the PSARC path index.

### Proof that nothing changed

- Every one of the 15 screenshots above is **byte-identical** to `main`'s
  (`cmp`), and a second run of the old binary reproduces its own PNG, so the
  comparison can see a difference.
- The headless path opens a `race::TextureSink` (`main/headless.rs`), so those
  frames went through the GPU upload path the player's load uses.
- The race load report (`RUST_LOG=oag_raceplay=debug,oag_game=debug`) is
  line-for-line identical on all eight scenarios plus HD's Talon's Junction,
  except one line: the race scene's texture-upload count on 2048 and Omega
  (2048 `36/582 reused` became `32/217 reused`), because textures shared by
  path now reach the upload cache as one `Arc` - see the texture section.
- `race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
  is twelve of twelve clean, every lap time and tick count identical before and
  after. It prints no shield figures, so none are quoted.

## Method

```sh
# a profiling build: frame pointers, because perf's DWARF unwinder returned
# leaf-only stacks for this binary (see "What did not help")
RUSTFLAGS="-C force-frame-pointers=yes" CARGO_TARGET_DIR=target/prof \
    cargo build --release -p oag-game
perf record -F 1999 -g -o hd.data target/prof/release/oag-game \
    data/images/hdfury-ps3-eu-dec.iso --no-audio --race \
    --track /data/environments/01_vineta_k/track.vex --screenshot x.png --ticks 1
perf script -i hd.data | <fold to root;...;leaf stacks>
```

Inclusive time per `oag_*` frame, and the immediate `oag_*` caller of a hot
leaf, located every item below in one pass each. Timings use the plain release
build, never the profiling one.

## What was cut, in order of payoff

### 1. A PSARC path is looked up in a map (`oag_assets::psarc`)

`Archive::index_of_path` scanned the manifest and built a normalised `String`
of every stored path on each lookup, and `Archives::holder_of` asks up to
seven archives per read. That was 14% of HD's load as self time in
`normalise` alone, 24% inclusive. The map is built once at open, keyed by the
same normalisation, first spelling winning a collision as the scan's
`position` did (`two_spellings_of_one_path_resolve_to_the_first_like_a_scan`).
HD Fury 7.94 to 5.49 s, Omega 4.12 to 3.25 s, 2048 2.95 to 2.67 s.

### 2. A race load memoises repeated reads (`oag_assets::read_memo`)

The biggest single item. Every material slot of every model reads its
`.rcsmaterial` once for each pass that inspects it - skin, roles, water, ice,
refraction, light cone, mag wave - and every pad class repeats the lot. On HD
Fury's Vineta K that was **15,194 reads of 758 entries, 3.2 GB inflated for
262 MB of data**; 13,828 of them were reads of 99 materials (5.8 MB). Omega
re-read 2.0 GB of `.gnf`, 2048 0.2 GB of `.rcsmaterial` and `.gxt`.

`Archives::memoising_reads` keeps what `read_name` returns, for entries up to
1 MiB and 64 MiB in all (both chosen, not measured; the largest
`.rcsmaterial` the three race loads read is 0.6 MB). `race::load` asks for it
on the archives it opens and drops them on return, so the memo lives for one
load and never for a session. The report gains one line:

```
archive reads: 14385 of 15194 answered from the read memo (683 entries, 52620 KiB kept)
```

and `race_load_read_memo_ground_truth` fails if the load stops asking for it
(checked by removing the call). HD Fury 5.49 to 2.89 s, HD PSN 6.47 to 2.44 s.
Peak RSS rose 40-50 MiB, the memo itself.

A hit still copies the bytes, because `oag_mesh::mesh::rcs::Textures` returns
an owned `Vec<u8>`; that copy is about 3% of the remaining HD load. Removing it
means changing that signature at every caller.

### 3. The start gantry's GO edge samples its animation once per frame (`oag_raceplay::gantry::clock`)

`go_edge` searches 361 frames for the board's first green and sampled the
texture-animation table inside the per-vertex texel closure, so every vertex
re-sampled every curve, allocating as it went: a third of HD's load once the
memo was in. The table is now built per frame up front with the same call, so
the edge, settle and exit frames are unchanged (the load report's gantry lines
are identical). HD Fury 2.89 to 1.98 s.

### 4. A 2048 or Omega model decodes each texture file once (`oag_mesh::mesh::rcs::psp2`)

`bind_textures` cached decodes by material, but materials share files:
Altima's track resolves 523 material slots to 164 distinct `.gxt` files, and
decoded all 523. The decode is now keyed by path and shared as one `Arc`;
each material keeps its own slot so a lightmap stays beside its diffuse, and a
`.gnf`'s form is counted per material as before, so the model report is
unchanged. With a texture sink open each file is uploaded once, which is the
one load-report line that moved, and Tech De Ra's control load in
`texture_stream_ground_truth` now holds 200 distinct textures (1.4 GiB)
rather than one per slot. 2048 2.50 to 1.72 s, Omega 3.11 to 2.10 s.
`psp2_texture_dedupe_ground_truth` fails on a per-material cache (checked).

**2048 and Omega: ported** - one builder serves both, so both are measured
above; the path index and the read memo apply to every PSARC title.

### 5. A long CHD read is split across threads (`oag_disc::chd_source`)

The Pulse PS2 disc keeps its music as raw PCM in `54748/PS2MUSIC.WAD`, and a
race reads one track whole: about 40 MiB of LZMA hunks, decompressed one at a
time on the caller's thread, which was over half that race's load and most of
its boot. `ChdSource::read_sectors` now splits a read spanning 64 hunks or
more per worker into runs of whole hunks, each read by its own `ChdSource` on
the same file, at most 8 threads (both chosen, not measured); an ISO 9660 walk's
small reads stay on one thread. `a_split_chd_read_is_byte_identical_to_a_sector_by_sector_one`
reads 20,011 sectors of the music archive, starting and ending mid-hunk, both
ways. Pulse PS2 race 2.28 to 1.13 s, boot 1.32 to 0.63 s; Pulse PSP race 0.88
to 0.82 s.

`oag-disc` is not in the determinism set, and the bytes are the serial read's
by construction; nothing about the simulation can see the split.

## What did not help

- **Inflating straight into the output buffer.** `psarc::Directory::read_entry`
  inflates each 64 KiB block with `decompress_to_vec_zlib`, which allocates,
  grows by doubling and is then copied into the entry. A fast path through
  `miniz_oxide::inflate::core::decompress` into the entry's own buffer was
  byte-identical over all 11,671 HD and 18,431 2048 entries, and saved **2%**
  (HD 1.98 to 1.94 s). Not kept: the cost is the Huffman decode, not the
  allocation.
- **`zlib-rs` as the inflate backend.** Pure Rust, `Zlib` licence, MSRV 1.75,
  runtime CPU-feature dispatch; byte-identical over the same 30,102 entries.
  Interleaved A/B, five runs each: HD 1.943 to 1.762 s (**-9.3%**), 2048 1.701
  to 1.587 s (**-6.7%**), Omega unchanged (its archives store no deflated
  block). Under the 10% this lane set for a new dependency in a determinism
  crate, so not taken; it is the next thing to take if a load-time budget ever
  needs it.
- **`perf record --call-graph dwarf`.** Every stack came back as its leaf
  frame alone on this binary, which made the first profile read as "inflate
  is 30%" with no caller. Use a frame-pointer build.
- **SIMD.** Nothing left in the profile is a tight loop over plain arrays that
  autovectorisation has missed: what remains is entropy decoding (inflate,
  PVRTC, MP3, ATRAC9) and tiling arithmetic with data-dependent addressing.

## Where the rest goes

After the five changes, on the headless path:

| Title | Main thread, by phase |
| --- | --- |
| HD Fury | `race::load` 53% (pads 17%, track model 14%, audio banks 6%), the screenshot capture 17%, race music 14% |
| 2048 | `race::load` 54% (track model 28%: PVRTC decode, `.gxt` twiddle, collision raycasts), capture 29%, race music 9% |
| Omega | `race::load` 27% (track model 12%: GNF untile and BC7), plus ATRAC9 music decode on ten threads |
| Pulse PS2 | race music 58%, before the CHD split; `race::load` 21% |

Inflate of distinct data is still 42% of HD's samples and 36% of 2048's.
HD's race music start decodes all fifteen MP3 tracks whole to measure their
lengths (`oag_music::measure`, whose own doc comment predicted the fix: a
first-block read on `psarc::Archive`); on the player's path that runs on the
`race-music` thread, not the frame.

## The player's path, and per-frame CPU

One `--measure-race-load 1` on HD Fury in an Xvfb display with lavapipe
(software Vulkan), the old binary against the new: the loading stage drew 615
frames before and 329 after, and `race-build`'s `build_race_stage` was 1,284
and 1,266 ms - unchanged, as it should be, since nothing above touched the
scene build. Under lavapipe every race frame misses 16.7 ms, so no frame time
from that run means anything; **the player's path was not measured on a
presenting GPU.**

A `perf` profile of the same run, kept to `oag_*` frames, puts the whole race
frame (`Scene::render`, `RaceStage::draw_hud`, `Race::tick`, the particle
stage) at well under 1% of samples against the software rasteriser's threads.
**No per-frame CPU hot spot was found in this build's own code**, and none was
optimised; a presenting-GPU profile is the open step.

## Reproducing

The scripts that produced every table are under the lane's scratch directory
and not committed; the commands are the `--race --screenshot` and
`--screenshot` runs above, three times each, with `XDG_*` pointed at a
scratch profile.
