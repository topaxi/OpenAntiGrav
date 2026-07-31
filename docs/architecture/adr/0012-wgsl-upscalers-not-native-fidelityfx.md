# ADR-0012: Port the FSR upscalers to WGSL rather than driving the native SDK

## Status

Accepted.

FSR 1's EASU and RCAS are ported and shipped behind `[graphics] upscaler`
([`oag_render::post::fsr1`](../../../crates/render/src/post/fsr1.rs)). FSR 3.1
is not built; this decides the route it will take when it is.

## Context

[modern-features.md](../../overview/modern-features.md) recorded, in 2026-07,
that AMD's FidelityFX SDK is MIT-licensed and that integrating it with a wgpu
renderer had two possible routes, deliberately left undecided:

1. drive the SDK's Vulkan backend through `wgpu-hal`'s `as_hal` escape hatches,
   keeping upstream's tested implementation; or
2. port the upscaler shaders to WGSL.

It also recorded a payoff that made route 1 attractive: FSR4, the closed ML
generation, upgrades any game exposing the FSR 3.1 API surface, so exposing that
surface would get FSR4 free without shipping anything proprietary. The document
asked for its licence statements to be re-verified when the milestone opened.

**Re-verified 2026-07-31, and the payoff does not survive it.** FSR 3.1 and its
Vulkan backend remain MIT with full source, unchanged. But FSR4 ships as
prebuilt, signed DLLs in FidelityFX SDK 2.0 (August 2025) with no source; a
brief accidental source publication was withdrawn, and SDK 2.2 adds FSR
Upscaling 4.1 on the same terms. The driver-side upgrade path is a
Windows/Adrenalin mechanism: for a native Linux ELF it does not exist, and on
Linux it reaches only *Windows* games, through Proton's DLL substitution.

So the argument that route 1 buys a free upgrade to a better upscaler is void on
the platforms this project targets first. What is left of route 1 is "reuse
upstream's tested implementation", and that has to pay for:

- **lifting `unsafe_code = "deny"`**, which is set at the workspace root and
  which no crate has ever lifted (ADR-0001 permits it "with a documented
  justification", and this would be the first);
- **a C++/CMake build**, against the precedent [ADR-0008](0008-av1-movie-cache.md)
  set when it chose `re_rav1d` specifically so that "no C toolchain and no
  `nasm` enters the build";
- **`ash` and `wgpu-hal` pinned to wgpu's exact version**, a coupling that
  breaks on every wgpu upgrade rather than at a boundary of our choosing;
- **Vulkan only**, when [goals.md](../../overview/goals.md) names macOS and the
  Steam Deck in the first tier and Android, WebAssembly and consoles later.

Meanwhile route 2's cost turned out to be lower than feared. FSR 1's EASU and
RCAS came to about 300 lines of WGSL, and the port is a transliteration: the tap
pattern, the magic constants and the bit-trick reciprocals are upstream's,
left as they are so the WGSL can be diffed against `ffx_fsr1.h` line for line.

## Decision

**Port the FSR upscalers to WGSL. Do not link, vendor or drive the native
FidelityFX SDK, and do not lift `unsafe_code = "deny"` for it.**

Consequences accepted:

- **Requesting GPU features stops being free.** FSR 3.1's compute passes need
  storage-texture formats and access modes outside the WebGPU baseline, and the
  device currently asks for `Features::empty()`. Any such request must go behind
  an adapter probe with a fallback chain - FSR 3.1, then FSR 1, then the plain
  blit - because a missing feature must degrade, never fail to boot. This is
  what makes FSR 1 load-bearing rather than a stepping stone.
- **Upstream's fixes do not arrive for free.** A port is a fork. The mitigation
  is that it is a *transliteration* and stays one, so a diff against a new
  `ffx_fsr1.h` remains readable.
- **Some things cannot be transliterated at all.** FSR 3.1's SPD-based pyramids
  rely on `globallycoherent` read-write textures plus an atomic counter, and
  WGSL has no equivalent memory-coherency guarantee; they will need a
  conventional multi-pass mip reduction. That deviation is known in advance
  rather than discovered mid-port.
- **Attribution and licence text travel with the code.** `ffx_fsr1.h`'s MIT
  notice is reproduced in `licences/AMD-FidelityFX-MIT.txt` and every ported
  file carries a header naming AMD and the upstream repository.
- **Nothing FSR4-specific gets hard-coded**, per the original strategy. If the
  driver-side upgrade path ever does reach a native Linux build, exposing the
  3.1 API surface remains the way to inherit it - but no design decision here
  depends on that happening.

## Alternatives considered

**Route 1, `as_hal` into the FidelityFX Vulkan backend.** Rejected above: its
one remaining benefit is outweighed by four costs, two of which contradict
standing decisions (ADR-0001's unsafe policy, ADR-0008's no-C-toolchain
precedent).

**Ship no upscaler and let players use their compositor's.** Rejected because
the render-scale setting already exists and already shipped; something resamples
that frame whatever happens, and choosing what does it is the whole point.
A compositor also has no access to the pre-UI frame, which is where a
scene-only upscaler has to run.

**Wait for wgpu to expose what FSR 3.1 needs.** Not an alternative so much as a
hope, and ADR-0001 already recorded the answer: "if wgpu turns out to be the
wrong abstraction for something the project needs, the renderer is one crate."

## Consequences for other documents

[modern-features.md](../../overview/modern-features.md) is updated with the
FSR4 finding and points here for the route. ADRs are immutable, so if the
licensing changes again - if AMD open-sources FSR4, or the upgrade path reaches
Linux - that is a new ADR superseding this one, not an edit.
