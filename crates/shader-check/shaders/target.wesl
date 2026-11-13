// What the render target holds, as the draws into it need to know it.

import package::colour::{srgb_decode, srgb_encode};

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
// the originals authored against, the square on 2048's (`square_decode`), or
// the piecewise sRGB curve on Wipeout HD's scene (`encoded_out`), whose texture
// units decode with it (docs/rendering/hd-track-tone.md, "The texture unit's
// decode is the sRGB curve").
public fn decode(rgb: vec3<f32>) -> vec3<f32> {
    if encoded_out > 0.5 {
        return srgb_decode(rgb);
    }
    return pow(rgb, vec3<f32>(select(2.2, 2.0, square_decode > 0.5)));
}

// The inverse of `decode`, for a value built in linear light that a later
// `decode` reads back: the two have to be one curve or the value does not
// survive the round trip.
public fn encode(rgb: vec3<f32>) -> vec3<f32> {
    let c = max(rgb, vec3<f32>(0.0));
    if encoded_out > 0.5 {
        return srgb_encode(c);
    }
    return pow(c, vec3<f32>(select(1.0 / 2.2, 0.5, square_decode > 0.5)));
}

// A gamma-authored colour as the target wants it: `amount` 1.0 decodes it to
// linear light with `decode`, 0.0 leaves it as it is. Callers pass
// `linear_out`, or `linear_out` scaled by a switch of their own.
public fn gamma_to_target(rgb: vec3<f32>, amount: f32) -> vec3<f32> {
    return mix(rgb, decode(rgb), amount);
}

// A colour of the scene's linear space back to gamma-authored: the inverse of
// `gamma_to_target` at `amount` (`encode` is `decode`'s inverse on every scene,
// the sRGB curve on Wipeout HD's included), what a mod's shader reads a scene
// picture as.
public fn target_to_gamma(rgb: vec3<f32>, amount: f32) -> vec3<f32> {
    return mix(rgb, encode(rgb), amount);
}

// 1.0 when the render target holds display (gamma-encoded) values while the
// draw computes in linear light: Wipeout HD's race scene, whose draws the
// console packs to sRGB before the blend (`SET_SHADER_PACKER` 1,
// docs/rendering/hd-track-tone.md, "HD's blends combine encoded values"), so a
// blended draw adds or mixes encoded values. The packer and the texture units'
// decode are one curve, the piecewise sRGB one, so this also selects it in
// `decode`. Only meaningful with `linear_out`; 0.0 everywhere else, which
// leaves every output as it was.
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

// 1.0 together with `packed_out` when the 5-6-5 card holds gamma-encoded values the
// way the PSP writes them: Wipeout Pulse's 128 x 128 advert and gantry card
// (`FRAMEBUFPIXFORMAT` 0 on target `0x154000` in a live GE dump, dither matrix zero), whose
// 8-bit result is truncated to 5, 6 and 5 bits and expanded back by replicating the high
// bits (docs/rendering/start-gantry.md, "Pulse's card is 5-6-5, truncated").
public override packed_gamma: f32 = 0.0;

// A gamma-encoded colour as the PSP's 5-6-5 target stores it: the 8-bit value truncated
// to its high bits and read back with the high bits replicated into the low ones.
public fn pack_565_gamma(encoded: vec3<f32>) -> vec3<f32> {
    let c = clamp(encoded, vec3<f32>(0.0), vec3<f32>(1.0));
    let eight = floor(c * 255.0 + vec3<f32>(0.5));
    let step = vec3<f32>(8.0, 4.0, 8.0);
    let kept = floor(eight / step);
    let top = vec3<f32>(4.0, 16.0, 4.0);
    return (kept * step + floor(kept / top)) / 255.0;
}

// A gamma-encoded colour rounded to the levels of a 5-6-5 target holding linear
// light. The decode is the sRGB curve the console's texture unit applies (a piecewise
// one, not the 2.2 power: against the live card it reproduces the red's mean level,
// 0.281 against the card's 0.267, where the power gives 0.139), and the value is stored
// encoded with the same curve, which the track quad's sampler decodes with, so the
// sampler returns the rounded level itself.
public fn pack_565(encoded: vec3<f32>) -> vec3<f32> {
    let levels = vec3<f32>(31.0, 63.0, 31.0);
    let c = clamp(encoded, vec3<f32>(0.0), vec3<f32>(1.0));
    let linear = srgb_decode(c);
    return srgb_encode(floor(linear * levels + vec3<f32>(0.5)) / levels);
}

// A linear-light colour as the target stores it: clamped and encoded with the
// sRGB curve the console's packer writes and the HD chain decodes with when
// `encoded_out` is set, as it is, and rounded to a 5-6-5 card's levels when
// `packed_out` is.
public fn target_out(rgb: vec3<f32>) -> vec3<f32> {
    if encoded_out > 0.5 {
        return srgb_encode(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)));
    }
    let packed = mix(pack_565(rgb), pack_565_gamma(rgb), packed_gamma);
    return mix(rgb, packed, packed_out);
}

// What a draw reads back from a scene-format target (the behind-glass grab),
// in linear light: the inverse of `target_out`.
public fn target_in(rgb: vec3<f32>) -> vec3<f32> {
    if encoded_out > 0.5 {
        return srgb_decode(clamp(rgb, vec3<f32>(0.0), vec3<f32>(1.0)));
    }
    return rgb;
}
