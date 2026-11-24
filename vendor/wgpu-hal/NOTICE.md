# OpenAntiGrav changes to wgpu-hal 30.0.2

This is `wgpu-hal` 30.0.2 from crates.io (MIT OR Apache-2.0, `LICENSE.MIT` and
`LICENSE.APACHE` unchanged) with one change, a per-device cache of the SPIR-V the
Vulkan backend writes for each pipeline stage, the Vulkan counterpart of
upstream's D3D12 `ShaderCache` (gfx-rs/wgpu#7729). Without it, every pipeline built from
a module with override constants re-resolves the constants and re-writes SPIR-V for
stages identical to ones already written. Why we carry it, how it was measured and
how to drop it: `docs/architecture/vendored-wgpu-hal.md` in the OpenAntiGrav
repository.

Changed files:

- `src/vulkan/shader_cache.rs` (new): the cache, its key and its unit tests.
- `src/vulkan/mod.rs`: `ShaderModule::Intermediate::cache_id`, `Device::shader_cache`.
- `src/vulkan/adapter.rs`: the device starts with an empty cache.
- `src/vulkan/device.rs`: `write_intermediate_stage` (the former body of
  `compile_stage`'s intermediate arm, unchanged), the cache lookup in
  `compile_stage`, the module id in `create_shader_module`, and eviction in
  `destroy_shader_module`.
