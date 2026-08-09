# ADR-0020: Gamma is the authoritative colour space; nothing linearises

## Status
Accepted

Supersedes the reasoning written into `crates/render/src/mesh_render.rs:186-201`,
which chose an sRGB render target on the premise that textures were linearised.
That premise is removed here, so the conclusion it supported does not carry.

## Context

The PSP's Graphics Engine blends **stored framebuffer bytes**. Every blend
equation this project has recovered is defined on them: `Mesh_SetBatchDrawState`
programs `Gu_BlendFunc(GU_ADD, GU_FIX 0xffffff, GU_FIX 0xffffff)` - "add the
stored bytes" - and `Gfx_Init`'s `GU_TFX_MODULATE` multiplies stored bytes by a
vertex colour that is itself a stored byte. The art was authored against that
arithmetic. See
[mesh-draw.md](../../ghidra/functions/psp-pulse-usa/mesh-draw.md).

Our pipeline was doing two different things at once, and had been since before
anyone looked:

| Stage | Boost plume | Other meshes | PSP |
| --- | --- | --- | --- |
| Texel into the shader | gamma (`race.rs` pre-encoded to cancel the sampler) | **linear** (sRGB sampler decoded) | gamma |
| Vertex colour into the shader | gamma | gamma | gamma |
| `mesh.wgsl` output | gamma x gamma | **linear x gamma** | - |
| `--screenshot` target | stored raw | stored raw | gamma |
| Window surface | encoded **again** | encoded | gamma |

Three things follow from that table, and the third is what forced the decision.

**The vertex colour was never in the same space as the texel.** `mesh.rs`
builds it as `f32::from(byte) / 255.0` and nothing transforms it; vertex
attributes are never sRGB-decoded by hardware. So `texel.rgb * in.colour.rgb`
multiplied a linear value by a gamma value, in one expression, for every lit
surface in the game. Only the texel's space ever varied.

**`One`/`One` addition is where it showed.** Additive blending on linear light
crushes halo mid-tones against the PSP's own arithmetic: `0.4 + 0.5 = 0.90`
stored, against `0.133 + 0.214 = 0.347` linearised and `0.62` re-encoded - about
30% of encoded brightness lost. That is why `race.rs` grew a load-time
re-encode of the boost plume's texels: a local compensation for a global
inconsistency.

**The window and the captures disagreed by up to 73/255.** The plume's shader
output was already gamma, so an sRGB window surface encoded it a second time
while an ordinary `--screenshot` (`Rgba8Unorm`) stored it raw:

| shader out | `--screenshot` | window | delta |
| ---: | ---: | ---: | ---: |
| 0.10 | 26 | 89 | +63 |
| 0.20 | 51 | 124 | +73 |
| 0.40 | 102 | 170 | +68 |
| 0.60 | 153 | 203 | +50 |
| 0.80 | 204 | 231 | +27 |

And in that same capture the scene *behind* the plume was stored as unencoded
linear light, so it read too dark. Any measurement of one against the other was
measuring two colour spaces at once.

## Decision

**Gamma is authoritative. Nothing in the pipeline linearises, and nothing
encodes on write.**

Concretely, and all in one change because no part of it is correct alone:

1. `mesh_render.rs`'s texture upload: `Rgba8UnormSrgb` -> `Rgba8Unorm`.
2. `race.rs`'s load-time plume re-encode: **deleted**.
3. `mesh_render::screenshot`'s target: `Rgba8UnormSrgb` -> `Rgba8Unorm`.
4. The **window surface stops encoding**: `config.format.remove_srgb_suffix()`,
   forced rather than accepted.
5. `race.rs`'s `--presented` capture target follows the window to
   `Rgba8Unorm`, so the two capture paths agree.
6. `oag-view`'s two surfaces and its texture-browser upload follow, so the
   viewer keeps showing what the game shows.

### Why the window, and not "`mesh.wgsl` encodes on write"

The two options are only symmetrical if the shader's output is linear. It is
not: once the texture upload is raw, `mesh.wgsl` emits gamma-space values, and
encoding those on write is the double-encode this ADR exists to remove. There is
no encode step anywhere in a gamma-authoritative pipeline, so the surface is the
only place the change can go.

### Why the `race.rs` re-encode becomes *wrong*, not merely unnecessary

This is the part most likely to be undone by a future reader, so it is stated
plainly. That loop pre-encoded the plume's texels so that an sRGB sampler's
decode would return the disc's original bytes - `decode(encode(x)) == x`. With a
raw upload there is no decode. The same loop would then hand the shader
`encode(x)` instead of `x`, brightening the plume by the full sRGB curve. It is
not a harmless leftover; it is the same bug in the other direction, which is why
it had to be deleted in the same commit that changed the upload.

## Consequences

**HANDOVER's 54.6/255 measurement is not contradicted.** That row measured, for
`oag-view`, that forking the *texture* format is wrong and encode-on-write is
right. It is conditional on textures being linearised: given a linear texel
multiplied by a light rig, the multiply lands between decode and encode and only
encode-on-write is equivalent. Change the premise and the conclusion is not
inherited. The figure remains useful as the size of error a **partial**
migration reintroduces - which is the argument for items 1-6 being one change.

**HANDOVER's "do the game's capture paths need encode-on-write?" is answered
no**, and its stated risk inverts. That question worried that switching capture
targets would newly encode the HUD's and front end's authored text and fill
colours, which were tuned against a window. Nothing gains an encode here; the
*window* loses one. Authored colours now reach the screen as authored on every
path, and the window and the captures agree instead of diverging. `render.rs`
already forks the sprite sheet's texture format on `format.is_srgb()` and simply
takes the raw side from now on.

**The post-processing chain is unaffected by construction.** FXAA, SMAA, FSR 1
and `upscale.rs` all derive their working format through `remove_srgb_suffix()`,
which is the identity on a format that has no suffix.

**This is a visible change to the front end**, not only to the race: authored
sprite and text colours previously reached an encoding surface and will now
appear as authored. That is the intended direction - it is what "authored equals
displayed" means - but it is a change, and a comparison against an older
screenshot of a menu will differ for this reason.

**What would overturn this.** Evidence that the GE's *lighting* stage operates
on linearised values. Nothing read so far suggests it does, and `GU_TFX_MODULATE`
on stored bytes says it does not.

## Alternatives considered

**Linear-light throughout** - textures sRGB, every target sRGB, physically
correct blending. Self-consistent, and the right choice for an engine that wants
to look modern. Rejected because it reproduces a *different* blend equation from
the one the original runs, and this project's goal is the original's picture.
The ~30% additive discrepancy above is the size of the difference on the surfaces
that matter most.

**Leave it, and keep compensating per surface.** This is what the `race.rs`
re-encode was. Rejected: one compensation had already appeared, a second was
implied for the ribbon, and each one hides the inconsistency a little better
while making the next comparison harder to trust.
