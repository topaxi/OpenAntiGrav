// What the render target holds, as the draws into it need to know it.

// 1.0 when the render target holds linear light - Wipeout HD's float scene
// target, where the bloom gate reads pre-exposure luminance and the encode
// happens in `post::hd_bloom`'s own pass - and 0.0 for every gamma target,
// where a draw encodes (or never decodes) exactly as it always has. Set from
// the target format at pipeline build, see `mesh_render::is_linear_target`, so
// no uniform needs a new field and a zeroed scene buffer cannot miss it.
public override linear_out: f32 = 0.0;

// A gamma-authored colour as the target wants it: `amount` 1.0 decodes it to
// linear light with the 2.2 power the originals authored against, 0.0 leaves
// it as it is. Callers pass `linear_out`, or `linear_out` scaled by a switch of
// their own.
public fn gamma_to_target(rgb: vec3<f32>, amount: f32) -> vec3<f32> {
    return mix(rgb, pow(rgb, vec3<f32>(2.2)), amount);
}
