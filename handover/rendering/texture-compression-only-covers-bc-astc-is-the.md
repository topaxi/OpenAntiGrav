# Texture compression only covers BC; ASTC is the unimplemented other half

2026-09-03. `crates/render::mesh_render::optional_features` probes and requests
exactly one compressed-texture feature, `wgpu::Features::TEXTURE_COMPRESSION_BC`
(`crates/render/src/mesh_render.rs:69`), and `texture::upload` decodes back to
`Rgba8Unorm` on the CPU when the adapter lacks it
(`crates/render/src/mesh_render/texture.rs`, "Uploads one `ModelTexture`, in
the form it came in"). That fallback exists for the GL backend this project
does not ship - it also fires, unintentionally, on every Apple Silicon Mac
today, since `Backends::PRIMARY` includes Metal and Apple GPUs decode ASTC
natively, not BC. **`TEXTURE_COMPRESSION_ASTC` is never requested, so that
adapter always takes the CPU-decode path**, the one comment on `upload`
explicitly frames as the exception rather than the rule.

**This is a GPU-format question, not an asset one - no source texture on any
of the six discs is authored in ASTC.** `docs/formats/gxt.md` puts Vita's own
compressed textures at `PVRTII4BPP` and `UBC1/2/3` (BC1-3); `docs/formats/gtf.md`
has PS3's at `DXT1`/`DXT45` (BC1/BC3). ASTC was standardized in 2012, after
both consoles' GPUs shipped, so it was never an option for either disc to
author. Adding it here means **compressing this project's own already-decoded,
already-faithful RGBA output** into an ASTC block texture for upload - not
inventing new pixels, the never-invent rule this file's siblings enforce is
about game content, and the texel values going in are unchanged. It is closer
to `mip_chain`'s box filter than to a `.pob` stand-in: a GPU-memory-format
transform of data already read off the disc, done at load time rather than
disc-authored. Worth saying explicitly since the shape looks adjacent to a
rule this codebase takes seriously.

**The "other half" is asymmetric with the BC path, and that asymmetry is the
actual work.** BC support today is a pass-through: the disc already ships
BC-compressed bytes (`Texels::Blocks`), and `upload` hands them to the GPU
unchanged when the feature is present. There is no equivalent ASTC payload to
pass through - it has to be **encoded**, once, from the RGBA bytes this
project already produces. ASTC encoding is comparatively expensive (block
search, not a fixed unpack), so unlike BC this is not a free branch in the
existing upload path; it wants doing at asset-cook time, not per-frame, and
probably not even per-process-startup for anything but small/dev iteration.

## Open

- No ASTC encoder is wired at all - not even a naive one. Candidates: shell
  out to `astcenc` (ARM's reference command-line encoder, quality/speed knobs
  built in) as an offline cook step, or a Rust ASTC-encoding crate if one
  exists at adequate quality - unresearched, check crates.io before writing
  one by hand
- **Where the compressed output would be cached.** BC bytes live in the
  asset's own container and need no cache; an ASTC re-encode has nowhere to
  live yet. A `data/`-adjacent derived-cache directory (gitignored, the same
  shape `data/` itself already has) is the obvious answer, not yet decided
- Today's overlap between "BC available" and "ASTC available" is narrow: only
  Apple Silicon Macs, since `Backends::PRIMARY` (Vulkan/Metal/DX12) excludes
  every backend where ASTC is the *only* option (Android, iOS - neither
  targeted) and includes only one where ASTC is native (Apple Silicon Metal).
  The overlap widens if/when a handheld port opens (Switch and Steam Deck are
  both in `docs/rendering/README.md`'s wider ambition, ADR-0001's "eventually
  portable to handhelds and the web") - Switch's Tegra decodes both BC and
  ASTC natively, Steam Deck's RDNA2 decodes BC only, so the "pick whichever
  performs better" question only has a real answer to choose between on
  Switch and Apple Silicon Mac, not on Steam Deck
- No benchmark exists comparing BC vs ASTC bandwidth/decode cost on real
  Apple Silicon hardware in this renderer - the "which wins" premise below is
  unmeasured, not assumed from general GPU literature
- General hardware-support claims above (which platforms decode which format)
  are from public documentation and vendor feature tables, not from this
  project's own RE work the way the BC/PVRTC finding is - treat them at a
  correspondingly lower confidence than a disc-measured fact, and recheck
  against `vulkan.gpuinfo.org`/Apple's Metal feature tables before relying on
  a specific device's row

## Next Steps

1. Pick or write the ASTC encoder (`astcenc` shellout is the fastest path to
   something measurable; a pure-Rust encoder avoids a build-time binary
   dependency but likely costs research time first)
2. Wire it as a load-time or cook-time path parallel to `texture::upload`'s
   existing BC branch, landing behind `TEXTURE_COMPRESSION_ASTC` probed the
   same way `optional_features` already probes BC - additive, no change to
   any adapter that only has BC
3. **On an adapter reporting both features**, benchmark actual frame cost
   (upload bandwidth in `oag_render::timing::PassTimer`, or a dedicated
   micro-benchmark) rather than assuming either format wins from spec sheets.
   Whichever measures faster becomes the default *on that adapter*; the loser
   stays selectable but not default. Apple Silicon Mac is the only hardware
   this can be measured on today - a Switch/Steam Deck port would each need
   their own measurement pass before this thread's "default" claim extends to
   them
4. Once a default exists, document the choice and the measurement it rests
   on - this is exactly the kind of claim that needs evidence recorded next
   to it, not just a code comment, per this project's own RE-documentation
   habit even though it's an engineering choice rather than an RE one

## From the HANDOVER.md index (moved 2026-09-25)

No disc authors ASTC (standardized after both consoles' GPUs shipped), so this is an encode of this project's own already-faithful RGBA output, not new content. Open: no encoder is wired, no cache location decided, and "pick whichever performs better as the default when both are supported" is unmeasured - needs a real benchmark on hardware that offers both, not a spec-sheet assumption
