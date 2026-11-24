# The vendored `wgpu-hal`

`vendor/wgpu-hal` is `wgpu-hal` 30.0.2 as published on crates.io with one change. The
Vulkan backend keeps the SPIR-V of every pipeline stage it writes, per device, and hands
it to the next pipeline built with the same stage. The root `Cargo.toml` puts it in place
of the published crate with `[patch.crates-io]`. Every renderer and every GPU test uses it.

The change is the Vulkan counterpart of a cache `wgpu` already merged for D3D12: pull
request gfx-rs/wgpu#7729, "[d3d12] add a shader cache to avoid calling into DXC/FXC",
which closed issue #7716, "wgpu should cache pipelines". It copies that PR's placement and
bound, and keys on what is expensive on Vulkan (below).

## Why we carry it

A shader module with pipeline-overridable constants (`override` in WGSL) stays naga IR in
`wgpu-hal` until a pipeline is created. For every stage of every pipeline,
`Device::compile_stage` then does two things:
- it resolves the override constants with
  `naga::back::pipeline_constants::process_overrides`, which revalidates the module;
- it writes SPIR-V with `naga::back::spv::write_vec`.

Our mesh pipelines are many variants of one module, `mesh.wesl`: blend, cull, depth and
vertex-layout variants of a material, whose constants come from the model and the
player's settings. Most of that work therefore repeats a stage already written. A Pulse
PSP race walk calls `compile_stage` 308 times for 126 distinct stages. Before the change
the call was about 25 % of the walk's user instructions.

Two cheaper routes fell short (`docs/architecture/load-time.md`, "Fifth round" and "Sixth
round"):
- Writing SPIR-V at build time cannot reach those stages. Their constants are run-time
  data, and naga 30's SPIR-V writer emits no specialization constants.
- Resolving the constants in our own crate before `wgpu` sees the module reached only
  6.2 %.

The cache is the change that clears the 10 % bar. It can only live inside `wgpu-hal`,
because the SPIR-V depends on the device's writer options and on the pipeline layout's
binding renumbering, both private to the backend.

Measured (`load-time.md`, "Seventh round"), in user instructions, with frames
pixel-identical:

| Launch | Before | After | Change |
| --- | ---: | ---: | ---: |
| Race walk | 16.72 G | 14.27 G | -14.7 % |
| Bare race launch | 9.08 G | 7.16 G | -21.2 % |
| A race-spawning ground-truth test | 27.09 G | 16.96 G | -37.4 % |

## What changed

All in `vendor/wgpu-hal/src/vulkan/`. `vendor/wgpu-hal/NOTICE.md` lists the same files.

- `shader_cache.rs` (new): `ShaderCache`, `ShaderCacheKey` and `ShaderCacheValue`, named
  and shaped like the D3D12 backend's types of the same names.
- `mod.rs`: `ShaderModule::Intermediate` gains `cache_id: u64`, and `Device` gains
  `shader_cache: Mutex<ShaderCache>`. That is the field the D3D12 `Device` has.
- `adapter.rs`: the device starts with an empty cache.
- `device.rs`:
  - The body that resolved the constants and wrote SPIR-V moved unchanged into
    `write_intermediate_stage`.
  - `compile_stage` looks the stage up first. A miss writes the stage and inserts it.
  - `create_shader_module` gives each intermediate module a fresh id.
  - `destroy_shader_module` drops that id's entries.

The Vulkan shader module is still created from the words and destroyed for each pipeline,
exactly as before. Only the naga work is skipped. No `unsafe` was added. The lock is
taken for the lookup and for the insert, never across naga work.

**The key differs from D3D12's, on purpose.** D3D12's expensive step is DXC/FXC, which runs
after naga, so #7729 keys on naga's output (the HLSL text) and still runs naga every time.
On Vulkan the driver's own shader cache makes `vkCreateShaderModule` cheap, and naga is the
cost. A key on the SPIR-V would save almost nothing, so this cache keys on naga's inputs:
- The module's id. It is a per-device counter, never an address, because a freed module's
  address can be reused. The id also covers the module's `runtime_checks` and debug source.
- The shader stage and the entry point.
- The override constants, sorted by name, with values compared by bit pattern.
- `zero_initialize_workgroup_memory`.
- The layout's binding map: the `(group, binding)` to Vulkan binding renumbering.

The device's writer options are fixed per device, and the cache lives on the device.
`ShaderCacheKey::new` destructures `ProgrammableStage` exhaustively, so a field that
`wgpu` adds there later fails to compile instead of silently missing from the key.

**The bound is D3D12's, with one fix.** Past 200 entries, only the 100 most recently used
entries stay. D3D12's counter advances only on a compile, so entries refreshed by hits all
share one stamp, and a prune after that can evict nothing: a working set that keeps hitting
its entries between misses grows the D3D12 cache without limit. Here every lookup, hit or
miss, advances the clock, stamps are unique, and the prune keeps entries strictly newer
than `now - 100`. A prune therefore always leaves at most 100 entries, and the cache never
holds more than 201. Because the key names a module by id rather than by content, a destroyed
module's entries also go at once, and the whole cache goes with the device. The Pulse PSP
race walk produces 126 distinct keys over its whole run. That count includes front-end
modules, whose entries go when they are freed.

**Inert elsewhere**: only the Vulkan backend changed. The browser build (`wasm32`, WebGPU)
does not link `wgpu-hal` at all: `cargo tree --target wasm32-unknown-unknown -p oag-game -i
wgpu-hal` prints nothing. Metal, Dx12 and GL compile unchanged code.

**Upstream, issue #8842** ("Split shader compilation into wgpu-shaders crate", open) would
move the naga calls out of `wgpu-hal`. That would move this cache with them rather than
conflict with it. The issue thread names no other Vulkan stage cache. In #7716,
`magcius` suggests "some cache keyed off the shader module (and codegen-relevant state)",
which is this key.

## How it is tested

- `oag-gpu`'s `shader_cache_tests` run in the gate, on every adapter the machine has.
  They build pipelines from one module with an `override` and read back what each one
  wrote. There are three cases:
  - two layouts that put the shader's binding 1 at different Vulkan bindings;
  - two override values on the same layout;
  - two modules alive at once with equal stages, which fails if module ids collide;
  - a module created after another was dropped.

  With `OAG_REQUIRE_GPU` set, the tests fail when no Vulkan device opens; without it they
  skip, as the crate's other GPU tests do. Each test fails when its part of the key is removed (checked by hand for the binding
  map, the constants and the module id).
- `vendor/wgpu-hal`'s own unit tests in `shader_cache.rs` cover key inequality, eviction by
  module (`forget_module` leaves no entry of the module), the prune leaving exactly 100,
  a hit-refreshed working set staying bounded, and a recently hit entry surviving a prune.
  The three bound tests fail on D3D12's counting. They run standalone, not in the gate:

  ```sh
  cp ~/.cargo/registry/src/index.crates.io-*/wgpu-hal-30.0.2/Cargo.lock vendor/wgpu-hal/
  cargo test --locked --manifest-path vendor/wgpu-hal/Cargo.toml --features vulkan --lib shader_cache
  ```

  The workspace's `exclude = ["vendor"]` is what lets the crate build on its own. Delete
  the copied lockfile afterwards.

## When to drop it

Drop the vendored crate, the `[patch.crates-io]` line and the `exclude` entry once a `wgpu`
release contains an equivalent Vulkan cache. When upgrading `wgpu` before then:

1. Copy the new `wgpu-hal` from the registry over `vendor/wgpu-hal`, in a commit of its own.
2. Re-apply this change: `git diff` from the commit that vendored the published crate to the
   last commit touching `vendor/wgpu-hal/src/vulkan/` is the patch.
3. Run `oag-gpu`'s `shader_cache_tests` and the measurement in `load-time.md`, "Seventh
   round".

If #8842 has moved `compile_stage` too far for the patch to apply, measure without it
first: a newer naga may already make the cost smaller.

## Upstream pull request text

The project has not opened it. This is the text for the maintainer to send to
<https://github.com/gfx-rs/wgpu>, with the diff of `vendor/wgpu-hal/src/vulkan/`:

> **[vulkan] add a shader cache to avoid re-running naga per pipeline**
>
> **Connections**
> The Vulkan counterpart of #7729, for #7716.
>
> **Description**
> A shader module with override constants stays IR in `wgpu-hal` until pipeline creation.
> So does any module under `Workarounds::SEPARATE_ENTRY_POINTS`. For every stage of every
> pipeline, `Device::compile_stage` runs `process_overrides` (with its revalidation) and
> the SPIR-V writer. Applications that build many pipelines from one module repeat that
> work for stages whose SPIR-V is identical. Examples are blend, depth or vertex-layout
> variants of one material, or the same shader with depth write on and off.
>
> This adds a per-device `ShaderCache` to the Vulkan backend, shaped like the D3D12 one
> from #7729:
> - It is a `Mutex<ShaderCache>` on `Device`.
> - Entries carry a `last_used` stamp.
> - Past 200 entries, only the 100 most recently used stay.
>
> One deliberate difference in counting: #7729 advances its counter only on a compile, so
> entries refreshed by hits share one stamp and a later prune can evict none of them, and
> a working set that keeps hitting between misses grows the D3D12 cache without limit.
> Here every lookup advances the counter and the prune keeps entries strictly newer than
> `now - 100`, so a prune leaves at most 100 entries and the cache never exceeds 201. The
> same fix would apply to the D3D12 cache.
>
> The key differs: on D3D12 the expensive step is DXC/FXC, after naga, so #7729 keys on the
> HLSL text. On Vulkan the expensive step is naga itself, so the key is naga's inputs:
> - a per-device module id (not an address);
> - the stage and the entry point;
> - the override constants, sorted and compared by bits;
> - `zero_initialize_workgroup_memory`;
> - the pipeline layout's binding map.
>
> Device-level writer options are fixed per device. Because the key names a module by id,
> `destroy_shader_module` drops that module's entries. The `vk::ShaderModule` is still
> created and destroyed per pipeline as before, so driver behaviour is unchanged. The lock
> is never held across naga work: two threads that miss on the same key both write it, and
> the second insert replaces equal words.
>
> The existing body moved verbatim into `write_intermediate_stage`. `ShaderCacheKey::new`
> destructures `ProgrammableStage` exhaustively, so a new field must be considered for the
> key.
>
> One application builds about 300 pipeline stages from a few modules, 126 of them
> distinct. On Linux with RADV and a warm Mesa shader cache (`perf stat -e
> instructions:u`):
> - the process's user instructions to its first race frame drop by 21 %;
> - `compile_stage` falls from 25 % to 11 % of a profile;
> - the output is pixel-identical.
>
> **Testing**
> - Unit tests for the key, eviction by module, the exact prune bound, and a hit-refreshed
>   working set staying bounded (these fail on the D3D12 counting).
> - An application-side test builds compute pipelines from one module in three cases:
>   - two layouts that renumber a binding differently;
>   - two override values;
>   - two live modules with equal stages;
>   - a new module after the old one is replaced.
>
>   It checks that each pipeline writes through its own bindings and its own constant.
>   Each case fails when its part of the key is removed.
>
> **Squash or Rebase?**
> Squash.
>
> `ShaderModule::Intermediate` gains a `cache_id` field. That is a breaking change for code
> that constructs the variant outside `wgpu-hal`, and nothing in the repository does that.

A related naga issue, separate from this PR: naga 30.0.2's `process_overrides` documents
its result as having "an empty `overrides` arena", but it restores the arena before
revalidating (`back/pipeline_constants.rs`). A caller that resolves first and hands over
the result is resolved again (`load-time.md`, "Sixth round").
