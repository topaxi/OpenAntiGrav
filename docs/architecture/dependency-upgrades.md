# Dependency upgrades

The state of every external dependency on 2026-11-09 (workspace `main` at
`34fef176a`, toolchain 1.99.0, `rust-version` 1.97.1), what each upgrade
changed, and every changelog read between the old version and the new. The rule
the lane followed: a dependency is at its newest release (majors included) or
is held back with a named reason below, and a change in a changelog is sorted
into **applied** (a commit), **threaded** (written into
the handover thread "Dependency upgrade benefits not yet applied", which lives
only until its items land) or **not relevant** (with the reason). Nothing here
is a measurement of the original; the numbers are this machine's, under load,
and say so.

Reproduce the inventory: `cargo update --dry-run --verbose` lists what the
lockfile can still take and what is "behind latest"; `cargo outdated --workspace
--root-deps-only` is **not enough on its own** (it skipped `aes`, `hrtf`,
`num-bigint` and `x11rb` here), so the newest release of each direct dependency
was also read from the crates.io sparse index.

## Gates

Determinism: the simulation-reachable crates (`glam`, `flate2`, `zlib-rs`,
`syn`, `unicode-ident`) went in first and alone; the six determinism-bound
crates' 1,209 tests passed, committed hash references untouched. No hash
moved, so no crate is held for determinism.

`race_ground_truth::a_lone_craft_gets_round_the_circuits_it_is_known_to_get_round`
runs the same on the baseline tree and on this branch: the lap table is
byte-identical (lap time, laps, respawns and tick count per circuit), and the
verdict is the same both times.

```text
clean laps: ["16_Track", "03_Track", "02_Track", "10_Track", "05_Track", "04_Track", "09_Track", "14_Track", "01_Track", "13_Track", "06_Track", "07_Track"]
no clean lap: []
```

Per-lap and end-of-run shield: this test prints neither (it prints lap time,
laps, respawns and tick count), so no shield figure is claimed. The lap table
above is the evidence; its run time is not compared across the two trees (the
baseline run came straight after a build, the later ones warm).

Suite time: `just test-data` is 438 s on the baseline (ceiling 450 s). On this
branch the first run took 476 s at a 15-minute load average of 25 (a browser
and a wine server were running) and failed only `check-test-budget`; a second,
full run (`OAG_TEST_DATA_FULL=1`, all 119 binaries, 11,633 tests, all passed)
took 432 s at 1-minute loads of 7.6 at the start and 12.7 at the end. So the
upgrade does not push the suite past its ceiling; the 476 s was machine load.

Not compiled: the optional `native-video` feature (`gstreamer` 0.25.4 and
its `-sys` crates took lockfile bumps), `ballisticng` beyond `just lint`, and
the web build with `wasm-bindgen` 0.2.130. Run `scripts/build-web.sh --dev`
and `cargo check -p oag-game --features native-video` before relying on them.

## Upgraded to a new major or a behaviour-changing release

| Crate | From | To | Source | What changed that matters to us | Verdict |
| --- | --- | --- | --- | --- | --- |
| `aes` | 0.8.4 | 0.9.4 | `aes/CHANGELOG.md` in <https://github.com/RustCrypto/block-ciphers> | `cipher` 0.5 and `hybrid-array` replace `generic-array`: `BlockCipher{En,De}crypt`, `Array`, `cast_from_core`; 0.9.2-0.9.4 fix an x86 regression, add VAES-256/512 backends by default, shrink the software round keys | **applied** (commit `ba47d12a2`). Call sites in `oag-disc` (`ps3_crypt.rs`, `ps4_pkg/crypto.rs`) moved; **PS3 `decrypt_sector` now decrypts the 2 KiB sector as one `decrypt_blocks` batch** (the cipher pipelines blocks, as the PS4 XTS path already did): 5.96 us to 1.20 us per sector (328 to 1,623 MiB/s) in a scratch micro-benchmark, and `ps3_crypt_ground_truth::every_file_on_the_disc_reads_byte_identical_to_the_decrypted_copy` 9.45 s to 3.29 s on the same machine (the second figure includes a warmer page cache, so read it as direction only). All 99 `oag-disc` tests pass with the disc images (PS3 and PS4 ground truths included) |
| `hrtf` | 0.9.0 | 0.10.0 | commit log of <https://github.com/mrDIMAS/hrtf> (no changelog; 0.10.0 published 2026-11-08) | `HrtfProcessor::new(sphere, block)` replaces `(sphere, steps, block)`; every block now crossfades the previous direction into the new one (the fix for buzzing under a rapidly changing sampling vector); per-ear tails become one `history`; `new_gain`/`prev_gain`; benchmarks and FFT performance work; rubato 5 (drops `realfft`, `audio-core`) | **applied** (commit `317bcc496`). `binaural.rs` ported; the convolver block is **513 frames** (a power of two plus one, which the crate's docs say plans the fast FFT): 1.29 ms of CPU per second of one voice at 513 against 3.35 ms at 512, 3.04 ms at 256 (the 0.9 build was not timed, so no before/after of the crate itself). `oag-audio`'s 142 tests pass. Gain interpolation and a Deck rerun are in the thread |
| `resvg` (with `usvg`) | 0.48.1 | 0.49.0 | `CHANGELOG.md` in <https://github.com/linebender/resvg> | `plus-lighter` blend, `feComposite lighter`, text baseline and RTL fixes, filters in rotated groups, marker clipping, clipped layer bounds | **applied** (commit `b4d2f4eb2`), no call-site change. **Not relevant** to the two SVGs we rasterise (`oag-game`'s cursor and the app icon); the 256 px icon rendered unchanged to the eye. `tiny-skia` 0.11 and 0.12 were both in the tree already (winit's `sctk-adwaita` takes 0.11), so no new duplicate |
| `flate2` | 1.1.9 | 1.1.11 | `CHANGELOG.md` in <https://github.com/rust-lang/flate2-rs> | see its row below; brings `miniz_oxide` 0.9 beside `png`'s 0.8 | **applied** (commit `3749a2733`); the `Cargo.toml` comment on `flate2` names the second copy, which is not the inflater (zlib-rs is) |

## Held back, with the reason

| Dependency | In tree | Newer | Reason |
| --- | --- | --- | --- |
| `num-bigint` | 0.4.8 | 0.5.1 | 0.5.0 only splits the `rand` features (`<https://github.com/rust-num/num-bigint>`, `RELEASES.md`) and changes none of the API `oag-disc` calls (`BigUint::from_bytes_be`, `modpow`). `num-rational` (through `av-data`, the AV1 decoder's) still takes 0.4, so moving would add a second copy for nothing. Revisit when `av-data` moves |
| `x11rb` | 0.13.2 | 0.14.0 | 0.14 drops abstract unix socket connections (as libxcb did), bumps its MSRV and `raw-window-handle`; nothing for RandR (commit log `v0.13.2...v0.14.0` of <https://github.com/psychon/x11rb>). winit 0.30 takes 0.13, so it would be a second copy. Revisit with winit 0.31 |
| `generic-array` | 0.14.7 | 0.14.9 | `crypto-common` 0.1.7 requires `=0.14.7`; it arrives through `sha2` 0.10 from `cranelift-codegen` (wasmtime 50.0.1, pinned `=50.0.1`). `cargo update -p generic-array --precise 0.14.9` fails on that bound. Leaves with the wasmtime pin |
| `wasmtime` | 50.0.1 | 51.0.0-rc.1 | only a release candidate; the exact pin is deliberate (determinism: fuel, NaN canonicalisation, the ABI word). `oag-ext-abi` and `oag-ext` are unchanged by this lane, so `oag-mod-author`'s skill needs no edit |
| `winit` | 0.30.13 | 0.31.0-beta.4 | beta; the migration is a thread item |
| `rustls` | 0.23.46 | 0.24.0-dev.2 | a development pre-release |
| `toml-schema` (git) | rev `fa97c07e` | trunk is newer | pinned at the newest tagged release, `rust-v1.0.0-rc.2` (2026-08-20), by ADR-0064; trunk past it is untagged |
| Rust toolchain | 1.99.0 | 1.99.1 | `rust-toolchain.toml` pins deliberately ("bump deliberately, not as a side effect"); a patch bump is the maintainer's call, not this lane's |
| GitHub Actions | `checkout` 7, `upload-artifact` 7, `download-artifact` 8, `setup-python` 7, `rust-cache` 2, `wrangler-action` 4, `setup-chrome` 2 | the same majors | already at each action's newest major; `dependabot.yml` keeps them. Not verified by running CI (CI is never pushed to from a lane) |

## Direct dependencies already at their newest release

`wgpu` 30.0.1, `naga` 30.0.1, `wesl` 0.6.0, `cpal` 0.18.2, `gilrs` 0.11.2,
`symphonia` 0.6.1, `tokio` 1.54.1, `clap` 4.6.7, `serde` 1.0.229,
`serde_json` 1.0.154, `thiserror` 2.0.21, `anyhow` 1.0.104, `bytemuck` 1.25.2,
`png` 0.18.1, `gltf` 1.4.1, `rcgen` 0.14.10, `ring` 0.17.14, `arboard` 3.7.0,
`dirs` 7.0.0, `chd` 0.4.0, `re_rav1d` 0.1.3, `atrac9dec` 0.9.1, `libm` 0.2.16,
`pipewire` 0.10.1, `libspa` 0.10.1, `rtrb` 0.4.1, `lzma-rs-perf-exp` 0.2.1
(exact pin), `url` 2.5.8 and every other direct dependency the tables above do not name. For these there is no old-to-new changelog to read. Because the lane
asked for benefits, wgpu 30's own release notes (`CHANGELOG.md`, `v30.0.0` and
`v30.0.1` in <https://github.com/gfx-rs/wgpu>) were read for features not yet
used:

- **Surface colour space and `display_hdr_info`**: threaded (the HDR thread).
- **`PipelineCache`**: not applicable, `create_pipeline_cache` is `unsafe`
  (`unsafe_code = "deny"`; the maintainer's rule against wgpu's unsafe escape
  hatches). Named in the thread so nobody re-derives it.
- **`Queue::present`, optional vertex buffer slots, `dispatch_workgroups`
  renames, `Buffer::get_mapped_range` returning `Result`**: already migrated
  (the tree builds against 30.0.1).
- **Relaxed locking in `wgpu-core` (submit while another thread polls)**: not
  relevant; no thread of ours blocks in a device poll while another submits
  (not measured).
- **`StagingBelt::finish_and_recall_on_submit`**: not relevant, `StagingBelt`
  is not used (`rg StagingBelt crates` finds none).

## Lockfile-only upgrades (semver-compatible)

`cargo update` took 113 more packages to their newest compatible release,
removing `defmt`, `objc2` 0.5/0.6 and `pin-utils` from the tree and adding
none (commit `1a5547382`; `cargo check --workspace --all-targets` is clean).
Changelogs were fetched from GitHub (`CHANGELOG.md` / release notes / the
commit log) and read for the direct dependencies and the hot paths; the rest
is sorted by what the notes say, and "nothing in the notes touches an API of
ours" is a claim about a read changelog, not a guess. Crates published with no
changelog or release notes were sorted from their commit log where it
mattered (`crc32fast`, `opus-rs`, `x11rb`).

| Crate | From | To | Source | Relevant changes | Verdict |
| --- | --- | --- | --- | --- | --- |
| `android_system_properties` | 0.1.5 | 0.1.6 | commit log of <https://github.com/nical/android_system_properties> (0.1.5 to 0.1.6) | patch | Android only; not relevant to the desktop gate |
| `annotate-snippets` | 0.12.16 | 0.12.17 | CHANGELOG.md in <https://github.com/rust-lang/annotate-snippets-rs> (0.12.16 to 0.12.17) | `no_graphics` renderer, hyperlink flag | transitive (naga diagnostics); not relevant |
| `bitflags` | 2.13.1 | 2.13.2 | CHANGELOG.md in <https://github.com/bitflags/bitflags> (2.13.1 to 2.13.2) | 2.13.2: const declaration hoist | transitive; not relevant |
| `bytemuck_derive` | 1.11.0 | 1.12.1 | changelog.md in <https://github.com/Lokathor/bytemuck> (1.11.0 to 1.12.1) | 1.12.1 derive patch (bytemuck 1.25.2: `NoUninit` for char/bool/NonZero arrays) | applied by the lock; no derive of ours changes |
| `cc` | 1.4.0 | 1.11.1 | CHANGELOG.md in <https://github.com/rust-lang/cc-rs> (1.4.0 to 1.11.1) | 1.4.0 to 1.11.1: MSVC CodeView remapping, wasm32-none-component, cl.exe `-Tp`, fewer panics | build scripts only; not relevant to runtime |
| `cfg-expr` | 0.20.8 | 0.20.10 | CHANGELOG.md in <https://github.com/EmbarkStudios/cfg-expr> (0.20.8 to 0.20.10) | builtin target list updated to 1.99.0 | build scripts only (system-deps) |
| `cfg-if` | 1.0.4 | 1.0.5 | CHANGELOG.md in <https://github.com/rust-lang/cfg-if> (1.0.4 to 1.0.5) | 1.0.5 marks the crate deprecated in favour of `cfg_select!` | transitive; we have no direct use (`rg cfg_if` finds none) |
| `clap_lex` | 1.1.0 | 1.1.1 | CHANGELOG.md in <https://github.com/clap-rs/clap> (1.1.0 to 1.1.1) | 1.1.1 patch (`unstable-v5` items) | transitive of clap; not relevant |
| `clipboard-win` | 5.4.1 | 5.4.3 | commit log of <https://github.com/DoumanAsh/clipboard-win> (5.4.1 to 5.4.3) | patch release | transitive (arboard, Windows); not relevant on Linux |
| `combine` | 4.6.7 | 4.6.9 | CHANGELOG.md in <https://github.com/Marwes/combine> (4.6.7 to 4.6.9) | partial-parsing fixes | transitive; not relevant |
| `crc32fast` | 1.5.0 | 1.5.4 | commit log of <https://github.com/srijs/rust-crc32fast> (1.5.0 to 1.5.4) | SIMD folds widened on x86, 3-way ARM, faster small inputs and tails (1.5.1-1.5.4) | applied by the lock (png, zip, flate2 checksums); not measured |
| `crossbeam-utils` | 0.8.22 | 0.8.23 | CHANGELOG.md in <https://github.com/crossbeam-rs/crossbeam> (0.8.22 to 0.8.23) | 0.8.23 patch | transitive; not relevant |
| `either` | 1.17.0 | 1.19.0 | RELEASES.md in <https://github.com/rayon-rs/either> (1.17.0 to 1.19.0) | 1.19.0 adds `is_left_or`/`is_right_or` | transitive; not relevant |
| `equivalent` | 1.0.2 | 1.0.3 | commit log of <https://github.com/indexmap-rs/equivalent> (1.0.2 to 1.0.3) | patch | transitive; not relevant |
| `find-msvc-tools` | 0.1.9 | 0.1.15 | CHANGELOG.md in <https://github.com/rust-lang/cc-rs> (0.1.9 to 0.1.15) | same release train as `cc` | build scripts only |
| `fixedbitset` | 0.5.7 | 0.5.9 | release notes of <https://github.com/petgraph/fixedbitset> (0.5.7 to 0.5.9) | overflow soundness fix in `grow_and_insert` | transitive (petgraph in naga); not relevant |
| `flate2` | 1.1.9 | 1.1.11 | CHANGELOG.md in <https://github.com/rust-lang/flate2-rs> (1.1.9 to 1.1.11) | 1.1.10-1.1.11: gzip header reads separated from data, sync flush across buffer boundaries, write decoders reject incomplete streams, `Decompress::reset` keeps window bits, native zlib-rs uninitialised output (no zeroing) | applied by the lock: we call `Decompress` (raw deflate and zlib) on PSARC, WAD, VPK and PFSC data, so the gzip changes are not reached; `Decompress::reset(false)` per PS4 PFSC sector (`oag-disc` `ps4_pkg/layers.rs`) is the path the reset fixes touch, and `ps4_pkg_ground_truth` reads it byte-identical; speed not measured separately |
| `futures` | 0.3.33 | 0.3.34 | CHANGELOG.md in <https://github.com/rust-lang/futures-rs> (0.3.33 to 0.3.34) | 0.3.34: preserved cloned waker identity, syn 3 | transitive; not relevant |
| `futures-channel` | 0.3.33 | 0.3.34 | CHANGELOG.md in <https://github.com/rust-lang/futures-rs> (0.3.33 to 0.3.34) | 0.3.34: syn 3, waker identity | transitive; not relevant |
| `futures-core` | 0.3.33 | 0.3.34 | CHANGELOG.md in <https://github.com/rust-lang/futures-rs> (0.3.33 to 0.3.34) | 0.3.34 | transitive; not relevant |
| `futures-executor` | 0.3.33 | 0.3.34 | CHANGELOG.md in <https://github.com/rust-lang/futures-rs> (0.3.33 to 0.3.34) | 0.3.34 | transitive; not relevant |
| `futures-macro` | 0.3.33 | 0.3.34 | CHANGELOG.md in <https://github.com/rust-lang/futures-rs> (0.3.33 to 0.3.34) | 0.3.34: syn 3 | proc macro; not relevant |
| `futures-sink` | 0.3.33 | 0.3.34 | CHANGELOG.md in <https://github.com/rust-lang/futures-rs> (0.3.33 to 0.3.34) | 0.3.34 | transitive; not relevant |
| `futures-task` | 0.3.33 | 0.3.34 | CHANGELOG.md in <https://github.com/rust-lang/futures-rs> (0.3.33 to 0.3.34) | 0.3.34 | transitive; not relevant |
| `futures-util` | 0.3.33 | 0.3.34 | CHANGELOG.md in <https://github.com/rust-lang/futures-rs> (0.3.33 to 0.3.34) | 0.3.34 | transitive; not relevant |
| `generator` | 0.8.11 | 0.8.12 | commit log of <https://github.com/Xudong-Huang/generator-rs> (0.8.11 to 0.8.12) | 0.8.12 patch | transitive (loom); not relevant |
| `glam` | 0.34.1 | 0.34.3 | CHANGELOG.md in <https://github.com/bitshifter/glam-rs> (0.34.1 to 0.34.3) | 0.34.2-0.34.3: camera `model` module, `rotate_towards` jitter fix for negative `max_angle`, neon/wasm min/max parity, serde fixes, `mul_transpose_vecn` renamed | not relevant: simulation uses scalar math and calls none of these (`rg rotate_towards` finds nothing); determinism and 1,209 simulation-crate tests unchanged |
| `gstreamer-app-sys` | 0.25.0 | 0.25.4 | crates.io page of <https://gitlab.freedesktop.org/gstreamer/gstreamer-rs> (0.25.0 to 0.25.4) | 0.25.4 | optional `gstreamer` feature only |
| `gstreamer-base` | 0.25.3 | 0.25.4 | crates.io page of <https://gitlab.freedesktop.org/gstreamer/gstreamer-rs> (0.25.3 to 0.25.4) | 0.25.4 bindings patch | optional `gstreamer` feature only; not compiled in the gate |
| `gstreamer-base-sys` | 0.25.3 | 0.25.4 | crates.io page of <https://gitlab.freedesktop.org/gstreamer/gstreamer-rs> (0.25.3 to 0.25.4) | 0.25.4 | optional `gstreamer` feature only |
| `gstreamer-sys` | 0.25.2 | 0.25.4 | crates.io page of <https://gitlab.freedesktop.org/gstreamer/gstreamer-rs> (0.25.2 to 0.25.4) | 0.25.4 | optional `gstreamer` feature only |
| `gstreamer-video-sys` | 0.25.3 | 0.25.4 | crates.io page of <https://gitlab.freedesktop.org/gstreamer/gstreamer-rs> (0.25.3 to 0.25.4) | 0.25.4 | optional `gstreamer` feature only |
| `hermit-abi` | 0.5.2 | 0.5.3 | commit log of <https://github.com/hermit-os/hermit-rs> (0.5.2 to 0.5.3) | 0.5.3 patch | transitive; not relevant |
| `icu_collections` | 2.3.0 | 2.4.0 | CHANGELOG.md in <https://github.com/unicode-org/icu4x> (2.3.0 to 2.4.0) | ICU4X 2.4: `split_normalized_utf8` soundness fix, segmenter performance | transitive (url/idna); not relevant |
| `icu_locale_core` | 2.3.0 | 2.4.0 | CHANGELOG.md in <https://github.com/unicode-org/icu4x> (2.3.0 to 2.4.0) | ICU4X 2.4 patch | transitive; not relevant |
| `icu_normalizer` | 2.3.0 | 2.4.0 | CHANGELOG.md in <https://github.com/unicode-org/icu4x> (2.3.0 to 2.4.0) | ICU4X 2.4: `split_normalized_utf8` soundness fix | transitive (url); not relevant |
| `icu_normalizer_data` | 2.3.0 | 2.4.0 | CHANGELOG.md in <https://github.com/unicode-org/icu4x> (2.3.0 to 2.4.0) | ICU4X 2.4 data | transitive; not relevant |
| `icu_properties` | 2.3.0 | 2.4.0 | CHANGELOG.md in <https://github.com/unicode-org/icu4x> (2.3.0 to 2.4.0) | ICU4X 2.4 patch | transitive; not relevant |
| `icu_properties_data` | 2.3.0 | 2.4.0 | CHANGELOG.md in <https://github.com/unicode-org/icu4x> (2.3.0 to 2.4.0) | ICU4X 2.4 data | transitive; not relevant |
| `icu_provider` | 2.3.1 | 2.4.0 | CHANGELOG.md in <https://github.com/unicode-org/icu4x> (2.3.1 to 2.4.0) | ICU4X 2.4 patch | transitive; not relevant |
| `indexmap` | 2.14.0 | 2.14.2 | RELEASES.md in <https://github.com/indexmap-rs/indexmap> (2.14.0 to 2.14.2) | macro hygiene, `const` empty `indexmap_with_default!` | transitive; not relevant |
| `inotify` | 0.11.4 | 0.11.5 | CHANGELOG.md in <https://github.com/hannobraun/inotify-rs> (0.11.4 to 0.11.5) | docs only | transitive (notify users); not relevant |
| `itoa` | 1.0.18 | 1.0.19 | release notes of <https://github.com/dtolnay/itoa> (1.0.18 to 1.0.19) | fixes a no-panic link failure for non-constant integers | transitive; not relevant |
| `jiff` | 0.2.35 | 0.2.41 | CHANGELOG.md in <https://github.com/BurntSushi/jiff> (0.2.35 to 0.2.41) | 0.2.41: performance and panic fixes for out-of-range timestamps | transitive (env_logger timestamps); not relevant |
| `jiff-core` | 0.1.0 | 0.1.4 | CHANGELOG.md in <https://github.com/BurntSushi/jiff> (0.1.0 to 0.1.4) | 0.2.41: panic fixes for out-of-range input | transitive; not relevant |
| `jiff-static` | 0.2.35 | 0.2.41 | CHANGELOG.md in <https://github.com/BurntSushi/jiff> (0.2.35 to 0.2.41) | 0.2.41 | proc macro; not relevant |
| `js-sys` | 0.3.106 | 0.3.107 | CHANGELOG.md in <https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/js-sys> (0.3.106 to 0.3.107) | 0.3.107: `Promise<T, E>`, `Nullable` | web build only; not relevant |
| `kio` | 0.6.3 | 0.6.4 | commit log of <https://github.com/moq-dev/moq> (0.6.3 to 0.6.4) | patch | transitive (redox); not relevant |
| `kstring` | 2.0.2 | 2.0.5 | CHANGELOG.md in <https://github.com/cobalt-org/kstring> (2.0.2 to 2.0.5) | `StackString` debug asserts at compile time; deprecation note | transitive (liquid-style string in wesl); not relevant |
| `lalrpop-util` | 0.23.1 | 0.23.2 | RELEASES.md in <https://github.com/lalrpop/lalrpop> (0.23.1 to 0.23.2) | `Location` no longer needs `Copy` | transitive (wesl's parser); not relevant |
| `lazy_static` | 1.5.0 | 1.5.1 | commit log of <https://github.com/rust-lang-nursery/lazy-static.rs> (1.5.0 to 1.5.1) | 1.5.1 patch | transitive; not relevant |
| `libc` | 0.2.189 | 0.2.191 | CHANGELOG.md in <https://github.com/rust-lang/libc> (0.2.189 to 0.2.191) | 0.2.191: platform constant additions | transitive; not relevant |
| `libredox` | 0.1.18 | 0.1.25 | crates.io page of <https://gitlab.redox-os.org/redox-os/libredox> (0.1.18 to 0.1.25) | patch | Redox only; not relevant |
| `litrs` | 1.0.0 | 1.0.1 | CHANGELOG.md in <https://github.com/LukasKalbertodt/litrs> (1.0.0 to 1.0.1) | fixes panic on a string literal with escapes and a multi-byte tail | proc-macro helper; not relevant |
| `log` | 0.4.34 | 0.4.35 | CHANGELOG.md in <https://github.com/rust-lang/log> (0.4.34 to 0.4.35) | 0.4.35: no source changes (CI/CD rework) | direct; not relevant |
| `loom` | 0.7.2 | 0.7.3 | CHANGELOG.md in <https://github.com/tokio-rs/loom> (0.7.2 to 0.7.3) | `try_update`/`update` on atomics, drops pin-utils | transitive (dev tooling of a dependency); not relevant |
| `memmap2` | 0.9.11 | 0.9.12 | CHANGELOG.md in <https://github.com/RazrFalcon/memmap2-rs> (0.9.11 to 0.9.12) | `map_if_safe`, `no_probe_handle` (Windows), OpenHarmony | transitive (winit, sctk); we have no mmap call of our own |
| `num-integer` | 0.1.46 | 0.1.47 | RELEASES.md in <https://github.com/rust-num/num-integer> (0.1.46 to 0.1.47) | Karatsuba square root for 128-bit `Roots::sqrt` | transitive (av-data); not relevant |
| `openssl-probe` | 0.2.1 | 0.2.2 | release notes of <https://github.com/rustls/openssl-probe> (0.2.1 to 0.2.2) | illumos SmartOS probing | transitive (rustls-native-certs); not relevant |
| `opus-rs` | 0.1.40 | 0.1.41 | commit log of <https://github.com/restsend/opus-rs> (0.1.40 to 0.1.41) | 0.1.41: encoder-side allocation trim like libopus, `OpusEncoder::lookahead` | not relevant: `oag-music` only decodes |
| `ordered-float` | 5.3.0 | 5.5.1 | release notes of <https://github.com/reem/rust-ordered-float> (5.3.0 to 5.5.1) | 5.5.1 restores `repr(transparent)`; MSRV 1.90; `From<OrderedFloat<f32>>` | transitive (naga); not relevant |
| `portable-atomic` | 1.14.0 | 1.15.0 | CHANGELOG.md in <https://github.com/taiki-e/portable-atomic> (1.14.0 to 1.15.0) | pre-v6 ARM atomics barrier workaround, s390x 128-bit CAS | transitive; not relevant to the targets we ship |
| `portable-atomic-util` | 0.2.7 | 0.2.8 | CHANGELOG.md in <https://github.com/taiki-e/portable-atomic-util> (0.2.7 to 0.2.8) | 0.2.8 patch | transitive; not relevant |
| `redox_syscall` | 0.9.0 | 0.9.5 | crates.io page of <https://gitlab.redox-os.org/redox-os/kernel> (0.9.0 to 0.9.5) | patch | Redox only; not relevant |
| `redox_users` | 0.5.2 | 0.5.3 | crates.io page of <https://gitlab.redox-os.org/redox-os/users> (0.5.2 to 0.5.3) | patch | Redox only; not relevant |
| `roxmltree` | 0.21.1 | 0.21.2 | CHANGELOG.md in <https://github.com/RazrFalcon/roxmltree> (0.21.1 to 0.21.2) | stack overflow on deeply nested elements fixed (iterative parse) | transitive (usvg for the cursor and icon SVGs, which we author); not relevant |
| `rustix` | 1.1.4 | 1.1.6 | CHANGES.md in <https://github.com/bytecodealliance/rustix> (1.1.4 to 1.1.6) | 1.1.6: patch | transitive; not relevant |
| `rustls-webpki` | 0.103.15 | 0.103.16 | release notes of <https://github.com/rustls/webpki> (0.103.15 to 0.103.16) | 0.103.16 distrusts a set of intermediate certificate keys for security reasons | applied by the lock; reaches `oag-net`'s TLS verification (the server and client link it) |
| `schannel` | 0.1.29 | 0.1.30 | commit log of <https://github.com/steffengy/schannel-rs> (0.1.29 to 0.1.30) | patch | Windows TLS; not relevant on Linux |
| `serde_spanned` | 1.1.1 | 1.1.3 | crates/serde_spanned/CHANGELOG.md in <https://github.com/toml-rs/toml> (1.1.1 to 1.1.3) | `size_hint` on deserialize | applied by the lock |
| `siphasher` | 1.0.3 | 1.0.4 | commit log of <https://github.com/jedisct1/rust-siphash> (1.0.3 to 1.0.4) | patch | transitive; not relevant |
| `smallvec` | 1.15.2 | 1.16.5 | release notes of <https://github.com/servo/rust-smallvec> (1.15.2 to 1.16.5) | `retain` backport, `may_dangle` ownership fix, fuzzing CI (1.16.x) | transitive (wgpu); applied by the lock, not relevant to an API of ours |
| `svgtypes` | 0.16.1 | 0.16.2 | CHANGELOG.md in <https://github.com/linebender/svgtypes> (0.16.1 to 0.16.2) | `hwb()` colours, hue angle units, no panic on bad `transform-origin` | transitive (usvg); not relevant |
| `syn` | 3.0.9 | 3.0.10 | release notes of <https://github.com/dtolnay/syn> (3.0.9 to 3.0.10) | 3.0.10: skip rescanning failed operands (macro compile time) | applied by the lock (proc-macro compile time); no call of ours |
| `system-deps` | 9.0.0 | 9.0.1 | CHANGELOG.md in <https://github.com/gdesmott/system-deps> (9.0.0 to 9.0.1) | faster pkg-config probes, truncated downloads reported | build scripts only (gstreamer-sys); not relevant |
| `tinystr` | 0.8.4 | 0.8.5 | CHANGELOG.md in <https://github.com/unicode-org/icu4x> (0.8.4 to 0.8.5) | ICU4X 2.4 patch | transitive; not relevant |
| `toml` | 1.1.3+spec-1.1.0 | 1.2.1+spec-1.1.0 | crates/toml/CHANGELOG.md in <https://github.com/toml-rs/toml> (1.1.3+spec-1.1.0 to 1.2.1+spec-1.1.0) | 1.1.4-1.2.1: fewer allocations parsing and serialising, faster ser of syntax/strings/keys, `size_hint`, duplicate-key check when deserialising to a `Table`, keys always encoded | applied by the lock (settings, menu, mod and track TOML); load time not measured separately |
| `toml_datetime` | 1.1.1+spec-1.1.0 | 1.1.3+spec-1.1.0 | crates/toml_datetime/CHANGELOG.md in <https://github.com/toml-rs/toml> (1.1.1+spec-1.1.0 to 1.1.3+spec-1.1.0) | `size_hint` on deserialize | applied by the lock |
| `toml_edit` | 0.25.13+spec-1.1.0 | 0.25.19+spec-1.1.0 | crates/toml_edit/CHANGELOG.md in <https://github.com/toml-rs/toml> (0.25.13+spec-1.1.0 to 0.25.19+spec-1.1.0) | 0.25.14-0.25.19: fewer allocations parsing and rendering, faster ser, `size_hint` | applied by the lock (`oag-raceplay` pilots editor writes through it); not measured |
| `toml_parser` | 1.1.2+spec-1.1.0 | 1.1.5+spec-1.1.0 | crates/toml_parser/CHANGELOG.md in <https://github.com/toml-rs/toml> (1.1.2+spec-1.1.0 to 1.1.5+spec-1.1.0) | removes possible unsoundness, fewer allocations for tokens | applied by the lock (every settings, menu and mod TOML parse) |
| `toml_writer` | 1.1.2+spec-1.1.0 | 1.2.0+spec-1.1.0 | crates/toml_writer/CHANGELOG.md in <https://github.com/toml-rs/toml> (1.1.2+spec-1.1.0 to 1.2.0+spec-1.1.0) | faster writing of syntax, strings and keys | applied by the lock (settings writes) |
| `unicode-ident` | 1.0.24 | 1.0.27 | release notes of <https://github.com/dtolnay/unicode-ident> (1.0.24 to 1.0.27) | trie layout for headroom, Unicode 18.0.0 | proc-macro helper; not relevant |
| `unicode-width` | 0.2.2 | 0.2.3 | commit log of <https://github.com/unicode-rs/unicode-width> (0.2.2 to 0.2.3) | 0.2.3 patch | transitive; not relevant |
| `utf8_iter` | 1.0.4 | 1.1.0 | commit log of <https://github.com/hsivonen/utf8_iter> (1.0.4 to 1.1.0) | 1.1.0 patch | transitive (idna); not relevant |
| `uuid` | 1.24.0 | 1.29.2 | release notes of <https://github.com/uuid-rs/uuid> (1.24.0 to 1.29.2) | 1.29.2: wasm64 `js`, v7 monotonicity fixes, serde adapters | transitive; not relevant |
| `wasip2` | 1.0.1+wasi-0.2.4 | 1.0.4+wasi-0.2.12 | commit log of <https://github.com/bytecodealliance/wasi-rs> (1.0.1+wasi-0.2.4 to 1.0.4+wasi-0.2.12) | 1.0.4 WASI preview 2 bindings | transitive (wasm32-wasip1 only); not relevant |
| `wasm-bindgen` | 0.2.129 | 0.2.130 | CHANGELOG.md in <https://github.com/wasm-bindgen/wasm-bindgen> (0.2.129 to 0.2.130) | 0.2.130: borrowed number/bool/char/str by copy, `Promise<T, E>` rejection type (default unchanged), `--js-string-builtins` flag, `Nullable<T>` | applied by the lock for the web build; the CLI is read from `Cargo.lock` (`scripts/build-web.sh`); no API of ours changes |
| `wasm-bindgen-futures` | 0.4.79 | 0.4.80 | CHANGELOG.md in <https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/futures> (0.4.79 to 0.4.80) | 0.4.80: `JsFuture<T, E>` follows `Promise<T, E>`, aarch64-windows artifacts | web build only; not relevant |
| `wasm-bindgen-macro` | 0.2.129 | 0.2.130 | CHANGELOG.md in <https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/macro> (0.2.129 to 0.2.130) | same release train as wasm-bindgen | web build only |
| `wasm-bindgen-macro-support` | 0.2.129 | 0.2.130 | CHANGELOG.md in <https://github.com/wasm-bindgen/wasm-bindgen/tree/main/crates/macro-support> (0.2.129 to 0.2.130) | same release train as wasm-bindgen | web build only |
| `wasm-bindgen-shared` | 0.2.129 | 0.2.130 | CHANGELOG.md in <https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/shared> (0.2.129 to 0.2.130) | same release train as wasm-bindgen | web build only |
| `wasm-encoder` | 0.262.0 | 0.263.0 | commit log of <https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasm-encoder> (0.262.0 to 0.263.0) | same release train as `wat` | dev only (`wat`) |
| `wasmparser` | 0.262.0 | 0.263.0 | commit log of <https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasmparser> (0.262.0 to 0.263.0) | same release train as `wat` | dev only (`wat`) |
| `wast` | 262.0.0 | 263.0.0 | commit log of <https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wast> (262.0.0 to 263.0.0) | same release train as `wat` | dev only (`wat`) |
| `wat` | 1.262.0 | 1.263.0 | release notes of <https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wat> (1.262.0 to 1.263.0) | 1.263: wasm-tools patch (wasm-wave and wit-parser fixes) | dev-dependency of the extension tests; not relevant |
| `wayland-backend` | 0.3.16 | 0.3.17 | wayland-backend/CHANGELOG.md in <https://github.com/smithay/wayland-rs> (0.3.16 to 0.3.17) | 0.3.17: patch (the changelog's Unreleased block lists the next breaking release) | idle inhibition (`oag_game::idle`) only; not relevant |
| `wayland-protocols` | 0.32.12 | 0.32.13 | wayland-protocols/CHANGELOG.md in <https://github.com/smithay/wayland-rs> (0.32.12 to 0.32.13) | 0.32.13 tracks wayland-protocols 1.48 and 1.49 XML | not relevant: `idle-inhibit` unchanged |
| `web-sys` | 0.3.106 | 0.3.107 | CHANGELOG.md in <https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/web-sys> (0.3.106 to 0.3.107) | 0.3.107: generated bindings refresh | web build only; not relevant |
| `web-transport-proto` | 0.6.2 | 0.6.4 | release notes of <https://github.com/moq-dev/web-transport> (0.6.2 to 0.6.4) | qmux stream-occupancy and ping coalescing; no quinn-path change | not relevant: `oag-net` uses the quinn transport |
| `web-transport-quinn` | 0.13.5 | 0.13.6 | release notes of <https://github.com/moq-dev/web-transport> (0.13.5 to 0.13.6) | 0.13.6: release notes empty, tag range only | applied by the lock; `oag-net` handshake tests pass |
| `web-transport-trait` | 0.5.0 | 0.5.2 | release notes of <https://github.com/moq-dev/web-transport> (0.5.0 to 0.5.2) | 0.5.2 makes the safe receive defaults sound; stream occupancy reporting | applied by the lock; no call of ours changes |
| `webpki-root-certs` | 1.0.9 | 1.0.11 | release notes of <https://github.com/rustls/webpki-roots> (1.0.9 to 1.0.11) | CCADB removal of five eMudhra/emSign roots | applied by the lock (root store for `oag-net`) |
| `winnow` | 1.0.4 | 1.0.5 | CHANGELOG.md in <https://github.com/winnow-rs/winnow> (1.0.4 to 1.0.5) | 1.0.5 patch | transitive parser of toml; not relevant |
| `wit-bindgen` | 0.46.0 | 0.57.1 | release notes of <https://github.com/bytecodealliance/wit-bindgen> (0.46.0 to 0.57.1) | 0.57.1 drops the `async` macro feature | transitive (wasip2); not relevant |
| `wl-clipboard-rs` | 0.9.4 | 0.9.5 | CHANGELOG.md in <https://github.com/YaLTeR/wl-clipboard-rs> (0.9.4 to 0.9.5) | fixes a `Watcher` panic with KWin's XWayland bridge | transitive (arboard); we never use `Watcher`: not relevant |
| `xcursor` | 0.3.10 | 0.3.11 | commit log of <https://github.com/esposm03/xcursor-rs> (0.3.10 to 0.3.11) | patch | transitive (winit); not relevant |
| `xml-rs` | 0.8.28 | 0.8.29 | Changelog.md in <https://github.com/kornelski/xml-rs> (0.8.28 to 0.8.29) | attribute whitespace normalisation, CDATA and PI checks | transitive (gltf/xml readers); not relevant |
| `yoke` | 0.8.3 | 0.8.4 | CHANGELOG.md in <https://github.com/unicode-org/icu4x> (0.8.3 to 0.8.4) | ICU4X 2.4 patch | transitive; not relevant |
| `zerocopy` | 0.8.55 | 0.8.63 | zerocopy/CHANGELOG.md in <https://github.com/google/zerocopy> (0.8.55 to 0.8.63) | 0.8.63: patch releases | transitive; not relevant |
| `zerocopy-derive` | 0.8.55 | 0.8.63 | release notes of <https://github.com/google/zerocopy> (0.8.55 to 0.8.63) | 0.8.63 | proc macro; not relevant |
| `zerovec` | 0.11.8 | 0.11.9 | CHANGELOG.md in <https://github.com/unicode-org/icu4x> (0.11.8 to 0.11.9) | ICU4X 2.4 patch | transitive; not relevant |
| `zerovec-derive` | 0.11.6 | 0.11.7 | CHANGELOG.md in <https://github.com/unicode-org/icu4x> (0.11.6 to 0.11.7) | ICU4X 2.4 | proc macro; not relevant |
| `zip` | 9.1.3 | 9.5.0 | CHANGELOG.md in <https://github.com/zip-rs/zip2> (9.1.3 to 9.5.0) | 9.2-9.5: `ExtractOptions` root filter, `files_with_reader`, `OwnedZipFileEntry`, boxed slices, security-patch formatting | not relevant: we read entries lazily from `.zip` mods and PPSSPP packs and never extract; mod and texture-pack tests pass |
| `zlib-rs` | 0.6.6 | 0.6.8 | release notes of <https://github.com/trifectatechfoundation/zlib-rs> (0.6.6 to 0.6.8) | 0.6.7-0.6.8: LSX (LoongArch) compare256/inflate, deflate `set_level` use-after-free fix, `Deflate` from config | not relevant: we only inflate, and not on LoongArch; the PSARC ground truth reads identical bytes |
| `zopfli` | 0.8.3 | 0.8.4 | release notes of <https://github.com/zopfli-rs/zopfli> (0.8.3 to 0.8.4) | `BitwiseWriter` invariants on retryable I/O errors; lz77 cutoff parity | transitive (zip's deflate encoder); not relevant, we read zips |
| `zrip` | 0.8.14 | 0.8.16 | CHANGELOG.md in <https://github.com/paddor/zrip> (0.8.14 to 0.8.16) | `FrameDecoder` keeps two windows of history instead of trimming after every block: a 128 MiB-window stream decodes at 2.5 GB/s instead of 24 MB/s | transitive (`chd`'s zstd): CHD hunks are small independent frames, so probably not reached (not measured); CHD ground truth passes |
| `zrip-decode` | 0.8.12 | 0.8.13 | CHANGELOG.md in <https://github.com/paddor/zrip> (0.8.12 to 0.8.13) | same release train as `zrip` | see `zrip` |
| `zrip-encode` | 0.9.3 | 0.9.4 | CHANGELOG.md in <https://github.com/paddor/zrip> (0.9.3 to 0.9.4) | encoder-side tuning (L1-L4 21-66 % faster), LDM fixes | not relevant: we never write CHD |

## A 2048 and Omega check

None of these crates is a format finding about 2048 or Omega. The PS4
package path (`aes`, `num-bigint`) is Omega's disc container and was run
against the Omega base and patch packages in `ps4_pkg_ground_truth` (every
test passes); 2048 ships through Vita PSARC and does not use `aes` here.
Verdict: **ported** for the aes batch (Omega's PS4 XTS path was already
batched), **not checkable** for the rest (no title-specific behaviour).

## What a new upgrade lane repeats

1. `cargo update --dry-run --verbose`, then the sparse-index check above, then
   one commit per coupled group, the simulation-reachable crates first and
   alone with the determinism binaries run.
2. Before moving a crate that is also a transitive dependency, `cargo tree -i
   <crate>` and `cargo tree -d`: a second copy is a cost, and the manifest
   comments on `num-bigint`, `x11rb`, `png` and `flate2` say which already
   depend on that.
3. A changed output (an audio or raster golden) holds the crate back; no golden
   or hash is regenerated by an upgrade lane.
