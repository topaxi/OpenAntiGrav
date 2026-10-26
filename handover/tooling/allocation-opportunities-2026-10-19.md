---
status: measure
titles: [all]
updated: 2026-10-19
summary: "Print the psys sprite-sheet fill for one race; measure original-tier hull clone and net correction rate"
---

# Allocation opportunities, ranked (2026-10-19)

A read-only scout of heap churn and peak memory across a race: load, the
per-frame path, the per-view path and the net client's snapshot copies. Nothing
here is implemented. Each item names its place, how often it runs, an estimated
saving (labelled), the command that measures it, and the risk. **Measured** means
a number from this machine. **Reading** means it comes from the code and is not
yet isolated.

Setup: `--race` on Talon's Junction (`data/images/pulse-psp-usa.chd`), `--ticks
600`, headless, `--no-audio`. Load average 26 to 48 during the runs.

## Measured

- **Peak heap, release build (`target/release/oag-game`, the main checkout's
  07:25 binary; its commit is not recorded): 254.9 MB** under heaptrack. Peak
  consumers by innermost crate: `oag_music` `read_cached` 101.6 MB, `oag_fx`
  `psys` sprite sheet 68.6 MB, `oag_mesh` 31.4 MB, `oag_vex` textures 20.6 MB,
  `oag_formats` `sblk` ADPCM 6.1 MB, `oag_sound` `sfx` layers 3.4 MB.
- **Quick build** (opt-level 0): peak 374 MB, but 135.8 MB of it is the Vulkan
  validation layer, and 165 MB is an unattributed `gimli` path that **does not
  appear in the release run**. Debug-build only, so not on this list.
- **Frame path, one bench frame (quick build, perf probe on):** 565 allocations,
  622 KiB. Marks: `scenery-anims` 162 allocs, 52 KB; `fog+zonevis` about 175
  allocs, 140 KB (this span also covers the code between the previous frame's
  last mark and this frame's first); `shadow-gather` 56 allocs, 279 KB;
  `scene-pass` 24 allocs, 135 KB.
- **`shadow-gather` with the hull cache hit** (`OAG_RENDER_BENCH_FRAMES_PER_TICK=1000`,
  so the gather is not redone per frame): still 23 allocs and 213 KB. Redoing the
  gather every frame (the default bench, which forgets the cache each frame) adds
  33 allocs and 66 KB. So the steady-state cost is the 213 KB, which is the
  `original` tier's clone and the shadow quads together. Not split.
- **Capture frame at tick 600:** 942 allocations, 1,044 KiB. The 377-allocation
  gap to the bench frame is outside `Scene::render`. Not isolated.

## Worth a lane now

1. **The particle sprite sheet is a 64 MiB CPU image and a 64 MiB GPU texture,
   4096 squared whatever is placed on it.** Measured: 68.6 MB of the 254.9 MB
   release peak.
   - Where: `crates/fx/src/psys/sprite.rs:58` (`SHEET_SIZE = 4096`), `:314`
     (`Sheet::place` allocates `vec![0; side * side * 4]` on first placement),
     and `crates/fx/src/psys/pipeline.rs:242` (the GPU texture). The `Library` is
     built at `crates/raceplay/src/load.rs:684` and stored in the race's own state
     at `load.rs:1005` (behind the `view` feature). **The CPU copy is resident
     for the whole race**, not only at load.
   - How often: once per race load, held until the race ends. `sync_sheet`
     (`pipeline.rs:316`) uploads it once per generation.
   - Expected win (estimate): 64 MiB of resident memory per race, about a quarter
     of the release peak. If the sheet is sized to its packed height (`Sheet`
     tracks `shelf`), the 64 MiB VRAM texture shrinks too. **The used fraction is
     not measured.** Placement packs from the top left, so the fill is probably
     small. Measure it first (open, below).
   - Measure: the release `heaptrack` command below. The peak should drop by about
     68 MB, and `sprite.rs:314` should leave the top peak list.
   - Risk: render side only, not simulation. Uploaded texels must stay identical
     for the same placements. Keep the `generation` contract: a changed sheet must
     report a new generation.

2. **`music::at3::read_cached` holds two copies of a track's PCM at once.**
   Measured: 101.6 MB of the release peak, the largest consumer.
   - Where: `crates/music/src/at3.rs:488`, `read_cached`:
     `from_s16le(&std::fs::read(path).ok()?, format)`. `std::fs::read` makes the
     whole raw file in a `Vec<u8>`, and `from_s16le` (`at3.rs:498`) then collects
     a `Vec<i16>` of the same length without `with_capacity`. Both are alive until
     `from_s16le` returns, and the collect regrows its buffer as it goes.
   - How often: once per track decode: race music and the front end
     (`at3.rs:410` and `:416`), and the movie audio
     (`crates/game/src/movie/container_audio.rs:92`).
   - Expected win (estimate): the raw bytes are a second copy of the PCM. Removing
     them cuts the transient peak by about one track, 40 to 60 MB, using the
     existing thread's 55 to 60 MiB per track. `with_capacity` removes the regrowth
     copies.
   - **The residency is not new:** the whole-track `i16` buffer and its 120 MiB
     peak are in `handover/audio/streaming-decode-for-audio-would-break-seek-and.md`.
     What is new is the transient second copy.
   - Measure: the release `heaptrack` run; `oag_music read_cached` should drop by
     one track's bytes.
   - Risk: audio, not simulation. The decoded samples must stay bit-identical:
     the existing `from_s16le` tests in `crates/music/src/at3/tests.rs` cover the
     shape rule, and a sample-by-sample comparison covers the values.

3. **The `original` shadow tier clones its hull gather every frame.**
   - Where: `crates/raceplay/src/scene/frame/hull_cache.rs:111` (`return
     current.vertices.clone();` when `alpha >= 1.0`), and `hull_cache.rs:73`
     (`models: Vec<Mat4>`, built on every call).
   - How often: **once per frame, not per view.** `upload_shadow_geometry` gates
     the gather on `does_frame_work()` (`crates/raceplay/src/scene/frame/shadow.rs:130`):
     the first layer does the frame's shared work and the rest reuse it. A split
     screen does not multiply this.
   - Expected win: the clone is the only per-frame copy of the gather, so most of
     the 213 KB steady-state stage (estimate; its exact share is **not measured**).
     Between ticks the blend path (`HullGather::blend`, `crates/raceplay/src/hull_gather.rs:75`)
     allocates a fresh `Vec` too, so a fix must cover both or it only halves the win.
   - Measure: `OAG_RENDER_PERF=1 OAG_RENDER_BENCH=50 OAG_RENDER_BENCH_FRAMES_PER_TICK=1000`
     on a `--features perf-probe` build. The `shadow-gather` mark should fall from
     23 allocs and 213 KB to a few.
   - Risk: render side. Bit-identical at `alpha >= 1`, since the same vertices are
     drawn.

4. **The net client copies a whole `RaceSim` twice per correction.**
   - Where: `crates/net/src/client.rs:611` (`let before = race.sim.clone();` in
     `correct()` at `:607`, used only by `weapon_sync::change`). Then
     `crates/raceplay/src/correction.rs:73` restores the authoritative sim by clone
     (`crates/raceplay/src/reconcile.rs:50`). The catch-up path at `client.rs:549`
     clones `confirmed` and then stores it.
   - How often: per correction. The count is **not measured**. The client keeps a
     `corrections` counter (`crates/net/src/stats.rs:97`), printed once a second by
     `log_second` in `crates/game/src/net_play.rs:391`.
   - Expected win: each `RaceSim` clone is 1.7 MiB and 63 us (measured, in
     `docs/networking/README.md:212`). A correction costs two clones now, about
     3.4 MiB transient and about 0.13 ms.
   - **Mostly already written down.** The 0.74 MiB immutable track half and the
     lever to split it are in `docs/networking/README.md:232`, and the choice to
     keep whole-struct clones is recorded in the split-screen network thread. The
     new part is the per-correction clone count. Removing `before` drops one of
     the two clones.
   - Measure: `OAG_SWEEP=1 OAG_SWEEP_TRACKS=09_Track OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all --no-capture -E 'test(/reconcile_cost/)'`
     for the per-clone cost. Count corrections in a two-seat session with the
     one-line-a-second log.
   - Risk: determinism-bound. Removing the `before` clone must not change what
     `weapon_sync::change` reports. Sharing the track half would touch
     `oag-physics`' `CollisionWorld` (a lazily built `OnceLock` broadphase).
     `online-weapons-polish` also works in `oag-net`: coordinate the merge.

5. **The HUD rebuilds every label string and every cropped bar per view per
   frame.** Reading, not isolated.
   - Where: `crates/hud/src/draw.rs:709` `draw_list`. The sprite loop (`:747`)
     calls `crop_horizontally` (`draw.rs:70`), which starts with `sprite.clone()`,
     and `Sprite` owns two `String`s (`crates/hud/src/widget.rs:107`). The label
     loop (`:877`) calls `text_for` (`:131`), which returns an owned `String` for
     every label. `crates/game/src/hud_overlay.rs:76` (`views_frame`) clones the
     whole `Readout` per view when a sight is present.
   - How often: per view per frame, every frame.
   - Expected win: several dozen `String` and `Vec` allocations per view per frame.
     Estimate: 15 to 40 percent of the 377-allocation gap above.
   - Measure: a `perfprobe::mark` before and after `draw_list` and `paint` in
     `crates/game/src/hud_overlay.rs`, on the perf-probe build. This needs a local,
     uncommitted mark first.
   - Risk: render side, no simulation. Label text must stay identical.

- **Negative result:** the sim tick allocates nothing that reading could find.
  `World` is fixed size (`crates/gameplay/src/world.rs:531`, an 18,816-byte
  ratchet). `crates/raceplay/src/tick.rs`, `crates/race/src/state.rs` and
  `crates/physics` build no `Vec` or `String` that I could find. This is a reading,
  not a per-tick count. A per-tick count needs the perf probe around `Race::tick`.

## Later

6. **Release peak, unisolated:** `oag_mesh` 31.4 MB and `oag_vex` textures 20.6 MB
   are the next two consumers. Mesh is probably CPU staging for model uploads. The
   textures are the per-load decodes that
   `handover/rendering/omega-race-peak-holds-every-texture-cpu-side-before-any-upload.md`
   covers. Read the heaptrack leaf frames for `oag_mesh` before naming a fix.
7. **Small per-frame `Vec`s:** `rank_references` (`crates/raceplay/src/scene/frame/shadow.rs:298`,
   one `Vec<Vec3>` a frame), `ghost_hull` (`crates/raceplay/src/drawable.rs:1024`,
   the draw-range `collect`; how often it runs is not checked), and
   `DrawList(Vec::with_capacity(64))` (`crates/raceplay/src/scene/frame.rs:936`,
   inside `prepare_view`, so once per view). Each is a handful of bytes to a few KB.
8. **`scenery-anims` (162 allocations a frame) is not isolated.** The mark covers
   `write_scenery_anims` (`crates/raceplay/src/scene/frame/anims.rs:8`),
   `render_behind_glass` and `adverts::render`. One candidate is
   `Model::sample_anim_nodes` (`crates/mesh/src/mesh/anim_node.rs:88`), which returns
   a fresh `Vec` per call from `write_node_anims` (`drawable.rs:745`). Put a mark
   between the calls before changing anything.
9. **`Drawable` keeps the CPU `Arc<Model>` after upload** (`crates/raceplay/src/drawable.rs:21`).
   The hull gather and the ghost need it, so this is not waste as it stands.

## Commands

```sh
# Peak and allocation sites, release build. Unset the display so heaptrack does not open its GUI.
env -u DISPLAY -u WAYLAND_DISPLAY heaptrack -o heap \
  target/release/oag-game data/images/pulse-psp-usa.chd --race --screenshot out.png --ticks 600 --no-audio
heaptrack_print -f heap.zst -F stacks.txt --flamegraph-cost-type peak
# Perf probe build (per-stage allocation marks; CPU numbers from a quick build are not usable)
cargo build --profile quick -p oag-game --features perf-probe
OAG_RENDER_PERF=1 OAG_RENDER_BENCH=50 target/quick/oag-game \
  data/images/pulse-psp-usa.chd --race --screenshot out.png --ticks 600 --no-audio
```

`heaptrack` launches `heaptrack_gui` on exit whenever a display is set, and that
blocked a wrapper shell for 550 seconds in one run. Unset `DISPLAY` and
`WAYLAND_DISPLAY` for every heaptrack run.

## Open

- **The sprite-sheet fill is not measured.** A one-off, uncommitted print of
  `max(y + h)` in `Sheet::place` on one race settles item 1: a small fill means
  shrink or drop the sheet; a large one means it is not the win it looks like.
- **The `original` tier's clone share is not measured** (item 3). A mark between
  the clone and the blend would count it.
- **The correction rate is not measured** (item 4). It needs a two-seat session
  and the one-line-a-second log.
- The release peak is one run on one circuit. Repeat on a second circuit before
  quoting a percentage.

## Next Steps

1. Print the sprite-sheet fill for one race (open, item 1). That decides shrink or drop.
2. Make the music decode read straight into `i16` with a capacity (item 2). Measure
   on the release heaptrack run.
3. Split the hull clone and the blend allocation (item 3), with the perf-probe
   before and after.
