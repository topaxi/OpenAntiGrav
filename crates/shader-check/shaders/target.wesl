// What the render target holds, as the draws into it need to know it.

// 1.0 when the render target holds linear light - Wipeout HD's float scene
// target, where the bloom gate reads pre-exposure luminance and the encode
// happens in `post::hd_bloom`'s own pass - and 0.0 for every gamma target,
// where a draw encodes (or never decodes) exactly as it always has. Set from
// the target format at pipeline build, see `mesh_render::is_linear_target`, so
// no uniform needs a new field and a zeroed scene buffer cannot miss it.
public override linear_out: f32 = 0.0;

// 1.0 when the linear scene is Wipeout 2048's gamma-2 space: its lit programs
// decode a texel by squaring it and the race composite's square root is the
// encode (docs/ghidra/functions/vita-2048-eu-v104/exposure.md). Only meaningful
// with `linear_out`; 0.0 everywhere else, which keeps the 2.2 power.
public override square_decode: f32 = 0.0;

// A gamma-authored colour decoded to the scene's linear space: the 2.2 power
// the originals authored against, or the square on 2048's (`square_decode`).
public fn decode(rgb: vec3<f32>) -> vec3<f32> {
    return pow(rgb, vec3<f32>(select(2.2, 2.0, square_decode > 0.5)));
}

// A gamma-authored colour as the target wants it: `amount` 1.0 decodes it to
// linear light with `decode`, 0.0 leaves it as it is. Callers pass
// `linear_out`, or `linear_out` scaled by a switch of their own.
public fn gamma_to_target(rgb: vec3<f32>, amount: f32) -> vec3<f32> {
    return mix(rgb, decode(rgb), amount);
}

// 1.0 when the render target holds display (gamma-encoded) values while the
// draw computes in linear light: Wipeout HD's race scene, whose draws the
// console packs to sRGB before the blend (`SET_SHADER_PACKER` 1,
// docs/rendering/hd-track-tone.md, "HD's blends combine encoded values"), so a
// blended draw adds or mixes encoded values. Only meaningful with
// `linear_out`; 0.0 everywhere else, which leaves every output as it was.
public override encoded_out: f32 = 0.0;

// 1.0 when the render target is a 5-6-5 colour card holding linear light:
// Wipeout HD's 512 x 256 advert and gantry cards (`R5G6B5`, read live off RPCS3;
// docs/rendering/start-gantry.md, "The card is 5-6-5 in linear light"). The
// console writes the program's linear result to the card rounded to 31, 63 and 31
// levels, and the track quad that shows it samples those levels. The card here is
// an 8-bit gamma target, so the value is quantised in linear light and stored
// encoded; the sampler decodes it back to the same levels. Only meaningful on a
// gamma target; 0.0 everywhere else, which leaves every output as it was.
public override packed_out: f32 = 0.0;

// A gamma-encoded colour rounded to the levels of a 5-6-5 target holding linear
// light. The decode is the sRGB curve the console's texture unit applies (a piecewise
// one, not the 2.2 power: against the live card it reproduces the red's mean level,
// 0.281 against the card's 0.267, where the power gives 0.139), and the value is stored
// encoded with the 2.2 power the sampler decodes with, so the sampler returns the rounded
// level itself.
public fn pack_565(encoded: vec3<f32>) -> vec3<f32> {
    let levels = vec3<f32>(31.0, 63.0, 31.0);
    let c = clamp(encoded, vec3<f32>(0.0), vec3<f32>(1.0));
    let linear = select(
        pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4)),
        c / 12.92,
        c <= vec3<f32>(0.04045),
    );
    return pow(floor(linear * levels + vec3<f32>(0.5)) / levels, vec3<f32>(1.0 / 2.2));
}

// A linear-light colour as the target stores it: clamped and encoded with the
// 2.2 power the HD chain decodes with when `encoded_out` is set, as it is, and
// rounded to a 5-6-5 card's levels when `packed_out` is.
public fn target_out(rgb: vec3<f32>) -> vec3<f32> {
    let stored = mix(rgb, pow(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2)), encoded_out);
    return mix(stored, pack_565(stored), packed_out);
}

// What a draw reads back from a scene-format target (the behind-glass grab),
// in linear light: the inverse of `target_out`.
public fn target_in(rgb: vec3<f32>) -> vec3<f32> {
    return mix(rgb, pow(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(2.2)), encoded_out);
}
