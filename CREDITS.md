# Credits

This project stands on other people's work. Everything below is something we
used, ran, read or reimplemented from. Each entry says what for and, where we
checked, the licence. Nothing here is vendored unless it says so, and nothing
here makes the project affiliated with or endorsed by its authors.

The original games are a specification, not source ([`README.md`](README.md),
[legal](docs/overview/legal.md)). WipEout is a trademark of Sony Interactive
Entertainment.

## Emulators and native ports used as references

We compare the original's behaviour against these. Their source is read for
behaviour only; what lands here is written by hand.

- [PPSSPP](https://www.ppsspp.org/) (GPL-2.0+): PSP. The software renderer is the picture reference for Pulse and Pure, and its debugger and symbol export drive the memory and trace captures.
- [PCSX2](https://pcsx2.net/) (GPL-3.0): PS2. Frames and memory traces for the PS2 pressing of Pulse.
- [RPCS3](https://rpcs3.net/) (GPL-2.0): PS3. Frames, GDB watchpoints (we build a patched copy) and the PS3 disc key layer, for HD/Fury and Pure's PS3 data.
- [Vita3K](https://vita3k.org/) ([source](https://github.com/Vita3K/Vita3K), GPL-2.0): Vita. Runs 2048 for frame pairs and memory reads. Its source was also read for behaviour: the `gxm` type definitions behind our GXP notes ([GXP](docs/formats/gxp.md)) and the package and zRIF handling.
- [shadPS4](https://shadps4.net/) ([source](https://github.com/shadps4-emu/shadPS4), GPL-2.0): PS4. Ran Omega for the first frame pairs, and its `AmdGpu::Image` was read for the GNF texture layout ([GNF](docs/formats/gnf.md)).
- [wipeout_pc](https://github.com/AbdelmoumenDeghbouche/wipeout_pc) (GPL-2.0): a native PC port of the Omega executable. It runs the PS4 code natively and vendors part of shadPS4's video core. Since 2026-11-01 it is the original we compare Omega race frames against, and its live process memory confirmed addresses in our Omega Ghidra pages ([compare page](docs/tools/compare-page.md), [patch addresses](docs/ghidra/functions/ps4-omega-eu/patch-v107-addresses.md)).
- [Waydroid](https://waydro.id/): Android container used to run the Android build.

## Reverse-engineering tools

- [Ghidra](https://ghidra-sre.org/) (Apache-2.0, NSA): disassembly and decompilation of every executable ([toolchain](docs/reverse-engineering/toolchain.md)).
- [ghidra-allegrex](https://github.com/kotcrab/ghidra-allegrex) (Apache-2.0): the Allegrex and VFPU processor module for the PSP ([Allegrex notes](docs/psp/allegrex-vfpu.md)).
- [ghidra-emotionengine-reloaded](https://github.com/chaoticgd/ghidra-emotionengine-reloaded) (Apache-2.0): the PS2 Emotion Engine processor module.
- [Ps3GhidraScripts](https://github.com/clienthax/Ps3GhidraScripts): the PS3 loader and script pack.
- [VitaLoaderRedux](https://github.com/CreepNT/VitaLoaderRedux) (BSD-3-Clause-Clear): the Vita loader, with [PSVita-RE-tools](https://github.com/TeamFAPS/PSVita-RE-tools) (GPL-3.0, CelesteBlue) as the reference for the NID and import tables.
- [GhidraOrbis](https://github.com/astrelsky/GhidraOrbis) (GPL-3.0): the PS4 loader.
- [Ghidra-Cpp-Class-Analyzer](https://github.com/astrelsky/Ghidra-Cpp-Class-Analyzer) (MIT): C++ class recovery, referenced in the toolchain notes.
- [GhidraMCP](https://github.com/LaurieWired/GhidraMCP) (Apache-2.0): the bridge our agents drive a running Ghidra through.
- [GDB](https://www.sourceware.org/gdb/): attached to RPCS3 (through our proxy) and the native Omega port for watchpoints and memory reads.

## Disc, package and container tools

- [psvpfstools](https://github.com/motoharu-gosuto/psvpfstools): opening Vita PFS packages (`scripts/build-psvpfstools.sh`).
- [libzRIF](https://github.com/imWatchful/libzRIF) (KorewaWatchful) and Vita3K's zRIF code: read to confirm how a zRIF becomes a klicensee ([`scripts/zrif-to-klicensee.py`](scripts/zrif-to-klicensee.py)).
- [pkg2zip](https://github.com/mmozeiko/pkg2zip) (Unlicense): the public description of the Vita package's outer layer ([Vita packages](docs/formats/vita-package.md)).
- [LibOrbisPkg](https://github.com/maxton/LibOrbisPkg) (LGPL-3.0): the PS4 package layout.
- [wipeout-pure-dlc2dlc](https://gitlab.com/thp/wipeout-pure-dlc2dlc) (ISC, Thomas Perl): the published per-pack keys for Pure's DLC ([ADR-0033](docs/architecture/adr/0033-external-key-material-for-decryption.md), [DLC packs](docs/formats/dlc-pack.md)).
- [GFD-Studio](https://github.com/tge-was-taken/GFD-Studio): read for the GNF texture header ([GNF](docs/formats/gnf.md)).
- [chdman](https://www.mamedev.org/) (`mame-tools`): the reference implementation our CHD reader is checked against, and the tool `just` uses to unpack images.
- [FFmpeg](https://ffmpeg.org/) (LGPL/GPL): an external program, last resort only ([ADR-0024](docs/architecture/adr/0024-in-process-codecs-and-ffmpeg-as-a-last-resort.md)): Bink and PS2 movie transcodes into the AV1 cache, PSP `.AT3` decoding, and an independent check of our IPU demuxer.
- [GStreamer](https://gstreamer.freedesktop.org/) (LGPL-2.1): the optional native video path on Linux ([ADR-0017](docs/architecture/adr/0017-gstreamer-native-video.md)).

## Algorithms and techniques we reimplemented or ported

- **FSR 1 and FSR 3.1**: [AMD FidelityFX SDK](https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK) (MIT). `crates/post` holds WGSL ports of the EASU and RCAS passes and the FSR 3.1 upscaler, diffable against tag v1.1.4. Licence text in [`licences/AMD-FidelityFX-MIT.txt`](licences/AMD-FidelityFX-MIT.txt); see [FSR 3](docs/rendering/fsr3.md).
- **SMAA**: [Jorge Jimenez et al.](https://github.com/iryoku/smaa) (MIT). A WGSL port of the MEDIUM preset. Licence text in [`licences/SMAA-MIT.txt`](licences/SMAA-MIT.txt).
- **FXAA**: Timothy Lottes' technique, written fresh from the published description, not from NVIDIA's FXAA 3.11 source, which carries its own terms (`oag_post::fxaa`).
- **Motion blur**: McGuire, Hennessy, Bukowski and Osman, *A Reconstruction Filter for Plausible Motion Blur* (I3D 2012), written from the paper.
- **AV1 movie cache**: [rav1d](https://github.com/memorysafety/rav1d) (via `re_rav1d`, BSD-2-Clause), a Rust port of [dav1d](https://code.videolan.org/videolan/dav1d), plays the cache ([ADR-0008](docs/architecture/adr/0008-av1-movie-cache.md)).
- **ATRAC9**: [atrac9dec](https://github.com/claimore22/atrac9dec) (MIT) decodes Vita and PS4 music. ATRAC3+ has no Rust decoder we use; PSP `.AT3` goes through FFmpeg.
- **MP3** (HD's music container): [Symphonia](https://github.com/pdeljanov/Symphonia) (MPL-2.0).
- **CHD**: [chd-rs](https://github.com/SnowflakePowered/chd-rs) (BSD-3-Clause), with the perf fork of [lzma-rs](https://github.com/SnowflakePowered/lzma-rs/tree/feature-perf-experiments) (MIT).
- **Networking**: the techniques in the reading list of [`docs/networking/README.md`](docs/networking/README.md), among them Jared Cone's GDC 2018 talk *It IS Rocket Science!*, Glenn Fiedler's networked physics series, Mauve et al.'s local-lag and timewarp, and Pantel and Wolf on racing-car collisions.
- **Deterministic maths**: [libm](https://github.com/rust-lang/libm) (MIT), a pure Rust port of MUSL's libm, so transcendentals give the same bits on every target.

## Format and research references

- [Vita3K](https://github.com/Vita3K/Vita3K)'s `gxm/types.h`, [GFD-Studio](https://github.com/tge-was-taken/GFD-Studio) and the [PlayStation GNF Image](https://rewiki.miraheze.org/wiki/PlayStation_GNF_Image) page: the GXP and GNF texture and shader containers.
- [CelesteBlue-dev's PSVita-RE-tools](https://github.com/TeamFAPS/PSVita-RE-tools) and the Vita3K package code: the Vita package and key layers.
- [SDL](https://github.com/libsdl-org/SDL) (Zlib): `SDL_gamepad.c` and `SDL_steam_virtual_gamepad.c`, read for how controllers and Steam Input are identified ([button prompts](docs/ui/button-prompts.md)).
- Patch and release history of the originals: [PlayStation Blog](https://blog.playstation.com/), [xtremepsvita](https://www.xtremepsvita.com/2012/06/19/wipeout-2048-updated-to-v1-04/), [Engadget](https://www.engadget.com/2012-06-19-wipeout-hd-and-fury-come-to-vita-as-2048-dlc-free-if-you-alread.html) and the [WipEout wiki](https://wipeout.fandom.com/wiki/Downloadable_Content), cited in [patches](docs/formats/patches.md).
- [shadPS4's game compatibility report](https://github.com/shadps4-compatibility/shadps4-game-compatibility/issues/487) and quickstart: the Omega set-up notes in [online lobby](docs/networking/online-lobby.md).
- [W3C WebCodecs codec registry](https://www.w3.org/TR/webcodecs-codec-registry/): codec strings for the web build.
- [TOML Schema](https://toml-schema.org/) ([tosd](https://github.com/brunoborges/toml-schema), MIT): the schema language for our TOML files ([ADR-0064](docs/architecture/adr/0064-project-toml-files-declare-a-toml-schema.md)).
- [WESL](https://wesl-lang.dev): the shader language our render shaders are written in ([ADR-0059](docs/architecture/adr/0059-render-shaders-are-wesl-compiled-to-wgsl-at-build-time.md)).

## Content we read

- **BallisticNG and Unity**: we read BallisticNG Workshop track and craft files through our own generic Unity asset-bundle reader (`oag-unity`, `oag-ballisticng`). It reads the content a player supplies; no BallisticNG code or assets are shipped here. BallisticNG is by Neognosis and Unity is by Unity Technologies.
- **The original games**: Wipeout Pure, Pulse, HD, Fury, 2048 and the Omega Collection, by Studio Liverpool, XDev, Clever Beans and EPOS Game Studios. Their data is read from the player's own disc image and never committed.

## Fonts and art

- [PromptFont](https://codeberg.org/shinmera/promptfont) by Shinmera (Yukari Hafner), SIL Open Font License 1.1: the substitute button prompts in `assets/ui/prompts/` are rasterised from it. Licence text in [`licences/PromptFont-OFL.txt`](licences/PromptFont-OFL.txt).
- [Blender](https://www.blender.org/) (GPL-2.0+): the track-authoring add-on is built on it ([Blender](docs/tools/blender.md)).

## Rust crates

The direct dependencies that matter most. The full set, with licences, is in `Cargo.lock` and the crates' own manifests.

- Graphics, windowing, shaders: [wgpu](https://wgpu.rs/) and naga (MIT or Apache-2.0), [winit](https://github.com/rust-windowing/winit) (Apache-2.0), [wesl](https://github.com/webgpu-tools/wesl-rs) (MIT or Apache-2.0), [pollster](https://github.com/zesterer/pollster), [resvg](https://github.com/linebender/resvg) (Apache-2.0 or MIT), [gltf](https://github.com/gltf-rs/gltf) (MIT or Apache-2.0), [png](https://github.com/image-rs/image-png).
- Audio and input: [cpal](https://github.com/RustAudio/cpal) (Apache-2.0), [gilrs](https://gitlab.com/gilrs-project/gilrs) (Apache-2.0 or MIT), [rtrb](https://github.com/mgeier/rtrb), [arboard](https://github.com/1Password/arboard).
- Maths and numbers: [glam](https://github.com/bitshifter/glam-rs) (MIT or Apache-2.0), [bytemuck](https://github.com/Lokathor/bytemuck), [num-bigint](https://github.com/rust-num/num-bigint).
- Crypto and compression: [aes](https://github.com/RustCrypto/block-ciphers) (RustCrypto), [miniz_oxide](https://github.com/Frommi/miniz_oxide), [zip](https://github.com/zip-rs/zip2) (MIT).
- Extensions and network: [wasmtime](https://github.com/bytecodealliance/wasmtime) (Apache-2.0 with LLVM exception), [web-transport-quinn](https://github.com/moq-dev/web-transport), [rustls](https://github.com/rustls/rustls), [ring](https://github.com/briansmith/ring), [rcgen](https://github.com/rustls/rcgen), [tokio](https://github.com/tokio-rs/tokio) (MIT).
- Web and mobile: [wasm-bindgen](https://github.com/wasm-bindgen/wasm-bindgen) with web-sys and js-sys, [web-time](https://github.com/daxpedda/web-time), [android_logger](https://github.com/rust-mobile/android_logger-rs).
- Data and plumbing: [serde](https://github.com/serde-rs/serde) and serde_json, [toml](https://github.com/toml-rs/toml) and toml_edit, [toml-schema](https://github.com/brunoborges/toml-schema), [clap](https://github.com/clap-rs/clap), [anyhow](https://github.com/dtolnay/anyhow), [thiserror](https://github.com/dtolnay/thiserror), [log](https://github.com/rust-lang/log) and [env_logger](https://github.com/rust-cli/env_logger), [dirs](https://codeberg.org/dirs/dirs-rs), [url](https://github.com/servo/rust-url), [semver](https://github.com/dtolnay/semver), [memory-stats](https://github.com/Arc-blroth/memory-stats).

## Development tooling

- [rustup](https://rustup.rs) and the Rust toolchain; [cargo-nextest](https://nexte.st) (test runner); [just](https://github.com/casey/just) (CC0, task runner); [cargo-shear](https://github.com/Boshen/cargo-shear) (unused dependencies); [cargo-ndk](https://github.com/bbqsrc/cargo-ndk) (Android builds).
- [uv](https://docs.astral.sh/uv/) (Apache-2.0 or MIT) and Python run the capture and check scripts, with [evdev](https://python-evdev.readthedocs.io/), python-xlib, websocket-client and NumPy pulled in by `uv run --with`.
- [Xvfb](https://www.x.org/) and [sway](https://swaywm.org/): the private displays the emulator and native-port captures run in.
- [binaryen](https://github.com/WebAssembly/binaryen)'s `wasm-opt` (web build), [appimagetool](https://github.com/AppImage/appimagetool) (Linux package), [act](https://github.com/nektos/act) (running the CI and release workflows locally), the Android SDK build tools (`aapt2`, `apksigner`).

## AI tooling

Most of the code and documentation here is written by AI agents, steered and reviewed by the maintainer. The agents are [Claude Code](https://claude.com/claude-code) running Anthropic's Claude models. Co-authorship is recorded in the commit trailers.
