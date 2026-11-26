# Dependency upgrades

The state of every external dependency on 2026-11-09 (workspace `main` at
`34fef176a`) and again on 2026-11-24 (`main` at `0a4d5af12`, toolchain 1.100.0,
`rust-version` 1.100): what each upgrade changed, and every changelog read
between the old version and the new. The sections up to "Lockfile-only upgrades"
are the first pass and keep their first-pass numbers; "Second pass" below
corrects what moved since, ranks what a newer release would give us, and has the
full direct-dependency inventory. The rule
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
| `wgpu` (with `wgpu-core`, `wgpu-hal`, `wgpu-types`, `naga`, `naga-types`, `wgpu-naga-bridge`, `wgpu-core-deps-*`) | 30.0.1 | 30.0.2 | `CHANGELOG.md` in <https://github.com/gfx-rs/wgpu> (`v30` branch, 30.0.1 to 30.0.2) | Metal: sRGB no longer oversaturated on wide-gamut displays (#10286); a texture id no longer leaks when a surface acquire returns no texture (#10184, our Outdated/Lost/Timeout skip); no use-after-free of a mapped buffer on device loss (#10593, our readbacks); `Send`/`Sync` on `Global` (#9953, compile-time only); naga `glsl-in` build fix and WebGPU `PreMultiplied` report (not relevant: neither used) | **applied** (2026-11-16), lockfile only, no call-site change |
| `wasmtime` (with `cranelift-*`, `pulley-*`, `wasmtime-internal-*`, `wasmtime-environ`) | 50.0.1 | 50.0.2 | release notes of <https://github.com/bytecodealliance/wasmtime/releases/tag/v50.0.2> | security fixes; two reach our build, which leaves out the `gc` and `threads` features: an abort in builds without `gc` (GHSA-rc4p-x33f-vpr2) and one without `threads` (GHSA-2674-w29h-wqvj). The rest are in `wasmtime-wasi`, components, GC heaps or `Caller::get_module_export`, none of which `oag-modhost` uses | **applied** (2026-11-16), the exact pin moved `=50.0.1` -> `=50.0.2` and the lockfile; no call-site change, `oag-ext-abi` and `oag-ext` unchanged, so `oag-mod-author`'s skill needs no edit |
| `wasmtime` (with `cranelift-*` 0.137.3 -> 0.138.0, `pulley-*`, `wasmtime-internal-*`, `wasmtime-environ`, `wasmprinter` 0.263) | 50.0.3 | 51.0.0 | `RELEASES.md` section 51.0.0 of <https://github.com/bytecodealliance/wasmtime> | see "wasmtime 51" below: the one API break (`MemoryType::new` and `TableType::new` return a `Result`) is not called by `oag-modhost`; no new features adopted | **applied** (2026-11-24), the exact pin moved `=50.0.3` -> `=51.0.0` and the lockfile; no call-site change, `oag-ext-abi` and `oag-ext` unchanged, committed `ext/*.wasm` unchanged, so `oag-mod-author`'s skill needs no edit |
| `web-transport-quinn` (with `web-transport-proto` 0.7, new `rind` 0.3.2, `rind-rustls` 0.23.3, `rustls-platform-verifier` 0.7) | 0.13.6 | 0.14.0 | `CHANGELOG.md` of <https://github.com/moq-dev/web-transport> (0.13.6 to 0.14.0), [#488](https://github.com/moq-dev/web-transport/pull/488), [#491](https://github.com/moq-dev/web-transport/pull/491) | #488 removes the `ring` and `aws-lc-rs` features and moves TLS and QUIC packet cryptography to `rind` (pure Rust) behind a provider-neutral adapter: the builder takes a process-installed rustls provider first and rind otherwise; `ClientError`/`ServerError` gain `UnsupportedCpu` and `Crypto`; the connection closes when the last `Session` drops (#480); default client sockets bind per address family (#490); `ServerBuilder`/`ClientBuilder` take a `TransportConfig` (#489); `reject` takes a `ConnectResponse` that can carry headers (#499) | **applied**. `crates/net/Cargo.toml`: the dependency is `"0.14"` with no feature, and our own `rustls` dependency drops its `ring` feature (it only names `rustls::pki_types`). `crates/net/src/wt.rs` needed **no code change**: `ServerBuilder::new().with_addr().with_certificate()`, `ClientBuilder::new().with_server_certificate_hashes()`, `Request::reject(StatusCode)` and the `Session` types all kept their signatures. No process provider is installed, so rind is the provider. `ring` stays as a direct dependency for `rcgen` (minting the self-signed certificate) and for the digests in `extract.rs`; it is no longer the TLS provider. `cargo tree -d --target all` gains one duplicate pair, `hax-lib` 0.3.7 and 0.4.2 (each a proc-macro annotation crate under `libcrux-intrinsics`, through `rind`), and loses `base64` 0.22. No `aws-lc-sys` is in the tree. What the duplicate check cannot see: `rind` is a second crypto implementation beside `ring` (`rind`, `rind-rustls`, `libcrux-aes`/`-sha2`/`-poly1305`, `curve25519-dalek` 5, `fiat-crypto`), so there are two stacks, because `rind` is not optional in 0.14 and `ring` stays for `rcgen`, `extract.rs` and the host-signature code. `crabgrind` (and `bindgen` under it) is locked but never built: `libcrux-secrets` depends on it only under `cfg(valgrind_ct_test)`, which no build of ours sets, and `cargo tree -i crabgrind` finds it on none of the Linux, Android, macOS, iOS, Windows or wasm32 targets, so no libclang requirement joins any build. `rind`'s CPU gate is `rind::cpu::check`: on x86_64 it requires nothing (its AVX2, ADX and AES paths are chosen at run time), so no x86 player is refused; it fails only on an aarch64 CPU without NEON (32-bit ARM requires nothing). All 137 `oag-net` tests pass; `cargo ndk -t arm64-v8a check -p oag-game --example oag_android --no-default-features --features android,extensions` and `just check-web` build, and the wasm graph holds neither `rind` nor `web-transport-quinn` (`oag-server` is native-only and does not build for wasm). Open upstream, not ours to fix: [#587](https://github.com/moq-dev/web-transport/issues/587) (an error `Display` repeats its sources, so a logged chain from this crate may name a cause twice) and [#565](https://github.com/moq-dev/web-transport/issues/565) (the qmux idle timer; `oag-net` uses the quinn transport) |

## wasmtime 51

`oag-modhost` is the only user (`crates/modhost`). Its pin is exact on purpose:
fuel, NaN canonicalisation and the pooled per-tick instance are the sandbox's
determinism, so the move was read and measured, not taken as a lockfile bump.

**Breaking changes in 51.0.0 against what we call** (`Config`: `consume_fuel`,
`cranelift_nan_canonicalization`, the `wasm_*` switches, `PoolingAllocationConfig`;
`Store::{set_fuel,get_fuel}`, `Linker`, `Instance`, `Memory`, `TypedFunc`):

- `MemoryType::new` and `TableType::new` return a `Result`: not called.
- `Direct{Source,Destination}::remaining` return a `Result`, `LinearMemory` was
  adjusted, WASI buffer sizes became configurable, the number of fibers in a
  store is limited by default: components, WASI and async, none of which
  `oag-modhost` builds (`runtime`, `cranelift`, `std`, `pooling-allocator` only).
- Result: `cargo check -p oag-modhost --all-features` compiled with no edit.

**Measured, 50.0.3 against 51.0.0**, `quick` profile, same machine, best of 7
(compile) and three runs (tick):

| What | 50.0.3 | 51.0.0 |
| --- | --- | --- |
| compile of the seven shipped guests (`checkpoint_circuit`, `energy_turbo`, `hillside_rain`, `tether`, `wall_penalty`, `window`, `slipstream`), sum of best | 79.9 ms | 78.9 ms |
| `counter` tick on Cranelift | 9.8 to 10.2 us, 22 fuel | 10.1 to 10.5 us, 22 fuel |
| `arithmetic` tick on Cranelift | 11.8 to 12.4 us, 9,058 fuel | 11.5 to 11.7 us, 9,058 fuel |
| `arithmetic` tick on Pulley | 324 to 329 us, 9,058 fuel | 325 to 327 us, 9,058 fuel |

The fuel totals are equal to the unit. The tick figures come from
`tests::engines::per_tick_wall_time_of_each_engine` (`--features pulley`,
`--no-capture`); the compile figures from a throwaway example that called
`Module::compile` on each shipped `ext/*.wasm`.

**Open upstream issues, and whether they reach us:**

- [#15382](https://github.com/bytecodealliance/wasmtime/issues/15382), memory
  growing quadratically when compiling thousands of GC global initializers
  (`array.new_fixed`): **does not reach us**. The reproducer needs the GC
  proposal, and our build has no `gc` feature; a guest cannot use it. Our guests
  are 2.5 to 12 KB with no globals of that kind.
- [#15363](https://github.com/bytecodealliance/wasmtime/issues/15363), a
  compile-time regression of +0.5 % geomean (up to +7 % on one large module)
  blamed on `regalloc2` 0.15.3: **reaches us in principle, costs nothing
  measured**. The lockfile held `regalloc2` 0.15.3 under 50.0.3 as well (wasmtime
  requires `^0.15`), so 51.0.0 does not change it; the table above is the same
  compile time on both sides.

**New in 51 and not adopted:**

- `Global::get_numeric`: the host reads state through the I/O buffer, not guest globals.
- Filling unknown imports with functions that return default values
  ([#14746](https://github.com/bytecodealliance/wasmtime/pull/14746)): `Module::compile_on` refuses a module with any import (the `oag` namespace is
  empty in ABI 1) and names it; a default-returning stub would let an
  unsupported guest load.
- `GuestProfiler` precomputed symbol tables ([#14753](https://github.com/bytecodealliance/wasmtime/pull/14753)):
  we do not profile guests per store.
- Cranelift `interrupt_poll` and the `volatile` memory flag: the sandbox stops
  runaway guests with fuel, which is deterministic; an interrupt is not.
- 128-bit atomics (AArch64, s390x), the WIT `map` type, WASI buffer sizes,
  `wasmtime serve` options and HTTP/component-async changes: no component model,
  no WASI, no threads.
- Faster compile of host-defined functions ([#14517](https://github.com/bytecodealliance/wasmtime/pull/14517)):
  taken for free; we define a handful of host functions and the compile total did
  not move beyond noise.

**Advisories in 51.0.0:** the without-`gc` and without-`threads` aborts were
already taken in 50.0.2; the rest are in `wasmtime-wasi`, HTTP, components, GC
heaps and `Caller::get_module_export`, which `oag-modhost` does not build or call.
The `PAGEMAP_SCAN` fix does not apply either: wasmtime's default is off
(`Config::pagemap_scan`, `Enabled::No`) and `build_engine` does not set it.

## Git pins

A dependency comes from crates.io unless a fix we measured is only on its default
branch. Then it is a git dependency pinned to a full commit SHA (`rev = "..."`,
never `branch =` or `tag =`), every crate of that project comes from the same
rev, and it goes back to crates.io in the first release that carries the fix.
A vendored copy under `vendor/` is for a small fix of ours that upstream lacks,
not for tracking someone else's unreleased work.

| Dependency | Rev | Why | Back to crates.io |
| --- | --- | --- | --- |
| `wgpu`, `naga` (with `wgpu-core`, `wgpu-hal`, `wgpu-types`, `naga-types`, `wgpu-naga-bridge`, `wgpu-sync`, `wgpu-core-deps-*`) | `60cef2be828e03ba54e2e1598001b4625c0c2163` (gfx-rs/wgpu trunk, 2026-11-25) | [gfx-rs/wgpu#10319](https://github.com/gfx-rs/wgpu/pull/10319): one barrier for all of a submission's `write_buffer`/`write_texture` calls instead of one per call. Merged 2026-10-25 on trunk, not on the `v30` branch (30.0.2 lacks it). Measured: "What the wgpu pin measured" below | wgpu 31 (milestone v31, overdue since 2026-10-09). Then `wgpu = "31"`, `naga` the same, re-run the picture check and the S21 run below |
| `toml-schema` (dev only) | `fa97c07e` | ADR-0064 | the next tagged release |

Both entries are in the root `Cargo.toml` `[workspace.dependencies]`; a member
takes `naga` with `workspace = true` so `oag-shader-check` validates every shader
with the compiler the runtime uses (one `naga` in `Cargo.lock`, checked with
`grep -A2 'name = "naga"' Cargo.lock`).

**What the wgpu pin changed in our code** (trunk "Unreleased" against 30.0.2, read
in its `CHANGELOG.md` at the pinned rev): `TextureDescriptor` gains
`texture_binding_view_dimension` (216 literals, all `None`, which keeps the
inference 30.0.2 did); `TextureFormat::is_srgb` is `has_srgb_suffix`; `Queue::submit`
returns `Option<SubmissionIndex>`, `None` when the submission never reached the
driver (`oag_gpu::split_finish` passes it on, and the race benchmark waits on all
work for a `None`). No `TextureViewDescriptor` or `DeviceDescriptor` literal needed
the new `swizzle` and `default_queue` fields: every one ends in `..Default::default()`.
Read and not hit: a `BufferView` now blocks unmap and `destroy` panics while one is
alive (our views live inside one readback call, and `Device::destroy` is called only by `lose-device`, between frames; `crates/game/src/app/device_loss/tests.rs` passes), a read view of a `MapMode::Write`
mapping is refused (none: every read view is of a `MAP_READ` buffer),
`@interpolate(linear)` needs a capability (no shader uses it), and
`create_shader_module` errors no longer quote the source (nothing parses their text; `web_fault` matches the variant).
The trunk version string is still `30.0.0`; the rev is the identity.

**What the wgpu pin measured** (2026-11-26, against the 30.0.2 tree it replaced):

- Picture: HD (Vineta K), Pulse and Omega `--race --seed 1 --autopilot --screenshot` at 960x540, ticks 300 and 900, the
  six frames `cmp`-equal between the two release builds (main against main also equal, so the check can see a change).
- S21, HD Vineta K: 2.4 to 12.0 ms faster per frame over whole races, three interleaved rounds
  (`docs/tools/android.md`, "wgpu trunk (the git pin) against 30.0.2").
- Host (RX 7800 XT, RADV), `just bench-render hd --size WxH --binary <build>`, main then trunk, two rounds each, p50 in us:

| Size, round | CPU 30.0.2 / trunk | GPU | Frame | `tick_us` (no wgpu: the noise control) | Load at start |
| --- | ---: | ---: | ---: | ---: | --- |
| 1920x1080, 1 | 1117 / 1146 | 734 / 791 | 1292 / 1320 | 156 / 157 | 8.5 / 8.0 |
| 1920x1080, 2 | 1149 / 1150 | 753 / 779 | 1327 / 1322 | 157 / 156 | 6.8 / 10.6 |
| 1920x1080, 3 | 1813 / 1419 | 793 / 793 | 2072 / 1620 | 226 / 183 | 62.4 / 61.8 |
| 1920x1080, 4 | 1184 / 1278 | 760 / 799 | 1365 / 1468 | 161 / 169 | 5.6 / 13.5 |
| 3840x2160, 1 | 1128 / 2035 | 1077 / 1538 | 1384 / 2340 | 156 / 226 | 34.7 / 18.7 |
| 3840x2160, 2 | 1389 / 1144 | 1146 / 1080 | 1596 / 1382 | 182 / 155 | 27.4 / 5.7 |

  The 4K rounds disagree with each other and so does the simulation's own `tick_us` (+44 % and -15 %), which no wgpu
  change can move: machine state, not the build. Where `tick_us` matches (1080p rounds 1 and 2), frame and CPU are
  unchanged. GPU p50 at 1080p over the four rounds: 30.0.2 734, 753, 793, 760 us; trunk 791, 779, 793, 799 us, so
  about 5 % (36 us at the medians) higher on trunk in three rounds and equal in one, inside 30.0.2's own 59 us spread.
  Small and open; the frame is CPU-bound and does not show it.
- Builds: Linux (`cargo check --workspace --all-targets`, `just lint`), wasm (`scripts/build-web.sh --check`), Windows
  (`CARGO_TARGET_DIR=target/win-check cargo check --target x86_64-pc-windows-gnu -p oag-game`), the Android dist APK.
  Every WESL shader is validated by trunk's `naga` through `oag-shader-check` at build time.

**Upstream reports since #10319** (searched 2026-11-26, `gh search issues --repo
gfx-rs/wgpu` for regression, Mali, RADV, Intel, Metal and barrier, created after
2026-10-25): none against the write-barrier change. [#11102](https://github.com/gfx-rs/wgpu/issues/11102)
(a panic in `PendingWrites::activate` when `begin_encoding` runs out of memory)
is in the code #10319 touched, but the `unwrap` predates it and needs an
out-of-memory at that call. #10920 (RADV, binding-array `arrayLength`) and #11115
(GLES, `PIXEL_UNPACK_BUFFER`) are unrelated to anything we bind.

## Held back, with the reason

| Dependency | In tree | Newer | Reason |
| --- | --- | --- | --- |
| `num-bigint` | 0.4.8 | 0.5.1 | superseded: `oag-disc` now takes 0.5.1 directly, and 0.4.8 stays only through `num-rational` 0.4 (`av-data`, the AV1 decoder's), a second copy that leaves when `av-data` moves. The 0.5.0 notes (`<https://github.com/rust-num/num-bigint>`, `RELEASES.md`) only split the `rand` features |
| `x11rb` | 0.13.2 | 0.14.0 | superseded: 0.14.0 is our direct dependency (RandR); 0.13.2 stays through `winit` 0.30.13 and `arboard` 3.7.0. 0.14 drops abstract unix socket connections (as libxcb did), bumps its MSRV and `raw-window-handle` (commit log `v0.13.2...v0.14.0` of <https://github.com/psychon/x11rb>). The copy leaves with winit 0.31 and an arboard on x11rb 0.14 |
| `generic-array` | 0.14.7 | 0.14.9 | `crypto-common` 0.1.7 requires `=0.14.7`; it arrives through `sha2` 0.10 from `cranelift-codegen`. `cargo update -p generic-array --precise 0.14.9` fails on that bound. **wasmtime 51 does not remove it**: `cranelift-codegen` 0.138.0 (wasmtime 51.0.0's) still requires `sha2 ^0.10.2` (crates.io index) |
| `winit` | 0.30.13 | 0.31.0-beta.4 | beta (2026-10-24); no stable 0.31 on crates.io; the migration is a thread item |
| `rustls` | 0.23.46 | 0.24.0-dev.2 | a development pre-release |
| `toml-schema` (git) | rev `fa97c07e` | trunk is newer | pinned at the newest tagged release, `rust-v1.0.0-rc.2` (2026-08-20), by ADR-0064; trunk past it is untagged ("Git pins" above) |
| Rust toolchain | 1.99.0 | 1.99.1 | `rust-toolchain.toml` pins deliberately ("bump deliberately, not as a side effect"); a patch bump is the maintainer's call, not this lane's |
| GitHub Actions | `checkout` 7, `upload-artifact` 7, `download-artifact` 8, `setup-python` 7, `rust-cache` 2, `wrangler-action` 4, `setup-chrome` 2 | the same majors | already at each action's newest major; `dependabot.yml` keeps them. Not verified by running CI (CI is never pushed to from a lane) |

## Direct dependencies already at their newest release

`wesl` 0.6.0, `cpal` 0.18.2, `gilrs` 0.11.2,
`symphonia` 0.6.1, `tokio` 1.54.1, `clap` 4.6.9, `serde` 1.0.229,
`serde_json` 1.0.154, `thiserror` 2.0.21, `anyhow` 1.0.104, `bytemuck` 1.25.2,
`png` 0.18.1, `gltf` 1.4.1, `rcgen` 0.14.10, `ring` 0.17.14, `arboard` 3.7.0,
`dirs` 7.0.0, `chd` 0.4.0, `re_rav1d` 0.1.3, `atrac9dec` 0.9.1, `libm` 0.2.16,
`pipewire` 0.10.1, `libspa` 0.10.1, `rtrb` 0.4.1, `lzma-rs-perf-exp` 0.2.1
(exact pin), `url` 2.5.9 and every other direct dependency the tables above do not name. For these there is no old-to-new changelog to read. Because the lane
asked for benefits, wgpu 30's own release notes (`CHANGELOG.md`, `v30.0.0` and
`v30.0.1` in <https://github.com/gfx-rs/wgpu>) were read for features not yet
used:

- **Surface colour space and `display_hdr_info`**: threaded (the HDR thread).
- **`PipelineCache`**: `create_pipeline_cache` is `unsafe`
  (`unsafe_code = "deny"`; the maintainer allows this one call only if the
  benefit is big). Measured at about 1 % of a racing launch over mesa's own cache (`docs/architecture/load-time.md`, "Third round"), so not built.
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

## Second pass (2026-11-24)

`main` at `0a4d5af12` did **not** carry the seven bumps of Dependabot PR #33
(`toml` 1.2.1, `zip` 9.5.1 and the rest were still in `Cargo.lock`), so this
lane took them with the rest: one `cargo update` (commit `510b28a9b`), then
`zlib-rs` 0.6.8 to 0.6.9 alone (`e2a867ed5`), because it is the only changed
crate that `cargo tree -e normal` reaches from the six determinism-bound crates
(`oag-core`, `oag-physics`, `oag-gameplay`, `oag-ai`, `oag-race`, `oag-formats`).
Those six crates' 1,268 tests (determinism binaries included, reference hashes
untouched) pass on the tip. `Cargo.toml` is untouched, so nothing here moves a
requirement; the toolchain stays 1.100.0.

What the lock moved (62 name changes; a source of every row is the crate's own
changelog or release page, read for the direct dependencies below):

| Crate | From | To | Note |
| --- | --- | --- | --- |
| `toml` / `toml_edit` / `toml_parser` | 1.2.1 / 0.25.19 / 1.1.5 | 1.2.2 / 0.25.20 / 1.1.6 | parser stack-overflow fix, [toml-rs/toml#1265](https://github.com/toml-rs/toml/pull/1265); `toml_parser` 1.1.6 carries it (`crates/toml_parser/CHANGELOG.md`), the other two re-release it |
| `zip` | 9.5.1 | 9.8.2 | 9.6-9.8.2: extended-timestamp and NTFS extra-field fixes on write, `ZipFile::comment_raw`, safe fixed-header parse methods (9.7.0), central-directory and local-header length check fixes (9.8.1, [#1340](https://github.com/zip-rs/zip2/pull/1340)), EOCD fallback validation (9.8.2, [#1347](https://github.com/zip-rs/zip2/pull/1347), [#1350](https://github.com/zip-rs/zip2/pull/1350)), the `zstd-rs` feature (9.8.0, [#1329](https://github.com/zip-rs/zip2/pull/1329)). Our readers (mods, texture packs, ffmpeg download) only read; the read-side fixes apply |
| `clap` 4.6.9, `clap_derive`, `clap_lex` | 4.6.7 | 4.6.9 | 4.6.8: derive uses less stack in the `dev` profile; 4.6.9: no `unused_result` from derive. No deprecation |
| `env_logger` / `env_filter` | 0.11.11 / 2.0.0 | 0.11.12 / 2.1.0 | MSRV 1.85 (ours is 1.100); `jiff` moved 0.2.41 to **1.0.3** with it (`jiff` is only env_logger's timestamp formatter) |
| `url` / `form_urlencoded` | 2.5.8 / 1.2.2 | 2.5.9 / 1.2.3 | quadratic `..` parse after a long authority fixed, `Serializer::for_suffix` panic fixed, backslash in a non-special URL's port rejected |
| `rustls-webpki`, `rustls-pki-types`, `webpki-root-certs` | 0.103.16 / 1.15.1 / 1.0.11 | 0.103.17 / 1.16.0 / 1.0.12 | 0.103.17 adds SCT and SPKI accessors only (release notes of <https://github.com/rustls/webpki>); root store refresh |
| `zlib-rs` | 0.6.8 | 0.6.9 | simulation-reachable (`oag-formats` inflates through `flate2`); own commit; PSARC and WAD ground truth read identical bytes |
| `zrip` family, `zstd` 0.14.4, `zstd-safe` 8.3.0 | 0.8.16 and the like | 0.8.25 and the like | CHD's zstd decoder: 0.8.25 stops a 14-byte header reserving 256 MiB of history ([`CHANGELOG.md`](https://github.com/paddor/zrip)); `zstd` 0.14.4 adds a `rust-backend` feature |
| `rubato` | 5.0.1 | 5.1.0 | public sinc interpolators; `hrtf` 0.10's resampler, nothing of ours calls it |
| `web-transport-trait` | 0.5.2 | 0.5.3 | patch |
| `anyhow` (direct) | 1.0.104 | 1.0.105 | `ensure!` refers to `Err` by absolute path, reallocated boxed errors are downcastable ([release notes](https://github.com/dtolnay/anyhow/releases/tag/1.0.105)); no call of ours changes |
| other transitive | | | `ahash`, `bumpalo`, `bytes`, `cc`, `crc32fast` 1.5.6, `errno`, `glib` family 0.22.11 (optional `native-video`), `hashbrown`, `mio`, `pin-project` 1.2.0, `postcard` 1.2.0, `rand` 0.10.4, `regex` 1.13.2, `smallvec` 1.16.6, `uuid`, `walkdir`, `zerocopy` 0.8.64, `zeroize` |

Removed from the tree: `getrandom` 0.3.4, `r-efi` 5.3.0, `wasip2`,
`wit-bindgen`. The list of crates locked at more than one version is one shorter than
main's (55 names to 54) and gains none.

**Held, so no second copy is added.** The plain `cargo update` also resolved
`nix` 0.32.0 beside 0.31.4 (`zip` 9.8.2 allows `>=0.30, <0.33`, `gilrs-core`
takes 0.31) and `cfg-expr` 0.17.2 beside 0.20.10 with a second `target-lexicon`
0.12.16 (`cfg-expr` 0.20.10 requires `=0.13.5`, and the free choice of 0.13.6
for `cranelift-codegen` made `system-deps` 7.0.8 fall back to `cfg-expr` 0.17).
`cargo update nix@0.32.0 --precise 0.31.4` and `cargo update
target-lexicon@0.13.6 --precise 0.13.5` fold both back; the lock carries
`nix` 0.31.4 and `target-lexicon` 0.13.5. Release both holds when `gilrs-core`
takes `nix` 0.32 and `cfg-expr` 0.20 moves its `target-lexicon` pin.

**wgpu-hal did not move** (30.0.2 before and after), so the parked
`gate-walltime-8` branch, which vendors `wgpu-hal` 30.0.2 under
`[patch.crates-io]`, does not have to re-vendor. (Since the git pin, "Git pins"
above, it does: a `[patch.crates-io]` row no longer reaches a git dependency.)

**Guests.** `assets/mods/examples` and `assets/mods/official` build their
`guest/` crates as separate workspaces, but each depends on `oag-ext` alone
(patched to this tree by `assets/mods/.cargo/config.toml`, then `oag-ext-abi`);
neither has a third-party dependency, and a guest's `Cargo.lock` is
git-ignored. There is nothing to bump, no submodule changed and no pointer moved;
`check-guests --strict` compares the committed `ext/*.wasm` against a fresh
build and is part of the gate.

**`atrac9dec`.** crates.io has 0.1.0, 0.9.0 and 0.9.1; 0.9.1 is the base of
`vendor/atrac9dec`, and the upstream repository
(<https://github.com/claimore22/atrac9dec>) has no commit after 0.9.1 (2026-08-23).
Our fixes (`docs/formats/atrac9.md`) are **not upstream yet**, so the
`[patch.crates-io]` stays.

**MSRV.** The highest `rust-version` any locked registry crate declares is 1.97.1
(`wesl`, `toml-schema`); ours is 1.100. No dependency forces a toolchain move,
and `wasmtime` 51.0.0 declares 1.97.0.

**`image` is not a dependency** (`cargo tree -i image` finds none): rasters go
through `png` 0.18.1 and `oag-texture`, so "what `image` added" has no item here.

## Benefits ranked

What a newer release, released or not, would give us, best first. "Applied" is
in this branch; every other row is **not applied** (adopting a feature is a
follow-up the lead proposes). Files are where the change would land. Unreleased
items are on the dependency's default branch only and are named as such.

| # | Benefit | Kind | Files | Source | State |
| --- | --- | --- | --- | --- | --- |
| 1 | The TOML parser no longer overflows the stack on invalid input: an unexpected closing delimiter could take the recursion-depth count below zero and walk past its guard. We parse TOML a stranger can write (a mod's `oag-mod.toml`, a campaign, a track manifest, a menu a mod extends) | security | `crates/mod/src/manifest.rs`, `crates/mod/src/lib.rs`, `crates/mod/src/campaign.rs`, `crates/trackfile/src/manifest.rs`, `crates/ui/src/menu/definition.rs` | [toml-rs/toml#1265](https://github.com/toml-rs/toml/pull/1265), `crates/toml_parser/CHANGELOG.md` 1.1.6 | **applied** by this lock (`toml_parser` 1.1.6). A follow-up could pin it with a test that feeds a mod manifest a few thousand `[` and `]`; none was written here |
| 2 | `wasmtime` 51.0.0: faster compile for host-defined functions ([#14517](https://github.com/bytecodealliance/wasmtime/pull/14517)), eight advisories listed (the without-`gc` and without-`threads` aborts we already took in 50.0.2; the others are in `wasmtime-wasi`, HTTP, components, GC heaps and `Caller::get_module_export`, none of which `oag-modhost` builds or calls), `Global::get_numeric` | performance, security | `crates/modhost/Cargo.toml` (`=51.0.0` pin, `pulley` feature), `crates/modhost/src` (no `MemoryType::new` or `TableType::new` call, whose `Result` is the 51 break), `docs/architecture/wasm-extensions/` | [release notes 51.0.0](https://github.com/bytecodealliance/wasmtime/releases/tag/v51.0.0) | **applied** (2026-11-24, "wasmtime 51" below); it was its own lane because the exact pin protects fuel, NaN canonicalisation and the ABI word. Its `cranelift-codegen` keeps `sha2` 0.10, so the second `generic-array` stays |
| 3 | `zip`'s `zstd-rs` feature (zstd's new pure-Rust backend) would let the web build read zstd mod entries and drop the C `zstd-sys` build from native too, deleting the "zstd is a C library" split | workaround deleted, build time | `crates/mod/Cargo.toml` (the `cfg(not(target_arch = "wasm32"))` block and its comment), `crates/mod/src/lib.rs:22` (the native-only module) and `crates/mod/src/pack.rs` (writes zstd) | [zip2#1329](https://github.com/zip-rs/zip2/pull/1329), zip 9.8.0; `zstd` 0.14.4 `rust-backend` | not built; whether the Rust backend compiles for `wasm32-unknown-unknown` and decodes our entries is unchecked (`cargo check -p oag-mod --target wasm32-unknown-unknown` with the feature is the first test) |
| 4 | `cpal` 0.19 reports a buffer under- or overrun as `CallbackInfo::xrun()` (and 0.18.2 already reports PipeWire, CoreAudio and AAudio xruns as `ErrorKind::Xrun` through the error callback). Our own timed "late callback" count in the output callback is what that replaces | workaround deleted | `crates/audio/src/output.rs` (the `callback_health.late` check around line 570 and the error callback at line 643, which today only logs) | `CHANGELOG.md` of <https://github.com/RustAudio/cpal> ("Unreleased" and 0.18.2) | 0.19 is unreleased and removes `ErrorKind::Xrun` again, so wait for it rather than wire 0.18.2's form |
| 5 | `Features::UNRESTRICTED_BYTES_PER_ROW`: `copy_texture_to_buffer` rows need only the texel size, not 256 bytes, so the padded-row readback and the de-pad loop go | workaround deleted, performance | `crates/view/src/offscreen.rs:62`, `crates/mesh/src/capture.rs:326`, `crates/gpu/src/writes.rs:171` | [wgpu#10619](https://github.com/gfx-rs/wgpu/pull/10619) (unreleased, trunk; not in 30.0.2) | wait for wgpu 31; capture and upload paths only, so a gain would be in screenshots and texture upload, unmeasured |
| 6 | `web-transport-quinn` 0.14.0 (2026-11-23): the `ring` and `aws-lc-rs` features are gone, the provider is `rind` (pure-Rust, forked from graviola) with `rustls-platform-verifier` and `jni` (Android's own certificate store), `web-transport-proto` 0.7 | dependency shape, Android | `crates/net/Cargo.toml`, `crates/net/src/wt.rs` | `web-transport-quinn` 0.13.6 to 0.14.0 `CHANGELOG.md`, [#488](https://github.com/moq-dev/web-transport/pull/488) | **applied** (see the upgraded table). Not adopted, each a small follow-up: `ClientBuilder::with_system_roots` (the OS verifier, for a CA-signed public server; we pin a hash), `with_transport_config` and `with_congestion_control(LowLatency)` (BBR; the link uses quinn's defaults, and the effect on a lossy path is unmeasured), a `ConnectResponse` with headers on `reject` (the 503 could say why), and the `qlog` feature for tracing a handshake |
| 7 | wgpu 30 surface colour space (`SurfaceConfiguration::color_space`, `Surface::display_hdr_info`, `SurfaceColorSpace::PassThrough` on trunk) | feature | `crates/game/src/app/gpu.rs:410` (`SurfaceColorSpace::Auto` today) | `CHANGELOG.md` v30.0.0, [#9658](https://github.com/gfx-rs/wgpu/pull/9658) | threaded (the HDR thread); unchanged here |
| 8 | winit 0.31 (beta.4, 2026-10-24): `ActiveEventLoop::clipboard`/`set_clipboard` on Wayland, X11, macOS and Windows, `MouseWheel::source`, and the second `x11rb` 0.13 and `sctk-adwaita`'s `tiny-skia` 0.11 would leave with it; `arboard` would be unnecessary for text | dependency removed | `crates/game/src/app/clipboard.rs` (the only `arboard` user), `crates/game/src/app/` (event loop and window) | [release notes](https://github.com/rust-windowing/winit/releases/tag/v0.31.0-beta.4) | beta only; the thread item stays |
| 9 | clap 4.6.7 `#[command(defer = true)]` lazy subcommand setup, 4.6.8 less derive stack in `dev` | startup, debug builds | `crates/trace/src/main.rs`, `crates/tools/src/bin/*.rs`, `crates/net/src/bin/oag_server.rs` | `CHANGELOG.md` of <https://github.com/clap-rs/clap> | not measured; four small `Subcommand` enums, so a gain is probably nil |
| 10 | `toml` 1.2: pre-allocated and faster serialisation, `size_hint` on deserialise | performance | `crates/game/src/settings/`, `crates/ui/src/menu/definition/build.rs` | `crates/toml/CHANGELOG.md` | **applied** by the lock, not measured; a menu build that is slow would be the place to measure |

Not worth a row (read, nothing for us): `serde` 1.0.229 (only the syn 3 move,
[release](https://github.com/serde-rs/serde/releases/tag/v1.0.229)), `log` 0.4.35
(no source change) and 0.4.34 (boxed loggers under `alloc`), `glam` 0.34.3 (a new
`camera` module and neon/wasm parity; the simulation uses scalar maths and its own
`oag_core::math::camera`, and the `look_at`/`perspective` methods 0.34.0 removed
from `Mat4` are not called), `naga` 30.0.2, `rustls-webpki` 0.103.17.

## Latest release of the big dependencies

What the newest release of each added, for the twelve the brief names (`wgpu`,
`naga`, `winit`, `cpal`, `wasmtime`, `glam`, `image`, `zip`, `clap`, `toml`,
`serde`, `log`), where we are already current. Sources are the changelog files and release pages named in each
row; "unreleased" is the dependency's default branch only.

| Dependency | We have | Newest | What the newest release added |
| --- | --- | --- | --- |
| `wgpu`, `naga` | trunk `60cef2be828e` (git pin, "Git pins") | 30.0.2 (31 unreleased) | 30.0.2: Metal sRGB wide-gamut fix and the Bt2100 colour space crash fix, texture id leak on an empty surface acquire, mapped-buffer use after free on device loss. 30.0.0 headline: `SurfaceColorSpace` and `display_hdr_info`, optional vertex buffer slots, integer shader I/O needs an explicit `@interpolate(flat)`, `Queue::present`, `BufferSlice` may be empty. Unreleased 31: `TEXTURE_COMPONENT_SWIZZLE`, `DeviceDescriptor::default_queue`, `BufferView` blocks `unmap`/`destroy` (a panic if one is alive), `Queue::submit` returns `Option<SubmissionIndex>`, `Features::UNRESTRICTED_BYTES_PER_ROW`, `Queue::present_with_damage`, `TextureFormat::is_srgb` renamed `has_srgb_suffix`, `ShaderStages` and `BufferUsages` split types. A 31 migration reads every `get_mapped_range` holder in `crates/gpu`, `crates/mesh` and `crates/view` for a view kept across `submit` or `poll` |
| `winit` | 0.30.13 | 0.30.13 (0.31.0-beta.4) | 0.30.13 is the March 2026 maintenance release. beta.4: clipboard and primary-selection data transfer, `ScrollSource`, IME and Wayland fixes, `winit-core` drops `url` |
| `cpal` | 0.18.2 | 0.18.2 (0.19 unreleased) | 0.18.2: xruns as `ErrorKind::Xrun` on AAudio, CoreAudio, PipeWire and WASAPI capture, monotonic timestamps across device changes, PipeWire streams no longer start before `play()`, rtkit burst-limit and real-time promotion fixes. 0.18.0: `StreamTrait::now`/`buffer_size`, `realtime` without D-Bus, more `ErrorKind` variants. Unreleased: `CallbackInfo::xrun`, graceful `stop`, duplex streams, `play` renamed `start` |
| `wasmtime` | 51.0.0 | 51.0.0 | applied, see ranked row 2 |
| `glam` | 0.34.3 | 0.34.3 | `camera` module (model, view, projection), handedness and Y-flip fixes in it, `transpose_mul_vecn` rename (old name deprecated), serde fixes for `BVec3A`/`BVec4A`, `lerp_monotonic`. 0.34.0 is the last breaking release before 1.0 |
| `zip` | 9.8.2 | 9.8.2 | `zstd-rs` (9.8.0), central-directory and EOCD validation (9.8.1, 9.8.2), safe header parse methods (9.7.0), `ZipFile::comment_raw` (9.6.0), Android `st_mode` fix |
| `clap` | 4.6.9 | 4.6.9 | `defer` attribute (4.6.7), `Command::get_overridden_usage` (4.6.6), derive stack and lint fixes; 5.0 is behind `unstable-v5` |
| `toml` | 1.2.2 | 1.2.2 | the stack-overflow fix (1.2.2), `size_hint` (1.2.1), serialiser speed-ups and allocation cuts (1.2.0), keys always encoded (1.1.8) |
| `serde`, `log` | 1.0.229, 0.4.35 | same | nothing user-visible (see above); `image` is not a dependency |

## Direct dependency inventory

Every dependency of a workspace crate that is not a path dependency, at the
lockfile of this branch, against the newest non-yanked release on the crates.io
sparse index (`https://index.crates.io`). "Newest" is the stable release; a
pre-release is named only where it is the next step.

| Behind or held | Locked | Newest | Reason |
| --- | --- | --- | --- |
| `wasmtime` | 51.0.0 | 51.0.0 | exact pin, applied (ranked row 2) |
| `winit` | 0.30.13 | 0.31.0-beta.4 | beta |
| `rustls` | 0.23.46 | 0.24.0-dev | development pre-release |
| `toml-schema` (git) | rev `fa97c07e` | not on crates.io | pinned by ADR-0064 |
| `gstreamer`, `gstreamer-app`, `gstreamer-video` | not locked (optional `native-video`) | 0.25.4, 0.25.2, 0.25.4 | not built by the gate; `-sys` crates are at 0.25.4 |

Every other direct dependency is at its newest stable release: `aes` 0.9.4,
`android_logger` 0.15.1, `anyhow` 1.0.105, `arboard` 3.7.0, `atrac9dec` 0.9.1
(vendored), `bytemuck` 1.25.2, `chd` 0.4.0, `clap` 4.6.9, `cpal` 0.18.2, `dirs` 7.0.0,
`env_filter` 2.1.0, `env_logger` 0.11.12, `flate2` 1.1.11, `gilrs` 0.11.2, `glam`
0.34.3, `gltf` 1.4.1, `hrtf` 0.10.0, `jni` 0.22.4, `js-sys` 0.3.107, `libc`
0.2.191, `libm` 0.2.16, `libspa` and `pipewire` 0.10.1, `log` 0.4.35,
`lzma-rs-perf-exp` 0.2.1, `lzma-rust2` 0.25.0, `memory-stats` 1.2.0, `naga`
30.0.2, `num-bigint` 0.5.1, `opus-rs` 0.1.42, `png` 0.18.1, `pollster` 1.0.3,
`rcgen` 0.14.10, `re_rav1d` 0.1.3, `resvg` 0.50.0, `ring` 0.17.14, `rtrb` 0.4.1,
`ruzstd` 0.9.1, `semver` 1.0.28, `serde` 1.0.229, `serde_json` 1.0.154, `sha2`
0.11.1, `smithay-client-toolkit` 0.21.1, `symphonia` 0.6.1, `tar` 0.4.46,
`thiserror` 2.0.21, `tokio` 1.54.1, `toml` 1.2.2, `toml_edit` 0.25.20, `ureq`
3.5.2, `url` 2.5.9, `wasm-bindgen` 0.2.130, `wasm-bindgen-futures` 0.4.80, `wat`
1.263.0, `wayland-backend` 0.3.17, `wayland-client` 0.31.15, `wayland-protocols`
0.32.13, `web-sys` 0.3.107, `web-time` 1.1.0, `wesl` 0.6.0, `wgpu` (git pin, "Git pins"),
`x11rb` 0.14.0, `zip` 9.8.2. The build-only and wasm32-only entries are among
them (`oag-shader-check`'s `naga` and `wesl`; `js-sys`, `web-sys` and
`wasm-bindgen*` for the web build). A transitive second copy of a direct
dependency still in the tree: `x11rb` 0.13.2 (`winit`, `arboard`), `num-bigint`
0.4.8 (`num-rational`), `sha2` 0.10.9 (`cranelift-codegen`), `thiserror` 1.0.69
(`calloop`, `ndk`), `lzma-rust2` 0.22.0 (`chd`), `smithay-client-toolkit` 0.19.2
(`sctk-adwaita`).

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

## wgpu trunk on the Steam Deck

Measured 2026-11-26 (`deck-wgpu`): gfx-rs/wgpu trunk at `60cef2be828e` (branch `wgpu-trunk`, not merged at the time)
against 30.0.2 on a Steam Deck OLED (SteamOS 3.9.5, RADV VANGOGH, Mesa 26.2.0-devel `git-5c65d4fc31`), AC power,
default clocks, Deck load average 0.5 to 4.7.

**Result: no picture change, no gain, and a small frame-time loss at 1280x800 only.** There trunk's median frame is
0.05 ms (windowed) to 0.07 ms (offscreen) longer, about 1 %, and under 1 % of the 11.1 ms budget at 90 Hz. At
1920x1080 and 3840x2160 the frame time does not move.

### Arms and race

- Arms: `main` at `5893b60d5` against the same tree with the `wgpu-trunk` diff applied
  (`git merge-tree --write-tree main wgpu-trunk`, `wgpu-trunk` at `f44898b22`), so wgpu is the only difference. The
  branch head itself was 84 commits behind `main`.
- Builds: Debian bookworm container (`packaging/appimage/Containerfile`),
  `cargo build --locked --profile dist -p oag-game --no-default-features --features extensions,dev-cli`.
- Race: Wipeout HD (PS3) on its default circuit, Talon's Junction, Single Race, seed 1, autopilot, three laps. The host
  bench's race; the S21 numbers used Vineta K.
- Offscreen: the race of `scripts/bench-render.py`, default settings in empty XDG directories,
  `OAG_RENDER_BENCH_RACE=<stem> oag-game <hd iso> --race --mode single_race --seed 1 --autopilot --no-audio
  --log-every 0 --size WxH --ticks 40000 --screenshot unused.png`.
- Windowed: the [in-game benchmark](../tools/in-game-benchmark.md) with sound on, vsync off and no frame limit,
  `gamescope --backend headless -W w -H h -w w -h h -r 90 -- oag-game <hd iso> --benchmark --benchmark-vsync off
  --benchmark-frame-limit unlimited --frame-log`, on a copy of the player's `settings.toml` with
  `target_fps = "off"` (dynamic resolution would hide a cost in the render scale) and `pause_on_focus_loss = false`.
  HD profile: render scale 100, FXAA, shadow resolution ultra, scene precision float, circuit lighting simplified,
  reflection auto. Present mode Mailbox.
- Rounds interleaved, trunk first on even rounds. Every offscreen run ends on state hash `d6348b6e5a5c9217`, every
  windowed run on `e9dac35b4bb7c005`.

### Frame time, ms (30.0.2 / trunk)

p50 and p99 are means over the rounds; worst is the range of each round's single slowest frame.

| Run | Rounds | p50 | p99 | worst |
| --- | ---: | ---: | ---: | --- |
| Offscreen 1280x800 | 4 | 5.036 / 5.103 | 6.785 / 6.891 | 9.95-10.70 / 9.14-10.04 |
| Offscreen 1920x1080 | 2 | 5.907 / 5.897 | 9.026 / 9.046 | 13.30-14.03 / 13.16-13.18 |
| Offscreen 3840x2160 | 2 | 14.590 / 14.554 | 26.111 / 26.074 | 33.37-33.94 / 33.87-34.08 |
| Windowed 1280x800 | 4 | 4.993 / 5.040 | 7.620 / 7.724 | 25.0-46.1 / 25.5-45.2 |
| Windowed 1920x1080 | 4 | 6.556 / 6.535 | 10.083 / 10.126 | 17.1-47.8 / 20.2-47.1 |
| Windowed 3840x2160 | 2 | 17.088 / 17.098 | 27.848 / 27.973 | 52.5-53.5 / 53.1-54.3 |

- Offscreen 1280x800: the p50 ranges do not overlap (5.008-5.051 against 5.071-5.116).
- Windowed 1280x800: trunk's p50 is higher in all four pairs (+0.006, +0.078, +0.053, +0.052).
- Windowed 1920x1080 p99 read +0.27 ms on trunk in rounds 1 and 2; rounds 3 and 4 gave -0.34 and -0.03. Noise.
- The worst frame is one sample and differs more between two runs of one build than between the builds.

### GPU time, ms (30.0.2 / trunk)

Offscreen is the timestamp sum per frame; windowed is the mean of every timed pass in the `--frame-log` lines.

| Size | Offscreen GPU p50 | Offscreen GPU p99 | Windowed all timed passes |
| --- | ---: | ---: | ---: |
| 1280x800 | 3.226 / 3.290 | 4.579 / 4.683 | 4.37 / 4.43 |
| 1920x1080 | 4.514 / 4.541 | 7.206 / 7.198 | 6.23 / 6.24 |
| 3840x2160 | 13.028 / 13.046 | 24.047 / 24.017 | 17.39 / 17.45 |

Offscreen GPU p50 is higher on trunk at every size (+2.0 %, +0.6 %, +0.1 %). At 1280x800 trunk's runs also read a lower
GPU clock (median 1098 to 1160 MHz against 1139 to 1280), so that row is not a fixed-clock GPU comparison.

### CPU encode and submit, offscreen means, ms (30.0.2 / trunk)

| Size | record | submit | record + submit |
| --- | ---: | ---: | ---: |
| 1280x800 | 1.683 / 1.618 | 2.511 / 2.647 | 4.194 / 4.266 |
| 1920x1080 | 1.816 / 1.726 | 2.772 / 2.874 | 4.588 / 4.600 |
| 3840x2160 | 2.417 / 2.318 | 3.395 / 3.562 | 5.812 / 5.880 |

At 1280x800 the offscreen frame is CPU-bound, and the mean frame's +0.061 ms is this CPU time (+0.072 ms), less a
0.017 ms shorter simulation tick, plus 0.006 ms of the rest. At the two larger sizes the GPU bounds enough of the frame
that it does not show.

Hypothesis, confidence 40: this is gfx-rs/wgpu#10319 moving barrier work from each write to the submit, which costs
RADV a little more than it saves. Trunk carries far more than that PR, and the two were not separated. The S21's gain
was in the present wait; here the windowed present phase is 0.03 to 0.08 ms shorter on trunk in all six pairs at
1920x1080 and 3840x2160, and the frame does not move with it.

### Picture

`oag-game <hd iso> --race --seed 1 --autopilot --no-audio --size WxH --ticks T --screenshot out.png`, rendered on the
Deck, 30.0.2, trunk, 30.0.2 again. **Decoded pixels are identical between the builds in 8 of 8 frames**: default
settings at 1280x800 ticks 300, 900 and 1800 and at 1920x1080 and 3840x2160 tick 900; the player's settings at
1280x800 tick 900 and at 3840x2160 ticks 900 and 1800. The PNG files differ only in the commit string in their `oag`
metadata, which the two builds were stamped with differently.

One of those frames is not deterministic run to run in either build: the player's settings at 1280x800 tick 900 gave
three variants in 11 runs, differing in a 152x31 region at +208+75 (a faint HUD panel). Both builds produced the two
variants that recurred; the third was seen once, on 30.0.2.

### One crash in gamescope

One windowed run (trunk, 3840x2160) ended with `free(): corrupted unsorted chunks` and SIGABRT after the result file
was written, while the harness was killing the session. The core is `/usr/bin/gamescope`'s own, not `oag-game`'s.
1 of 18 windowed runs on trunk and 0 of 19 on 30.0.2 ended this way, counting 8 short 3840x2160 runs per build
repeated to provoke it. Not attributed to either.
