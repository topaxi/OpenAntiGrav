// `lit_texel`, the one fragment shading function every entry point calls.

import package::absorb::{absorb_shading};
import package::bindings::{albedo, albedo_sampler, emissives, lightmap, pad_mask, scene, uniforms, wave_map};
import package::flame::{flame_alpha_scale, flame_colour_scale, flame_rim_min, flame_rim_power, flame_rim_scale, flame_shading, flame_speed};
import package::pad::{pad_normal};
import oag_shaders::target::linear_out;
import package::shading_modes::{colour_is_light, nova_prelit};
import package::shadow::{receives_shadow, sun_occlusion};
import package::texture_lod::{sample_at_slope_level, texlod_bias, texlod_slope};
import package::types::{VertexOutput};
import package::zone::{zone_base_term, zone_glow, zone_sample, zone_set};

// **A branch on `slots` keeps its derivatives.** `slots` is a flat varying,
// and a 2x2 fragment quad - helper lanes included - is always shaded for one
// primitive, so every lane of a quad reads the same flat value and takes the
// same side of the branch. An implicit-derivative fetch inside it therefore
// has the neighbours it needs. WGSL's analysis cannot see that about a
// varying, so it is told here.
@diagnostic(off, derivative_uniformity)
fn lit_texel(in: VertexOutput) -> vec4<f32> {
    let n = normalize(in.normal);
    // **HD's pads** (`slots::PAD_NE`): the `_ne` texel replaces the vertex
    // normal in the program's own `N.L` and `N.H` below, and its alpha gates
    // the glow added after the light. `n` itself stays the vertex normal for
    // everything else this function reads it for.
    let pad_ne = (in.slots & 16384u) != 0u;
    var pad_ne_texel = vec4<f32>(0.0);
    var n_lit = n;
    if pad_ne {
        pad_ne_texel = textureSample(pad_mask, albedo_sampler, in.texcoord);
        n_lit = pad_normal(pad_ne_texel, n, in.world, in.texcoord);
    }

    // The stand-in: two invented directions, kept for every title whose own
    // rig has not been recovered. See this file's header.
    let key = max(dot(n, normalize(vec3<f32>(0.4, 0.8, 0.5))), 0.0);
    let fill = max(dot(n, normalize(vec3<f32>(-0.5, 0.2, -0.7))), 0.0);
    let stand_in = vec3<f32>(0.15 + 0.75 * key + 0.25 * fill);

    let ndl = clamp(dot(n_lit, scene.light.direction), 0.0, 1.0);

    // **Everything from here to `authored` is the authored rig's, and only
    // runs when the rig is on.** Off - `Light::stand_in`, which is every
    // circuit with no `.envsettings` to read, Pulse's and Pure's included -
    // the output mixes this whole path in at weight zero, and it was costing
    // a lightmap fetch, the sun-occlusion lookup and half a dozen `pow`s a
    // pixel for nothing: half the race pass on an integrated GPU.
    // `enabled` is a uniform, so the fetches keep their derivatives.

    // **The circuit's own baked lighting.** A material naming an
    // `lmaps/*-lmap.gtf` in its second texture slot is drawn through it,
    // sampled at the `lightmapUV` the chunk's vertex declaration names - see
    // `docs/formats/rcsmaterial.md`. Every draw without one binds a black,
    // alpha-1 placeholder, which zeroes the prelit term and passes the sun
    // below - exactly the equation the original's own lightmap-less shader
    // variants state.
    // **Only when the second texture actually is one.** Binding the second
    // slot unconditionally is what let its other roles be read at all, and it
    // is also how the prelit term could silently start sampling a cloud mask:
    // a non-lightmap chunk declares no `lightmapUV`, so this would read one
    // texel at (0, 0) of whatever is bound instead of the black placeholder
    // that switches the term off. `slots::SECOND_IS_LIGHTMAP` is the material's
    // own four-signal reading; without it the value is the placeholder's, to
    // the bit.
    var authored = vec3<f32>(0.0);
    var mask = 0.0;
    var emissive = false;
    if scene.light.enabled != 0.0 {
        let atlas = textureSample(lightmap, albedo_sampler, in.lightmap_texcoord);
        let baked = select(
            vec4<f32>(0.0, 0.0, 0.0, 1.0),
            atlas,
            (in.slots & 1u) != 0u,
        );

        // The authored rig, and the combination is no longer this project's: it is
        // the one every lit variant of an HD circuit `.rcsmaterial` computes,
        // read out of the fragment microcode
        // (docs/ghidra/functions/ps3-hdfury-eu/renderer.md, "The lit track
        // material"). The lightmap enters through a power curve - `Prelit ambient
        // colour scale/power`, the .envsettings names - and is *added* to a
        // constant, not multiplied by one:
        //
        //     block #9, lightmapped:  pow(lightmap, power) * scale + f[TC1]
        //     block #8, no lightmap:  f[TC1] + constantAmbientColour
        //
        // and that sum multiplies the albedo. **Neither block has an `N.L` or a
        // sun colour of its own** - `track_surface`'s 15 chunks of Talon's
        // Junction compute exactly this and nothing more.
        //
        // `scene.light.ambient` is `constantAmbientColour` - the preimage of the
        // hash the microcode patches into block #8's `{const}`. `f[TC1]` is HD's
        // per-vertex light, which `mesh/rcs.rs` decodes out of the colour set and
        // hands over in `in.colour.rgb`; it is **added**, as both variants add
        // it, and is zero on a chunk that declares no colour set - which is what
        // the original's own vertex programs broadcast there. Note `f[TC1]` is
        // that material's interpolator and not a convention: the same closer
        // arrives on `f[TC0]` in `talons_junction/bluemetal`, so a per-material
        // path will have to read which varying carries it.
        // **Raw, no sRGB decode (2026-10-05).** Block #9 is `TEX` then
        // `LG2`/`MUL`/`EX2` then `MAD` by the scale, and the RSX decodes no texture
        // on the disc (renderer.md, "HD's lightmap is read raw"), so the curve
        // runs on the stored byte, exactly as Omega's does below. The `pow(_,
        // 2.2)` that stood here was ADR-0026's decode of every sample and was
        // the darkness: on Amphiseum (power 3.5) it compounded to a 7.7 power.
        let baked_linear = baked.rgb;
        // **Wipeout: Omega Collection's combination, on a lightmapped draw.** Read
        // out of its circuit pixel shaders (ps4-omega-eu/lightmap-prelit.md):
        // `v_log_f32` / `v_mul_f32 power` / `v_exp_f32` / `v_mad_f32 scale, bias` on
        // the *raw* atlas - its descriptor is BC7 UNORM on all 1,028 atlases, so
        // there is no sRGB decode in front of the curve - and no constant ambient
        // joins the sum (0 of 4,664 nova shaders declare one). `scale`, `bias` and
        // `power` are the authored triple's three scalars. Gated on the lightmap
        // bit: a draw with no lightmap binds the black placeholder, and the bias
        // must not light it.
        let nova = nova_prelit != 0.0 && scene.light.nova > 0.5 && (in.slots & 1u) != 0u;
        let prelit_hd = scene.light.prelit_scale * pow(baked_linear, scene.light.prelit_power);
        let prelit_nova = scene.light.prelit_scale * pow(baked.rgb, scene.light.prelit_power)
            + vec3<f32>(scene.light.prelit_bias);
        let prelit = select(prelit_hd, prelit_nova, nova);
        // `select` rather than a multiply by the override, so the value is not
        // touched at all where the flag is set. (Measured: it makes no difference
        // to the frame either way - both forms move the same 9 pixels of 1,175,040
        // by one level, and that movement is the `tint` line below rather than this
        // one. Kept because not multiplying is the clearer statement.)
        let vertex_light = select(vec3<f32>(0.0), in.colour.rgb, colour_is_light > 0.5);

        // `slots::NO_AMBIENT`/`slots::NO_SUN`, bits 6/7 - read here, ahead of
        // `emissive`'s own paragraph below, because the ambient fix that follows
        // needs `no_ambient` before that point and this keeps the two bit reads
        // from drifting apart.
        let no_ambient = (in.slots & 64u) != 0u;
        let no_sun = (in.slots & 128u) != 0u;
        emissive = no_ambient && no_sun;

        // **The disc's own compiled shader table carries three ambient sources
        // per chunk, not one** - see
        // docs/ghidra/functions/ps3-hdfury-eu/renderer.md, "The per-material
        // microcode sweep: none of the four ceiling programs ever references
        // the ambient constant...". Selected by `Features::chunk_word`:
        // `IleLightmap` -> the `prelit` curve above (already this chunk's only
        // term, untouched here); `IleVertex` -> the same curve shape applied to
        // the baked per-vertex colour set instead of the lightmap texel -
        // `pow(colour_set, prelitBias) * prelitScaleSpecular`; `Ambient`
        // (neither) -> flat `constantAmbientColour`, i.e. `scene.light.ambient`
        // unmodified. `slots::NO_AMBIENT` is exactly "this chunk's own program
        // is not the `Ambient` case" - confirmed on every chunk measured on
        // Amphiseum, Anulpha Pass and Talon's Junction. Dropping
        // `scene.light.ambient` alone on these chunks regresses an
        // `IleVertex`/no-lightmap/sun-occluded chunk to near-black, because the
        // raw (uncurved) vertex colour this project already wires as
        // `vertex_light` is far dimmer than the curved value - so the drop and
        // the curve are one change, not two.
        //
        // `prelitBias`/`prelitScaleSpecular` resolve to the same two
        // `.envsettings` keys already bound above as `prelit_scale`/
        // `prelit_power`: `Scene_PrepareFrame` binds `Prelit ambient colour
        // scale`/`power` into the per-draw shader-parameter table at offsets
        // `+0x198`/`+0x1b8` (renderer.md, "`Constant ambient color` (`+0x420`)
        // is confirmed wired..."), and the engine's own 81-entry parameter
        // table uses `offset = 0x18 + slot * 0x20` (renderer.md, "The engine's
        // own parameter table") - solving that for `0x198`/`0x1b8` gives slots
        // 12/13, which the same table names `prelitScaleSpecular`/`prelitBias`.
        // Two independently-traced offset chains landing on the same slot
        // indices, not a name-string or value/shape match alone.
        //
        // No sRGB predecode here, unlike `prelit` above: the colour-set vertex
        // program reads `v[3]` raw (`LG2 -> MUL -> EX2 -> MUL`), not through the
        // lightmap's `pow(_, 2.2)` step. `EMISSIVE` chunks (`no_ambient &&
        // no_sun`) are excluded from the curve below - this is additive to that
        // branch, not a rewrite of it.
        let vertex_light_curved = scene.light.prelit_scale
            * pow(vertex_light, scene.light.prelit_power);
        let vertex_light_term = select(vertex_light, vertex_light_curved, no_ambient && !emissive);
        let ambient_term = select(scene.light.ambient, vec3<f32>(0.0), no_ambient || nova);

        // **The sun-occlusion mask, restored 2026-08-20.** Two independently
        // decoded carriers of the same scalar: a lightmapped chunk's shadow lives
        // in its lightmap's own alpha, and a vertex-lit chunk's lives in its
        // colour set's fourth byte - `in.sun_mask`, from
        // `oag_formats::rcsmodel::Mesh::vertex_light`. They never both carry real
        // data (a chunk bakes into the lightmap atlas *or* its vertices, never
        // both - see `mesh/rcs.rs`), and each side's absence is `1.0`
        // (unmasked): the no-lightmap placeholder's alpha, and `sun_mask`'s own
        // default for a chunk with no colour set. So the product reads whichever
        // side is real and is the identity where neither is.
        //
        // **A prior version of this comment said no sun term belonged here at
        // all**, because the general block #8/#9 formula above has none. That
        // was the read on two materials; seven read variant-by-variant on
        // 2026-08-20 refute it. `diffuse_with_specular_from_alpha` (86 chunks of
        // Talon's Junction), `..._scalar` (80), `diffusewithalphachannel` (43),
        // `track_wall` (33) and `glasstest` (33) - **275 of Talon's Junction's
        // 301 drawn materials, against `track_surface`'s 15** - each normalise
        // their interpolated world normal, dot it against the sun direction,
        // multiply by the sun colour, gate it by this same mask, add the prelit
        // term and multiply the sum into the albedo. So this is the rule and
        // `track_surface` the exception, applied to every chunk alike for want of
        // the per-material branch that would tell the two apart - a stand-in of
        // the same shape as the shared specular exponent below, and the
        // 15-of-301 minority it is wrong for reads unlit rather than lit, which
        // this project has not yet measured against the frame.
        //
        // **What this replaces.** A `sun * (ndl * baked.a)` summand once stood
        // here with `baked.a` fixed at the no-lightmap placeholder's `1.0` -
        // full sun, unoccluded, everywhere - measured at 14.3 % of the frame
        // clipped to white against the reference's 5.8 %, worse than the sun
        // term being absent entirely, which is why it was removed rather than
        // left wrong. `mesh/rcs.rs` used to take only `.rgb` from the colour set
        // and this mask did not exist yet.
        // **Times the road's mask under a craft**, `sun_occlusion` above: 1.0 on
        // every draw that is not a hull with a map, so nothing else here moves.
        // **Not even called on the track's pipelines** (`receives_shadow` 2,
        // `ShadowReceiver::Both`, which is the track and the gantry and never
        // a hull): its early return was already taken there, but the
        // function's four-tap loop held registers for it, 0.25 ms of HD's
        // 22.5 ms opaque track at 3200x1800 on the Raphael iGPU. Only a hull
        // names a layer, and a hull is `ShadowReceiver::Mapped`.
        mask = baked.a * in.sun_mask;
        if receives_shadow != 2.0 {
            mask = mask * sun_occlusion(in.world);
        }
        let sun_diffuse = scene.light.sun * (ndl * mask);
        // **`scene.light.ambient` and `vertex_light` are gated above** (`ambient_term`,
        // `vertex_light_term`), on `slots::NO_AMBIENT` - 251 of Anulpha Pass's 309
        // drawn materials, 922 of its 1,101 chunks. Gating `scene.light.ambient`
        // on that bit *alone* was tried on 2026-08-24 and was a regression: the
        // materials without an ambient are three families, not one, and
        // `sign_emissive` and its kin (declaring only `fogColour`) went black
        // when multiplied by an `authored` with the ambient simply dropped. The
        // fix above is the drop *and* the vertex-colour curve together, which is
        // what keeps the `IleLightmap`/`IleVertex` families lit; `EMISSIVE`
        // below is the third family, excluded from both terms the same way it
        // always was.
        //
        // **A program fed no scene light is emissive, and the rig must not touch
        // it.** `slots::EMISSIVE` is the material's own declaration: neither
        // `constantAmbientColour` nor `directionalLight0*`. On Anulpha Pass that
        // is 33 materials over 95 chunks and on Talon's Junction 35 over 151, and
        // every name in both is a sign or a glow - `sign_emissive`, `cf_glow_tube`,
        // `cf_plasma_glow2`, `dc_lightcone`, `scanlinebillboard` - which is a
        // confirmation the split was not designed for.
        //
        // Multiplying by `1.0` rather than adding anything: the albedo *is* the
        // picture for these, exactly as the microcode leaves it. The specular goes
        // with it, because a program with no sun has no half-vector term either.
        // **Plus Wipeout HD's SPU vertex lights**, which the `SVC1` fragment
        // programs add in exactly this slot - the pre-albedo diffuse sum beside
        // the ambient, the sun and the lightmap, never the specular, the fog or
        // the emissive add (renderer.md, "The `SVC1` combine is read"). Zero on
        // every draw that binds no list, and on an `EMISSIVE` chunk the whole
        // sum is replaced below, which is the original's own no-op there.
        let lit_sum = ambient_term + prelit + vertex_light_term + sun_diffuse + in.spu_light;
        authored = select(lit_sum, vec3<f32>(1.0), emissive);
    }

    // **Which texture is the picture and which is the coverage, off the
    // material's own microcode** - see `oag_mesh::mesh::slots` and
    // `docs/formats/rcsmaterial.md`. The common case is `slots::DEFAULT`:
    // colour and alpha both from the first texture, which is what every title
    // but Wipeout HD carries and what an HD material whose program this
    // reading could not follow keeps.
    //
    // The second texture is sampled at the **diffuse** coordinate, which is a
    // stated approximation rather than a reading: the cloud plate's own
    // program addresses both units from coordinates it builds out of the same
    // interpolator, and a chunk in this role declares no second coordinate set
    // for us to use instead. A material that tiles its two textures
    // differently would come out wrong here and would show as a mismatched
    // scale rather than as a missing surface.
    // **`slots::FACING_RAMP_SHEEN`: the glass family's facing-ramp combine,
    // fully traced on one material - see `oag_mesh::mesh::rcs::glass_sheen`
    // and `docs/formats/rcsmaterial.md`, "Three sampler roles identified by
    // what they bind, and the glass floor stops painting a ramp".** Where
    // this bit is set, `mesh::rcs::skin::picks` has already pointed `albedo`
    // at the material's facing-ramp texture and `lightmap` at its grid, so
    // the only thing left to do here is address the ramp by `dot(V, N)`
    // rather than the diffuse UV every other material samples it at - the
    // view-angle sheen the traced microcode computes
    // (`DP3 R2.w, -R1, R2` then `TEX H2.xyz, -R2.wwww unit2`).
    let ramp_sheen = (in.slots & 1024u) != 0u;
    let view_dir = normalize(scene.fog.camera - in.world);
    let ramp_facing = dot(view_dir, n);
    let ramp_uv = vec2<f32>(ramp_facing, ramp_facing);
    var first_uv = select(in.texcoord, ramp_uv, ramp_sheen);
    // **The Plasma explosion's ring and halo scroll on the model's own clock**
    // (`slots::CLOCK_SCROLL_RING`/`_HALO`): their programs' `UV_offset` is
    // `node + 0xc0`, the blast's age, not the scene's `time`. Read off the
    // two programs' own microcode (docs/ghidra/functions/ps3-hdfury-eu/
    // plasma.md, 2026-10-05) - ring: `n = tex(u, v + 0.01 t).a`, then
    // `uv + 0.15 n + 0.1 t` on both axes; halo: `n = tex(u, v + 0.4 t).a`,
    // then `(2u + 0.1 t, v + 0.14 t + 0.1 n)`. **Only where the colour tap
    // samples**; the programs' own colour and alpha combine is not
    // reproduced here, the path below stands in for it.
    let clock = uniforms.model_clock;
    if (in.slots & 32768u) != 0u {
        let n = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(in.texcoord.x, in.texcoord.y + 0.01 * clock),
        ).a;
        first_uv = in.texcoord + vec2<f32>(0.15 * n + 0.1 * clock);
    } else if (in.slots & 65536u) != 0u {
        let n = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(in.texcoord.x, in.texcoord.y + 0.4 * clock),
        ).a;
        first_uv = vec2<f32>(
            2.0 * in.texcoord.x + 0.1 * clock,
            in.texcoord.y + 0.04 * clock + 0.1 * n + 0.1 * clock,
        );
    }
    // **Pulse's slope-mode level selection**: `log2(|z| * slope) + bias` off view
    // depth, clamped to the levels the texture has - the GE's rule, in place of
    // the derivatives `textureSample` would use. On only for a model carrying
    // the disc's own chains; see `mesh_render::PSP_TEXLOD_SLOPE`. The player's
    // TEXTURE DETAIL shifts the level, which is one multiplier on the law.
    var first: vec4<f32>;
    if texlod_slope > 0.0 {
        let lod = max(
            log2(max(in.view_depth * texlod_slope, 1.0e-6)) + texlod_bias + scene.fog.texlod_shift,
            0.0,
        );
        first = sample_at_slope_level(first_uv, lod);
    } else {
        first = textureSample(albedo, albedo_sampler, first_uv);
    }
    let second = textureSample(lightmap, albedo_sampler, in.texcoord);
    let picture = select(first, second, (in.slots & 2u) != 0u);
    let coverage = select(first, second, (in.slots & 4u) != 0u);
    let texel = vec4<f32>(picture.rgb, coverage[(in.slots >> 3u) & 3u]);

    // **The combine itself**, `vertexLight * ramp + ramp`, plus the grid's
    // red as an additive term and as the output alpha - block #7's own
    // `ADD H4.xyz, H6.xxxx, H4` and `MOV H0.w, H6.xxxx END`. Two omissions,
    // both named rather than guessed shut:
    //
    // - **The per-material constant `c`** (`ADD H4.xyz, H2, {c}` -
    //   parameter `0x512f8e65`, measured `0.26562` on `etched_glass_tech`,
    //   not zero) is left out of `ramp + c` - `glass_sheen`'s own doc names
    //   the plumbing this needs and why it did not land in this change.
    // - **`paraboloidReflectionTex`'s reflection tint**, weighted by the same
    //   grid red, is left out entirely: this renderer has no dual-paraboloid
    //   probe and does not invent one, per `CLAUDE.md`.
    //
    // `vertexLight` is `f[TC0] + f[TC1] + f[TC5].x * (sun.colour * N.L)` in
    // the traced microcode (`@0x02`/`@0x05`: `DP3_SAT` against the sun
    // direction, `MUL` by the sun colour - exactly `scene.light.sun * ndl`
    // this file already computes as `sun_diffuse` without the mask, since
    // this program declares no lightmap to mask by). `f[TC5].x` is
    // `in.texcoord`'s own first component (line 830 of `rcsmaterial.md`
    // reads `f[TC5].zw` as this file's `in.texcoord`). `f[TC0]` and `f[TC1]`
    // are a stated approximation, not a reading: both stand in as
    // `in.colour.rgb`, the one per-vertex light term this project already
    // decodes for every HD material, on the same terms this file's own
    // second-texture coordinate above is already a stated approximation
    // rather than a reading.
    let ramp_sun = scene.light.sun * ndl;
    let vertex_light_sheen = in.colour.rgb + in.texcoord.x * ramp_sun;
    let sheen_rgb = vertex_light_sheen * first.rgb + first.rgb + vec3<f32>(second.r);
    let sheen = vec4<f32>(sheen_rgb, second.r * in.colour.a);

    // **Wipeout HD's additive glow.** Its emissive family samples unit 1 at
    // `(u, (v + a) * b + time)`, multiplies by a tint and adds the result to
    // the albedo gated by the diffuse alpha:
    //
    //     MAD H0.xyz, H0.wwww, H1, H0
    //
    // `slots::ADD_SECOND` says a material does this and its index selects the
    // three constants. Every other surface indexes slot 0, whose tint is zero,
    // so this costs one sample and no branch - the same shape the two anim
    // tables take. See `docs/formats/rcsmaterial.md` for the disassembly, and
    // `oag_mesh::mesh::rcs::emissive` for which materials qualify.
    //
    // **The `u` is the diffuse coordinate, which is a stated approximation and
    // not a reading**, exactly as the second texture's own sampling above
    // already is: the program addresses unit 1 from `f[TC3]`, and whether the
    // UV set feeding that interpolator is the one this renderer carries as
    // `texcoord` is unestablished. Only `v` moves, so a mismatch shows as a
    // glow tiled wrongly across the surface rather than as a missing one.
    let glow_slot = in.slots >> 20u;
    let glow_tint_offset = emissives.tint_offset[glow_slot];
    let glow_scale = emissives.scale[glow_slot];
    // **The clock is gated, and `b` defaults to 1 rather than 0.** A material
    // that adds its second texture is not necessarily one that scrolls it:
    // some read `time` while keeping their coordinate constants as the
    // shader's own inline literals, which this reading has not recovered, and
    // some never read it at all. A `b` of 0 with the clock ungated is the
    // worst of both - it annihilates the surface's own `v` and marches one row
    // of the texture across it. See `oag_mesh::mesh::Emissive::rate`.
    let glow_v = (in.texcoord.y + glow_tint_offset.w) * glow_scale.x
        + scene.time.x * glow_scale.y;
    // Fetched only where `ADD_SECOND` is set: everywhere else `glow_gate`
    // below is zero and the sample is multiplied away. A branch on `slots`,
    // which `lit_texel`'s own attribute explains.
    var glow_sample = vec4<f32>(0.0);
    if (in.slots & 256u) != 0u {
        glow_sample = textureSample(
            lightmap,
            albedo_sampler,
            vec2<f32>(in.texcoord.x, glow_v),
        );
    }
    // Gated by the *diffuse* alpha, which is `first.a` rather than the
    // resolved `texel.a`: the microcode's `H0.wwww` is the unit-0 fetch's own
    // fourth channel, before any of the role bits above choose where the
    // output's alpha comes from.
    let glow_gate = first.a * select(0.0, 1.0, (in.slots & 256u) != 0u);
    let glow = glow_sample.rgb * glow_tint_offset.rgb * glow_gate;
    // **The same layer in the linear domain, for the authored path**, which
    // adds it below. The sample is sRGB-decoded and the tint is not, on the
    // terms the albedo already sets on that path: `glow_sample` is a picture -
    // a sign, a tube, a light cone - authored in the space every other texture
    // on the disc is, and `glow_tint_offset.rgb` is a shader constant, an
    // authored magnitude like `light.sun`. `first.a` is an alpha, and alphas
    // never decode.
    //
    // **Neither domain reproduces the RSX**, which multiplies raw 8-bit
    // throughout and sRGB-decodes nothing - renderer.md, "Per-texture
    // sRGB/`GAMMA` decode: settled negative". What a domain can preserve is
    // the glow's magnitude *relative to the albedo it is added to*, and only
    // the decoded form does that: undecoded, a 0.5 sample lands at 0.5 beside
    // an albedo that decoded to 0.22, so the layer arrives at about twice the
    // weight the disc gives it. Both were measured - `scripts/hd-glow-sweep.py`
    // and the table in renderer.md - and the principle rather than the
    // brightness is what chose.
    let glow_linear =
        pow(glow_sample.rgb, vec3<f32>(2.2)) * glow_tint_offset.rgb * glow_gate;
    // **HD's pad light bars**: `_ne.a * colour`, the program's `MAD H.xyz,
    // R0.wwww, C, lit` - added after the light on the far side of the albedo
    // multiply, ungated by the diffuse alpha, and not decoded in either
    // domain (an alpha and an authored magnitude). The colour is the pad's
    // own authored `W_Cycle`/`Colour`, riding in this material's glow-table
    // tint; zero for every other material. Fog follows in `fogged`, which is
    // where the program's own final `MAD` puts it too.
    let pad_glow = pad_ne_texel.a * glow_tint_offset.rgb * select(0.0, 1.0, pad_ne);

    // **HD's magstrip wave** (`slots::MAG_WAVE`, `oag_mesh::mesh::rcs::mag_wave`):
    //
    //     m = lerp(e.rgb, wave(uv * k + time), e.a)
    //     d = e.rgb / (1 - m * Colour)
    //
    // `e` is the emissive picture (the third binding), `k` the glow-table
    // entry's scale, `Colour` its tint, and the clock is added to both axes with
    // no multiplier - read off the microcode, `mag_wave`'s doc has the listing.
    // The wave texture repeats, so the band sweeps once a second.
    //
    // **Chosen, not measured: the denominator is floored at 0.05.** `Colour.g`
    // is 1 on Talon's Junction and the wave's bright band reaches 1, so the
    // quotient diverges, and a black emissive texel makes it `0 / 0`. The RSX
    // writes 8 bits and saturates; this floors the divisor and clamps to 1 so
    // no inf or NaN reaches the bloom's targets. The dodge is also taken on the
    // sampled (sRGB-decoded) texels rather than raw bytes, the same domain
    // choice the glow above makes.
    //
    // **Not in a Zone race.** Every `ZoneMode` variant of the wave family
    // (`mageffect08`, `mageffect08_floor`, `mageffectloop`,
    // `mag_effect_loop_opaque`, `chevron_pulse`, `mageffect_modded`) declares no
    // wave sampler, no emissive picture, no `time` and no iridescent ramp (every one, all four archives -
    // `hd_zone_wave_census_ground_truth`), so a Zone frame draws
    // these surfaces as the ordinary Zone surface and this term, and the floor
    // combine below, stay off. The condition is a branch and not a mix so the
    // Time Trial picture is untouched to the byte.
    var mag_glow = vec3<f32>(0.0);
    if (in.slots & 131072u) != 0u && scene.zone.enabled == 0.0 {
        let e = textureSample(pad_mask, albedo_sampler, in.texcoord);
        let w = textureSample(
            wave_map,
            albedo_sampler,
            in.texcoord * glow_scale.x + vec2<f32>(scene.time.x * glow_scale.y),
        );
        let m = mix(e.rgb, w.rgb, e.a);
        mag_glow = min(
            e.rgb / max(vec3<f32>(1.0) - m * glow_tint_offset.rgb, vec3<f32>(0.05)),
            vec3<f32>(1.0),
        );
    }
    let mag_glow_gamma = pow(mag_glow, vec3<f32>(1.0 / 2.2));

    // The Zone surface, in both domains, from one sample. See `zone_sample`.
    //
    // `rim` is the microcode's own `1 - dot(N, -V)`, where its `-V` points
    // from the surface back at the eye - so this is 0 dead-on and 1 at the
    // silhouette, and the two summands are silhouette-weighted. The eye comes
    // out of the fog block, the one slot in bind group 2 that carries a
    // position; the specular term below reads it from there too.
    //
    // **Branched, not just weighted.** Every reader below mixes these in by
    // `enabled`, so off a Zone race they contribute nothing - but computing
    // them anyway costs nine texture fetches a pixel, about a fifth of the
    // race pass on an integrated GPU (`OAG_RENDER_GPU_BENCH`). `enabled` is a
    // uniform, so the branch is uniform control flow and the fetches inside it
    // keep their implicit derivatives. Zero is what the mixes already reduced
    // these to, so the picture is unchanged to the byte.
    var zone_linear = vec3<f32>(0.0);
    var zone_gamma = vec3<f32>(0.0);
    var zone_glow_term = vec3<f32>(0.0);
    if scene.zone.enabled != 0.0 {
        let zone_to_eye = normalize(scene.fog.camera - in.world);
        let zone_colours = zone_set(in.slots, in.world);
        let zone_base = zone_base_term(1.0 - dot(n, zone_to_eye), zone_colours);
        let zone = zone_sample(in.texcoord, in.slots, in.world);
        zone_linear = pow(zone, vec3<f32>(2.2)) * zone_colours.effect.rgb + zone_base;
        zone_gamma = zone * zone_colours.effect.rgb + zone_base;
        // The visualiser glow - see `zone_glow`. Gated by `enabled` explicitly
        // rather than trusting `effect.w` to be zero off a Zone race, the same
        // belt-and-braces `surface_linear`/`plain` below already take.
        zone_glow_term =
            zone_glow(n, in.clip.z, in.texcoord, in.slots, in.world) * scene.zone.enabled;
    }

    // The read specular term: half-vector against the sun, raised to
    // `in.specular_exponent` - each material's own inline constant, decoded
    // off its fragment microcode by
    // `oag_formats::rcsmaterial::fragment::Program::specular_exponent`
    // rather than shared, per `oag_mesh::mesh::vertex::GpuVertex::specular_exponent`.
    // `32.0` remains only the *fallback*, for a material the decoder could
    // not resolve - see that field for what a resolved value means and what
    // it does not. Gated by `mask` above - the same sun-occlusion scalar the
    // diffuse term reads, exactly where the disc's own microcode gates its
    // specular by it too - and by the diffuse texture's alpha (gloss lives
    // there; a DXT1 diffuse has alpha 1 everywhere, which is full gloss, as
    // the original samples it too). Zero whenever the authored rig is off:
    // the stand-in never had one.
    let to_eye = normalize(scene.fog.camera - in.world);
    // The authored rig's half of the output, computed only when the rig is
    // on - see the same branch above `atlas`.
    var authored_rgb = vec3<f32>(0.0);
    if scene.light.enabled != 0.0 {
        let half_vector = to_eye + scene.light.direction;
        let ndh = clamp(
            dot(half_vector, n_lit) / max(length(half_vector), 1e-6),
            0.0,
            1.0,
        );
        let specular = scene.light.sun
            * (pow(ndh, in.specular_exponent) * ndl * mask * texel.a * scene.light.specular_scale
                * scene.light.enabled * in.lit * select(1.0, 0.0, emissive));

        // The authored path shades in linear light, as the RSX does: the samples
        // are sRGB-decoded and lit by the authored magnitudes. On a gamma target
        // the result is saturated (the stand-in for HD's exposure stage) and
        // encoded back here; on the linear float target it leaves **unclamped**,
        // because values above 1.0 are exactly what the bloom gate reads, and the
        // saturate-and-encode happens in `post::hd_bloom`'s own pass instead.
        // The vertex colour is **inside** `authored` on this path, not a factor
        // outside it: the microcode's `(prelit + f[TC1]) * albedo` adds the two
        // light terms and multiplies the albedo once. It stays a factor on the
        // stand-in path below, where it is a tint and every other title authors
        // it as one.
        let texel_linear = pow(texel.rgb, vec3<f32>(2.2));
        // `surface = zoneCol`, then `colour = light * surface` - the microcode's
        // own order, and on this path both terms are linear. The Zone surface
        // **replaces** the albedo rather than adding to it, so `enabled` is the
        // mix weight: it is 0.0 for every draw that is not a Zone race with all
        // its inputs resolved, and there this is `texel_linear` unchanged.
        let surface_linear = mix(texel_linear, zone_linear, scene.zone.enabled);
        // `colour = light * surface + glow` - the microcode's own order, and
        // **both glows are on the far side of the multiply because that is where
        // the disc puts them**. Read off `uvanim_diffuse_emissive`'s lit fragment
        // block on Amphiseum, block #3, with `scripts/ps3-microcode.py`:
        //
        //     @0x22  MUL H4.xyz, H0, H4      ; albedo * (prelit + ambient + ndl*sun)
        //     @0x23  MAD H0.xyz, H0.wwww, H1, H4   ; + diffuse alpha * tinted glow
        //
        // Its unlit block #2 (`MUL` at 0x0b, `MAD` at 0x0f) and its second lit
        // block #4 (0x23, 0x25) have the same pair in the same order. Three
        // variants, no exceptions - so the accumulate reads a value the light has
        // already multiplied, and a glow is not itself lit.
        //
        // **`glow_linear` belongs here and used to reach only `plain`**, which is
        // what made HD's emissive layer very nearly invisible: this fragment
        // resolves `mix(plain_rgb, authored_rgb, enabled * in.lit)` and HD's rig is
        // always enabled, so every `in.lit == 1` chunk - the track, the tubes, the
        // signs, everything near the camera - dropped it. Only the far background
        // and the hull ever saw it.
        let lit_linear =
            surface_linear * authored + specular + zone_glow_term + glow_linear + pad_glow + mag_glow;
        let encoded = pow(
            clamp(lit_linear, vec3<f32>(0.0), vec3<f32>(1.0)),
            vec3<f32>(1.0 / 2.2),
        );
        authored_rgb = mix(encoded, lit_linear, linear_out);
    }

    // The stand-in path, byte-for-byte what every other title always drew.
    // Prelit geometry (`lit` 0.0) takes it even under the authored rig: its
    // lighting is baked into its vertex colours, in the gamma space every
    // Pulse-shaped asset authors, so the linear round-trip above would
    // re-shade what is already shaded. On the linear target its gamma result
    // is decoded, so `post::hd_bloom`'s encode returns it byte-for-byte.
    let light = mix(vec3<f32>(1.0), stand_in, in.lit);
    // **The vertex colour is a tint here and a light term on the authored
    // path, so this multiply must not see HD's.** Without the guard, any HD
    // draw that binds `Light::stand_in` - the front end, or a circuit whose
    // `.envsettings` fails to yield a rig - would multiply by a baked light
    // that is zero wherever no colour set exists, and render black.
    //
    // **This used to key on `linear_out` and that was wrong twice.** It missed
    // HD's capture and viewer paths, which draw the same models into a gamma
    // target - `model_probe` found them rendering dark, 1,062 lit pixels of a
    // 1024x1024 frame against 6,988 once fixed. And it over-fired on the race
    // target, forcing *every* model's tint to white there including the sky
    // cube, whose vertex colour genuinely is a tint because `mesh/sky_cube.rs`
    // builds it rather than the HD loader. `colour_is_light` is a property of
    // the model and answers both.
    let tint = mix(in.colour.rgb, vec3<f32>(1.0), colour_is_light);
    // The gamma-domain sum, for the same reason this whole path is gamma: a
    // prelit surface's light is baked into its vertex colours in the space the
    // asset authors, so a linear summand would be the one term shaded twice.
    // The glow is **added after the light**, which is where the microcode puts
    // it: the accumulate reads `H0` once the lightmap and the interpolated
    // term have already multiplied the albedo, so a glow is not itself lit.
    // The visualiser glow rides here too, undecoded on the same terms
    // `zone_gamma` already is on this path - it is a shader parameter and
    // texture-lookup product, not an albedo sample, so it owes no sRGB
    // decode either way.
    let plain = mix(texel.rgb, zone_gamma, scene.zone.enabled) * tint * light
        + glow + zone_glow_term + pad_glow + mag_glow_gamma;
    let plain_rgb = mix(plain, pow(plain, vec3<f32>(2.2)), linear_out);

    // Vertex colour modulates the texture on all four channels, as the GE's
    // texture-env does - RGB and alpha alike, not RGB alone. Dropping the
    // vertex colour's own alpha here is what made the boost plume's baked
    // falloff vanish; see `every_psp_teams_boost_plume_vertex_alpha_is_bimodal`
    // in `crates/game/tests/boost_plume_ground_truth.rs`.

    // **The flame, when this model is one.** `flame_shading` is 1.0 only for a
    // craft's `engineflare` model, and then this replaces everything above
    // rather than modifying it - the program it reproduces has no ambient, no
    // sun, no lightmap and no specular, so lighting the flame and then
    // overwriting it is the honest shape as well as the cheap one.
    //
    // `rim` is the same angle HD's program computes. Its own vertex program
    // dots an *untransformed* normal attribute against
    // `eyePositionWorldSpace - position`, which is only meaningful if the eye
    // reaches it in model space; the angle between the two is the same number
    // under a rigid transform, so it is computed here from the world normal and
    // the real camera.
    //
    // **`texel.a` is deliberately absent.** The program's `TEX H0.xyz` writes
    // three components and the fourth is the vertex ramp from the moment
    // `MOV H0.w, f[TC2]` puts it there, so the sampled alpha never reaches the
    // output. Multiplying it in - which the line below this one does for every
    // other surface - is what held the flame at about a third of its
    // brightness.
    //
    // **Only on a flame pipeline.** `flame_shading` is an `override`, so this
    // branch is a constant the driver deletes on every other pipeline, which
    // otherwise paid the two fetches below on every pixel for a mix weight of
    // zero.
    var flame = vec4<f32>(0.0);
    if flame_shading != 0.0 {
        let rim = clamp(1.0 - dot(to_eye, n), 0.0, 1.0);
        let flame_alpha = flame_alpha_scale
            * (1.0 - (pow(rim, flame_rim_power) * flame_rim_scale + flame_rim_min));
        // **The program's two taps, both of them.** It samples `unit0` twice from
        // one texture: a *noise* tap whose `v` scrolls with the engine clock, and
        // a *colour* tap at doubled coordinates displaced by the noise the first
        // returned. Neither is this file's invention - block #1 and block #2 of
        // `flame_test.rcsmaterial` schedule the identical pair through different
        // registers, and `time`'s provider is engine parameter slot 0. See
        // `oag_mesh::mesh::Flame` and
        // docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md.
        //
        //     @0x0a  MAD R0.y, {Speed}, {time}, Uv1.y   <- the scrolled row
        //     @0x11  TEX R2.w, R0 unit0                 <- its ALPHA is the noise
        //     @0x1e  ADD R1.xy, R1, R2.zwzz             <- (2u, 2v + noise)
        //     @0x1f  TEX H0.xyz, R1 unit0               <- the colour
        //
        // The doubling stays a plain multiply rather than a fract: the sampler
        // repeats, and the authored `Uv1` runs outside 0..1 on one node already.
        let flame_noise = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(in.texcoord.x, in.texcoord.y + flame_speed * scene.time.x),
        ).a;
        let flame_texel = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(in.texcoord.x * 2.0, in.texcoord.y * 2.0 + flame_noise),
        );
        let flame_rgb = flame_texel.rgb * flame_colour_scale;
        // **Not decoded on the linear target, on purpose - and this is a change
        // of mind recorded in place.** The decode used to sit here on the "what a
        // sampler hands back is authored-space" argument, but the flame's own
        // program is read instruction by instruction and applies no transfer
        // function anywhere: the original multiplies the raw sample into a
        // target the exposure resolve then scales, `ADD_SAT`s and presents with
        // no gamma arithmetic (renderer.md, "no `1/2.2` ... exists anywhere in
        // it"). Decoding here dimmed the flame's mid-tones by up to a third
        // against that arithmetic - the same divergence the HD trail's path had,
        // fixed the same day. See
        // docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md.
        flame = vec4<f32>(flame_rgb, flame_alpha * in.colour.a);
    }

    let shaded = vec4<f32>(
        mix(plain_rgb, authored_rgb, scene.light.enabled * in.lit),
        texel.a * in.colour.a,
    );
    // **The facing-ramp combine replaces the generic rig entirely**, the
    // same shape `flame` below already takes: block #7 computes its own
    // light (the traced `vertexLight`) rather than being multiplied by
    // `authored`'s ambient/prelit/sun sum, so running both would shade it
    // twice.
    let shaded_or_sheen = select(shaded, sheen, ramp_sheen);
    // **The magstrip floor's own combine** (`slots::MAG_LOOP`), traced in
    // `oag_mesh::mesh::rcs::mag_wave`: `vertexLight * grid * (ramp + c) +
    // (grid + ramp) * d`, alpha the grid. `albedo` is the grid and `lightmap`
    // the facing ramp, addressed by `dot(V, N)` as the glass sheen's is. `c`
    // is the glow entry's `offset`. Left out, named: the paraboloid
    // reflection term, which this renderer has no probe for.
    var mag_floor = shaded_or_sheen;
    if (in.slots & 262144u) != 0u && scene.zone.enabled == 0.0 {
        let grid = textureSample(albedo, albedo_sampler, in.texcoord).r;
        let ramp = textureSample(lightmap, albedo_sampler, ramp_uv).rgb;
        let loop_light = in.colour.rgb + in.texcoord.x * ramp_sun;
        let loop_rgb = loop_light * grid * (ramp + vec3<f32>(glow_tint_offset.w))
            + (vec3<f32>(grid) + ramp) * mag_glow;
        mag_floor = vec4<f32>(loop_rgb, grid * in.colour.a);
    }

    // **HD's light cone** (`slots::LIGHT_CONE`, `oag_mesh::mesh::rcs::light_cone`):
    // the program's colour is the noise's red times the authored intensity `K`
    // (`MAD R2.xyz, H2.xxxx, K, -fog`, then the fog lerp `fogged` does) and its
    // alpha is `noise * s * H2.x` where `H2.x` is the ramp tap at `dot(N, V)`
    // **only when the program's predicated `TEX` executes** and the noise
    // itself (register `H2.x` is left holding it) when it does not. The
    // predicate is `NE(wwww)` on a condition set from `f[TC1]`, whose `w` is
    // the vertex normal's `y` - zero on every authored cone, whose normal is
    // the constant `(0, 0, 1)` - so on a gated variant the ramp is skipped and
    // the alpha is `s * noise^2`. `glow_scale.y` says the variant is gated.
    // That reading of the condition register is a hypothesis (confidence 60,
    // light_cone's doc): it is what a matched frame favours, not something the
    // instruction set documents. `albedo` is the ramp and `lightmap` the
    // noise, `K` the glow entry's tint, `s` its scale.
    // Raw samples, undecoded: the program applies no transfer function. **The
    // colour is saturated before it blends**, as the 8-bit surface the original
    // draws into does: `K` is 100 on Talon's Junction, so an unclamped colour
    // on this linear float target is `100 * noise` of light times the alpha, a
    // white wall, where the original's is a white of at most 1 at that alpha.
    var cone_or_floor = mag_floor;
    if (in.slots & 524288u) != 0u {
        let noise = textureSample(lightmap, albedo_sampler, in.texcoord).r;
        let cone_facing = clamp(ramp_facing, 0.0, 1.0);
        let ramp = textureSample(albedo, albedo_sampler, vec2<f32>(cone_facing)).r;
        let ramp_skipped = glow_scale.y > 0.5 && in.normal.y == 0.0;
        cone_or_floor = vec4<f32>(
            clamp(vec3<f32>(noise) * glow_tint_offset.rgb, vec3<f32>(0.0), vec3<f32>(1.0)),
            noise * glow_scale.x * select(ramp, noise, ramp_skipped),
        );
    }

    // **The absorb shell, when this model is one.** Fragment block #1 of
    // `hd_absorbinternal.rcsmaterial`, which the fogged blocks repeat before
    // their fog lerp (`fogged` below does that part):
    //
    //     @0x04  TEX R0.w, (u, v) unit0             <- the ALPHA is the offset
    //     @0x05  MAD R1.xy, R0.w, 0.5, (u, v)
    //     @0x07  MAD R1.xy, time, 5.0, R1           <- both axes, 5 per second
    //     @0x09  MOV H0.w, f[TC2]                   <- VertexColour1.w * ShieldColour
    //     @0x0a  TEX H0.xyz, R1 unit0
    //
    // So the colour is the texture sampled at its own alpha-displaced, clock-
    // scrolled coordinate, and the alpha is the vertex ramp times the fade the
    // caller writes into it. `texel.a` does not reach the output. Not decoded
    // on the linear target, for the reason the flame gives: the program
    // applies no transfer function.
    // Branched on its `override` for the flame's reason.
    var absorb = vec4<f32>(0.0);
    if absorb_shading != 0.0 {
        let absorb_offset = textureSample(albedo, albedo_sampler, in.texcoord).a * 0.5
            + 5.0 * scene.time.x;
        let absorb_rgb = textureSample(
            albedo,
            albedo_sampler,
            in.texcoord + vec2<f32>(absorb_offset, absorb_offset),
        ).rgb;
        absorb = vec4<f32>(absorb_rgb, in.colour.a);
    }
    let composed = mix(mix(cone_or_floor, flame, flame_shading), absorb, absorb_shading);

    // **Wipeout HD's two rim-shaded weapon glows**, `slots::RIM_GLOW` (the
    // LeachBall) and `slots::RIM_EDGE` (the Plasma bolt's head), each set only
    // where `mesh::rcs::rim_glow` matched the material's own program - its
    // mnemonics, its literals and its declared parameters. Read instruction by
    // instruction in docs/rendering/hd-unlit-programs.md; every number below
    // is a literal of that program. Neither program reads a light, the vertex
    // colour or the texture's own alpha as coverage, so these replace
    // everything above rather than modifying it, as `flame` does; and like
    // `flame` they are not decoded on the linear target, since the programs
    // apply no transfer function.
    //
    //     TEX R1.w, (u, v + 0.0001 time)            <- the noise is the ALPHA
    //     MAD R3.zw, noise, 0.2, (u, v)
    //     MAD R1.zw, time, 0.4, R3.zw               <- both axes
    //     TEX H1.xyz, R1.zwzz                       <- the colour tap
    //     ADD_SAT rim, -dot(N, V), 1
    //
    // `rim` is `flame`'s own angle, for `flame`'s own reason: the vertex
    // program dots an untransformed normal against `eye - position`, and the
    // angle survives the rigid transform this path applies first.
    //
    // **Branched on the two bits**, so the two fetches run on the two weapon
    // glows that read them and nowhere else. `slots` is a flat varying, which
    // WGSL's uniformity analysis cannot see is constant per quad, hence this
    // function's `derivative_uniformity` diagnostic - see its own attribute.
    let rim_glow_on = (in.slots & 2048u) != 0u;
    let rim_edge_on = (in.slots & 4096u) != 0u;
    if !(rim_glow_on || rim_edge_on) {
        return composed;
    }
    let rim_noise = textureSample(
        albedo,
        albedo_sampler,
        vec2<f32>(in.texcoord.x, in.texcoord.y + 0.0001 * scene.time.x),
    ).a;
    let rim_tap = in.texcoord + vec2<f32>(0.2 * rim_noise + 0.4 * scene.time.x);
    let rim_c = textureSample(albedo, albedo_sampler, rim_tap).rgb;
    let rim_angle = clamp(1.0 - dot(to_eye, n), 0.0, 1.0);
    // `LG2`/`MUL 5`/`EX2` in the program: `rim^5`, multiplied out so `0^5` is
    // `0` rather than whatever `pow(0, 5)` answers on a given backend.
    let rim_5 = rim_angle * rim_angle * rim_angle * rim_angle * rim_angle;
    // RIM_GLOW: `MAD_SAT -x, 0.9, 0.9`, then `^5` again.
    let rim_fade_base = clamp(0.9 - 0.9 * rim_5, 0.0, 1.0);
    let rim_fade = rim_fade_base * rim_fade_base * rim_fade_base * rim_fade_base * rim_fade_base;
    // `c / (1 - c)` through three `RCP`s of half registers. A texel at 255
    // makes that `1 / 0`; **chosen, not measured**: it is held at the largest
    // finite half, 65504, rather than let an infinity into the float target
    // and its bloom.
    let rim_expand = min(vec3<f32>(1.0) / (vec3<f32>(1.0) - rim_c), vec3<f32>(65504.0));
    let rim_glow = vec4<f32>(rim_fade * rim_c * rim_expand, rim_fade);
    // RIM_EDGE: `MUL H1.w, rim^5, 1000`, times the tap; the alpha is the
    // material's `0x7611a2d8`, which `rim_glow::classify` only routes at 1.0.
    let rim_edge = vec4<f32>(1000.0 * rim_5 * rim_c, 1.0);
    return select(select(composed, rim_glow, rim_glow_on), rim_edge, rim_edge_on);
}
