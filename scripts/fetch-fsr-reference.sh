#!/usr/bin/env bash
# Fetches AMD's FidelityFX SDK shader sources, which the FSR 1 and FSR 3.1
# ports under crates/post/src/ are transliterations of.
#
# **Into ~/.cache, never into the repository.** ADR-0012 chose a WGSL port over
# linking or vendoring the SDK, and the port's only defence against silently
# drifting from upstream is that it stays diffable against a *named* tag. So
# this fetches a reference to read, at a pinned commit, and nothing it writes is
# ever committed.
#
# v1.1.4 specifically: 1.1.x is the last line that is MIT source all the way
# down. SDK 2.x adds FSR4 as prebuilt signed binaries with no source, and
# nothing in this repository should point at a tree containing them. See
# docs/rendering/fsr3.md.
set -euo pipefail

TAG="v1.1.4"
COMMIT="c6efa6bf7f2027b3ec94f28578bb5965eabb9e55"
REPO="GPUOpen-LibrariesAndSDKs/FidelityFX-SDK"
DEST="${OAG_FSR_REFERENCE:-$HOME/.cache/oag-fsr}/$TAG"
RAW="https://raw.githubusercontent.com/$REPO/$COMMIT"

# Only the GPU-side sources and the one host file that fills the constant
# buffer. The rest of the SDK is a sample framework, its media, and backends
# for APIs this project does not use.
paths=(
    sdk/include/FidelityFX/gpu/ffx_common_types.h
    sdk/include/FidelityFX/gpu/ffx_core.h
    sdk/include/FidelityFX/gpu/ffx_core_cpu.h
    sdk/include/FidelityFX/gpu/ffx_core_glsl.h
    sdk/include/FidelityFX/gpu/ffx_core_gpu_common.h
    sdk/include/FidelityFX/gpu/ffx_core_gpu_common_half.h
    sdk/include/FidelityFX/gpu/ffx_core_hlsl.h
    sdk/include/FidelityFX/gpu/ffx_core_portability.h
    # FSR 1's own header: FSR 3.1's `rcas` pass is FSR 1's RCAS with
    # `FSR_RCAS_DENOISE` on, so the two ports share a lineage and the denoise
    # branch lives here rather than in the 3.1 tree.
    sdk/include/FidelityFX/gpu/fsr1/ffx_fsr1.h
    sdk/include/FidelityFX/gpu/fsr3upscaler/ffx_fsr3upscaler_accumulate.h
    sdk/include/FidelityFX/gpu/fsr3upscaler/ffx_fsr3upscaler_callbacks_glsl.h
    sdk/include/FidelityFX/gpu/fsr3upscaler/ffx_fsr3upscaler_callbacks_hlsl.h
    sdk/include/FidelityFX/gpu/fsr3upscaler/ffx_fsr3upscaler_common.h
    sdk/include/FidelityFX/gpu/fsr3upscaler/ffx_fsr3upscaler_debug_view.h
    sdk/include/FidelityFX/gpu/fsr3upscaler/ffx_fsr3upscaler_luma_instability.h
    sdk/include/FidelityFX/gpu/fsr3upscaler/ffx_fsr3upscaler_luma_pyramid.h
    sdk/include/FidelityFX/gpu/fsr3upscaler/ffx_fsr3upscaler_prepare_inputs.h
    sdk/include/FidelityFX/gpu/fsr3upscaler/ffx_fsr3upscaler_prepare_reactivity.h
    sdk/include/FidelityFX/gpu/fsr3upscaler/ffx_fsr3upscaler_rcas.h
    sdk/include/FidelityFX/gpu/fsr3upscaler/ffx_fsr3upscaler_reproject.h
    sdk/include/FidelityFX/gpu/fsr3upscaler/ffx_fsr3upscaler_resources.h
    sdk/include/FidelityFX/gpu/fsr3upscaler/ffx_fsr3upscaler_sample.h
    sdk/include/FidelityFX/gpu/fsr3upscaler/ffx_fsr3upscaler_shading_change.h
    sdk/include/FidelityFX/gpu/fsr3upscaler/ffx_fsr3upscaler_shading_change_pyramid.h
    sdk/include/FidelityFX/gpu/fsr3upscaler/ffx_fsr3upscaler_upsample.h
    sdk/include/FidelityFX/gpu/spd/ffx_spd.h
    sdk/include/FidelityFX/gpu/spd/ffx_spd_callbacks_hlsl.h
    sdk/include/FidelityFX/gpu/spd/ffx_spd_downsample.h
    sdk/include/FidelityFX/gpu/spd/ffx_spd_resources.h
    sdk/include/FidelityFX/host/ffx_fsr3upscaler.h
    sdk/src/backends/dx12/shaders/fsr3upscaler/ffx_fsr3upscaler_accumulate_pass.hlsl
    sdk/src/backends/dx12/shaders/fsr3upscaler/ffx_fsr3upscaler_luma_instability_pass.hlsl
    sdk/src/backends/dx12/shaders/fsr3upscaler/ffx_fsr3upscaler_luma_pyramid_pass.hlsl
    sdk/src/backends/dx12/shaders/fsr3upscaler/ffx_fsr3upscaler_prepare_inputs_pass.hlsl
    sdk/src/backends/dx12/shaders/fsr3upscaler/ffx_fsr3upscaler_prepare_reactivity_pass.hlsl
    sdk/src/backends/dx12/shaders/fsr3upscaler/ffx_fsr3upscaler_rcas_pass.hlsl
    sdk/src/backends/dx12/shaders/fsr3upscaler/ffx_fsr3upscaler_shading_change_pass.hlsl
    sdk/src/backends/dx12/shaders/fsr3upscaler/ffx_fsr3upscaler_shading_change_pyramid_pass.hlsl
    sdk/src/components/fsr3upscaler/ffx_fsr3upscaler.cpp
    sdk/src/components/fsr3upscaler/ffx_fsr3upscaler_private.h
)

echo "fetching FidelityFX-SDK $TAG ($COMMIT) into $DEST"
for path in "${paths[@]}"; do
    mkdir -p "$DEST/$(dirname "$path")"
    curl -fsSL -o "$DEST/$path" "$RAW/$path"
done
echo "${#paths[@]} files. Nothing here is committed - see docs/rendering/fsr3.md."
