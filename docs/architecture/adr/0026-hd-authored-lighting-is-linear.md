# ADR-0026: Wipeout HD's authored lighting is computed in linear light

## Status

Accepted. Narrows [ADR-0020](0020-gamma-authoritative-colour-space.md) rather
than superseding it, the way ADR-0025 narrows ADR-0023: gamma remains the
authoritative *storage and interchange* space everywhere - texture uploads stay
`Rgba8Unorm`, blending still operates on stored bytes, the PSP and PS2
pipelines are untouched to the byte. What changes is the arithmetic inside one
shader branch that no other title takes.

## Context

ADR-0020's argument is the PSP's: the Graphics Engine blends stored framebuffer
bytes, the art was authored against that arithmetic, and linearising would
re-shade it. Every recovered `Gu_*` call supports that, and nothing here
disputes it for Pulse or Pure.

Wipeout HD's own data argues the other way, and
[envsettings.md](../../formats/envsettings.md) had been carrying the tension as
an open question since the file was first read:

- `track.envsettings` authors a sun colour of 4.0 and an ambient of 3.0 -
  values that are only meaningful to a renderer that works in linear light
  under an exposure stage, and that ADR-0020's pipeline can only clip.
- The circuit materials' own fragment microcode - readable since
  [`scripts/ps3-microcode.py`](../../../scripts/ps3-microcode.py) - states the
  lighting combination outright (`prelitScale * lightmap^prelitPower`, the sun
  gated by the lightmap's shadow alpha, an inline `pow 32` specular), and
  those are HDR-shaped terms: on Talon's Junction the prelit peak alone is 4.

Drawing that equation in gamma space was tried first, and **measured against
an rpcs3 reference frame of the same Talon's Junction grid** - the reference
ADR-0020 said would be needed before this question could move. It failed in
both directions at once: 13.2 % of the frame clipped to white (the track
surface losing its panel detail entirely, which reads as "missing textures"),
while surfaces lit by ambient alone - authored 0.40 linear, displayed as the
byte 0.40 - sat far darker than the same surfaces in the reference.

## Decision

`mesh.wgsl`'s **authored branch** - the one `scene.light.enabled` selects,
which only a Wipeout HD race sets - shades in linear light:

- the diffuse and lightmap samples are sRGB-decoded (`pow 2.2`),
- the read equation is applied at its authored magnitudes,
- the result is saturated and sRGB-encoded back (`pow 1/2.2`) into the same
  gamma target every other draw writes.

The saturate is this project's stand-in for HD's exposure stage, whose
adaptive parameters (`Tone adaption boost`, `Tone maximum brightness`) are
read and not yet understood; it is the one term of the chain that is not the
disc's, and `mesh_render::Light` says so.

The stand-in path, the prelit path (`lit` 0.0) and every title that does not
author a rig are bit-identical to before: the branch is selected by
`enabled * lit`, and both factors are zero everywhere ADR-0020's argument
applies.

## Consequences

- On the reference frame's framing, the clipped share falls from 13.2 % to
  7.4 % (the reference's own is 12.6 %, mostly its bloom) and the track's
  authored panel detail is visible again where it was a white sheet.
- The `2.2` power is the sRGB approximation, not a measured display curve;
  if HD's RSX texture setup is ever read (the `GAMMA` remap bits are not in
  the `.gtf` file), the per-texture truth replaces the blanket decode.
- A future exposure implementation replaces the saturate, not the decode.

## Alternatives considered

- **Keep gamma arithmetic and drop the authored magnitudes to hue** (what
  shipped before this ADR): measured above; it cannot show the authored
  midtones and the authored shadows at the same time.
- **sRGB render targets and samplers throughout** (pre-ADR-0020): re-breaks
  the PSP titles for the reasons ADR-0020 records.
- **Tonemap with an invented curve** (Reinhard and friends): rejected for the
  reason the fog was left undrawn until its curve was read - a plausible
  curve is unfalsifiable, and the saturate at least claims nothing.
