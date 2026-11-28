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

Spread within each triple was under 5% except where a run caught the desktop:
Pulse PS2 before (2.246-2.577 s), 2048's boot before (0.478-0.749 s) and HD
PSN after (1.518-1.737 s).

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
noise (the "after" set ran at a load average of 11; the CHD split's own A/B
below puts the PSP boot cost under its spread). The PS2 row is the CHD split
below; 2048's and Omega's are the PSARC path index.

### On a Steam Deck

The same comparison on an LCD Steam Deck (Zen 2, 8 threads, SteamOS, on AC;
`scaling_governor` `powersave` with EPP `balance_performance`, its defaults),
**with audio on** as a player runs it: two `just appimage-portable` builds,
`main` at 4c8a2372c and this change, each run from `/tmp` against the Deck's
own copies of the discs, interleaved before/after, median of three after one
warm-up pass. All runs exited 0.

| Scenario | Before | After |
| --- | ---: | ---: |
| Race, Pulse PSP `16_Track` | 1.567 s | 1.560 s |
| Race, Pulse PS2 | 3.997 s | **2.291 s** |
| Race, Pure | 1.390 s | 1.367 s |
| Race, HD Fury Vineta K | 15.183 s | **3.476 s** |
| Race, 2048 Altima | 5.665 s | **3.234 s** |
| Race, Omega Tech De Ra | 8.945 s | **4.710 s** |
| Boot, Pulse PS2 | 2.110 s | **1.023 s** |
| Boot, 2048 | 1.016 s | **0.556 s** |

The ratios match the desktop's, a little larger: HD is 4.4x faster on the Deck
against 4.1x here. The Deck's CHD split runs at its 8-thread cap, all of the
APU's threads, and the PS2 rows still halve.

### Proof that nothing changed

- Every one of the 15 screenshots above is **byte-identical** to `main`'s
  (`cmp`), and a second run of the old binary reproduces its own PNG, so the
  comparison can see a difference.
- The headless path opens a `race::TextureSink` (`app/headless.rs`), so those
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

The memo also keeps a neighbour read (`read_preceding`, `read_following`) up to
16 MiB, under the same 64 MiB budget. A Pulse PS2 circuit's texture set is the entry
before its model, and the load asks for it four times (`ps2_texture_set` in
`load/track_inputs.rs`); the trackfile walk of all PS2 circuits asked nine times
per circuit and read 1,091 MiB of disc for 152 MiB of distinct ranges, 46% of its
CPU in LZMA hunk decoding. With the memo the walk is 62-64 s of CPU against
90-114 s (the PS2 walk, now six `..._slice_N` tests, load 20-42).

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
its boot. `ChdSource::read_sectors` now splits a read that gives each worker
4 MiB or more into runs of whole hunks, each read by its own `ChdSource` on the
same file, at most 8 threads (both chosen); smaller reads stay on one thread.

The first threshold was 64 hunks per worker, and an interleaved A/B (five runs
each, the split forced off against on) showed it cost a PSP boot 0.301 to
0.320 s: every worker reopens the file and decodes its hunk map, which a
medium read did not repay. At 4 MiB per worker the same A/B reads: Pulse PSP
boot 0.305 against 0.312 s and Pure 0.407 against 0.410 s (spreads overlap),
Pulse PS2 boot 1.299 to 0.666 s, PS2 race 2.240 to 1.130 s, PSP race 0.904 to
0.890 s. `a_split_chd_read_is_byte_identical_to_a_sector_by_sector_one`
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
  crate, so not taken then. Taken later, see "Inflate unified on `flate2`"
  in `docs/architecture/workspace-layout.md`, which measured it again with the
  whole `Archive::read` path (including the Fury disc this entry used) and
  found a larger gain: HD Fury 9.2 to 10.5 s down to 6.7 s.
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
HD's race music start used to inflate all fifteen MP3 tracks whole to measure
their lengths (`oag_music::measure`). It reads each track's first PSARC block
since 2026-10-19 (`psarc::Archive::peek`): 305 ms became 23 ms on the Steam
Deck, on the `race-music` thread ([music threads](music-threads.md)).

## `memchr` and `CharSearcher` in a debug build

A headless HD Fury race load (`--race --screenshot <png> --ticks 1`,
`hdfury-ps3-eu-dec.iso`, default circuit, `target/debug/oag-game` built with
`-Cforce-frame-pointers=yes`) spent 22.5% of its CPU samples in
`core::slice::memchr::memchr_aligned` (15.3%) and `CharSearcher::next_match`
(7.2%). Every one of those samples is under
`naga::span::Span::location` called from `naga::back::spv::BlockContext::write_block`
(1,051 of 1,051): naga's SPIR-V writer resolves a line and column for each
statement by rescanning the shader source, when wgpu's `InstanceFlags::DEBUG`
is set. `InstanceFlags::from_build_config` sets it in a debug build only, so
the release build never paid it (0.18% of samples there). It is not string
scanning in `oag-tables` or `oag-ui`.

`oag_game::adapter::instance` now removes `DEBUG` and keeps `VALIDATION`.
Same load, three interleaved runs each, `perf stat -e instructions:u`, load
average 3.5 to 3.6:

| | before | after |
| --- | ---: | ---: |
| instructions:u | 82.18 / 82.19 / 82.20 G | 44.24 / 44.17 / 44.18 G (**-46%**) |
| user time | 4.72 / 4.68 / 4.73 s | 2.96 / 3.00 / 2.97 s (**-37%**) |
| `memchr_aligned` + `CharSearcher` | 22.5% of samples | 0.07% |

The captured PNG is byte-identical in all six runs. The cost left is inflate
(17% of samples), music decode and the shader pipeline itself. The release
build is untouched by the change.

**To get the labels back:** `OAG_WGPU_DEBUG=1` keeps `InstanceFlags::DEBUG` in any build (`oag_game::adapter::DEBUG_LABELS_VAR`), for RenderDoc or a validation message read by label. It costs the load the quarter above and changes no picture.

**To try a smaller device:** `OAG_MAX_TEXTURE_SIDE=<n>` makes the device with `max_texture_dimension_2d` at most `n` (`oag_mesh::mesh_render::required_limits`), so a Steam Deck's 16,384 or a phone's 8,192 is a desktop run away. Unset changes nothing.

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

## A scripted launch: what a `--drive` test pays

The tests of the `gpu-vram` group (`docs/architecture/gate-wall-time.md`) are
mostly one `oag-game --drive` launch each, and a launch is where their seconds
go. Measured 2026-11-24 on the development desktop (AMD, 24 hardware threads),
debug test profile (our crates and dependencies at opt-level 2), Pulse PSP EU,
`--no-audio --no-intro`, a fresh profile per launch, the load average between
17 and 60 (other lanes' gates ran throughout). **User CPU is the number to
read**: wall clock moves with the load, and the instruction count of a launch
(`perf stat -e instructions:u`) moved less than 1% between repeats.

| Launch (script ends at) | Wall before | Wall after | User s before | User s after |
| --- | ---: | ---: | ---: | ---: |
| `boot; ticks 1` (first frame) | 0.83 s | 0.59 s | 1.45 s | 0.80 s |
| Main menu | 0.95 s | 0.76 s | 1.70 s | 0.76 s |
| RACE CAMPAIGN open, `ticks 10` | 1.81 s | 1.30 s | 1.99 s | 1.00 s |
| Race, first tick | 2.55 s | 3.15 s | 3.53 s | 2.82 s |
| Race, 400 ticks | 4.39 s | 3.70 s | 4.25 s | 2.86 s |
| Race walk of the pause test (menu, race, pause, GAME OPTIONS, 2 presses) | 4.44 s | 4.08 s | 4.37 s | 2.91 s |

(Medians of three. The race row's wall clocks are not ordered at this load, a
run can wait for a core; their user seconds are.) User instructions, `perf stat -e instructions:u`, interleaved before and after
(both changes below), with the peak RSS of the process (`/usr/bin/time -f %M`,
median of three):

| Launch | Before | After | Peak RSS before / after |
| --- | ---: | ---: | ---: |
| Pulse PSP, menu walk (RACE CAMPAIGN open) | 21.05 G | 9.4 G | 200 / 208 MiB |
| Pulse PSP, race to 400 ticks, pause, GAME OPTIONS, two presses | 43.6 G | 26.7 G | 307 / 316 MiB |
| Pulse PS2, the same race walk | 80.5 G | 62.7 G | 457 / 470 MiB |

The pool costs 8 to 13 MiB of idle handles (a decoded hunk map each; measured with up to twelve kept per path, now sixteen in all).

Where a launch's CPU went before (`perf record -e cpu-clock:u --call-graph fp`
on the frame-pointer build, share of the launch's samples; the shares of
threads add up, the process has about fifteen):

- **`Chd::open`: 25 %** of a PSP launch, 148 calls in one run (`circuit-lengths`
  36, the main thread 14 on the Pulse PSP disc alone, the rest on the music and
  source threads reading the other discs in `data/images/`). Each decodes the
  whole hunk map, a Huffman walk and a CRC-16 per hunk: 8 to 14 ms of CPU for a
  200 MB PSP image, 10 to 14 ms for the 3.7 GB PS2 one.
- **PSARC path digests: 22 %** of a menu-only launch. The soundtrack survey
  and the source list identify every 2048, HD and Omega archive on the disk,
  and each `match_paths_to_entries` hashes every manifest path (tens of
  thousands per archive).
- **Pipeline compile in `race-build`: 31 to 35 %** of a launch that races
  (`create_render_pipeline` -> naga `process_overrides` and SPIR-V writing, 20
  % and 10 %). Untouched: the scene already shares modules and pipelines
  across its drawables (`oag_mesh::mesh_render::pipeline_cache`), so what is
  left is naga's cost per distinct override set.
- **PS2 music, 36 % of a PS2 launch**: the menu and race songs are LZMA hunks
  of the CHD, decoded on `menu-music` and `race-music` also under `--no-audio`,
  whose null mixer keeps the song selection (the marquee) identical to a real
  run. Untouched.
- Boot to the first frame is 0.8 s wall here against the 0.3 s above because
  a launch now also surveys every image in `data/images/`; the first frame
  itself is not what it waits for.

### Idle CHD handles are reused (`oag_disc::chd_source`)

A dropped `ChdSource` hands its opened handle (the decoded map, its buffers and
the hunk it last read) to a pool; the next `ChdSource::open` of the same file
takes one back instead of decoding the map again. "The same file" is read from
the open handle, never from the path before the open: on Unix its device, inode,
size, modification and change times (two spellings of one image share a handle,
a file replaced under its path does not), elsewhere its resolved path, size and
modification time. A handle belongs to one source at a time, so no two threads
share a read position; sixteen are kept in all, the one returned longest ago
dropped first (`chd_source/pool.rs`); the web build, which opens a registered
blob, does not pool. Bytes are the same: `a_reopened_chd_reads_the_bytes_a_new_handle_does`
and the split-read test pass, as do the disc crate's 100 tests. Menu launch
21.05 G -> 10.6 G user instructions, race walk 43.6 G -> 27.8 G (the pool alone).

### PSARC path digests (`oag_formats::psarc`)

`md5::digest` copied the message to pad it and picked each step's mixing
function inside a 64 step loop; `path_digest` allocated the uppercased path.
Now the tail is padded on the stack, each round is its own loop, and an ASCII
path uppercases into a stack buffer. The digests are the same (RFC 1321 and
`hashlib` vectors at the block boundaries, every entry of every real archive in
`psarc_ground_truth`): 600,000 path digests 3.56 G -> 1.22 G user instructions
(release build), a menu launch 10.6 G -> 9.4 G.

### A whole module

`campaign_cursor_ground_truth` (33 launching tests) alone with nextest at its
cap of 4, six interleaved before/after pairs, load 17 to 42:

| | Before (median, range) | After |
| --- | ---: | ---: |
| User CPU of the run | 243 s (224 to 283) | 171 s (155 to 206) |
| Wall clock of the run | 37.3 s (34.6 to 57.5) | 32.4 s (29.2 to 54.6) |
| Summed test seconds | 127 s (117 to 206) | 107 s (94 to 194) |

The wall clock gains less than the CPU because at the cap of 4 a module is
bound by its longest tests as well; in the gate, with the cores shared by the
other 17 tests of a slice, CPU is the larger part of a launch's seconds
(the module summed 210 s there against 116 s alone).

### Third round (2026-11-24): archive manifests, pipelines, MENU BACK launches

Same machine and method (frame-pointer release build, `perf stat -e instructions:u`
and `perf record -e cpu-clock:u --call-graph fp`), before and after interleaved,
load 29 to 66 throughout (other lanes' gates); the instruction counts repeated to
under 1 % between runs of one binary on the menu launch.

**A PSARC archive's parsed manifest is shared by every open of the same bytes**
(`oag_assets::psarc::Archive::from_source`, `psarc/parsed.rs`). A Pulse launch
opens each 2048, HD and Omega archive on the disk three to five times (the
soundtrack scan, the title's source, the launcher's survey: 153 opens of about
45 archives, 110 ms of `match_paths_to_entries` alone), and each open inflated
the manifest, MD5-hashed every path and built the path index. The parse is kept by
the content of the table of contents and the stored manifest (lengths and two
64-bit digests), 128 archives, oldest out first; a second opener waits for the
first on the archive's own lock. The reads are unchanged (the same ranges are
read and recorded), only what is derived from them is not repeated.

| Launch (Pulse PSP, release build) | Before | After | Peak RSS before / after |
| --- | ---: | ---: | ---: |
| Menu to RACE page, `ticks 30` | 4.85 G (4.34 to 4.85) | 3.24 G (3.24 to 3.25), -33 % | 194 / 223 MiB |
| Race, 400 ticks | 20.5 G (20.49 to 20.52) | 18.9 G (18.88 to 18.89), -8 % | 319 / 332 MiB |

(Instructions: median and range of eight, then five interleaved pairs; the menu
launch's before side has an occasional run that is cut short at 4.34 G, the after
side none. RSS: median of three, `/usr/bin/time -f %M`, the before side's range
179 to 224 and 309 to 330 MB overlapping the after side's; the shelf keeps every
surveyed archive's paths and index for the life of the process, about 10 to 30 MB.)

**The race build's pipeline compile, measured and left alone.** 158 distinct
pipelines are built for a Pulse PSP race (the scope's cache already turns about
9,800 requests into them), 1.0 s of wall clock on the `race-build` thread at load
40: `fs_main_blend` 66 (0.44 s), `fs_main_alpha_test_velocity` 28, `fs_main_velocity`
19, `fs_main_stamp_velocity` 16, `fs_main_blend_velocity` 16, the rest 13. `race-build`
is 42 to 45 % of a racing launch's CPU, 35 % of it `compile_stage` (naga's
`process_overrides` 21 %, of which `revalidate` 9 % and the module clone 5 %, then the
SPIR-V write 12 %); the driver's own compile is under 1 % because RADV links
lazily.

- **A disk pipeline cache cannot pay, so it is not built.** The one `unsafe`
  wgpu call it needs (`Device::create_pipeline_cache`,
  `docs/architecture/dependency-upgrades.md`, the `PipelineCache` bullet) is
  allowed only if the benefit is big. It is not, for two reasons.
  - The cache goes to `vkCreateGraphicsPipelines` only (`wgpu-hal` 30.0.1
    `vulkan/device.rs`, `create_render_pipeline`), after `compile_stage` has run
    naga's override processing, validation and SPIR-V write, so the 35 % above is
    paid with or without it.
  - What it skips is the driver's compile, and mesa's own shader disk cache
    (`~/.cache/mesa_shader_cache`, no code of ours) already skips that on a second
    launch. Pulse PSP race launch, 400 ticks, `target/debug`, `perf stat -e
    instructions:u`, three pairs at load 62 to 64: a fresh empty
    `MESA_SHADER_CACHE_DIR` 65.5 / 65.9 / 65.9 G, the warmed one 24.0 / 24.1 /
    23.8 G (-63 %). That is the whole cold-to-warm saving a pipeline cache could
    offer, and the player and the gate already have it. What a pipeline cache adds
    on top is the warm driver's own share: `perf record -e instructions:u -c
    5000000` puts `libvulkan_radeon.so` at 0.50 % of the warm launch's 24.0 G
    (`oag-game` 90.3 %, `libc` 8.7 %); `cpu-clock:u` sampling gave 1.97 % and
    0.97 % in two runs. Ceiling: about 1 %, against the 10 % bar. RADV only;
    NVIDIA's driver cache was not measured here.
- **One module per stage did not pay.** `mesh.wesl` as a 10 KB vertex module
  (no entry point but `vs_main`, two overrides) and a 90 KB fragment module cut
  `process_overrides` on the vertex stage to a clone of 10 KB: 18.9 -> 18.5 G on
  a race launch (1.5 to 2 %, six interleaved pairs at load 15 to 62), because the
  fragment stage is where naga's time goes. Dropped, not committed.
- **Not tried, with its estimate.** 32 of the 158 are the unblended pair of every
  distinct override set (`unblended_pipeline`, and the plain circuit's), selected
  only by a transparent draw of `BlendClass::None`; not building it where no draw selects it is about 7 % of a racing launch and
  needs a rule for a model whose draws change after the build
  (`Model::transparent_draws` is pushed to by `oag-livery` before it, never after).
  The additive pair is also drawn by `Drawable::draw_additive` and the alpha-over
  pair by the absorb overlay and the track shine, so those are not candidates.
  Building lazily on first draw would put the compile on the frame thread. (Done in the fourth round,
  by what the draws can pick rather than by lazy builds.)

**A launch per MENU BACK choice became one.** The drive's new `config <setting>=<value>`
step applies a setting as a changed menu row does, from any screen
(`docs/tools/oag-game.md`). `campaign_cursor_ground_truth`'s
`leaves_on_the_resolved_button` and `help_opens_on_the_displaced_button` (14 tests)
and four tests of `footer_hint_ground_truth` run the three choices in one launch,
every `expect` kept, going back onto the screen with the last forward steps of its
path. Whole modules at the gate's profile (`target/debug`, nextest, user instructions
of the `cargo nextest run`, two interleaved rounds, load 50 to 80):

| Module | Before | After |
| --- | ---: | ---: |
| `campaign_cursor_ground_truth` (33 tests) | 1,753 G, 1,755 G; 60.5 s, 70.3 s wall | 1,141 G, 1,159 G; 42.1 s, 42.1 s wall (-35 % instructions) |
| `footer_hint_ground_truth` (14 tests) | 671 G, 677 G | 545 G, 552 G (-19 %) |

**The last per-choice launches (2026-11-28).** The footer-hint tests for Cell
Selection, photo mode and Pure's race page now launch once. Cell Selection reads the
campaign cursor, so its script puts the cursor back and `expect`s it before leaving;
photo mode switches the choice inside one race. The Pulse PS2 photo-and-viewer pair
(`..._under_original`, `..._under_circle`) became one test,
`pulse_ps2_photo_mode_and_viewer_leave_on_the_resolved_button`: one race launch and
one viewer launch for both choices, the viewer booting on the choice the race saved
last (so that one is still read from `settings.toml`; photo mode now reaches `circle`
only through `config`, and `boots_under_menu_back` covers the boot read). Both builds
of the test binary run directly, six interleaved pairs, `--test-threads=4`, load 4 to
20 (24 threads), medians; summed is the per-test seconds of `--report-time`, what the
`gpu-vram` group adds up:

| Module | User CPU | Wall | Summed |
| --- | ---: | ---: | ---: |
| `footer_hint_ground_truth` (14 tests) | 23.4 -> 20.5 s (-12 %) | 6.66 -> 6.72 s | 21.9 -> 19.3 s |
| `campaign_cursor_ground_truth` (37 -> 36 tests) | 52.4 -> 48.9 s (-7 %) | 21.2 -> 20.5 s | 61.2 -> 57.9 s |

Per test: Cell Selection 1.24 -> 0.70 s, photo-mode hints 4.23 -> 2.18 s, Pure race
page 0.59 -> 0.31 s, the PS2 photo-and-viewer pair 8.37 -> 5.36 s. Together that is
about 6 s of a group that summed to 1,788 s in the selection `gate-wall-time.md`
measured: no gate hold moves visibly. Neither module's wall clock moves much, its
slowest tests being elsewhere.

```sh
cargo test -p oag-game --test it_a_g --no-run   # keep a copy of the binary before the edit
OAG_REQUIRE_GAME_DATA=1 RUSTC_BOOTSTRAP=1 /usr/bin/time -f '%U %e' <it_a_g> \
  --ignored footer_hint_ground_truth:: --test-threads=4 -Z unstable-options --report-time
```

**PS2 music under `--no-audio`, read and not changed.** The 36 % of a PS2 launch
is the CHD's LZMA hunks of `PS2MUSIC.WAD`, read to build a `Sound` whose length
the null mixer still advances a song by (the marquee shows each race song once).
Skipping the read means a length-only `Sound` from the WAD's index, an audio
change, not a quiet one.

### Fourth round (2026-11-24): a model builds only the pipelines its draws can pick

**What a race binds against what it builds.** Pulse PSP race, 400 ticks and again 3,000
ticks of autopilot (`oag_mesh::mesh_render::pipeline_cache`'s scope, a counter at each
`create_render_pipeline` miss and a set of every pipeline passed to `set_pipeline`; the
instrumentation was not kept): **158 distinct pipelines created, 31 bound**, the same 31 at
both lengths. The rest are not drawn in a race that fires no weapon, absorbs nothing and
wrecks nothing, but a weapon, a shield or a wreck can bind them at any time, so none of
them is built lazily: a first-use compile on the frame thread is a hitch in play. What can
be dropped with no hitch is a pipeline **no draw of the model can pick**, which is known at
build because a built `Model` is immutable (`Drawable` holds it in an `Arc`).

Where the 158 came from (`mesh_render::build_with`, per model, before): the alpha-over,
additive and unblended pairs (two-sided and culled twin of each), a pair per authored
blend state, the glow stamp's two pairs, the plain-draw copies of all of those on a
circuit, and the model-level cutout pipeline, for every model whatever its draws name. A
plasma halo of two blended draws built 6 blended pipelines, a hull with one transparent
draw built 6 blended and 4 stamp pipelines, and a model with no cutout draw a cutout
pipeline.

**The change** (`mesh_render/reach.rs`): `build` asks which pipelines its draws can pick and
builds those.

- The pick is one rule, `blend::Pick::of`, read by `TransparentPipelines::select` and by
  the build (`Stamp::is_additive` likewise for the stamp), so what is built and what is
  drawn cannot drift apart. A draw's twin is its own `culled`.
- The overlay and shine passes index the alpha-over (absorb overlay, track shine) or
  additive (`draw_additive`) pair by any draw's `culled`, whatever its list, so those two
  pairs are built for the twins of every list; the unblended pair, each authored pair and
  the stamp pairs only for the transparent draws that select them (a circuit's plain draws
  select the plain set, the rest the model-wide one).
- A twin no draw picks holds the other twin's pipeline; a pair none picks holds the
  alpha-over pair's two-sided pipeline (the stamp's additive pair, the stamp's alpha-over
  one). The cutout pipeline a draw with no reference picks is built only where a cutout draw
  names none; a model with no transparent draw has no stamp pairs.
- The viewer, the capture and the race draw each `Built` with the model it was built from;
  every writer of a model's draws (`oag-livery`, `flyer::clip`, the weapon loader's
  `cull_as_authored`) runs before `build`.

| Pulse PSP, 400 ticks | Before | After |
| --- | ---: | ---: |
| Pipelines created (mesh scope, which is about 97 % of the launch's `create_render_pipeline`: `oag-post`, `oag-fx` and `oag-render` together are about 1 %) | 158 | 104 |
| Race launch, warm Mesa cache, `perf stat -e instructions:u` (`target/debug`), four interleaved pairs, load 35 to 41 | 24.12, 24.12, 24.22, 24.12 G | 21.13, 21.13, 21.13, 21.11 G (-12.4 %) |
| Headless race load to first frame (`--race --screenshot --ticks 1`), five interleaved pairs, load 30 to 34 | 1.10 to 1.23 s | 0.89 to 0.94 s |
| `photo_mode_ground_truth` (14 tests, whole races), `cargo nextest run`, two interleaved rounds, load 74 to 97 | 1,036.7 G, 1,025.0 G | 939.5 G, 931.5 G (-9.4 %, -9.1 %) |

Nothing moved onto the frame thread: the pipelines are built where they were, at load.
Pixels: the PNG's decoded scanlines are identical, base against new, for Pulse PSP
(400 ticks held cross, 1,500 and 4,000 ticks autopilot), Pulse PS2, Pure PSP, Pulse PSP
USA, HD/Fury (PS3) and Omega (600 ticks autopilot); the files differ only in the embedded
command-line metadata (their own output path). None of those frames draws a weapon
halo, a wreck or the absorb overlay, so the claim for them rests on
`reach::tests::every_transparent_draw_selects_a_pipeline_built_for_it` (a model mixing all
four classes and both cull modes picks a different pipeline for each draw), the shared
rule above, and the frame tests of the gate.

**Measured and left.** After the change a launch's `create_render_pipeline` is still
about 29 % of its samples, 27 to 28 % of it the mesh scope. The largest piece left is
`foliage::pipelines` (4 pipelines, 4 % of samples): they are drawn only under
`graphics.scenery_sort = depth`, but that setting is applied every frame and changes in
the pause menu, so a race that starts on `original` still needs them the moment it
changes. Dropping them needs a rebuild on the change. Under the same constraint the
masked (`Velocity::Gated`) and culled solid sets, the circuit's four cutout-reference sets
and a role for each overlay build (the absorb overlay never draws additive; a shine pass
only alpha-over) would cut more.

### Fifth round (2026-11-24): build-time SPIR-V cannot reach the mesh pipelines

**Question.** naga runs twice per race launch: `create_shader_module` parses and validates
the WGSL, and `wgpu-hal`'s Vulkan `compile_stage` (inside every `create_render_pipeline`)
processes the override constants, revalidates and writes SPIR-V. Would SPIR-V that naga
writes at build time, handed over with `Device::create_shader_module_passthrough`, remove
enough of that (a stop bar of about 10 % of the launch)?

**Split** (Pulse PSP race, 400 ticks, warm Mesa cache, `target/debug` built with
`-Cforce-frame-pointers=yes`; `perf record -e instructions:u -c 5000000 --user-callchains
--call-graph fp`, two runs, load 70 to 73; children percentages):

| Where | Run 1 | Run 2 |
| --- | ---: | ---: |
| `compile_stage` (all of `create_render_pipeline` but 0.1 %) | 25.46 % | 25.36 % |
| of it `process_overrides` (override substitution, compaction, `revalidate`) | 15.43 % | 15.43 % |
| of it `spv::write_vec` | 9.35 % | 9.30 % |
| `create_shader_module` (WGSL parse and validate) | 5.47 % | 5.59 % |
| Launch total, `perf stat -e instructions:u`, three runs (the frame-pointer build, hence above the Fourth round's 21.13 G) | 21.82, 21.82, 21.82 G | |

By caller (run 1, stacks walked up to the first `oag_` frame): `compile_stage` 25.0 % from
the mesh scope (`mesh_render::build_with`, `foliage`, `stamp`, `prepass`) and 0.5 % from
everything else; `create_shader_module` 4.3 % mesh and 1.1 % everything else. An earlier
`cpu-clock:u` record (load 24) gave the same shape: 29.0 % compile, 28.6 of it mesh.

**Why the mesh share is out of reach.** Every mesh pipeline carries override constants
that only exist at run time: `slots_present` and `emissive_rows` from the model's
textures on every pipeline, `alpha_test_ref` from a model's materials, the six `flame_*`
numbers off the disc, `material_lod_bias` from the title, `aniso_max` from the player's
setting, and about twenty 0/1 flags decided by the model (`mesh_render::build_with`,
`texlod::constants`). The build has no disc, and naga 30's SPIR-V writer cannot emit
specialization constants (`back/spv/block.rs:724` and `writer.rs:2772` return
`Error::Override`), so there is no build-time blob to hand over for any mesh stage. What
build-time SPIR-V can reach is the non-mesh 1.6 % (0.5 + 1.1), about 0.35 G per race
launch, a fixed amount whatever the test does after. The cheapest race launch a module can
make, `--race --screenshot --ticks 1` (same build and cache, three runs, load 47), is 11.94,
11.94, 11.94 G, so no race-spawning module can save more than about 2.9 %. Below the bar:
nothing was built.

**Traps recorded for any later passthrough work** (`wgpu-hal` 30.0.2, `wgpu-core` 30.0.2):

- Constants are silently dropped. `wgpu-core` passes each stage's `constants` down
  (`device/resource.rs:4681`, `:4826`), and `compile_stage`'s `ShaderModule::Raw` branch
  (`vulkan/device.rs:706-707`) never reads them. A passthrough module with overrides draws
  with the defaults and reports nothing. Its interface is not validated either
  (`validation.rs:325`, `ShaderMetaData::Passthrough` has none).
- Bindings are renumbered per layout. Every bind group layout entry gets a dense Vulkan
  binding by its position (`vulkan/device.rs` around 1416 to 1455), and the pipeline
  layout hands that map to naga (around 1611 to 1630). A blob must carry the same numbers
  and binding-array sizes as the layout it is used with.
- The writer options depend on the device: SPIR-V version from the device's API version,
  `Unchecked` against `Restrict` bounds checks from robust buffer and image access,
  `LABEL_VARYINGS` off on Qualcomm, `DEBUG` from the instance flags (`vulkan/adapter.rs`
  around 2774 to 2870). These live in a private field (`naga_options`).
- `SEPARATE_ENTRY_POINTS` is set on every Vulkan device (`vulkan/adapter.rs:2299`), so
  every module stays naga IR and is compiled again for each pipeline, entry point by
  entry point.

**Repeated work, measured.** A throwaway line at the mesh scope cache's miss (not kept)
counted the stages the race compiles: 101 pipelines, 202 stages, **64 distinct**
(module, entry point, constants) keys (22 vertex, 42 fragment), all on the two mesh
modules. Blended twins and the plain-draw copies differ in blend, cull or depth state only,
so two thirds of the stage compiles write SPIR-V that an earlier pipeline already wrote.

**Designs left to the maintainer** (both break the approved invariant, "the blob is
naga's build-time output", so neither was built):

1. Memoise in process: write each distinct stage once with naga at run time and hand it to
   later pipelines through passthrough. Ceiling about 2/3 of the mesh `compile_stage`, so
   about 17 % of the launch, on the first launch and every later one. Needs the device's
   writer options mirrored by hand (above) and a guard against the binding renumbering.
2. A SPIR-V disk cache keyed on naga's version, the shader text, entry point, constants,
   layout and the device's writer options. Ceiling all of naga, about 31 % (25.4 + 5.5),
   on a warm launch. A cache read from disk is input the driver trusts, so it needs the
   recovery a passthrough path would need anyway.

3. Unmeasured, and with no `unsafe`: resolve the overrides once per distinct key with
   `naga::back::pipeline_constants::process_overrides` and hand `wgpu` the resulting IR
   (`ShaderSource::Naga`, behind `wgpu`'s `naga-ir` feature). A module with one entry point
   and no overrides skips the compaction and revalidation (`pipeline_constants.rs` around
   line 93), but `wgpu-core` still validates each module and the SPIR-V write still runs per
   pipeline, so the gain is some part of `process_overrides`' 15.4 %. Measured in the Sixth
   round: 6.2 % of a race walk, 8.6 % of a bare race launch.

Neither of the first two has a device-lost fallback to lean on: the native build registers no device-lost
callback (only the web build does, `oag-game`'s `app/gpu.rs`).

Reproducing: build `oag-game` with `RUSTFLAGS=-Cforce-frame-pointers=yes` into a separate
`CARGO_TARGET_DIR`, prime `MESA_SHADER_CACHE_DIR` with one run, then run the Pulse PSP
`--drive` walk of the Fourth round (`boot; select racebox; ...; until stage=race within
4000; ticks 400;`, `--seed 7 --no-audio --no-intro`, headless, `OAG_MODS` and `XDG_*` on a
scratch profile) under `perf stat -e instructions:u` and the `perf record` above; split with
`perf report --children --no-inline --sort sym`.

### Sixth round (2026-11-24): resolving the overrides once per stage reaches 6 %, not 10

**Question.** The Fifth round's safe design 3: resolve each distinct (variant, stage, entry
point, constants) once with `naga::back::pipeline_constants::process_overrides` and hand
`wgpu` the result as `ShaderSource::Naga` (`wgpu`'s `naga-ir` feature on `oag-mesh`, no
`unsafe`, no passthrough), so `wgpu-hal` finds a module with one entry point and no overrides
and returns it unchanged. Bar: about 10 % of a race launch, else docs only.

**Prototype** (not landed): `mesh.wesl`'s two variants parsed and validated once into naga IR
(the shared module created from that IR, so the WGSL is parsed once and not twice); on a
pipeline cache miss, each stage got its own module from a memo; the pipeline then carried no
constants. `cached_pipeline`'s closure received the two modules in place of the shared one.

**Measured** (Pulse PSP race walk of the Fifth round, 400 ticks, same frame-pointer build
flags, warm Mesa cache, `perf stat -e instructions:u`; baseline runs before and after the
prototype runs, the bare launches alternated):

| Build | Race walk, G user instructions | Race launch (`--race --screenshot --ticks 1`), G |
| --- | ---: | ---: |
| Baseline | 22.009, 22.009, 22.006 | 12.034, 12.032 |
| Resolved modules, overrides arena left as naga returns it | 22.958 (**+4.3 %**) | not run |
| Resolved modules, arena cleared (the prototype) | 20.632, 20.629 (**-6.2 %**) | 10.997, 10.997 (**-8.6 %**) |
| The prototype with the shared module replaced by a stub, a ceiling test | 20.590, 20.587 (-6.4 %) | not run |

The run-to-run spread is 3 M instructions in 22 G. Instruction counts do not move with load
(the runs were at load 23 to 55). The 1-tick headless screenshot of the baseline and the
prototype is byte-identical (`cmp` on the PNGs, two pairs). HD and the menu were not captured:
the result stopped below the bar first.

**The trap that cost the first attempt.** `process_overrides` documents its result as having
"an empty `overrides` arena", but naga 30.0.2 puts the arena back before it revalidates
(`back/pipeline_constants.rs`, `module.overrides = overrides;` just above the final
`revalidate`). A resolved module that still lists overrides fails `process_overrides`'s early
return (`entry_points.len() <= 1 && overrides.is_empty()`, around line 93), so `wgpu-hal`
resolved every one of the 202 stages again on top of our 44 and the launch got 4.3 % worse.
Every use of an override is a constant by then, so assigning `module.overrides = Default::default()`
on the resolved module restores the documented contract; the prototype did exactly that, and a
naga bump that changes it shows as the early return failing (a unit test on `Cow::Borrowed`
would pin it).

**Why it stops at 6 %.** `perf record` after the change (`children` percentages of the
walk): `compile_stage` fell from 25.2 % to 11.5 %, and of what is left 9.9 % is
`spv::write_vec`, which still runs for every pipeline (`wgpu-hal` keeps no SPIR-V between
pipelines, and `SEPARATE_ENTRY_POINTS` keeps every module as IR). `process_overrides` fell
from 15.3 % to 6.8 %, now all of it ours, for 44 resolved stages (counted on a 50-tick run;
the Fifth round's census found 64 keys, a difference this round did not chase). Each
resolved stage still validates twice, once inside `process_overrides` (`revalidate`, 3.7 %)
and once in `wgpu-core`'s `create_shader_module`, and neither can be skipped without `unsafe`.
The ceiling test shows the shared module's own validations are 0.2 % and not worth removing.

**What could reach the bar**, as an estimated ceiling and not a measurement, is a cache in `wgpu-hal`'s `compile_stage` keyed on (module,
entry point, constants, binding map), which also removes the 9.9 % SPIR-V write for the
repeated stages: the stages compile 44 times instead of 202, so at most about 78 % of
`compile_stage`'s 25 % (about 19 % of the launch, 44/202 arithmetic), with no change to what
is drawn. That is a patch to
`wgpu-hal` (vendored or upstream), which this lane did not make without the lead's word.
The prototype above is the cheaper half: 6 % of a race walk and 8.6 % of a bare race launch,
no `unsafe`, byte-identical in the one frame compared, about 150 lines in `oag-mesh`'s
`pipeline_cache`, and it is what a later `wgpu-hal` cache would sit on top of.

## Reproducing

The scripts that produced every table are under the lane's scratch directory
and not committed; the commands are the `--race --screenshot` and
`--screenshot` runs above, three times each, with `XDG_*` pointed at a
scratch profile.
