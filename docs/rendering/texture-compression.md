# Texture compression: BC pass-through and the opt-in ASTC re-encode

Every block-compressed texture a disc ships is BC: HD's and Fury's `.gtf` are DXT1/3/5
(`docs/formats/gtf.md`), 2048's `.gxt` carry UBC1-3, Omega's `.gnf` BC7. **No disc authors ASTC**:
it was standardised in 2012, after both consoles' GPUs shipped.

## Three outcomes of one upload

`oag_mesh::mesh_render::texture::upload` is the one place a `ModelTexture` reaches the GPU. For a
block texture it does one of three things, chosen by the device's features:

| Device | Setting | Upload | Memory against RGBA8 |
| --- | --- | --- | ---: |
| `TEXTURE_COMPRESSION_BC` (every desktop GPU, Apple Silicon Macs) | any | the disc's own blocks, unchanged | 1/8 (BC1) or 1/4 |
| no BC (phones: Mali, Adreno, Apple's iOS GPUs; the GL backend) | `original` (default) | decoded to `Rgba8Unorm` with the box-filtered mip chain | 1 |
| no BC, samples ASTC LDR | `astc` | that same decoded picture and chain, re-encoded as ASTC 4x4 | 1/4 |

The device descriptor asks for `TEXTURE_COMPRESSION_ASTC` **only on an adapter without BC**
(`mesh_render::device`), so a desktop device is requested exactly as before. The re-encode is
**hardware's, not a platform's**: the rule is the two features, on any operating system.

The `astc` row is this project's own transform of its own already-decoded output, not new
content: the texels going in are the ones `original` uploads, and what changes is the GPU memory
format and a small loss. It is closer to `mip_chain`'s box filter than to a stand-in asset.

## The setting

`[graphics] texture_compression = "original" | "astc"`, the TEXTURE COMPRESSION row on the
graphics page (`OAG_GRAPHICS_TEXTURE_COMPRESSION`). Default `original`. Global rather than per
title, because the device is. Read by the next race load.

**The row is shown only on a device that takes the re-encode** (`oag_game::texture_cache`). A
boot builds its menu before the window and device exist, so the row is kept while no device was
asked for and trimmed in `App::open_with` the moment one is; a live rebuild (language, mods) and
the `--menu-page` capture trim it in `menu_tree::finish_definition` and `capture::menu_page`. On a
device with BC the setting is inert even when the file says `astc`: `astc::active` checks the
device, not the setting alone.

## The encoder

Arm's `astcenc`, vendored and compiled by the [`ctt-astcenc`](https://crates.io/crates/ctt-astcenc)
crate (0.6.1; the binding is MIT OR Apache-2.0 OR Zlib, astcenc itself Apache-2.0). No binary
download and no `unsafe` in this tree: the crate builds the C++ with `cc`, selects an ISA variant
at run time (SSE2/SSE4.1/AVX2, NEON on aarch64) and exposes a safe `compress`. Its build script
supports x86_64 and aarch64 only, so the dependency and `mesh_render::astc` are compiled for those
two architectures; the web build and any other target have no encoder and never ask for ASTC.
On Android, `cc` links `libc++_shared.so` for C++ by default, which the APK does not ship (the app
failed to load with `dlopen failed: library "libc++_shared.so" not found`); `scripts/build-apk.sh`
sets `CXXSTDLIB_<target>=c++_static` so libc++ is linked into `liboag_android.so`.

- **Block 4x4** (8 bits a texel) by constraint: a compressed texture's base must be a whole number
  of blocks, and 6x6 (3.56 bits) divides no power-of-two side. A base that is not a multiple of 4
  stays RGBA8.
- **Profile `Ldr`, format `Astc { B4x4, Unorm }`**: linear, like the `Rgba8Unorm` it replaces
  (ADR-0020); an sRGB profile would shift every texel.
- **Preset `medium`** (60 on astcenc's scale), chosen by the measurement below.
- Output is ISA-dependent (`ASTCENC_NO_INVARIANCE`): two machines may write different, equally
  valid entries. A test checks quality, never encoded bytes.

### Choosing the preset

`astc_quality_ground_truth` (a sweep, `OAG_SWEEP=1`), base levels against our RGBA8 decode, one
host thread (Ryzen 9 7900):

| Circuit | Preset | Textures | PSNR median | worst | texels > 2 levels off | max error | encode |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| HD Vineta K (track, sky, craft) | fast | 359 | 54.5 dB | 34.0 dB | 14.5 % | 120 | 13.2 s |
| HD Vineta K | **medium** | 359 | 56.1 dB | 35.8 dB | 11.1 % | 63 | 34.8 s |
| HD Vineta K | thorough | 359 | 56.7 dB | 35.8 dB | 10.7 % | 54 | 80.5 s |
| Omega Tech De Ra (BC7) | medium | 199 | 52.6 dB | 36.3 dB | 3.0 % | 71 | 433.9 s |

`fast` halves the time but doubles the largest error; `thorough` costs 2.3 times `medium` for
0.6 dB. Medium agrees with the earlier offline `astcenc` 5.7.1 command-line run on Vineta K's 341
circuit textures (median 56.2 dB, `docs/tools/android.md`, "HD's textures as ASTC"). Omega's BC7
textures are larger (1.4 GiB of blocks on Tech De Ra) and take 12 times as long to encode.

Pulse, Pure and the PS2 pressing ship no BC textures (`Texels::Rgba8` and `Texels::Chain`), so
the setting changes nothing there.

## The cache

`<cache dir>/oag/textures` (`data/cache/textures` in a checkout; the app's cache directory on
Android, `oag_source::cache::default_texture_cache_dir`). One file per texture,
`<sha256>.astc`: an 8-byte magic, the level count, and each level's length and blocks. The key is
SHA-256 over the source level's blocks, its block format, size and level count, the ASTC block
size, the preset, the encoder build and a format version (`astc::Key::of`), so a change to any of
them is a miss, never a stale read. An entry whose header or level sizes do not match what the
upload expects is treated as absent. Files are written whole or not at all
(`oag_io::write_bytes`). Deleting the directory is always safe.

**How it fills.** A load that finds an entry uploads it. One that does not uploads RGBA8 as
`original` would and queues the texture (its source blocks, at most 128 MiB waiting at once) for
**one** background thread, `astc-encode`, which decodes, builds the same mip chain, encodes every
level and writes the entry. So the first race on a circuit draws RGBA8, the next finds the
entries the thread finished, and the queue never holds decoded texels. Nothing runs on the render
or load threads but a hash and a file read per texture.

The loader report says what a race got (`debug`, after the race builds):
`astc: N texture(s) uploaded as ASTC 4x4 from the cache (X MiB), M as RGBA8 and queued for encoding`,
and the thread logs `astc: encoded N texture(s) into DIR in T s` each time its queue drains.

## Measured on the Galaxy S21

Not yet measured with the real encode. The earlier diagnostic arm (ASTC 4x4 void-extent blocks of the right size and rate) read frame -1.1 to -1.8 ms and process GPU memory 2.62-2.66 to 1.50-1.59 GB on HD Vineta K (`docs/tools/android.md`, "HD's textures as ASTC").

## Known limits

- **Wipeout Zone's two-stage arrays** (`mesh_render::zone::layered`) copy the inner and outer
  stage into one array and need matching formats. While the cache is still filling, one stage can
  be ASTC and the other RGBA8; the copy then draws the inner stage on both sides of the transition
  sphere and warns, until the next load finds both entries.
- The per-model texture census (`texture::log_census`) prices the `astc` path's textures as RGBA8
  on the windowed `--race` route, where textures are uploaded at scene build; the `astc:` line
  carries the ASTC bytes.
- The cache has no size cap. An HD circuit's entries are about a quarter of its RGBA8 texture
  memory; see the table above.
