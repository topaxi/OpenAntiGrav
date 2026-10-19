# ADR-0072: The ray tier takes wgpu's experimental-features token

## Status

**Accepted for the `raytrace-spikes` branch (maintainer, 2026-10-19):** `unsafe` is
allowed on that branch for what ray tracing needs - the experimental-features token
and any acceleration-structure call that requires it - scoped as narrowly as
possible. **Not merged into main.** If the branch merges, this ADR is re-read first.

Relates to [`docs/rendering/ray-tracing.md`](../../rendering/ray-tracing.md), which
holds the design, and to the workspace's `unsafe_code = "deny"` (`Cargo.toml`).

## Context

wgpu 30.0.1 exposes hardware ray queries as `Features::EXPERIMENTAL_RAY_QUERY`. A
device may only request an `EXPERIMENTAL_*` feature if its descriptor carries
`ExperimentalFeatures::enabled()`, and that constructor is `unsafe fn`: calling it
acknowledges that the feature "may result in undefined behavior when used
incorrectly" and that bugs found will be reported. The workspace denies
`unsafe_code`, and nothing else in it needs the token.

## Decision

1. **One function, one `unsafe` block.** `oag_mesh::mesh_render::ray::experimental_token`
   carries `#[allow(unsafe_code)]` and a `// SAFETY:` comment. No crate- or
   workspace-wide lift, and no other `unsafe` anywhere in the tier: BLAS, TLAS and
   ray-query calls are all safe wgpu API once the device has the feature.
2. **The token is taken only when the tier is on and the adapter reports the
   feature** (`ray::wanted_for`). Every `request_device` in the workspace reaches it
   through `mesh_render::device_descriptor`, so the window, the captures and the viewer
   agree, and a device that never asked for the tier gets the descriptor it always did
   (pinned by `a_device_that_never_asked_for_the_tier_is_left_exactly_as_it_was`).
3. **Render side only.** Nothing in a simulation crate changes, so
   [determinism](../determinism.md) is not involved.

## Consequences

- A bug in wgpu's ray-query implementation can be undefined behaviour in a build that
  turned the tier on. The default build never takes the token.
- The workspace's `unsafe_code = "deny"` still holds everywhere else; a second
  `#[allow(unsafe_code)]` anywhere in the tier is a finding against it.
- When wgpu stabilises the feature the token goes away and this ADR is superseded.
