// `lit_texel`, the one fragment shading function every entry point calls.

import package::absorb::{absorb_shading, shield_shading};
import package::bindings::{behind_glass_tex, emissive_scale, emissive_tint, env_probe_tex, program_in, rate_from, rate_in, scene, sh_probe_table, shadow_sampler, sky_probe_tex, uniforms};
@if(!bindless) import package::bindings::{albedo, albedo_sampler, fifth_map, lightmap, pad_mask, sixth_map, wave_map};
@if(bindless) import package::bindings::{material, samplers, textures};
import package::flame::{flame_alpha_scale, flame_colour_scale, flame_rim_min, flame_rim_power, flame_rim_scale, flame_shading, flame_speed};
import package::glow::{glow_first, glow_pad, glow_second};
import package::holo::{holo_billboard};
import package::hull_normal::{hull_frame, hull_normal};
import package::omega_rock::{OmegaRock, omega_rock};
import package::omega_snow::{OmegaSnow, omega_snow, omega_snow_normal};
import package::own_shield::{own_shield, own_shield_shading};
import package::panel::{panel_march};
import package::pad::{pad_normal};
import oag_shaders::target::{decode, encode, encoded_out, linear_out, square_decode, target_in};
import package::shading_modes::{colour_is_light, light_rig, lit_by_rig, nova_prelit, nova_vertices, rig_specular, simplified_lighting, tangents_present, zone_race, zone_stage_light};
import package::shadow::{receives_shadow, sun_occlusion};
import package::water::{enhanced_water, reflective_sea_weight, water_kind, water_normal, water_specular_colour, water_tap_uv};
import package::texture_lod::{MATERIAL_ALBEDO, MATERIAL_FIFTH_MAP, MATERIAL_LIGHTMAP, MATERIAL_PAD_MASK, MATERIAL_SIXTH_MAP, MATERIAL_WAVE_MAP, ge_level, material_lod_bias, material_sample, material_sample_grad, sample_at_slope_level, texlod_slope, uv_gradients};
import package::types::{Lit, SunTerms, VertexOutput};
import package::zone::{zone_aniso, zone_base_term, zone_glow, zone_sample, zone_set};

// The largest finite half-precision float. The original's half registers turn the
// `RCP` of a zero divisor into an infinity; it is held here instead, rather than
// let an infinity into the float target and its bloom: **chosen, not measured**
// (`docs/rendering/hd-unlit-programs.md`, "What they compute"). The `c / (1 - c)`
// expansions of the Bomb's fireball and of the rim glow use it.
const HALF_MAX: f32 = 65504.0;

// The Bomb halo's `0.1` literal, the half-precision value nearest it (`0x2e66`):
// the last factor of its alpha, `hd_bomb_halo` block `@0x19c0`
// (`docs/rendering/hd-unlit-programs.md`, "The Bomb's fireball and shockwaves").
const BOMB_HALO_ALPHA_SCALE: f32 = 0.099975586;

// `grade-sky`'s overcast: the sky's luma times `SKY_OVERCAST_LUMA` plus
// `SKY_OVERCAST_TINT` - darker, desaturated, slightly cool. See the `grade-sky`
// entry of `docs/architecture/wasm-extensions/README.md`.
const SKY_OVERCAST_LUMA: f32 = 0.45;
const SKY_OVERCAST_TINT: vec3<f32> = vec3<f32>(0.16, 0.18, 0.21);

// **Where a normal looks in Wipeout HD's environment probe**: the dual paraboloid the
// hull's `IBL` programs address, read off the live microcode (confidence 85,
// `docs/rendering/hd-ship-materials.md`, "Finding 4"). `x > 0` is the right-hand
// disc, `z` its horizontal and `y` its vertical.
fn env_probe_uv(n: vec3<f32>) -> vec2<f32> {
    let u = 0.25 + select(-n.z / 4.0, 0.5 + n.z / 4.0, n.x > 0.0);
    return vec2<f32>(u, 0.5 - 0.5 * n.y);
}

// The engine's probe tint, parameter `0x7480de6d` (`iblScalePower`), `(4, 4, 4, 0)` on
// Talon's Junction and Metropia and `(0.698, 0.698, 0.698, 1)` in a Zone race (read live on
// RPCS3 at zones 3 and 4 of Zone mode): both probe maps are multiplied by it, and `.w` lerps
// the result towards its own luma, so a Zone hull's probe light is grey and a fifth the
// strength. The draw names it: `uniforms.env_gain` is the rgb scale and `uniforms.env_desat`
// the `.w` (`oag_raceplay::env_probes::ProbeTint`).
fn probe_tint(texel: vec3<f32>) -> vec3<f32> {
    let luma = dot(texel, vec3<f32>(0.300049, 0.589844, 0.109985));
    // HD's Zone frame: one tint for every craft hull, from the stage table, over the hull's
    // own (`mesh_render::Light::hull_probe_tint`).
    if uniforms.env_layer > 0.5 && scene.light.hull_probe_tint.w > 0.5 {
        return luma * scene.light.hull_probe_tint.rgb;
    }
    let scaled = texel * uniforms.env_gain;
    let grey = vec3<f32>(luma * uniforms.env_gain);
    return mix(scaled, grey, uniforms.env_desat);
}

// The array holds two layers per craft, each 128x64: the irradiance map in its top-left
// 64x32 corner, then the reflection map in all of its layer. `which` is 0 or 1.
fn env_probe_layer(which: i32) -> i32 {
    return (i32(uniforms.env_layer + 0.5) - 1) * 2 + which;
}

// Where the irradiance map's own `0..1` coordinate lands in its layer: halved, and clamped
// to the map's texel centres first, so the filter never reaches past the 64x32 corner (a
// dedicated map's `ClampToEdge`).
fn env_probe_irradiance_uv(uv: vec2<f32>) -> vec2<f32> {
    let clamped = clamp(uv, vec2<f32>(0.5 / 64.0, 0.5 / 32.0), vec2<f32>(1.0 - 0.5 / 64.0, 1.0 - 0.5 / 32.0));
    return clamped * 0.5;
}

// **Omega's hull ambient**, `(Ks + (K - Ks) * shadow) * max(0.01, SH9(n))`, from the craft's row of
// `sh_probe_table` (`oag_raceplay::sh_probes`; `docs/ghidra/functions/ps4-omega-eu/hull-tone.md`,
// "The program"): nine rgb coefficients, then `(K, Ks, 1, 0)`. `.w` of the result is 1 where the
// draw's craft has a row, 0 where it has none (the table is the placeholder or the row was never written) and the caller keeps its other ambient. `n` is the
// normalised normal-mapped normal and `shadow` the hull's own sun shadow term.
fn sh_hull_ambient(n: vec3<f32>, shadow: f32) -> vec4<f32> {
    let row = i32(uniforms.env_layer + 0.5) - 1;
    if row < 0 || row >= 8 {
        return vec4<f32>(0.0);
    }
    let at = u32(row) * 10u;
    let weights = sh_probe_table[at + 9u];
    if weights.z < 0.5 {
        return vec4<f32>(0.0);
    }
    let c0 = sh_probe_table[at].rgb;
    let c1 = sh_probe_table[at + 1u].rgb;
    let c2 = sh_probe_table[at + 2u].rgb;
    let c3 = sh_probe_table[at + 3u].rgb;
    let c4 = sh_probe_table[at + 4u].rgb;
    let c5 = sh_probe_table[at + 5u].rgb;
    let c6 = sh_probe_table[at + 6u].rgb;
    let c7 = sh_probe_table[at + 7u].rgb;
    let c8 = sh_probe_table[at + 8u].rgb;
    let sh = c0 - c1 * n.z + c2 * n.y + c3 * n.x - c4 * (n.z * n.x) - c5 * (n.z * n.y)
        + c6 * (2.0 * n.y * n.y - n.x * n.x - n.z * n.z) + c7 * (n.x * n.y) + c8 * (n.x * n.x - n.z * n.z);
    let kmix = weights.y + (weights.x - weights.y) * shadow;
    return vec4<f32>(kmix * max(vec3<f32>(0.01), sh), 1.0);
}

// **A branch on `slots` keeps its derivatives.** `slots` is a flat varying,
// and a 2x2 fragment quad - helper lanes included - is always shaded for one
// primitive, so every lane of a quad reads the same flat value and takes the
// same side of the branch. An implicit-derivative fetch inside it therefore
// has the neighbours it needs. WGSL's analysis cannot see that about a
// varying, so it is told here.
// **Every `slots` bit some vertex of this model carries** (`mesh_render::build_with`), and all
// ones for a pipeline that does not set it. ANDed in before any branch reads the word, which
// changes nothing (`x & present == x` for every vertex of the model) but lets the driver drop
// every branch the model can never take. On the Steam Deck at 1080p HD's track solid pass went
// 8.23 to 7.65 ms and Pulse's 3.38 to 1.64, byte-identical (`docs/rendering/hires-gpu-levers.md`).
override slots_present: u32 = 4294967295u;

// **Whether any draw of this pipeline reads the second texture at the diffuse coordinate**
// (`mesh_render::specialise::reads_second`): 0.0 where no vertex carries a role bit that reads
// it, names no emissive row (the pane and twin readers) and has no glow mask sourced from it, so
// the fetch goes; 1.0, fetched, by default. Where it is 0.0 every reader below sees zero and no
// reader runs.
override second_read: f32 = 1.0;

// `lit_colour` with the sun terms `shadowed` needs returned beside the colour. Its derivative-uniformity
// diagnostic is off for `lit_colour`'s reason: the call below is judged against this body.
@diagnostic(off, derivative_uniformity)
fn lit_texel(raw: VertexOutput) -> Lit {
    var sun = SunTerms(vec3<f32>(0.0), 0.0, 0.0, vec3<f32>(0.0), vec3<f32>(0.0));
    let colour = lit_colour(raw, &sun);
    return Lit(colour, sun);
}

// **Why its derivative-uniformity diagnostic is off.** Tint treats every fragment input, flat ones
// included, as possibly different between the lanes of a 2x2 quad, so it rejects each implicit-level
// fetch and `dpdx`/`dpdy` that sits after an early return or inside a branch. Every guard on the
// path to one of them here is the same on all lanes of a quad, because a quad never spans two
// primitives: an `override` or a uniform; `slots`, `specular_exponent` or `glow`, which are `flat`;
// a row of the emissive table, indexed by `slots`; or `colour.a > 1.5`, the Zone stage tag, which
// the loader writes onto every vertex of a submesh. **The one guard that is not constant across a
// primitive is the length of the interpolated tangent** (`panel`, `omega_snow_on`, `omega_rock_on`
// and the hull normal map's test), so a fetch behind it takes explicit gradients
// (`material_sample_grad`, gradients from `uv_gradients` taken before the test) and a fetch the
// other side of `panel` runs is hoisted ahead of it. The audit that found this, site by site, is
// R1 of the rendering shaders review in `docs/reviews/`.
@diagnostic(off, derivative_uniformity)
fn lit_colour(raw: VertexOutput, sun: ptr<function, SunTerms>) -> vec4<f32> {
    // Our own shield draws no texture and takes none of the lighting below.
    if own_shield_shading != 0.0 {
        return own_shield(raw);
    }
    var in = raw;
    in.slots = raw.slots & slots_present;
    // Under `lit_by_rig` the rig is on and every vertex has `lit` 1.0 (what that override
    // promises), so both weights are the constant one and the driver folds their multiplies.
    let rig_on = select(select(0.0, scene.light.enabled, light_rig != 0.0), 1.0, lit_by_rig != 0.0);
    in.lit_light.x = select(raw.lit_light.x, 1.0, lit_by_rig != 0.0);
    in.tangent = select(raw.tangent, vec4<f32>(0.0), tangents_present == 0.0);
    // **2048's `zonefc06_colour_emissive_scalar` surface**, a vertex that names its
    // slot in `colour.a` (`mesh::rcs::psp2::ZONE_COLOUR_ALPHA`) while the Zone table
    // authors the stage's colours: `Colour * (ambient + sun * sat(N.L) + Emissive)`,
    // the arithmetic of the material's unlightmapped fragment programs, in the
    // gamma domain they run in (`docs/formats/2048-material-params.md`, "The Zone
    // surface"). Fog and shadow are applied by the caller, as for every surface.
    if scene.zone.stage_light[0].w > 0.5 && raw.colour.a > 1.5 {
        let slot = clamp(i32(round(raw.colour.a)) - 2, 0, 7);
        let stage = scene.zone.stage_colours[slot];
        let up = normalize(raw.normal);
        let sat_ndl = max(dot(up, normalize(scene.zone.stage_light[2].xyz)), 0.0);
        let lit = stage.rgb * (scene.zone.stage_light[0].xyz + scene.zone.stage_light[1].xyz * sat_ndl + vec3<f32>(stage.w));
        return vec4<f32>(mix(lit, decode(lit), linear_out), 1.0);
    }
    let zone_on = select(0.0, scene.zone.enabled, zone_race != 0.0);
    // **A chunk a Zone race draws on its plain program** (`slots::ZONE_PLAIN`, the speed and weapon
    // pads, signs and screens; `docs/ghidra/functions/ps3-hdfury-eu/zone-plain-chunks.md`): none of
    // the Zone surface, the program's own variants, and the stage's ambient and sun as its only
    // light. `zone_program` is `zone_on` for every other chunk: the tests below that say "the Zone
    // variant of this program does not declare X" ask about the program, which a plain chunk
    // does not run.
    let zone_plain = zone_on != 0.0 && (in.slots & 1073741824u) != 0u;
    let zone_program = select(zone_on, 0.0, zone_plain);
    // The `bindless` variant: this draw's slot of each group 1 array, under the
    // names the per-draw variant binds them by - see `bindings.wesl`.
    @if(bindless) {
        let albedo = textures[material.albedo];
        let albedo_sampler = samplers[material.albedo_sampler];
        let lightmap = textures[material.lightmap];
        let pad_mask = textures[material.pad_mask];
        let wave_map = textures[material.wave_map];
        let fifth_map = textures[material.fifth_map];
        let sixth_map = textures[material.sixth_map];
    }
    // A holographic billboard's Zone variants drop the advert and draw the generic Zone surface
    // (`docs/rendering/hd-zone-recolour.md`, "Holographic billboards in a Zone frame").
    if (in.slots & 8388608u) != 0u && zone_program == 0.0 {
        return holo_billboard(in);
    }
    // The row of the emissive table this vertex names, read once for every test below.
    let row_scale = emissive_scale((in.slots >> 24u) & 63u);
    let row_tint = emissive_tint((in.slots >> 24u) & 63u);
    // **Omega's `_shadow` crowds** (`Emissive::program` mode 16, `mesh::rcs::psp2::crowd`): the light
    // is `Ambient + constantAmbientColour + sun * sat(N.L)` and the lit picture is times `Shadowed`.
    let crowd_lit = program_in(row_scale.z, 16u, 16u);
    // **Omega's `RT_snow1`** (mode 21, `mesh::rcs::psp2::snow`, `omega_snow.wesl`): its own normal
    // blend, Fresnel-weighted wrapped diffuse and sun lobe. Off under CIRCUIT LIGHTING `simplified`.
    let omega_snow_entry = (in.slots >> 24u) & 63u;
    let omega_snow_on = omega_snow_entry != 0u && program_in(row_scale.z, 21u, 21u)
        && simplified_lighting == 0.0 && dot(in.tangent.xyz, in.tangent.xyz) > 0.25;
    // **Omega's `DC_Rocks`** (mode 22, `mesh::rcs::psp2::snow`, `omega_rock.wesl`): its own normal
    // blend, wrapped sun, shadow-light factor and sun lobe. Off under CIRCUIT LIGHTING `simplified`.
    let omega_rock_on = omega_snow_entry != 0u && program_in(row_scale.z, 22u, 22u)
        && simplified_lighting == 0.0 && dot(in.tangent.xyz, in.tangent.xyz) > 0.25;
    // A Zone frame lights the surface with its own program, never this permutation's.
    let omega_rock_drawn = omega_rock_on && !zone_plain
        && !(zone_program != 0.0 && scene.zone.shape.w > 0.5);
    // **HD's scrolled-tap water** (`Emissive::program` mode 17, `oag_mesh::mesh::rcs::water`):
    // 1 the sea, 2 Amphiseum's pool, 3 `water_test_2`, 4 `water_noref`; 0 elsewhere.
    let water_entry = (in.slots >> 24u) & 63u;
    let water_raw = water_kind(water_entry, row_scale);
    // **2048's own sea** (kind 6, `mesh::rcs::psp2::water`): the generic lit path with a
    // scrolled normal map and Phong highlight, so none of the HD families' branches apply.
    let vita_water = water_raw > 5.5;
    // **2048's `fc17` water** (kinds 7 and 8, `mesh::rcs::psp2::water`): kind 6's frame and light
    // with `Watercolour` for the colour (the entry's tint) and `SpecularColour` for the highlight,
    // no gloss from the map's alpha; kind 8, the blended edge, also takes the alpha chain below.
    let vita_fc17 = water_raw > 6.5;
    let vita_edge = water_raw > 7.5 && water_raw < 8.5;
    // **Omega's reflective sea** (kinds 9 and 10, `mesh::rcs::psp2::water`): the same generic
    // lit path, `Watercolour` for the colour and `SpecularColour` for a Phong highlight, with
    // two `.ag` taps and the reflection added below; kind 10, the blended edge, takes its
    // alpha from `VertexColour1.y * Opacity`.
    let omega_sea = water_raw > 8.5;
    let omega_edge = water_raw > 9.5;
    let water = select(water_raw, 0.0, vita_water);
    let n = normalize(in.normal);
    // **Omega's panel-map road** (`Emissive::program` mode 15, `mesh::rcs::psp2::panel_map`): the
    // diffuse, normal and emissive are sampled at the coordinate the march into the panel map
    // (fifth binding) lands on, with the unshifted coordinate's gradients. The next two entries
    // hold the specular's and the march's numbers.
    let panel_entry = (in.slots >> 24u) & 63u;
    let panel = panel_entry != 0u && program_in(row_scale.z, 15u, 15u)
        && dot(in.tangent.xyz, in.tangent.xyz) > 0.25;
    // **Omega's `tarmac_markings_dirt_markingsnormal[_emis]` roads** (mode 14, below).
    let tarmac_entry = (in.slots >> 24u) & 63u;
    let tarmac = tarmac_entry != 0u && program_in(row_scale.z, 14u, 14u);
    let uv_grad = uv_gradients(in.texcoord);
    var panel_uv = in.texcoord;
    if panel {
        panel_uv = panel_march(in.texcoord, in.world, n, in.tangent, emissive_tint(panel_entry + 2u).xyz);
    }
    // **HD's pads** (`slots::PAD_NE`): the `_ne` texel replaces the vertex
    // normal in the program's own `N.L` and `N.H` below, and its alpha gates
    // the glow added after the light. `n` itself stays the vertex normal for
    // everything else this function reads it for.
    let pad_ne = (in.slots & 16384u) != 0u;
    var pad_ne_texel = vec4<f32>(0.0);
    var n_lit = n;
    if pad_ne {
        pad_ne_texel = material_sample(MATERIAL_PAD_MASK, in.texcoord);
        n_lit = pad_normal(pad_ne_texel, n, in.world, in.texcoord);
    }

    // **A hull's normal map** (`GpuVertex::tangent`, bound third like the pad
    // mask): the program's own `N = nx T + ny B + nz N0`, with `B = N0 x T * w`.
    // The mapped normal feeds the sun's `N.L`, the specular's `N.H` and the probe
    // lookups; a pad takes its own frame above and has no tangent.
    var n_hull = n;
    var omega_rock_lit: OmegaRock;
    // **Sebenco's snow-capped rock** (`Emissive::program` mode 4, `oag_mesh::mesh::rcs::snow_rock`):
    // the mask (third binding, second pair) mixes the snow's normal (fourth) with the rock's
    // (sixth) before the frame, and the rock (fifth) into the snow below. Both normals and the
    // rock are read at the diffuse coordinate times the entry's scale (the rock's; the snow's
    // normal at the coordinate itself).
    let snow_entry = (in.slots >> 24u) & 63u;
    let snow = snow_entry != 0u && program_in(row_scale.z, 4u, 4u);
    var snow_mask = 0.0;
    var rock = vec4<f32>(0.0);
    if snow {
        let rock_uv = in.texcoord * row_scale.x;
        snow_mask = material_sample(MATERIAL_PAD_MASK, in.lightmap_texcoord.zw).x;
        rock = material_sample(MATERIAL_FIFTH_MAP, rock_uv);
        let bump = mix(
            material_sample(MATERIAL_WAVE_MAP, in.texcoord),
            material_sample(MATERIAL_SIXTH_MAP, rock_uv),
            snow_mask,
        );
        n_hull = hull_normal(bump, n, in.tangent);
        n_lit = n_hull;
    } else if panel {
        let bump = material_sample_grad(MATERIAL_PAD_MASK, panel_uv, uv_grad[0], uv_grad[1]);
        n_hull = hull_normal(bump, n, in.tangent);
        n_lit = n_hull;
    } else if omega_snow_on {
        n_hull = omega_snow_normal(omega_snow_entry, in, n, uv_grad);
        n_lit = n_hull;
    } else if omega_rock_on {
        omega_rock_lit = omega_rock(omega_snow_entry, in, n, uv_grad);
        n_hull = omega_rock_lit.normal;
        n_lit = n_hull;
    } else if simplified_lighting == 0.0 && dot(in.tangent.xyz, in.tangent.xyz) > 0.25 {
        let map = material_sample_grad(MATERIAL_PAD_MASK, in.texcoord, uv_grad[0], uv_grad[1]);
        // The crowd's programs weight the map's `z` by `eye - position` (`row_tint.w`, the entry's `offset`).
        let third = select(n, scene.fog.camera - in.world, crowd_lit && row_tint.w > 0.5);
        n_hull = hull_frame(map, n, in.tangent, third);
        n_lit = n_hull;
    }
    // The water's normal is its scrolled taps' sum (`water.wesl`), for the light, the
    // specular and the reflection alike.
    if water > 0.5 {
        n_hull = water_normal(water_entry, water, select(in.texcoord, in.world.xz, water > 4.5), n, in.tangent);
        n_lit = n_hull;
    }
    if vita_water {
        n_hull = water_normal(water_entry, water_raw, in.texcoord, n, in.tangent);
        n_lit = n_hull;
    }

    // The stand-in: two invented directions, kept for every title whose own
    // rig has not been recovered. See the module doc of
    // `crates/mesh/src/mesh_render/uniforms/light.rs`.
    let key = max(dot(n, normalize(vec3<f32>(0.4, 0.8, 0.5))), 0.0);
    let fill = max(dot(n, normalize(vec3<f32>(-0.5, 0.2, -0.7))), 0.0);
    let stand_in = vec3<f32>(0.15 + 0.75 * key + 0.25 * fill);

    var omega_snow_lit: OmegaSnow;
    if omega_snow_on {
        omega_snow_lit = omega_snow(omega_snow_entry, in, n_lit, uv_grad);
    }
    let ndl_raw = dot(n_lit, scene.light.direction);
    let ndl = clamp(ndl_raw, 0.0, 1.0);
    // The specular's facing term in every HD lit program: `MOV_SAT x8` of the unsaturated
    // `N.L` (`MUL R1.w, pow, sat(8 N.L)`), so the highlight is full strength from about
    // seven degrees off the horizon, not scaled by `N.L` itself. Read on the hull's own
    // `diffuse_with_specular_from_alpha_*` and the track's (`@0x3d` of `track_surface`'s lit block, live
    // on Modesto Heights' disc copy), which share the snippet. On the track, with the road's bump
    // in `N`, using it in place of `N.L` moves Vineta K's bottom band 17.74 to 14.29 and no pair the
    // wrong way by more than 0.03 (`docs/formats/rcsmaterial.md`, "The track's road and the
    // lightmapped glass", item 5).
    let ndl_spec = clamp(8.0 * ndl_raw, 0.0, 1.0);

    // **Everything from here to `authored` is the authored rig's, and only
    // runs when the rig is on.** Off - `Light::stand_in`, which is every
    // circuit with no `.envsettings` to read, Pulse's and Pure's included -
    // the output mixes this whole path in at weight zero, so running it would
    // cost a lightmap fetch, the sun-occlusion lookup and half a dozen `pow`s a
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
    // `sun_mask` below zero is `-1 - mask` on a chunk whose `VertexColour1` is a factor
    // (`oag_mesh::mesh::factor_sun_mask`); only a hull (`receives_shadow` not 2, the
    // track's) reads it as one - the track keeps the additive reading until it is
    // measured against its own references.
    //
    // **Both flags are tested half a unit inside their range, not on the boundary.** The mask
    // is interpolated across the triangle, and a surface whose every vertex is fully occluded
    // carries exactly `2.0` (nova) or `-1.0` (marked), so rounding in the interpolation lands
    // either side of a boundary test pixel by pixel: the occluded surface then reads as lit
    // wherever it falls on the wrong side, a binary speckle on a smooth surface.
    let marked = in.sun_mask < -0.5;
    let colour_factor = marked && receives_shadow != 2.0;
    // `sun_mask` at 2 or above marks a 2048 vertex-lit surface (`NOVAColor`): `colour` is its
    // baked light and `sun_mask - 2` the sun's mask (oag_mesh::mesh::rcs::psp2).
    let nova_vertex = nova_vertices != 0.0 && in.sun_mask >= 1.5;
    let sun_mask = select(
        select(in.sun_mask, -1.0 - in.sun_mask, marked),
        in.sun_mask - 2.0,
        nova_vertex,
    );
    var authored = vec3<f32>(0.0);
    // The sun's share of `authored`, kept apart so the track's shadowed redraw
    // can take exactly that out - see `sun_removed` below.
    var sun_light = vec3<f32>(0.0);
    var mask = 0.0;
    var emissive = false;
    if rig_on != 0.0 {
        let atlas = material_sample(MATERIAL_LIGHTMAP, in.lightmap_texcoord.xy);
        let baked = select(
            vec4<f32>(0.0, 0.0, 0.0, 1.0),
            atlas,
            (in.slots & 1u) != 0u,
        );

        // The authored rig, and the combination is the disc's, not this project's: it is
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
        // **Raw, no sRGB decode.** Block #9 is `TEX` then
        // `LG2`/`MUL`/`EX2` then `MAD` by the scale, and the RSX decodes no texture
        // on the disc (renderer.md, "HD's lightmap is read raw"), so the curve
        // runs on the stored byte, exactly as Omega's does below. A `pow(_, 2.2)`
        // decode of every sample (ADR-0026's) would darken it: on Amphiseum
        // (power 3.5) it compounds to a 7.7 power.
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
        let vertex_light = select(vec3<f32>(0.0), in.colour.rgb, colour_is_light > 0.5 || nova_vertex);

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
        // lightmap's `pow(_, 2.2)` step, and per vertex: `vs_main` curves it and the
        // interpolated result arrives in `lit_light.yzw`. `EMISSIVE` chunks (`no_ambient &&
        // no_sun`) are excluded from the curve below - this is additive to that
        // branch, not a rewrite of it.
        // A 2048 vertex-lit surface's curve is its vertex program's `scale * rgb + bias`
        // (no power), the Nova triple the lightmapped programs read as well.
        let vertex_light_curved = select(
            select(vec3<f32>(0.0), in.lit_light.yzw, colour_is_light > 0.5),
            scene.light.prelit_scale * vertex_light + vec3<f32>(scene.light.prelit_bias),
            nova_vertex,
        );
        let vertex_light_term = select(vertex_light, vertex_light_curved, no_ambient && !emissive);
        let ambient_term = select(scene.light.ambient, vec3<f32>(0.0), no_ambient || nova);

        // **The sun-occlusion mask.** Two independently
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
        // **A sun term belongs here**, though the general block #8/#9 formula
        // above has none: that formula is the read of two materials, and seven
        // read variant-by-variant say otherwise (`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`,
        // "The darkness gap does not reduce to an offset, a scale, or a single curve").
        // `diffuse_with_specular_from_alpha` (86 chunks of
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
        // **Why the mask is not optional.** A `sun * (ndl * baked.a)` summand with
        // `baked.a` fixed at the no-lightmap placeholder's `1.0` is full sun,
        // unoccluded, everywhere: measured at 14.3 % of the frame clipped to
        // white against the reference's 5.8 %, worse than no sun term at all.
        // **Times the road's mask under a craft**, `sun_occlusion` above: 1.0 on
        // every draw that is not a hull with a map, so nothing else here moves.
        // **Not even called on the track's pipelines** (`receives_shadow` 2,
        // `ShadowReceiver::Both`, which is the track and the gantry and never
        // a hull): its early return was already taken there, but the
        // function's four-tap loop held registers for it, 0.25 ms of HD's
        // 22.5 ms opaque track at 3200x1800 on the Raphael iGPU. Only a hull
        // names a layer, and a hull is `ShadowReceiver::Mapped`.
        mask = baked.a * select(sun_mask, 1.0, colour_factor);
        var hull_shadow = 1.0;
        if receives_shadow != 2.0 {
            hull_shadow = sun_occlusion(in.world);
            mask = mask * hull_shadow;
        }
        let sun_diffuse = scene.light.sun * (ndl * mask);
        // **`scene.light.ambient` and `vertex_light` are gated above** (`ambient_term`,
        // `vertex_light_term`), on `slots::NO_AMBIENT` - 251 of Anulpha Pass's 309
        // drawn materials, 922 of its 1,101 chunks. Gating `scene.light.ambient`
        // on that bit *alone* is a regression: the
        // materials without an ambient are three families, not one, and
        // `sign_emissive` and its kin (declaring only `fogColour`) went black
        // when multiplied by an `authored` with the ambient simply dropped. The
        // fix above is the drop *and* the vertex-colour curve together, which is
        // what keeps the `IleLightmap`/`IleVertex` families lit; `EMISSIVE`
        // below is the third family, excluded from both terms the same way.
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
        // **`VertexColour1` as a factor** (`oag_mesh::mesh::FACTOR_SUN_MASK`): the
        // hull programs `MUL` it into `ambient + sun * N.L` (and the `SVC1` twins
        // add the SPU light inside that same product), and carry no lightmap
        // term and no added vertex light - `skin/light_inputs.rs` has the
        // blocks.
        // **A hull's ambient is `4 * paraboloidIblTex[n]` where the draw names an
        // environment-probe layer** (the original's `HalfBrightIBL...` variants,
        // word `0x847`/`0x047`), the constant ambient everywhere else. The layer
        // is a uniform, so the fetch keeps its derivatives, and level 0 is the
        // only level the 64x32 map has. Both arms below take it: the hull programs
        // that multiply `VertexColour1` in and the ones that do not both read the
        // IBL variant of their own block.
        var hull_ambient = ambient_term;
        if uniforms.env_layer > 0.5 && !no_ambient {
            let sh = sh_hull_ambient(normalize(n_hull), hull_shadow);
            if sh.w > 0.5 {
                hull_ambient = sh.rgb;
            } else {
                hull_ambient = probe_tint(textureSampleLevel(
                    env_probe_tex, shadow_sampler, env_probe_irradiance_uv(env_probe_uv(n_hull)), env_probe_layer(0), 0.0,
                ).rgb);
            }
        }
        let spu = select(in.spu_light, vec3<f32>(0.0), simplified_lighting != 0.0);
        // **The tarmac roads' shadow-light factor on the prelit term** (Omega's lightmapped
        // programs, `docs/ghidra/functions/ps4-omega-eu/lightmap-prelit.md`):
        //
        //     Fsl = 0.75 + 0.25 sat(N.SL) + SLSD.y sat(R.SL)^SLSD.x
        //
        // `N` the mapped normal, `R` the view reflected about it, `SL` the circuit's authored
        // `Lighting.ShadowLight direction`. Applied to the road family read here; the other
        // lightmapped families carry it too and are unread (`omega-status.md`).
        var shadow_light_factor = 1.0;
        if tarmac && nova && scene.light.shadow_light.w > 0.5 {
            let seen = normalize(scene.fog.camera - in.world);
            let r = reflect(-seen, n_lit);
            let sl = scene.light.shadow_light.xyz;
            shadow_light_factor = 0.75 + 0.25 * clamp(dot(n_lit, sl), 0.0, 1.0)
                + scene.light.shadow_light_specular.y
                    * pow(clamp(dot(r, sl), 0.0, 1.0), scene.light.shadow_light_specular.x);
        }
        let lit_sum = select(
            hull_ambient + prelit * shadow_light_factor + vertex_light_term + sun_diffuse + spu,
            in.colour.rgb * (hull_ambient + sun_diffuse + spu),
            colour_factor,
        );
        authored = select(lit_sum, vec3<f32>(1.0), emissive);
        sun_light = select(
            select(sun_diffuse, in.colour.rgb * sun_diffuse, colour_factor),
            vec3<f32>(0.0),
            emissive,
        );
        // `water_test_2`: `ambient + sun * sat(N.L)` with no vertex light or
        // prelit term added (the vertex colour is the picture there).
        if (in.slots & 4194304u) != 0u && rate_from(row_scale.y, 2u) {
            authored = ambient_term + sun_diffuse;
            sun_light = sun_diffuse;
        }
        // The tapped water: the sea and the pool are `(ambient or the bake + sun * sat(N.L))
        // * ColourConstant` with no vertex light (the picture is the constant, set below);
        // `water_test_2` is `vertex colour * (ambient + sun * sat(N.L))`, the vertex colour
        // its picture.
        if water > 0.5 && water < 2.5 {
            authored = ambient_term + prelit + sun_diffuse;
            sun_light = sun_diffuse;
        } else if water > 2.5 {
            authored = ambient_term + sun_diffuse;
            sun_light = sun_diffuse;
        }
        // `cf_vex_billboard` (`Emissive::program` mode 1): `ambient + sun * sat(N.L)`, no
        // prelit term, no sun mask and no shadow - `oag_mesh::mesh::rcs::billboard_glass`.
        if program_in(row_scale.z, 1u, 1u) {
            authored = ambient_term + scene.light.sun * ndl;
            sun_light = scene.light.sun * ndl;
        }
        // **A plain chunk in a Zone frame** is an `Ambient` permutation: the frame has no
        // lightmap and no baked vertex light on any draw (`EnvSettings+0x4b0` is cleared), so
        // `albedo * (stage ambient + sat(N.L) * stage sun)`, with no occlusion mask. A program
        // fed no scene light (`emissive`) keeps drawing its picture.
        // Its `SVC1` twin adds the SPU vertex light to that sum, as every plain program does
        // (`spu` above).
        if zone_plain && !emissive {
            authored = scene.light.ambient + scene.light.sun * ndl + spu;
            sun_light = scene.light.sun * ndl;
        }
        // **A chunk on its Zone program** is the same `Ambient` permutation where the title's
        // Zone frame is read to be one (`shape.w`): `light * surface` with the stage's ambient
        // and sun as the light, never the circuit's lightmap or baked vertex light. A material
        // whose plain program is fed no scene light is lit here too:
        // every material's Zone variant has a compile that declares the ambient constant
        // (`docs/ghidra/functions/ps3-hdfury-eu/zone-frame.md`, "The Zone surface's light").
        if zone_program != 0.0 && scene.zone.shape.w > 0.5 {
            authored = scene.light.ambient + scene.light.sun * ndl;
            sun_light = scene.light.sun * ndl;
        }
        // **The two branches above again, unconditionally**, for a pipeline of a race where one
        // of them always applies (`zone_stage_light`): nothing then depends on the chain before
        // them, so the driver drops it. An emissive plain chunk takes neither and keeps its
        // program's `1.0` and no sun, which `specialise::zone_light_exact` has checked is what
        // the chain gives it (`docs/rendering/hd-zone-recolour.md`, "What a Zone race costs").
        if zone_stage_light != 0.0 {
            let own = zone_plain && emissive;
            let plain_spu = select(vec3<f32>(0.0), spu, zone_plain);
            authored = select(scene.light.ambient + scene.light.sun * ndl + plain_spu, vec3<f32>(1.0), own);
            sun_light = select(scene.light.sun * ndl, vec3<f32>(0.0), own);
        }
        // The `_shadow` crowd's sum: no prelit or vertex-light term, the material's `Ambient` added.
        if crowd_lit {
            authored = row_tint.rgb + scene.light.ambient + sun_diffuse;
            sun_light = sun_diffuse;
        }
        // Omega's snow: the Fresnel-weighted wrapped sun plus the Nova term, multiplying the picture
        // and the vertex tint (`omega_snow.wesl`); its specular replaces the generic one below.
        if omega_snow_on {
            authored = omega_snow_lit.diffuse;
            sun_light = omega_snow_lit.sun;
        }
        // Omega's rock: the Nova term times the shadow light's factor plus the wrapped sun gated by
        // the lightmap's alpha (`omega_rock.wesl`).
        if omega_rock_drawn {
            sun_light = scene.light.sun * (omega_rock_lit.wrapped * mask);
            authored = prelit * omega_rock_lit.shadow_factor + sun_light;
        }
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
    // **Not in a Zone race.** Every Zone variant of `etched_glass_tech` (blocks 10 to 19, the ones
    // that declare `zoneColourTint`) samples no facing ramp (`0x94b2b285`): the surface is the generic
    // Zone surface, as the magstrip's is (`docs/formats/rcsmaterial.md`, "Zone-mode wave materials").
    let ramp_sheen_bit = (in.slots & 1024u) != 0u;
    let ramp_sheen = ramp_sheen_bit && zone_program == 0.0;
    let view_dir = normalize(scene.fog.camera - in.world);
    let ramp_facing = dot(view_dir, n);
    let ramp_uv = vec2<f32>(ramp_facing, ramp_facing);
    let first_uv = select(in.texcoord, ramp_uv, ramp_sheen);
    // The model's own clock: `UV_offset`, `Shockwave_scalar` and `AlphaAnim` of
    // the weapon programs answered whole below are all `node + 0xc0`, the blast's
    // age, not the scene's `time`.
    let clock = uniforms.model_clock;
    // The Bomb's two programs are carried as pairs of the weapon bits and are
    // answered whole below (`slots::BOMB_FIRE`, `slots::BOMB_SHOCK`).
    let bomb_fire = (in.slots & 6144u) == 6144u;
    let bomb_shock = (in.slots & 98304u) == 98304u;
    // The Missile blast's three programs are pairs of the same bits
    // (`slots::MISSILE_CORE`, `_RAYS`, `_SHOCK`), answered whole below.
    let missile_core = (in.slots & 34816u) == 34816u;
    let missile_rays = (in.slots & 67584u) == 67584u;
    let missile_shock = (in.slots & 36864u) == 36864u;
    // The Plasma blast's three: the ring and the halo a bit each, the sphere a
    // pair (`slots::CLOCK_SCROLL_RING`, `_HALO`, `PLASMA_SPHERE`), compared under
    // the four weapon bits (`slots::WEAPON_PROGRAM_BITS`).
    let weapon_program = in.slots & 104448u;
    let plasma_ring = weapon_program == 32768u;
    let plasma_halo = weapon_program == 65536u;
    let plasma_sphere = weapon_program == 69632u;
    // **Pulse's slope-mode level selection**: `log2(|z| * slope) + bias` off view
    // depth, clamped to the levels the texture has - the GE's rule, in place of
    // the derivatives `textureSample` would use. On only for a model carrying
    // the disc's own chains; see `mesh_render::PSP_TEXLOD_SLOPE`. The player's
    // TEXTURE DETAIL shifts the level, which is one multiplier on the law.
    var first: vec4<f32>;
    if texlod_slope > 0.0 {
        let lod = max(
            ge_level(in.view_depth, in.lightmap_texcoord.w, scene.fog.texlod_frame) + scene.fog.texlod_shift,
            0.0,
        );
        first = sample_at_slope_level(first_uv, lod);
    } else {
        // The implicit-level fetch comes before the `panel` test, not in its `else`: `panel`
        // reads the interpolated tangent, which is not constant across a primitive, so a fetch
        // that only the other side of it runs would have derivatives in non-uniform flow. A
        // panel pixel pays the extra fetch; every other pixel runs the one it always ran.
        first = material_sample(MATERIAL_ALBEDO, first_uv);
        if panel {
            first = material_sample_grad(MATERIAL_ALBEDO, panel_uv, uv_grad[0], uv_grad[1]);
        }
    }
    // **A Detonator hull panel's iridescent overlay** (`slots` entry of rate 10,
    // `mesh::rcs::hull_overlay`): the second binding is a ramp the program reads at the facing
    // term `dot(N, V)` of the mapped normal, not at the surface's own coordinate
    // (`detonator_ship_dg_iridescent`, block `0x46d0` `@0x22`; `TC1` is eye minus position). It is
    // fetched here, in uniform control flow, in place of the ordinary second fetch.
    let overlay_hull = ((in.slots >> 24u) & 63u) != 0u && rate_in(row_scale.y, 10u, 10u);
    let overlay_facing = clamp(dot(view_dir, normalize(n_hull)), 0.0, 1.0);
    var second = vec4<f32>(0.0);
    if second_read != 0.0 {
        second = material_sample(
            MATERIAL_LIGHTMAP,
            select(in.texcoord, vec2<f32>(overlay_facing), overlay_hull),
        );
    }
    var picture = select(first, second, (in.slots & 2u) != 0u);
    // **HD's Sebenco ice pool** (`slots::ICE`, `oag_mesh::mesh::rcs::ice`): the
    // picture is replaced by the program's own lerp,
    //
    //     water = deep + F (cyan - deep);  ice = pale + F (snow - pale)
    //     A     = water + mask (ice - water)
    //
    // `F` the facing term, `mask` the pond map (third binding) at the second
    // UV set, `snow` the first binding, the three colours in three
    // consecutive glow-table entries. Chosen, not measured: `F` uses the
    // vertex normal, and the constants are display values decoded to the
    // domain the textures are sampled in.
    if water > 0.5 {
        // **The tapped water's picture** (`mesh::rcs::water`): `ColourConstant` (linear, the
        // entry's tint) for the sea and the pool, the vertex colour for `water_test_2`, black
        // for `water_noref`, whose colour is all reflection and glint. Opaque.
        let water_picture = select(
            select(vec3<f32>(0.0), in.colour.rgb, water < 3.5),
            row_tint.rgb,
            water < 2.5,
        );
        picture = vec4<f32>(encode(water_picture), 1.0);
    } else if (in.slots & 4194304u) != 0u && rate_from(row_scale.y, 2u) {
        // **`water_test_2`** (`mesh::rcs::water`): the program's picture is the
        // vertex colour itself (`TC1 * (ambient + sun * sat(N.L))`), kept in
        // the gamma domain the branch below decodes. The light is set where
        // `authored` is built.
        picture = vec4<f32>(encode(in.colour.rgb), picture.a);
    } else if (in.slots & 4194304u) != 0u {
        let ice_slot = (in.slots >> 24u) & 63u;
        let deep = decode(row_tint.rgb);
        let cyan = decode(emissive_tint(ice_slot + 1u).rgb);
        let pale = decode(emissive_tint(ice_slot + 2u).rgb);
        let pond = material_sample(MATERIAL_PAD_MASK, in.lightmap_texcoord.zw).x;
        let facing = clamp(ramp_facing, 0.0, 1.0);
        let water = deep + facing * (cyan - deep);
        let ice = pale + facing * (first.rgb - pale);
        picture = vec4<f32>(water + pond * (ice - water), picture.a);
    }
    // **HD's `nr_twinblend`** (`Emissive::program` mode 6, `oag_mesh::mesh::rcs::twin_blend`):
    //
    //     w   = (1 - wipe(uv_w.x + rate * time, uv_w.y).x)^4
    //     out = bars(uv_b) * lerp(first, second, w)
    //
    // the wipe third at `lightmap_texcoord.xy`, the bars fourth at `.zw`, `rate` the entry's
    // tint `x` (the vertex block's literal). The colour taps are decoded and the wipe read
    // raw (chosen, not measured); the product is put back in the domain `picture` is
    // decoded from below.
    let twin_entry = (in.slots >> 24u) & 63u;
    if twin_entry != 0u && program_in(row_scale.z, 6u, 6u) {
        let wipe_uv = vec2<f32>(
            in.lightmap_texcoord.x + row_tint.x * scene.time.x,
            in.lightmap_texcoord.y,
        );
        let fade = 1.0 - material_sample(MATERIAL_PAD_MASK, wipe_uv).x;
        let w = (fade * fade) * (fade * fade);
        let bars = decode(material_sample(MATERIAL_WAVE_MAP, in.lightmap_texcoord.zw).rgb);
        let linear = bars * mix(decode(first.rgb), decode(second.rgb), vec3<f32>(w));
        picture = vec4<f32>(encode(linear), first.a);
    }
    // **2048's `fc07_blend_vertred_lambert_2diff`** (`Emissive::program` mode 18,
    // `oag_mesh::mesh::rcs::psp2::vertex_blend`), read off its fragment programs:
    //
    //     albedo = mix(DiffuseMap^2, DiffuseMap01^2, VertexColour1.r)
    //
    // the second picture third, at the first's coordinate, the weight in the second coordinate
    // pair's `x`. Mixed decoded (`decode` is the square on 2048) and put back in the domain
    // `picture` is decoded from below; the light and the alpha are the first picture's own.
    let blend_entry = (in.slots >> 24u) & 63u;
    if blend_entry != 0u && program_in(row_scale.z, 18u, 18u) {
        let other = material_sample(MATERIAL_PAD_MASK, first_uv);
        let w = clamp(in.lightmap_texcoord.z, 0.0, 1.0);
        let linear = mix(decode(first.rgb), decode(other.rgb), vec3<f32>(w));
        picture = vec4<f32>(encode(linear), first.a);
    }
    // **Omega's `tarmac_markings_dirt_markingsnormal[_emis]` roads** (`Emissive::program` mode 14,
    // `oag_mesh::mesh::rcs::psp2::tarmac`), read off the pixel programs:
    //
    //     c = lerp(c, dirt.rgb,     dirt.a)       dirt     (sixth binding) at `colour.xy`
    //     c = lerp(c, markings.rgb, markings.a)   markings (fifth binding) at `lightmap_texcoord.zw`
    //
    // over the albedo, dirt first. The vertex colour carries the dirt pair here, so the tint
    // below is white on this mode. The taps are mixed in the domain `picture` is sampled in.
    if tarmac {
        let dirt = material_sample(MATERIAL_SIXTH_MAP, in.colour.xy);
        let markings = material_sample(MATERIAL_FIFTH_MAP, in.lightmap_texcoord.zw);
        let laid = mix(mix(picture.rgb, dirt.rgb, dirt.a * row_tint.y), markings.rgb, markings.a);
        picture = vec4<f32>(laid, picture.a);
    }
    // **HD's `uvdistortion_*plusoverlay*` banner** (`Emissive::program` mode 13,
    // `oag_mesh::mesh::rcs::uv_overlay`): the grille is the lit albedo and the program adds,
    // after the light,
    //
    //     a    = VertexColour1 * teal * clouds((uv + A t + D clouds(uv).a) S) * grille.a
    //     b    = logo((uv + B t) R)
    //     over = a < 0.5 ? 2 a b : 1 - 2 (1 - a)(1 - b)
    //
    // the clouds second, the logo third, `teal` and `D` the entry's tint and offset, `A`, `S`,
    // `B` and `R` the next entry's tint `x`, `y`, `z` and offset. The colour taps are decoded
    // and the clouds' alpha read raw (chosen, not measured); the sum rides `mag_glow` below.
    var uv_overlay = vec3<f32>(0.0);
    let overlay_entry = (in.slots >> 24u) & 63u;
    if overlay_entry != 0u && program_in(row_scale.z, 13u, 13u) {
        let values = emissive_tint(overlay_entry + 1u);
        let clock_now = scene.time.x;
        let warp = material_sample(MATERIAL_LIGHTMAP, in.texcoord).a;
        let cloud_uv = (in.texcoord + vec2<f32>(values.x * clock_now + row_tint.w * warp)) * values.y;
        let cloud = decode(material_sample(MATERIAL_LIGHTMAP, cloud_uv).rgb);
        let logo_uv = (in.texcoord + vec2<f32>(values.z * clock_now)) * values.w;
        let logo = decode(material_sample(MATERIAL_PAD_MASK, logo_uv).rgb);
        let a = in.colour.rgb * row_tint.rgb * cloud * first.a;
        uv_overlay = select(
            vec3<f32>(1.0) - 2.0 * (vec3<f32>(1.0) - a) * (vec3<f32>(1.0) - logo),
            2.0 * a * logo,
            a < vec3<f32>(0.5),
        );
    }
    // **The 2048 lineage's shield shells, and HD's** (`Emissive::program` mode 11,
    // `oag_mesh::mesh::rcs::psp2::shield_lightning` and `oag_mesh::mesh::rcs::shield_scroll`),
    // read off their pixel programs:
    //
    //     colour    = ShieldColour * diffuse(u + A time, v)
    //     noise     = lightning(u2, B v2 + C time).a
    //     lightning = M * lightning(uv2 + time + noise).rgb
    //
    // the lightning third, `M`, `A`, `C` and `B` the entry's tint `x`, `y`, `z` and offset. HD's
    // shell has no lightning (`M` is zero), only the scrolled colour tap. The diffuse's alpha
    // stays the unscrolled tap's, and the lightning is added after the shell's `ShieldColour`
    // multiply below, as the program does not tint it. The Vita's program squares the
    // lightning tap, its gamma-2 scene's decode; Omega's takes it as the sampler gives it.
    var shield_lightning = vec3<f32>(0.0);
    let lightning_entry = (in.slots >> 24u) & 63u;
    if lightning_entry != 0u && program_in(row_scale.z, 11u, 11u) {
        let row = row_tint;
        let clock_now = scene.time.x;
        let scrolled = material_sample(MATERIAL_ALBEDO, vec2<f32>(in.texcoord.x + row.y * clock_now, in.texcoord.y));
        picture = vec4<f32>(scrolled.rgb, picture.a);
        let uv2 = in.lightmap_texcoord.zw;
        let noise = material_sample(MATERIAL_PAD_MASK, vec2<f32>(uv2.x, row.w * uv2.y + row.z * clock_now)).a;
        let bolt = material_sample(MATERIAL_PAD_MASK, uv2 + vec2<f32>(clock_now + noise)).rgb;
        shield_lightning = row.x * mix(bolt, bolt * bolt, square_decode * linear_out);
    }
    // **HD's Zone-mode light cells** (`slots::MAG_WAVE` with the glow entry's rate 3,
    // `oag_mesh::mesh::rcs::damage_lights`): the picture gains a scrolling flash,
    //
    //     flash = mask(uv2 + (rate * time, -shift)) * (blue(uv2 + (shift, 0)) - C) + C
    //
    // `mask` is the fourth binding, `blue` the third, `C` the entry's tint, the scroll
    // rate its offset and `shift` its scale. The program adds the picture itself unlit
    // as well (`out = P * (light + 1)`), which `mag_glow` carries below.
    var cell_picture = vec3<f32>(0.0);
    let cell_entry = (in.slots >> 24u) & 63u;
    let is_cell = (in.slots & 131072u) != 0u && cell_entry != 0u
        && rate_in(row_scale.y, 3u, 3u);
    if is_cell {
        let shift = row_scale.x;
        let flash_c = row_tint.rgb;
        let uv2 = in.lightmap_texcoord.zw;
        let mask = material_sample(MATERIAL_WAVE_MAP, vec2<f32>(uv2.x + row_tint.w * scene.time.x, uv2.y - shift),
        ).x;
        let blue = material_sample(MATERIAL_PAD_MASK, vec2<f32>(uv2.x + shift, uv2.y)).rgb;
        picture = vec4<f32>(picture.rgb + mask * (blue - flash_c) + flash_c, picture.a);
        cell_picture = picture.rgb;
    }
    // **A PSP hull's alpha-weighted extra pass** (`PROGRAM_SHINE_MASK`, `oag_mesh::mesh::shine_pass`):
    // the environment map times the batch's own texture's alpha, bound second and read at
    // the batch's authored coordinate - the `GU_SRC_ALPHA` the original's ordinary pass
    // blends this pass's result by (`BLENDMODE 0x2a`, read off a recorded GE list of Pure).
    let mask_entry = (in.slots >> 24u) & 63u;
    if mask_entry != 0u && program_in(row_scale.z, 5u, 5u) {
        picture = vec4<f32>(picture.rgb * material_sample(MATERIAL_LIGHTMAP, in.lightmap_texcoord.xy).a, picture.a);
    }
    // **HD's `dc_hologramwithstatic`** (`Emissive::program` mode 7,
    // `oag_mesh::mesh::rcs::static_hologram`):
    //
    //     colour = advert(u + S time, v) + static(u, v + V time)
    //     alpha  = scan(u, v).x * K
    //
    // the advert second, the static third (`MATERIAL_PAD_MASK`), the scanline mask first;
    // `S`, `V` and `K` the entry's tint `x`, `y`, `z`. **All three units are sRGB-decoded on
    // fetch**, read live (RPCS3, Amphiseum time trial start, draw 700: `SET_TEXTURE_ADDRESS`
    // `0x60710101` on units 0, 1 and 2, gamma nibble 7), so the mask's red is decoded too;
    // the colour sum, halved by the program's closing `/2` (`scene.light.output_scale`), is put
    // back in the domain `picture` is decoded from below.
    var coverage = select(first, second, (in.slots & 4u) != 0u);
    if twin_entry != 0u && program_in(row_scale.z, 7u, 7u) {
        let holo = row_tint;
        let advert = material_sample(MATERIAL_LIGHTMAP, vec2<f32>(in.texcoord.x + holo.x * scene.time.x, in.texcoord.y));
        let noise = material_sample(MATERIAL_PAD_MASK, vec2<f32>(in.texcoord.x, in.texcoord.y + holo.y * scene.time.x));
        let linear = scene.light.output_scale * (decode(advert.rgb) + decode(noise.rgb));
        picture = vec4<f32>(encode(linear), picture.a);
        coverage = vec4<f32>(decode(vec3<f32>(first.r)).r * holo.z);
    }
    if vita_water {
        // **2048's seas multiply the light by the uniform as it is** (`fc15` lit fragment `@22`
        // and `fc17` `@23`, lightmapped `@24`: `mul light.xyz, ColourConstant` or `Watercolour`),
        // where a picture is squared first: `encode` hands the entry's constant to the `decode`
        // every picture takes below, which returns it unchanged (`docs/rendering/water.md`,
        // "2048 (Vita)", "The constant is not squared"). Omega's seas (kinds 9 and 10) keep the
        // decoded constant.
        picture = vec4<f32>(select(encode(row_tint.rgb), row_tint.rgb, omega_sea), picture.a);
    }
    let texel = vec4<f32>(picture.rgb, coverage[(in.slots >> 3u) & 3u]);

    // **The glass sheen's combine**, block #7 of `etched_glass_tech`
    // (`rcsmaterial.md`, "The glass floor's gamma, curve and sun weight"):
    //
    //     light = pow(vertexColour, power) * scale + sunMask * sun * sat(N.L)
    //     rgb   = light * (ramp + c) + ramp + grid.r,   alpha = grid.r
    //
    // with `ramp` and `grid` sRGB-decoded (the captured draw's texture address
    // words carry gamma bits `0x7` on both) and `c` the material's
    // `0x512f8e65`, carried in the glow-table entry `glass_sheen::bias` writes.
    // The sun weight is the colour set's fourth byte (`o[TC5].x = v[3].w` in
    // the paired vertex program), `in.sun_mask`, not the UV's `u`. Left out,
    // named: `paraboloidReflectionTex`'s tint (weighted by the grid red),
    // which needs a dual-paraboloid probe this renderer does not have.
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
    let glow_slot = (in.slots >> 24u) & 63u;
    let glow_tint_offset = row_tint;
    let glow_scale = row_scale;
    let ramp_sun = scene.light.sun * ndl;
    let sheen_prelit = scene.light.prelit_scale * pow(in.colour.rgb, scene.light.prelit_power);
    // **The lightmapped variant** (`SECOND_IS_LIGHTMAP` beside the sheen bit, fragment `0x7704c0`
    // on Tech de Ra, read live): the atlas takes `lightmap`, the grid moves to the third binding,
    // and the light is `TC0 + scale * atlas^power + atlas.a * N.L * sun` - the atlas alpha is the
    // sun mask, where the unlit-by-atlas variant weights the sun by the colour set's fourth byte.
    // The atlas is raw (its address word's gamma field is 0). Fetched under explicit gradients so
    // the branch stays out of the derivative-uniform flow.
    let sheen_lightmapped = ramp_sheen_bit && (in.slots & 1u) != 0u;
    let sheen_dx = dpdx(in.texcoord);
    let sheen_dy = dpdy(in.texcoord);
    var vertex_light_sheen = sheen_prelit + in.sun_mask * ramp_sun;
    var sheen_grid = second.r;
    if sheen_lightmapped {
        let atlas_dx = dpdx(in.lightmap_texcoord.xy);
        let atlas_dy = dpdy(in.lightmap_texcoord.xy);
        let glass_atlas = textureSampleGrad(
            lightmap, albedo_sampler, in.lightmap_texcoord.xy,
            atlas_dx * exp2(material_lod_bias), atlas_dy * exp2(material_lod_bias),
        );
        vertex_light_sheen = sheen_prelit
            + scene.light.prelit_scale * pow(glass_atlas.rgb, scene.light.prelit_power)
            + glass_atlas.a * ramp_sun;
        sheen_grid = textureSampleGrad(
            pad_mask, albedo_sampler, in.texcoord,
            sheen_dx * exp2(material_lod_bias), sheen_dy * exp2(material_lod_bias),
        ).r;
    }
    // **Both taps are sRGB-decoded**, as the RSX does: the capture of this
    // material's draw on RPCS3 has `SET_TEXTURE_ADDRESS` bits 20-23 (the
    // gamma field) at `0x7` on the ramp (unit 2) and the grid (unit 0) and
    // `0x0` on the normal maps, so the arithmetic below runs on linear values
    // and so does the output alpha (`MOV H0.w, H6.x`).
    let ramp_linear = decode(first.rgb);
    let grid_linear = decode(vec3<f32>(sheen_grid)).r;
    // **Times the program's output scale**, as the authored path above is: block #7 ends on
    // `ADD /2 H0.xyz, R0, H2` (read live on Tech de Ra, fragment `0x76fac0`), so the scene holds
    // half of the sum. The output alpha, `MOV H0.w, H6.x`, is not scaled.
    let sheen_linear = scene.light.output_scale
        * (vertex_light_sheen * (ramp_linear + vec3<f32>(glow_tint_offset.w))
            + ramp_linear
            + vec3<f32>(grid_linear));
    // Unclamped on the linear target, whose bloom gate reads values above 1;
    // clamped and encoded on a gamma one, as the authored path does.
    let sheen_rgb = mix(
        pow(clamp(sheen_linear, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2)),
        sheen_linear,
        linear_out,
    );
    let sheen = vec4<f32>(sheen_rgb, grid_linear);
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
    //
    // **Two materials multiply this sample instead of adding it**
    // (`Emissive::program`, `oag_mesh::mesh::rcs::billboard_glass`): mode 1, the lit
    // billboard, whose glow tap's `u` is scaled too, and mode 2, Tech de Ra's stand glass,
    // which reads `bwgradient` at the chunk's second coordinate set.
    let glow_program = row_scale.z;
    let glow_is_billboard = glow_program > 0.5 && glow_program < 1.5;
    let glow_is_bw_glass = glow_program > 1.5 && glow_program < 2.5;
    let glow_u_scale = select(1.0, row_scale.w, row_scale.w > 0.0);
    let glow_uv = select(
        vec2<f32>(in.texcoord.x * glow_u_scale, glow_v),
        in.lightmap_texcoord.zw,
        glow_is_bw_glass,
    );
    var glow_sample = vec4<f32>(0.0);
    if (in.slots & 256u) != 0u {
        glow_sample = material_sample(MATERIAL_LIGHTMAP, glow_uv);
    }
    // What a texel-sourced program mask reads - see `glow.wesl`'s `program_glow`.
    glow_first = first;
    glow_second = select(second, glow_sample, (in.slots & 256u) != 0u);
    glow_pad = pad_ne_texel;
    // Gated by the *diffuse* alpha, which is `first.a` rather than the
    // resolved `texel.a`: the microcode's `H0.wwww` is the unit-0 fetch's own
    // fourth channel, before any of the role bits above choose where the
    // output's alpha comes from.
    let glow_gate = first.a * select(0.0, 1.0, (in.slots & 256u) != 0u && glow_program < 0.5);
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
        decode(glow_sample.rgb) * glow_tint_offset.rgb * glow_gate;
    // **HD's pad light bars**: `_ne.a * colour`, the program's `MAD H.xyz,
    // R0.wwww, C, lit` - added after the light on the far side of the albedo
    // multiply, ungated by the diffuse alpha, and not decoded in either
    // domain (an alpha and an authored magnitude). The colour is the pad's
    // own authored `W_Cycle`/`Colour`, riding in this material's glow-table
    // tint; zero for every other material. Fog follows in `fogged`, which is
    // where the program's own final `MAD` puts it too.
    // **2048's `*_emmissive_alpha` add** (`Emissive::program` mode 8, `mesh::rcs::psp2::emissive_add`):
    // `Texture2Alpha.rgb^2 * Constant1` joins the pad glow, added after the light and before the
    // fog, so it is neither lit nor shadowed (mode 9, the plain `*_emmissive`: `Texture2.a * Constant1`, unsquared). The picture is the third binding, the tint the
    // entry's. `decode` is the square on 2048.
    var emissive_add = vec3<f32>(0.0);
    // **The tarmac roads' `EmissiveTexture`** (mode 14): the fourth binding's rgb at the diffuse
    // coordinate, weighted by the entry's tint `x` (1 when the material has one, else 0), the dirt weighted by
    // tint `y` (`DIRT_SWITCH`), added
    // after the light like the mode 8 term, as the pixel programs add it
    // (`out = emissive + ambient * c`).
    if tarmac {
        emissive_add = decode(material_sample(MATERIAL_WAVE_MAP, in.texcoord).rgb) * glow_tint_offset.x;
    }
    // The panel-map road's `EmissiveMap` (mode 15), added unscaled at the marched coordinate, where
    // the material has one (the second entry's tint `z`).
    if panel && emissive_tint(panel_entry + 1u).z > 0.5 {
        let e = material_sample_grad(MATERIAL_WAVE_MAP, panel_uv, uv_grad[0], uv_grad[1]);
        emissive_add = decode(e.rgb);
    }
    if glow_program > 7.5 && glow_program < 9.5 {
        let mask = material_sample(MATERIAL_PAD_MASK, in.texcoord);
        emissive_add = select(vec3<f32>(mask.a), decode(mask.rgb), glow_program < 8.5) * glow_tint_offset.rgb;
    }
    // **The emissive of a lightmapped HD slot** (modes 31 and 32,
    // `oag_mesh::mesh::rcs::lightmapped_emissive`): added after the light, on a plain program only
    // (the Zone variants declare no emissive), so a chunk on its Zone program adds nothing and a
    // `ZONE_PLAIN` chunk keeps it. Mode 31 is `E * tint` at the diffuse coordinate, ungated by the
    // diffuse alpha, bound third (fourth where the second lane is set: the track's third is its
    // normal map). Mode 32 samples the third at the scrolled `glow_uv` and gates it by the diffuse
    // alpha. The tap is decoded like every colour unit (its gamma nibble was not read live).
    if glow_program > 30.5 && glow_program < 31.5 {
        var tap = material_sample(MATERIAL_PAD_MASK, in.texcoord);
        if row_scale.w > 0.5 {
            tap = material_sample(MATERIAL_WAVE_MAP, in.texcoord);
        }
        emissive_add = decode(tap.rgb) * glow_tint_offset.rgb * (1.0 - zone_program);
    }
    if glow_program > 31.5 && glow_program < 32.5 {
        emissive_add = decode(material_sample(MATERIAL_PAD_MASK, glow_uv).rgb) * glow_tint_offset.rgb
            * first.a * (1.0 - zone_program);
    }
    // **2048's `fc10_Lambert_2UV_Emissive`** (mode 19, `mesh::rcs::psp2::emissive_add`): `D.a * R^2`
    // after the light, `R` the third binding (`ReflectMap`) at `0.5 + 0.5 * reflect(-V, N).xy`.
    if glow_program > 18.5 && glow_program < 19.5 {
        let r = reflect(-normalize(scene.fog.camera - in.world), n);
        let map = material_sample(MATERIAL_PAD_MASK, vec2<f32>(0.5) + 0.5 * r.xy);
        emissive_add = decode(map.rgb) * texel.a * glow_tint_offset.rgb;
    }
    // **2048's `dc_diffusetexturealphaemissive`** (mode 20): the lit picture's square joins the sum
    // (`mad light, D^2, D^2`), neither lit nor shadowed.
    if glow_program > 19.5 && glow_program < 20.5 {
        emissive_add = decode(picture.rgb) * glow_tint_offset.rgb;
    }
    // **HD's emissive-in-alpha families** (`Emissive::program` mode 12, `mesh::rcs::alpha_emissive`):
    // `MAD H.xyz, T.wwww, C, lit`, `T` the picture's own fetch and `C` the material's emissive
    // colour in the entry's tint. The mask is the unit-0 alpha; a `dn_diffuse_specular_emiss` entry (second lane set) binds its SpecularTexture third, read in the specular below. The Zone
    // variants of these programs do not declare the colour, so a Zone race adds nothing.
    if glow_program > 11.5 && glow_program < 12.5 {
        emissive_add = first.a * glow_tint_offset.rgb * (1.0 - zone_program);
    }
    // No Zone variant of an `_ne` program on the disc adds its light-bar colour (77 materials,
    // `hd_zone_ne_census_ground_truth`), so a chunk on the Zone surface drops the term; a pad drawn
    // on its plain program in a Zone race (`ZONE_PLAIN`) keeps it.
    let pad_glow = pad_ne_texel.a * glow_tint_offset.rgb * select(0.0, 1.0, pad_ne) * (1.0 - zone_program)
        + emissive_add;

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
    var mag_glow = decode(cell_picture) + uv_overlay;
    if (in.slots & 131072u) != 0u && !is_cell && zone_program == 0.0 {
        let e = material_sample(MATERIAL_PAD_MASK, in.texcoord);
        let w = material_sample(MATERIAL_WAVE_MAP, in.texcoord * glow_scale.x + vec2<f32>(scene.time.x * glow_scale.y),
        );
        let m = mix(e.rgb, w.rgb, e.a);
        mag_glow = min(
            e.rgb / max(vec3<f32>(1.0) - m * glow_tint_offset.rgb, vec3<f32>(0.05)),
            vec3<f32>(1.0),
        );
    }
    let mag_glow_gamma = encode(mag_glow);

    // **HD's chevron pulse** (`Emissive::program` mode 3 on a `slots::LIT_ALPHA` slot,
    // `oag_mesh::mesh::rcs::chevron_pulse`): the emask `e` (fifth binding) adds to the
    // albedo's alpha inside the light multiply, and `e.a * Colour * wave` (sixth binding, at
    // the coordinate the vertex stage laid) adds after it: `ADD H6, H4.wwww, H6` then
    // `MAD H5, light, H6, H7`. Both decoded like every colour unit read live so far (gamma
    // nibble 7 on the albedo, the probes and the leach noise): **chosen, not measured** for
    // these two units, whose address words were not read; the alpha they join is raw.
    // **Not in a Zone race**: every Zone variant of `cf_chevron_pulse` (blocks #15-#34, #45-#64,
    // #75-#93) declares neither the emask nor the wave, as the magstrips' do not (`mag_glow`).
    // 2048's and Omega's centre lane (the entry's `program[1]` 2) lights its colour and takes the
    // same entry without `LIT_ALPHA`.
    let chevron = ((in.slots & 2147483648u) != 0u || row_scale.w > 1.5) && glow_program > 2.5
        && glow_program < 3.5 && zone_program == 0.0;
    var chevron_surface = vec3<f32>(0.0);
    var chevron_glow = vec3<f32>(0.0);
    if chevron {
        let e = material_sample(MATERIAL_FIFTH_MAP, in.texcoord);
        let w = material_sample(MATERIAL_SIXTH_MAP, in.lightmap_texcoord.zw);
        let e_linear = decode(e.rgb);
        let w_linear = decode(w.rgb);
        // **2048's and Omega's combines** (the entry's `program[1]`, `oag_mesh::mesh::rcs::psp2`'s
        // `chevron`, `docs/rendering/hd-track-tone.md`, "2048 and Omega"): no mask inside the light;
        // after it the chevron's `max(E, Colour * mix(E, W, E.a))` (1) or the centre lane's
        // `E.a * W`, HD's own glow (2).
        let lineage = row_scale.w > 0.5 && row_scale.w < 1.5;
        chevron_surface = select(e_linear, vec3<f32>(0.0), row_scale.w > 0.5);
        chevron_glow = select(
            e.a * glow_tint_offset.rgb * w_linear,
            max(e_linear, glow_tint_offset.rgb * mix(e_linear, w_linear, e.a)),
            lineage,
        );
    }

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
    // **A surface the Zone grade is switched off for** (`ZONE_EXEMPT_RATE`): the
    // glow-table entry its material index names says so. Applied to the three
    // mix weights and not to the `enabled` branch below, which has to stay uniform.
    let zone_entry = (in.slots >> 24u) & 63u;
    let zone_weight = select(
        zone_on,
        0.0,
        zone_plain || (zone_entry != 0u && rate_in(row_scale.y, 4u, 7u)),
    );
    var zone_linear = vec3<f32>(0.0);
    var zone_gamma = vec3<f32>(0.0);
    var zone_glow_term = vec3<f32>(0.0);
    var zone_glow_gamma = vec3<f32>(0.0);
    // How much of the albedo the Zone surface replaces: all of it on a circuit,
    // none of it on an arena, where the zone colour is added instead
    // (`zone_add_*`). See `mesh_render::Zone::shape`.
    var zone_replace = 1.0;
    var zone_add_linear = vec3<f32>(0.0);
    var zone_add_gamma = vec3<f32>(0.0);
    if zone_on != 0.0 {
        let zone_to_eye = normalize(scene.fog.camera - in.world);
        let zone_colours = zone_set(in.slots, in.world);
        // `1 - |N.V|`: the microcode's `ADD rim, -|dot|, 1`, so a face turned
        // away from the eye rims like one turned towards it (`zone-shader.md`,
        // "The rim is two-sided").
        let zone_rim = 1.0 - abs(dot(n, zone_to_eye));
        let zone = zone_sample(in.texcoord, in.slots, in.world);
        if scene.zone.shape.x > 0.5 {
            // The arena shape: `surface = albedo + zoneCol * (1 - blackMask)`,
            // `zoneCol = zoneTex * zoneEffect + 2 * zoneAnisoPalette[pow(rim,
            // zoneAnisoPower)]`, `blackMask = saturate((albedo.r + albedo.g +
            // albedo.b) * 100000)` - the zone colour lands only where the
            // material's own albedo is black (`zone-shader.md`, 82).
            let zone_black = 1.0 - clamp((texel.r + texel.g + texel.b) * 100000.0, 0.0, 1.0);
            let zone_rim_colour = 2.0 * zone_aniso(zone_rim, in.slots, in.world);
            zone_replace = 0.0;
            zone_add_linear = (decode(zone) * zone_colours.effect.rgb + zone_rim_colour) * zone_black;
            zone_add_gamma = (zone * zone_colours.effect.rgb + zone_rim_colour) * zone_black;
        } else {
            let zone_base = zone_base_term(zone_rim, zone_colours);
            zone_linear = decode(zone) * zone_colours.effect.rgb + zone_base;
            zone_gamma = zone * zone_colours.effect.rgb + zone_base;
        }
        // The visualiser glow - see `zone_glow`. Gated by `enabled` explicitly
        // rather than trusting `effect.w` to be zero off a Zone race, the same
        // belt-and-braces `surface_linear`/`plain` below already take.
        let zone_vis = zone_glow(n, in.clip, in.texcoord, in.slots, in.world);
        zone_glow_term = zone_vis.linear * zone_weight;
        zone_glow_gamma = zone_vis.gamma * zone_weight;
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
    // What the sun adds to `authored_rgb`, in its output space.
    var sun_removed = vec3<f32>(0.0);
    var zone_after = vec3<f32>(0.0);
    var zone_redraw = vec3<f32>(0.0);
    if rig_on != 0.0 {
        let half_vector = to_eye + scene.light.direction;
        let ndh = clamp(
            dot(half_vector, n_lit) / max(length(half_vector), 1e-6),
            0.0,
            1.0,
        );
        // `NO_SPECULAR` (`oag_mesh::mesh::NO_SPECULAR`) is a program with no `pow(N.H, e)`
        // in it (no `LG2` reads a dot product, `Program::has_dot_power`), hull or track
        // surface; any other draw keeps the shared stand-in there.
        let no_specular = in.specular_exponent <= 0.0;
        var specular_exponent = select(in.specular_exponent, 32.0, in.specular_exponent <= 0.0);
        // **`slots::LIT_ALPHA`**: the program lights the albedo's alpha and tints the
        // specular with its RGB (`light * albedo.a + spec * albedo.rgb`, read off
        // `reversed_diffuse_specular_normal` and `cf_chevron_pulse` on Anulpha Pass);
        // every other lit program is the other way round (`light * albedo.rgb +
        // spec * albedo.a`). The albedo unit decodes its RGB (gamma nibble 7) and
        // not its alpha, so the tint is the decoded colour and the grey albedo the
        // raw alpha. `docs/rendering/hd-track-tone.md`, "The blue cast".
        let lit_alpha = (in.slots & 2147483648u) != 0u;
        var albedo_linear = decode(texel.rgb);
        var specular_mask = select(texel.a, 1.0, lit_alpha);
        var specular_tint = select(vec3<f32>(1.0), albedo_linear, lit_alpha);
        // **Sebenco's snow-capped rock**: the snow shaded toward `pale` by `N.V` (the bumped
        // normal), the rock mixed in by the mask, and the exponent and the alpha-weighted
        // specular colour each mixed the same way - the program's `@0x29`..`@0x43`. `pale` is
        // the program's constant beside the decoded texels, as its arithmetic has it.
        if snow {
            let snow_c = row_tint;
            let shaded = mix(snow_c.rgb, albedo_linear, dot(n_lit, to_eye));
            albedo_linear = mix(shaded, decode(rock.rgb), snow_mask);
            specular_exponent = mix(snow_c.w, row_scale.w, snow_mask);
            specular_mask = 1.0;
            specular_tint = mix(
                texel.a * emissive_tint(snow_entry + 1u).rgb,
                rock.a * emissive_tint(snow_entry + 2u).rgb,
                snow_mask,
            );
        }
        // **A pane's specular** is weighted by `1 - a` alone (`pane_clear`, below): the traced
        // `glass_texture_customr` program has no alpha gloss term, so the generic gloss-in-alpha
        // mask must not weight it a second time.
        let pane_entry = (in.slots >> 24u) & 63u;
        if pane_entry != 0u
            && rate_in(row_scale.y, 9u, 9u)
            && row_tint.y > 0.5
        {
            specular_mask = 1.0;
        }
        // **A window whose specular is multiplied by a map** (the entry's `program[1]`,
        // `pb_glass_2nduv_reflect_specular1_glow`): `specular * S.rgb`, `S` the second binding at
        // the picture's coordinate. The picture's alpha gates the reflection there, not the gloss.
        if pane_entry != 0u
            && rate_in(row_scale.y, 9u, 9u)
            && row_scale.w > 0.5
        {
            specular_mask = 1.0;
            specular_tint = decode(second.rgb);
        }
        // **A specular multiplied by a map** (`Emissive::program[1]` on a mode 12 entry,
        // `dn_diffuse_specular_emiss`): `spec * S.rgb`, `S` the third binding at the picture's
        // coordinate. No alpha gates it: the picture's alpha is the glow mask, not a gloss.
        if glow_program > 11.5 && glow_program < 12.5 && row_scale.w > 0.5 {
            specular_mask = 1.0;
            specular_tint = decode(material_sample(MATERIAL_PAD_MASK, in.texcoord).rgb);
        }
        var specular = vec3<f32>(0.0);
        if rig_specular != 0.0 {
            specular = scene.light.sun
                * (pow(ndh, specular_exponent) * ndl_spec * mask * specular_mask * scene.light.specular_scale
                    * rig_on * in.lit_light.x * select(1.0, 0.0, emissive))
                * specular_tint
                * select(vec3<f32>(1.0), in.colour.rgb, colour_factor)
                * select(1.0, 0.0, no_specular);
        }
        // The sea's own specular: `sun * SpecularColour * sat(N.H)^SpecularPower * sat(8 N.L)`
        // times the bake's sun mask (`@0x2d`..`@0x5b` of its lit block), the colour in the tap
        // entries' `program` lanes and the power in `scale`. The pool and `water_test_2` have none; `water_noref`
        // keeps the generic glint.
        if water > 0.5 && water < 3.5 {
            specular = scene.light.sun * water_specular_colour(water_entry)
                * (pow(ndh, row_scale.x) * ndl_spec * mask * rig_on * select(0.0, 1.0, water < 1.5));
        }
        // 2048's sea: `liveLighting0specular * t.a * sat(R.L)^SpecularPower`, `R` the view
        // reflected about the mapped normal, `t.a` the first tap's alpha (fc15 fragment `@0x0c`
        // to `@0x18`). No `sat(8 N.L)` gate. **`liveLighting0specular` is the sun's diffuse
        // colour here, chosen, not measured:** 2048's envsettings author no sun specular colour
        // or intensity (Omega's carry both), so the loader's `specular_scale` reads 0 and is
        // not applied.
        if vita_water {
            let gloss = material_sample(MATERIAL_PAD_MASK, water_tap_uv(water_entry, water_raw, 0u, in.texcoord)).a;
            let reflected = reflect(-to_eye, n_lit);
            let facing = clamp(dot(reflected, scene.light.direction), 0.0, 1.0);
            let highlight = select(vec3<f32>(1.0), water_specular_colour(water_entry), vita_fc17);
            specular = scene.light.sun * (pow(facing, row_scale.x) * select(gloss, 1.0, vita_fc17) * mask * rig_on * in.lit_light.x)
                * highlight;
            // Omega's sea: `liveLighting0specular * SpecularColour * sat(R.L)^SpecularPower`
            // (`reflectplane_mid` pixel `@0x314`..`@0x380`), the light's own specular colour.
            // The program's `reflectiveBlurCoeffs.z` lane, which would widen the lobe, reads 0
            // live (`panel-displacement.md`, "Frame constants read live"), so the lobe is the
            // plain power.
            if omega_sea {
                specular = scene.light.sun_specular.rgb * (pow(facing, row_scale.x) * mask * rig_on * in.lit_light.x)
                    * highlight;
            }
        }

        if omega_snow_on {
            specular = omega_snow_lit.specular;
        }
        if omega_rock_drawn {
            specular = omega_rock_lit.specular * albedo_linear * (mask * rig_on);
        }

        // **The reflection probe**, on the hull materials whose `IBL` variant declares it
        // (`slots` index into the glow table, `rate` 8): `probe(reflect(-V, N))` times
        // the probe tint, the livery's alpha (the gloss mask) and the material's own
        // reflectivity, added after the albedo multiply and before the fog - read off
        // the live microcode, `@0x5c`-`@0x70` of `diffuse_with_specular_from_alpha_n`'s
        // `IBL` block. The probe array is `Rgba8UnormSrgb`, so both probe fetches are linear
        // light, as the original's gamma-nibble-7 units 4 and 5 give it
        // (`docs/rendering/hd-ship-materials.md`, "Hull brightness").
        var probe_reflection = vec3<f32>(0.0);
        let probe_entry = (in.slots >> 24u) & 63u;
        if uniforms.env_layer > 0.5 && probe_entry != 0u && rate_from(row_scale.y, 8u) {
            let r = reflect(-to_eye, n_hull);
            let probe = textureSampleLevel(
                env_probe_tex, shadow_sampler, env_probe_uv(r), env_probe_layer(1), 0.0,
            ).rgb;
            probe_reflection = probe_tint(probe)
                * (texel.a * row_tint.x);
        }

        // **The rim term** of a hull material whose `IBL` program declares `fresnelBase`
        // (`tint.y` of the same entry, `mesh::rcs::probe_lit`): `base + (1 - base) * (1 - N.V)^5`,
        // one scalar added to every channel of the lit albedo, so neither the albedo nor the
        // probe tint colours it - `@0x25`..`@0x47` of `zoneship01`'s `IBL` block
        // (`docs/ghidra/functions/ps3-hdfury-eu/zone-hull-light.md`, "The rim term").
        var hull_rim = 0.0;
        if uniforms.env_layer > 0.5 && probe_entry != 0u && rate_in(row_scale.y, 8u, 8u)
            && row_tint.y > 0.0 {
            let grazing = clamp(1.0 - dot(to_eye, normalize(n_hull)), 0.0, 1.0);
            hull_rim = row_tint.y + (1.0 - row_tint.y) * pow(grazing, 5.0);
        }

        // **Scenery glass's reflection of the sky** (`slots` entry of rate 9, `mesh::rcs::window_glass`):
        // `probe_tint(sky(reflect(-V, N))) * reflectivity`, added before the fog like the hull's. The
        // map is the circuit's `sky.gtf` as the engine's `paraboloidReflectionTex`
        // (`mesh::sky_paraboloid`), read at the same dual-paraboloid address the hull's is. `N` is the
        // vertex normal: the program perturbs it with a tangent-space normal map, which a track
        // vertex carries no frame for here.
        //
        // **A pane** (`tint.y` 1, `glass_texture_customr`) splits its sum by the coverage `a`:
        // `a * light * picture + (1 - a) * (specular + reflection)`, alpha `a`, drawn
        // `One`/`OneMinusSrcAlpha`, so the clear part of the pane shows what is behind it.
        // `tint.z` 1 weights the reflection by `1 - a` (the pane, and
        // `glass_2nduv_reflect_glow`'s opaque windows), `tint.z` 2 by `a` (`ds_booth_glass`, whose
        // picture is its constant with the normal map's alpha), `tint.z` 3 by one minus the glow
        // mask's red.
        var sky_reflection = vec3<f32>(0.0);
        var pane_cover = 1.0;
        var pane_clear = 1.0;
        let sky_entry = (in.slots >> 24u) & 63u;
        if sky_entry != 0u && rate_in(row_scale.y, 9u, 9u) {
            let r = reflect(-to_eye, n);
            let sky = textureSampleLevel(sky_probe_tex, shadow_sampler, env_probe_uv(r), 0.0).rgb;
            if row_tint.y > 0.5 {
                pane_cover = texel.a;
                pane_clear = 1.0 - texel.a;
            }
            let gate = row_tint.z;
            // The glow mask, bound third at the second coordinate set: its alpha gates the glow
            // (`offset` 1) and, where `tint.z` is 3, its red the reflection
            // (`glass_2uv_rgb_reflect_glow`).
            var glow_mask = vec4<f32>(1.0);
            if row_tint.w > 0.5 || gate > 2.5 {
                glow_mask = material_sample(MATERIAL_PAD_MASK, in.lightmap_texcoord.zw);
            }
            var reflect_clear = select(select(1.0, 1.0 - texel.a, gate > 0.5), texel.a, gate > 1.5);
            reflect_clear = select(reflect_clear, 1.0 - decode(glow_mask.rgb).r, gate > 2.5);
            sky_reflection = probe_tint(sky) * (row_tint.x * reflect_clear);
            // `offset` 1: the program also adds `colour * (1 - mask.a)`, the colour in the next
            // entry's tint (`glass_2nduv_reflect_glow`). The colour is added raw, as the program
            // adds it.
            if row_tint.w > 0.5 {
                sky_reflection += emissive_tint(sky_entry + 1u).xyz * (1.0 - glow_mask.a);
            }
        }

        // **The tapped water's sky reflection** (`mesh::rcs::water`), at the mapped normal:
        // the pool's `Reflectivity * probeTint(sky)`, `water_test_2`'s `vertex colour *
        // probeTint(sky)`, and `water_noref`'s `probeTint(sky) * a` (lightmapped block `@0x3870`,
        // `@0x5c`) or `* (0.2 a + 0.4)` (block `@0x24b0`, the entry's `offset` 1), `a` the alpha
        // of its second map (fourth binding) at `uv` plus the third tap.
        //
        // **The sea's reflection is the planar target, `Reflectivity * R`** (`docs/rendering/water.md`,
        // "The planar reflection, drawn"): the point pushed `distortion * (N.x, 0, N.z)`, projected
        // through `reflectProject` (the main view-projection, in `scene.refraction` flipped as the
        // target is drawn) and read at `(0.5 + q.x / 2w, 0.5 - q.y / 2w)`. The original's target is
        // a 5-6-5 surface the sky and scenery programs wrote, which the sea fetches raw (unit 3's
        // gamma nibble 0, read live), so `R` is the stored value: the encoded light, saturated,
        // where the main view's taps are decoded. Where no target is bound
        // (`scene.sea_reflection.y` 0: a front-end capture, the viewer), the sea carries
        // `Reflectivity * encode(sky)` at the mapped normal in its place, chosen, not measured.
        if water > 0.5 && water < 4.5 {
            let r = reflect(-to_eye, n_lit);
            let sky = probe_tint(textureSampleLevel(sky_probe_tex, shadow_sampler, env_probe_uv(r), 0.0).rgb);
            let gate = material_sample(MATERIAL_WAVE_MAP, in.texcoord + water_tap_uv(water_entry, water, 2u, in.texcoord)).a;
            let glint_weight = select(gate, 0.2 * gate + 0.4, row_tint.w > 0.5);
            let weight = select(select(in.colour.rgb, vec3<f32>(glint_weight), water > 3.5), vec3<f32>(row_tint.w), water < 2.5);
            var sea = encode(sky);
            if scene.sea_reflection.y > 0.5 {
                let pushed = in.world + scene.sea_reflection.x * vec3<f32>(n_lit.x, 0.0, n_lit.z);
                let q = scene.refraction * vec4<f32>(pushed, 1.0);
                let ndc = q.xy / max(q.w, 1e-6);
                // Kept inside the rectangle the target was drawn into (`scene.sea_reflection.zw`).
                let uv = min(vec2<f32>(0.5 + 0.5 * ndc.x, 0.5 - 0.5 * ndc.y), scene.sea_reflection.zw);
                let texel = textureSampleLevel(behind_glass_tex, shadow_sampler, uv, 0.0).rgb;
                // A gamma target already stores the encoded value, which is `R` as it is.
                let stored = clamp(texel, vec3<f32>(0.0), vec3<f32>(1.0));
                sea = select(stored, encode(clamp(target_in(texel), vec3<f32>(0.0), vec3<f32>(1.0))), linear_out > 0.5);
            }
            sky_reflection = select(sky, sea, water < 1.5) * weight;
        }

        // **Omega's reflective sea** (kinds 9 and 10): the program adds
        // `weight * tex(screenSpaceReflectionTex)`, the planar target, which is not drawn. The
        // sky at the mapped normal stands in for it (chosen, not measured), read as the
        // panel-map road below reads Omega's sky: the first level, linear.
        if omega_sea {
            let r = reflect(-to_eye, n_lit);
            let sky = textureSampleLevel(sky_probe_tex, shadow_sampler, env_probe_uv(r), 0.0).rgb;
            sky_reflection = sky * reflective_sea_weight(water_entry, n_lit, to_eye);
        }

        // **The panel-map road's sun lobe and sky sheen** (mode 15, `mesh::rcs::psp2::panel_map`),
        // both gated by the diffuse alpha and, on `_mask`, multiplied by `ReflectionMask` (sixth
        // binding, the unshifted coordinate):
        //
        //     spec = sun_specular SpecStrength Dm.a LM.a sat(R.L)^p 0.0698 / max(0.0698, 0.1745 min(1, B))
        //     refl = Amount Dm.a (Min + Scale sat(1 - N.V)^Power) sky(R; level 2B)
        //
        // `p = SpecPower (1 - 0.75 min(1, B))`, `R` the view reflected about the mapped normal. The
        // lobe replaces the generic specular. The sky is the circuit's `skyCube.gnf` as the sky
        // map's four levels, read at the dual-paraboloid address, which composes to the program's
        // `(R.x, R.y, -R.z)` cube lookup (`mesh::sky_paraboloid`). The fresnel takes the mapped
        // normal where the program builds one with a doubled bitangent (`panel-displacement.md`,
        // "Open").
        if panel {
            let reflectivity = row_tint;
            let spec_row = emissive_tint(panel_entry + 1u);
            let blur = row_scale.x * 0.5;
            let r = reflect(-to_eye, n_lit);
            let p = spec_row.y * (1.0 - 0.75 * min(1.0, blur));
            let lobe = pow(clamp(dot(r, scene.light.direction), 0.0, 1.0), p)
                * 0.06981317 / max(0.06981317, 0.17453294 * min(1.0, blur));
            let gloss = first.a;
            var weight = vec3<f32>(1.0);
            if row_scale.w > 0.5 {
                weight = material_sample_grad(MATERIAL_SIXTH_MAP, in.texcoord, uv_grad[0], uv_grad[1]).rgb;
            }
            specular = scene.light.sun_specular.rgb * (spec_row.x * gloss * mask * lobe * rig_on) * weight;
            let facing = clamp(1.0 - dot(n_lit, to_eye), 0.0, 1.0);
            let fresnel = reflectivity.y + reflectivity.z * pow(facing, reflectivity.w);
            let level = row_scale.x;
            let at = env_probe_uv(r);
            let sky = mix(
                textureSampleLevel(sky_probe_tex, shadow_sampler, at, floor(level)).rgb,
                textureSampleLevel(sky_probe_tex, shadow_sampler, at, floor(level) + 1.0).rgb,
                vec3<f32>(fract(level)),
            );
            sky_reflection = sky * (reflectivity.x * gloss * fresnel) * weight;
        }

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
        let texel_linear = select(albedo_linear, vec3<f32>(texel.a) + chevron_surface, lit_alpha);
        // `surface = zoneCol`, then `colour = light * surface` - the microcode's
        // own order, and on this path both terms are linear. The Zone surface
        // **replaces** the albedo rather than adding to it, so `enabled` is the
        // mix weight: it is 0.0 for every draw that is not a Zone race with all
        // its inputs resolved, and there this is `texel_linear` unchanged.
        let surface_linear =
            mix(texel_linear, zone_linear, zone_weight * zone_replace) + zone_add_linear * zone_weight;
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
        // **`glow_linear` belongs here, not only on `plain`**: this fragment
        // resolves `mix(plain_rgb, authored_rgb, enabled * in.lit_light.x)` and HD's rig is
        // always enabled, so every `in.lit_light.x == 1` chunk - the track, the tubes, the
        // signs, everything near the camera - would drop a glow added to `plain`
        // alone. Only the far background and the hull would see it, and HD's
        // emissive layer would be very nearly invisible.
        // **Times the program's own output scale.** The final instruction of every
        // lit HD program carries `/2` in its scale field (renderer.md, "Every lit
        // program ends on a /2 output scale"), so the sum below is what the
        // arithmetic states and the scene holds half of it.
        //
        // **The two program modes** (`glow_is_billboard`, `glow_is_bw_glass`): the billboard's
        // colour is `picture * k * (glow.a + light * glow.rgb)` and has no specular term; the
        // stand glass multiplies its lit sum and its specular by `bwgradient`. Both samples
        // are decoded like every other texture on this path. The glass's `bwgradient` gamma
        // bits were not read off a live draw: decoded like its picture is **chosen, not
        // measured**.
        let glow_decoded = decode(glow_sample.rgb);
        let program_light = select(
            authored,
            glow_tint_offset.x * (vec3<f32>(glow_sample.a) + authored * glow_decoded),
            glow_is_billboard,
        );
        let program_sun = select(
            sun_light,
            glow_tint_offset.x * sun_light * glow_decoded,
            glow_is_billboard,
        );
        let program_specular = select(select(specular, vec3<f32>(0.0), glow_is_billboard) * pane_clear, vec3<f32>(0.0), simplified_lighting != 0.0);
        let program_multiplier = select(
            select(vec3<f32>(1.0), glow_decoded, glow_is_bw_glass),
            vec3<f32>(row_scale.w),
            crowd_lit,
        );
        // **The Detonator hull's overlay** `2 * Colour * picture * ramp`, added after the light
        // multiply like `hull_rim` (`mesh::rcs::hull_overlay`); `second` is the ramp fetched above.
        let hull_overlay = select(
            vec3<f32>(0.0),
            2.0 * row_tint.x * texel_linear * decode(second.rgb),
            overlay_hull,
        );
        let lit_linear = scene.light.output_scale
            * ((surface_linear * program_light * pane_cover + program_specular) * program_multiplier
                + zone_glow_term + glow_linear + pad_glow + mag_glow + chevron_glow + probe_reflection
                + sky_reflection + vec3<f32>(hull_rim) + hull_overlay);
        let encoded = pow(
            clamp(lit_linear, vec3<f32>(0.0), vec3<f32>(1.0)),
            vec3<f32>(1.0 / 2.2),
        );
        authored_rgb = mix(encoded, lit_linear, linear_out);
        // **The same fragment lit without the sun**, which is what the original's
        // shadowed track redraw draws: the permutation `Shadow_CompileShadowedTrackRedraw`
        // asks for is `ShadowToAlpha` + the chunk's ambient source + `Spot0`, and no
        // variant on the disc pairs `ShadowToAlpha` with `Sun` (16,554 of them, none),
        // so the program is ambient/prelit times albedo, fogged, and no sun or
        // specular term at all. See
        // `docs/ghidra/functions/ps3-hdfury-eu/shadow-track-redraw.md`.
        let sun_linear = scene.light.output_scale
            * ((surface_linear * program_sun * pane_cover + program_specular) * program_multiplier);
        let sunless = lit_linear - sun_linear;
        let sunless_encoded = pow(
            clamp(sunless, vec3<f32>(0.0), vec3<f32>(1.0)),
            vec3<f32>(1.0 / 2.2),
        );
        sun_removed = authored_rgb - mix(sunless_encoded, sunless, linear_out);
        // **HD's SPU vertex light on a chunk drawn on its Zone program.** The `SVC1` Zone twins
        // (`HalfBrightZoneMode*`, `HalfBrightZoneTrans*`) add it last, after the fog lerp and
        // outside `light * surface`, inside the closing `/2`: `out = (fogged + spu) / 2`, so it
        // reaches a Zone surface whose stage colours are black (Detonator's stage 1). Their
        // `ShadowToAlpha` redraw takes it inside its light instead, `surface * spu`. Only on a
        // chunk the Zone grade applies to (not a plain or exempt one), where the title's Zone frame
        // is read to be one; a pane's clear part takes none
        // (`docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`, "The blue wash of a volley").
        if zone_program != 0.0 && zone_weight > 0.0 && scene.zone.shape.w > 0.5
            && simplified_lighting == 0.0 {
            let spu_zone = in.spu_light * pane_cover;
            zone_after = scene.light.output_scale * spu_zone;
            let redraw = sunless + scene.light.output_scale * surface_linear * program_multiplier * spu_zone;
            let redraw_encoded = pow(clamp(redraw, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2));
            zone_redraw = mix(redraw_encoded, redraw, linear_out) - mix(sunless_encoded, sunless, linear_out);
        }
    }

    // The stand-in path, byte-for-byte what every other title always drew.
    // Prelit geometry (`lit` 0.0) takes it even under the authored rig: its
    // lighting is baked into its vertex colours, in the gamma space every
    // Pulse-shaped asset authors, so the linear round-trip above would
    // re-shade what is already shaded. On the linear target its gamma result
    // is decoded, so `post::hd_bloom`'s encode returns it byte-for-byte.
    let light = mix(vec3<f32>(1.0), stand_in, in.lit_light.x);
    // **The vertex colour is a tint here and a light term on the authored
    // path, so this multiply must not see HD's.** Without the guard, any HD
    // draw that binds `Light::stand_in` - the front end, or a circuit whose
    // `.envsettings` fails to yield a rig - would multiply by a baked light
    // that is zero wherever no colour set exists, and render black.
    //
    // **Keying this on `linear_out` would be wrong twice.** It would miss
    // HD's capture and viewer paths, which draw the same models into a gamma
    // target and render dark (1,062 lit pixels of a 1024x1024 frame against
    // 6,988 with the guard). And it would over-fire on the race
    // target, forcing *every* model's tint to white there including the sky
    // cube, whose vertex colour genuinely is a tint because `mesh/sky_cube.rs`
    // builds it rather than the HD loader. `colour_is_light` is a property of
    // the model and answers both.
    let tint = mix(in.colour.rgb, vec3<f32>(1.0), select(colour_is_light, 1.0, nova_vertex || tarmac));
    // The gamma-domain sum, for the same reason this whole path is gamma: a
    // prelit surface's light is baked into its vertex colours in the space the
    // asset authors, so a linear summand would be the one term shaded twice.
    // The glow is **added after the light**, which is where the microcode puts
    // it: the accumulate reads `H0` once the lightmap and the interpolated
    // term have already multiplied the albedo, so a glow is not itself lit.
    // The visualiser glow rides here too, with the stored strip texel: on a linear target
    // `decode(plain)` below decodes the sum, which stands in for the original's decode of
    // the texel; on a gamma target the texel stays as stored.
    let plain = (mix(texel.rgb, zone_gamma, zone_weight * zone_replace) + zone_add_gamma * zone_weight)
        * tint * light
        + glow + zone_glow_gamma + pad_glow + mag_glow_gamma;
    let plain_rgb = mix(plain, decode(plain), linear_out);

    // Vertex colour modulates the texture on all four channels, as the GE's
    // texture-env does - RGB and alpha alike, not RGB alone. Dropping the
    // vertex colour's own alpha here is what made the boost plume's baked
    // falloff vanish; see `every_psp_teams_boost_plume_vertex_alpha_is_bimodal`
    // in `crates/raceplay/tests/boost_plume_ground_truth.rs`.

    // **The flame, when this model is one.** `flame_shading` is 1.0 only for a
    // craft's `engineflare` model, and then this replaces everything above
    // rather than modifying it - the program it reproduces has no ambient, no
    // sun, no lightmap and no specular, so lighting the flame and then
    // overwriting it is the honest shape as well as the cheap one.
    //
    // `rim` is the same angle HD's program computes. The program the console
    // runs takes the position and the normal through the plain model matrix and
    // subtracts the world position from the camera's world position, so `n` and
    // `to_eye` here are its two vectors, and the flame surfaces' block carries
    // the view's eye (`docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`,
    // "The live vertex program").
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
        // **The colour tap is sRGB-decoded on fetch on HD's race target; the noise
        // tap's alpha is not.** Unit 0 of all five flame draws carries gamma nibble 7
        // (`SET_TEXTURE_ADDRESS` `0x60710101`, bits 20-23: RGB decoded, alpha raw) on
        // three live RPCS3 frames over two sessions (engine-trail.md, "The flame's
        // colour tap is decoded on fetch"), and the packer encodes the program's
        // output before the blend, so the console adds `enc(dec(t) * k / 2)`. This
        // replaces an earlier choice, made before that read and before the encoded
        // store, to use the raw texel as light: on the encoded store that drew the
        // rest flame white and clipped where the original's is blue.
        //
        // Gated on `encoded_out`, which only HD's measured store sets
        // (`Looks::blends_encoded`): the decode is the unit's nibble, read on HD
        // alone, and it rides that switch so Omega's and 2048's flames (their units
        // unread) draw as they did.
        let flame_tap = mix(flame_texel.rgb, decode(flame_texel.rgb), encoded_out * linear_out);
        let flame_rgb = flame_tap * flame_colour_scale * scene.light.output_scale;
        flame = vec4<f32>(flame_rgb, flame_alpha * in.colour.a);
    }

    // Omega's edge (kind 10; `reflectplane_edge` vertex `@0x194`, pixel `@0x46c`) is
    // `VertexColour1.y * Opacity` alone.
    // **The blended water edge's alpha** (kind 8; `fc17_..._wateredge_vertexcol` fragment `@27` to
    // `@29`): `x = a * vc.x`, `alpha = vc.y * (Opacity - x) + x`, `a` the first tap's alpha,
    // `vc` `VertexColour1` (the vertex's second coordinate pair), `Opacity` the entry's offset.
    let edge_map = material_sample(MATERIAL_PAD_MASK, water_tap_uv(water_entry, water_raw, 0u, in.texcoord)).a;
    let edge_x = edge_map * in.lightmap_texcoord.z;
    let edge_alpha = in.lightmap_texcoord.w * (row_tint.w - edge_x) + edge_x;
    // The rig's result alone where the weight is one for every pixel of the pipeline
    // (`lit_by_rig`): `mix(a, b, 1.0)` is `b`, and `a` then goes uncomputed.
    var shaded_rgb = authored_rgb;
    if lit_by_rig == 0.0 {
        shaded_rgb = mix(plain_rgb, authored_rgb, rig_on * in.lit_light.x);
    }
    let shaded = vec4<f32>(
        shaded_rgb,
        select(
            select(texel.a * in.colour.a, edge_alpha, vita_edge),
            in.lightmap_texcoord.w * row_tint.w,
            omega_edge,
        ),
    );
    // **The facing-ramp combine replaces the generic rig entirely**, the
    // same shape `flame` below already takes: block #7 computes its own
    // light (the traced `vertexLight`) rather than being multiplied by
    // `authored`'s ambient/prelit/sun sum, so running both would shade it
    // twice.
    var shaded_or_sheen = select(shaded, sheen, ramp_sheen);
    // A glass chunk in a Zone race keeps the grid as its coverage, vertex-lit or lightmapped: every
    // Zone block of the family that is not a `ShadowToAlpha` redraw ends `MOV H0.w, <grid>.x`
    // (`docs/formats/rcsmaterial.md`, "The Zone glass program"). Only the sheen's colour is dropped.
    if ramp_sheen_bit && !ramp_sheen {
        shaded_or_sheen.a = sheen.a;
    }
    // **The magstrip floor's own combine** (`slots::MAG_LOOP`), traced in
    // `oag_mesh::mesh::rcs::mag_wave`: `vertexLight * grid * (ramp + c) +
    // (grid + ramp) * d`, alpha the grid. `albedo` is the grid and `lightmap`
    // the facing ramp, addressed by `dot(V, N)` as the glass sheen's is. `c`
    // is the glow entry's `offset`. Left out, named: the paraboloid
    // reflection term, which this renderer has no probe for.
    var mag_floor = shaded_or_sheen;
    if (in.slots & 262144u) != 0u && zone_program == 0.0 {
        let grid = textureSample(albedo, albedo_sampler, in.texcoord).r;
        let ramp = textureSample(lightmap, albedo_sampler, ramp_uv).rgb;
        let loop_light = in.colour.rgb;
        // Times the program's output scale: the live floor program (Talon's `0x78d0c0`, the shadowed
        // pass of `mag_effect_loop_opaque`) ends `MAD /2 H0.xyz, fog, H1, fogColour`.
        let loop_rgb = scene.light.output_scale
            * (loop_light * grid * (ramp + vec3<f32>(glow_tint_offset.w))
                + (vec3<f32>(grid) + ramp) * mag_glow);
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
    // **Both taps are sRGB-decoded on fetch**, as the live draw's sampler state says
    // (`SET_TEXTURE_ADDRESS` `0x60710101` on the noise unit and `0x60750102` on the
    // ramp, gamma nibble 7, Amphiseum's cone draws 724 and 730): the program itself
    // applies no transfer function, the sampler does. **The
    // colour is saturated before it blends**, as the 8-bit surface the original
    // draws into does: `K` is 100 on Talon's Junction, so an unclamped colour
    // on this linear float target is `100 * noise` of light times the alpha, a
    // white wall, where the original's is a white of at most 1 at that alpha.
    var cone_or_floor = mag_floor;
    if (in.slots & 524288u) != 0u {
        let noise = decode(vec3<f32>(textureSample(lightmap, albedo_sampler, in.texcoord).r)).r;
        let cone_facing = clamp(ramp_facing, 0.0, 1.0);
        let ramp = decode(vec3<f32>(textureSample(albedo, albedo_sampler, vec2<f32>(cone_facing)).r)).r;
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
    // applies no transfer function. The fogged blocks the race draws close on
    // `MAD /2` (`ps3-microcode.py fp-file hd_absorbinternal.rcsmaterial`,
    // blocks #2 to #9), so the colour is halved like every scene program's
    // (`scene.light.output_scale`, 1.0 where a title has no such scale).
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
        absorb = vec4<f32>(absorb_rgb * scene.light.output_scale, in.colour.a);
    }
    let composed_plain = mix(mix(cone_or_floor, flame, flame_shading), absorb, absorb_shading);
    let composed_lit = vec4<f32>(
        composed_plain.rgb * select(vec3<f32>(1.0), in.colour.rgb, shield_shading != 0.0)
            + shield_lightning,
        composed_plain.a,
    );
    // **`enhanced` water** (`mesh::vex_water`, `water.wesl`) replaces the whole sum: the program
    // has its own light, glint and sky and no texture, so running the surface's generic path
    // and overwriting it would only shade it twice. The sky is the probe map's, encoded back to
    // the gamma the rest of the sum is in.
    var enhanced_rgb = vec3<f32>(0.0);
    if water > 4.5 {
        let sky = encode(textureSampleLevel(
            sky_probe_tex, shadow_sampler, env_probe_uv(reflect(-to_eye, n_lit)), 0.0,
        ).rgb);
        enhanced_rgb = enhanced_water(
            row_tint, row_scale.x, water_specular_colour(water_entry), in.colour.rgb, n_lit, to_eye, sky,
        );
    }
    let composed = select(
        composed_lit,
        vec4<f32>(mix(enhanced_rgb, decode(enhanced_rgb), linear_out), 1.0),
        water > 4.5,
    );

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
    // **The Bomb's fireball** (`slots::BOMB_FIRE`, `hd_bombfire_glow` block
    // `@0x1d90`): the LeachBall's rim skeleton with the blast's two scalars
    // in it. Read instruction by instruction in
    // docs/rendering/hd-unlit-programs.md, "The Bomb's fireball and
    // shockwaves"; `time` is the scene's, `AlphaAnim` the model's clock and
    // `ColourAnim` the blast's own float (`uniforms.model_colour`).
    //
    //     nA   = tex(u, 0.3 t + v).a                       @0x0d
    //     B    = tex(u, nA + v + 0.04 t)                   @0x33
    //     c    = tex(6u, 6 (0.3 t + v)).rgb                @0x1b
    //     rim  = sat(1 - N.V);  h = sat(0.86 - 0.86 rim^5)^50
    //     rgb  = (B.rg + 100 h, B.b) / (1 - c) / (1 - ColourAnim)
    //     a    = sat((0.5 B.a + 0.5 - AlphaAnim) / (1 - AlphaAnim))
    if bomb_fire {
        let fire_t = scene.time.x;
        let fire_noise = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(in.texcoord.x, 0.3 * fire_t + in.texcoord.y),
        ).a;
        let fire_b = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(in.texcoord.x, fire_noise + in.texcoord.y + 0.04 * fire_t),
        );
        let fire_c = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(6.0 * in.texcoord.x, 6.0 * (0.3 * fire_t + in.texcoord.y)),
        ).rgb;
        // **Both colour taps are sRGB-decoded on fetch on HD's race target**; the
        // two alphas (the noise and the burn) are not. The fireball's two draws bind
        // unit 0 with gamma nibble 7 (`SET_TEXTURE_ADDRESS` `0x60710101`) on a live
        // blast (docs/rendering/hd-unlit-programs.md, "The Bomb's units, read
        // live"); gated on `encoded_out` (HD's measured store), as the flame's is.
        let fire_decode = encoded_out * linear_out;
        let fire_b_rgb = mix(fire_b.rgb, decode(fire_b.rgb), fire_decode);
        let fire_c_rgb = mix(fire_c, decode(fire_c), fire_decode);
        let fire_rim = clamp(1.0 - dot(to_eye, n), 0.0, 1.0);
        let fire_rim_5 = fire_rim * fire_rim * fire_rim * fire_rim * fire_rim;
        let fire_h = pow(clamp(0.86 - 0.86 * fire_rim_5, 0.0, 1.0), 50.0);
        // `RCP` of a half register: held at `HALF_MAX` where the divisor reaches
        // zero, as the LeachBall's is.
        let fire_expand = min(vec3<f32>(1.0) / (vec3<f32>(1.0) - fire_c_rgb), vec3<f32>(HALF_MAX));
        let fire_boost = min(1.0 / (1.0 - uniforms.model_colour), HALF_MAX);
        let fire_rgb = (fire_b_rgb + vec3<f32>(100.0 * fire_h, 100.0 * fire_h, 0.0))
            * fire_expand * fire_boost;
        // `DIV_SAT`: a divisor of zero is `AlphaAnim` = 1, the blast's first
        // three quarters of a second. A numerator below zero is then `-inf`
        // and saturates to nothing; the `0 / 0` of a fully opaque texel is
        // read as nothing as well - **chosen, not measured**.
        let fire_numerator = 0.5 * fire_b.a + 0.5 - uniforms.model_clock;
        let fire_divisor = 1.0 - uniforms.model_clock;
        let fire_alpha = select(
            clamp(fire_numerator / fire_divisor, 0.0, 1.0),
            select(0.0, 1.0, fire_numerator > 0.0),
            fire_divisor <= 0.0,
        );
        // **The material's alpha test is `GL_LESS` against 0.5**
        // (`alpha_func 0x0201`, `alpha_ref 0.5`), the one cut-out on the disc
        // that keeps the *low* alpha: so the fireball is whole while `AlphaAnim`
        // is 1 (alpha 0) and burns away texel by texel as it falls. The engine's
        // cut-out pipeline hardcodes `GL_GREATER` and leaves this surface to the
        // opaque one, so the test is made here.
        if fire_alpha >= 0.5 {
            discard;
        }
        return vec4<f32>(fire_rgb, 1.0);
    }
    // **The laid Bomb's halo** (`slots::BOMB_HALO`, `hd_bomb_halo` block
    // `@0x19c0`): a Fresnel shell. `in.texcoord.y` already carries the vertex
    // program's `+ Speed` (`mesh::rcs`).
    //
    //     ramp = tex(u, v + Speed)                         @0x0e, 4x16
    //     rim  = sat(1 - N.V)
    //     a    = 50 ramp.a * rim^(10 - 10 ramp.a) * ramp.a * 0.1
    //     out  = (ramp.rgb, a)
    //
    // Blended SrcAlpha/One by the material's own state (`0x79`), so no alpha
    // test. The alpha is held to `1` as the 8-bit target it was written to
    // would: **chosen, not measured**, the float target here would otherwise
    // multiply the rows above `0.45` several times over.
    if (in.slots & 2097152u) != 0u {
        let halo_ramp = textureSample(albedo, albedo_sampler, in.texcoord);
        let halo_rim = clamp(1.0 - dot(to_eye, n), 0.0, 1.0);
        let halo_alpha = halo_ramp.a * 50.0 * pow(max(halo_rim, 1.0e-6), 10.0 - 10.0 * halo_ramp.a)
            * halo_ramp.a * BOMB_HALO_ALPHA_SCALE;
        return vec4<f32>(halo_ramp.rgb, min(halo_alpha, 1.0));
    }
    // **The Missile blast's three programs** (`hd_missile_explosion_*_glow`,
    // fogged blocks `@0x17f0`, `@0x17f0` and `@0x19b0`). `UV_offset` and
    // `Shockwave_scalar` are both the blast's age in seconds
    // (`uniforms.model_clock`; the entry lives one second, so the age is also
    // its `p`).
    //
    //     core   rgb = tex(u, v + 0.5 U).rgb * VertexColour.rgb
    //            a   = tex(u, v + 0.5 U).a * VertexColour.a       @0x00-@0x09
    //     rays   rim = sat(1 - N.V)
    //            f   = sat(0.9 - 0.9 rim^5)^5
    //            t   = tex(u, U);  out = (f t.rgb, t.a)            @0x00-@0x17
    //     shock  n   = tex(u, v + 0.01 S).a
    //            c   = 0.45 S - (v + 0.05 n)
    //            rgb = tex(u + n, v + 0.05 n).rgb * VertexColour1.rgb
    //            a   = VertexColour2.a * sum(tex(c, c).rgb) * (1 - S)
    //
    // The alphas are held to `1` as the 8-bit target they were written to
    // would hold them (**chosen, not measured**).
    if missile_core {
        let core_tap = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(in.texcoord.x, in.texcoord.y + 0.5 * clock),
        );
        // **The colour tap is sRGB-decoded on fetch on HD's race target**: the blast's
        // core draw binds unit 0 with gamma nibble 7 (`0x60710101`), the alpha raw
        // (docs/rendering/hd-unlit-programs.md, "The weapon units, read live").
        let core_rgb = mix(core_tap.rgb, decode(core_tap.rgb), encoded_out * linear_out);
        return vec4<f32>(core_rgb * in.colour.rgb, min(core_tap.a * in.sun_mask, 1.0));
    }
    if missile_rays {
        let rays_tap = textureSample(albedo, albedo_sampler, vec2<f32>(in.texcoord.x, clock));
        let rays_rim = clamp(1.0 - dot(to_eye, n), 0.0, 1.0);
        let rays_rim_5 = rays_rim * rays_rim * rays_rim * rays_rim * rays_rim;
        let rays_base = clamp(0.9 - 0.9 * rays_rim_5, 0.0, 1.0);
        let rays_fade = rays_base * rays_base * rays_base * rays_base * rays_base;
        // Nibble 7 on both of its draws (`bloom` and `rays` share the program).
        let rays_rgb = mix(rays_tap.rgb, decode(rays_tap.rgb), encoded_out * linear_out);
        return vec4<f32>(rays_fade * rays_rgb, min(rays_tap.a, 1.0));
    }
    if missile_shock {
        let wave_n = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(in.texcoord.x, in.texcoord.y + 0.01 * clock),
        ).a;
        let wave_v = in.texcoord.y + 0.05 * wave_n;
        let wave_c = 0.45 * clock - wave_v;
        let wave_diagonal = textureSample(albedo, albedo_sampler, vec2<f32>(wave_c, wave_c)).rgb;
        let wave_tap = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(in.texcoord.x + wave_n, wave_v),
        ).rgb;
        let wave_alpha = in.sun_mask * (wave_diagonal.x + wave_diagonal.y + wave_diagonal.z)
            * (1.0 - clock);
        return vec4<f32>(wave_tap * in.colour.rgb, clamp(wave_alpha, 0.0, 1.0));
    }
    // **The Plasma blast's three programs** (`hd_plasmaring_glow` `@0x1900`,
    // `hd_plasmahalo_glow` `@0x1960`, `plasmasphere_glow` `@0x2690`), `t` the
    // blast's age (`UV_offset`); docs/ghidra/functions/ps3-hdfury-eu/plasma.md,
    // "The blast's three programs". Each ends on the lit programs' `/2`.
    //
    //     ring    n   = tex(u, 4 (v + 0.01 t)).a
    //             c   = tex((u, v) + 0.15 n + 0.1 t).rgb          raw (unit 0 nibble 0)
    //             out = (c VertexColour1.rgb, VertexColour2.a sum(c))
    //     halo    n   = tex(u, v + 0.4 t).a
    //             c   = tex(2u + 0.1 t, v + 0.14 t + 0.1 n).rgb   decoded (nibble 7)
    //             out = (c VertexColour.rgb VertexColour.a, sum(c) / 3)
    //     sphere  n   = plasma(u, 4 (v + 0.01 t)).a
    //             c   = plasma((u, v) + 0.2 n + 0.4 t).rgb        decoded
    //             f   = 2 sat(1 - N.V)^(2 t)
    //             rgb = f c (sat(N.L) sun + ambient) (1.93, 1.2, 8)
    //                   + 20 noise(5u + 10 t, 15v + 10 t) (N.H)^32 sat(8 N.L) sun
    //             a   = f sum(c) (1 - 0.585 t)
    //
    //             rgb += probe(reflect(-V, N)) iblScalePower
    if plasma_ring {
        let ring_n = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(in.texcoord.x, 4.0 * (in.texcoord.y + 0.01 * clock)),
        ).a;
        let ring_c = textureSample(
            albedo,
            albedo_sampler,
            in.texcoord + vec2<f32>(0.15 * ring_n + 0.1 * clock),
        ).rgb;
        return vec4<f32>(
            scene.light.output_scale * ring_c * in.colour.rgb,
            clamp(in.sun_mask * (ring_c.x + ring_c.y + ring_c.z), 0.0, 1.0),
        );
    }
    if plasma_halo {
        let halo_n = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(in.texcoord.x, in.texcoord.y + 0.4 * clock),
        ).a;
        let halo_texel = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(
                2.0 * in.texcoord.x + 0.1 * clock,
                in.texcoord.y + 0.14 * clock + 0.1 * halo_n,
            ),
        ).rgb;
        let halo_c = mix(halo_texel, decode(halo_texel), encoded_out * linear_out);
        return vec4<f32>(
            scene.light.output_scale * halo_c * in.colour.rgb * in.sun_mask,
            clamp((halo_c.x + halo_c.y + halo_c.z) * 0.333252, 0.0, 1.0),
        );
    }
    if plasma_sphere {
        let sphere_n = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(in.texcoord.x, 4.0 * (in.texcoord.y + 0.01 * clock)),
        ).a;
        let sphere_texel = textureSample(
            albedo,
            albedo_sampler,
            in.texcoord + vec2<f32>(0.2 * sphere_n + 0.4 * clock),
        ).rgb;
        let sphere_c = mix(sphere_texel, decode(sphere_texel), encoded_out * linear_out);
        let noise_texel = textureSample(
            lightmap,
            albedo_sampler,
            vec2<f32>(5.0 * in.texcoord.x + 10.0 * clock, 15.0 * in.texcoord.y + 10.0 * clock),
        ).rgb;
        let sphere_noise = mix(noise_texel, decode(noise_texel), encoded_out * linear_out);
        let sphere_rim = clamp(1.0 - dot(n, to_eye), 0.0, 1.0);
        // `LG2`, `MUL 2 t`, `EX2 x2`; `0^0` taken as `1`, the age-0 frame's value
        // everywhere else on the shell.
        let sphere_f = 2.0 * pow(max(sphere_rim, 1.0e-6), 2.0 * clock);
        let sphere_ndl = dot(n, scene.light.direction);
        let sphere_half = normalize(to_eye + scene.light.direction);
        let sphere_spec = pow(clamp(dot(n, sphere_half), 0.0, 1.0), 32.0)
            * clamp(8.0 * sphere_ndl, 0.0, 1.0);
        let sphere_light = clamp(sphere_ndl, 0.0, 1.0) * scene.light.sun + scene.light.ambient;
        // The paraboloid reflection, at the hull's address. The map bound for a draw that
        // is no craft's is the circuit's sky, as the scenery glass reads it, and
        // `iblScalePower` there is `(1, 1, 1, 0)`: the glass's measured value, not read on
        // this draw (docs/rendering/hd-track-tone.md, "The glass program").
        let sphere_sky = textureSampleLevel(
            sky_probe_tex,
            shadow_sampler,
            env_probe_uv(reflect(-to_eye, n)),
            0.0,
        ).rgb;
        let sphere_rgb = sphere_f * sphere_c * sphere_light * vec3<f32>(1.93025, 1.20047, 8.0)
            + 20.0 * sphere_noise * sphere_spec * scene.light.sun
            + sphere_sky;
        let sphere_a = sphere_f * (sphere_c.x + sphere_c.y + sphere_c.z) * (1.0 - 0.584961 * clock);
        return vec4<f32>(scene.light.output_scale * sphere_rgb, clamp(sphere_a, 0.0, 1.0));
    }
    // **The Bomb's shockwave rings** (`slots::BOMB_SHOCK`,
    // `hd_bombfire_shockwaves_glow` block `@0x1950`), `Shockwave_scalar` being
    // the model's clock. The diagonal tap `TEX H0.xyz, R1.xxxx` reads the
    // texture at `(c, c)` and sums its channels into the alpha:
    //
    //     n   = tex(u, v + 0.01 S).a                       @0x08
    //     c   = v + 0.05 n + 0.45 S                        @0x0d
    //     rgb = tex(u + n, c).rgb * VertexColour1.rgb      @0x14, @0x19
    //     a   = VertexColour2.a * sum(tex(c, c).rgb)       @0x11-@0x16
    if bomb_shock {
        let shock_s = uniforms.model_clock;
        let shock_n = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(in.texcoord.x, in.texcoord.y + 0.01 * shock_s),
        ).a;
        let shock_c = in.texcoord.y + 0.05 * shock_n + 0.45 * shock_s;
        let shock_diagonal = textureSample(albedo, albedo_sampler, vec2<f32>(shock_c, shock_c)).rgb;
        let shock_tap = textureSample(
            albedo,
            albedo_sampler,
            vec2<f32>(in.texcoord.x + shock_n, shock_c),
        ).rgb;
        return vec4<f32>(
            shock_tap * in.colour.rgb,
            in.sun_mask * (shock_diagonal.x + shock_diagonal.y + shock_diagonal.z),
        );
    }
    let rim_glow_on = (in.slots & 2048u) != 0u;
    let rim_edge_on = (in.slots & 4096u) != 0u;
    // The magstrip floor's `ShadowToAlpha` redraw (`0x78d0c0`) draws the very colour the lit pass
    // does - its light is `TC1 + TC2`, no sun in it - so the original's craft shadow is invisible
    // on it; the stand-in multiply would show one.
    (*sun).exempt = select(
        0.0,
        1.0,
        ramp_sheen || ((in.slots & 262144u) != 0u && zone_program == 0.0),
    );
    // **HD's Detonator mine** (`Emissive::program` mode 23, `oag_mesh::mesh::rcs::mine_rim`): an
    // unlit rim, `rim^(4 sat(1 - B) + 1) / (1 - 0.9 C)` per channel, `B` the entry's offset
    // (`MineBrightness`) and `C` its tint (`MineColour`), read off the program's own microcode
    // (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-material.md`, "The mine's fragment
    // program"). The map (the program's unit 0) is the third binding, at the chunk's one
    // coordinate set (`TC3`; the mine's vertices declare no second set), put through the chunk's
    // tangent frame as a hull's is; the program dots a
    // tangent-space eye vector with it, which is the same angle in the world frame. Its alpha is
    // the program's output alpha: both mine models are cutouts (state word 62), so the draw's alpha
    // test cuts the lattice's holes. The fogged variant the race resolves ends in `/2`
    // (`output_scale`); the fog is applied by the caller after `lit_texel` returns.
    //
    // **Chosen, not measured: a divisor at or below zero** (a dark mine's literal `2` makes
    // `1 - 0.9 C` negative) **draws black**: the program's negative colour is what the RSX target
    // does with it, unread.
    // **HD's `electricity` ring** (`Emissive::program` mode 25, `oag_mesh::mesh::rcs::electricity`):
    // the Detonator mine's visualiser, music-driven lightning scrolled through three pictures.
    // `n0` and `n1` are the bass and treble tap of unit 0, `d` the distortion taps of unit 2 and
    // `m` the pitch tap of unit 1 (`Tempo` is the entry's scale, `P` its second lane); the entry
    // after it carries `(PitchVolume, BassHit, TrebleHit)` and `PitchOffset`. The colour is the
    // overlay blend of the lightning against `ElectricityColour`, times `|N.V|^P`. The colour tap
    // is decoded (chosen, not measured: the unit's gamma nibble is unread). See the module's doc
    // for the program's instructions. 2048's program squares the bass tap's and the pitch tap's
    // red itself (the next entry's scale is 1, `oag_rcs::gxp::electricity`).
    let electric_entry_on = program_in(row_scale.z, 25u, 25u);
    if electric_entry_on {
        let audio = emissive_tint(((in.slots >> 24u) & 63u) + 1u);
        let squared_taps = emissive_scale(((in.slots >> 24u) & 63u) + 1u).x > 0.5;
        let alpha_v = sun_mask;
        let n0_texel = material_sample(MATERIAL_ALBEDO, vec2<f32>(in.lightmap_texcoord.z, in.lightmap_texcoord.w - audio.y)).x;
        let n0 = select(n0_texel, n0_texel * n0_texel, squared_taps);
        let n1 = material_sample(MATERIAL_ALBEDO, vec2<f32>(in.texcoord.x, in.texcoord.y - audio.z)).w;
        let s = in.lightmap_texcoord.y + scene.time.x + n0 + n1;
        let d = material_sample(MATERIAL_PAD_MASK, vec2<f32>(in.lightmap_texcoord.x, row_scale.x * s)).w - n0 - n1;
        let pitch_texel = material_sample(MATERIAL_LIGHTMAP, vec2<f32>(in.lightmap_texcoord.x, audio.w)).x;
        let pitch = select(pitch_texel, pitch_texel * pitch_texel, squared_taps);
        let m = (pitch * audio.x + d) * row_tint.w;
        let lightning = decode(material_sample(
            MATERIAL_PAD_MASK,
            vec2<f32>(in.lightmap_texcoord.x, in.lightmap_texcoord.y + 0.3 * m),
        ).rgb) * alpha_v;
        let colour = row_tint.rgb;
        let over = select(
            vec3<f32>(1.0) - 2.0 * (vec3<f32>(1.0) - lightning) * (vec3<f32>(1.0) - colour),
            2.0 * lightning * colour,
            lightning < vec3<f32>(0.5),
        );
        let facing = pow(max(abs(dot(to_eye, n)), 1.0e-6), row_scale.w);
        return vec4<f32>(min(scene.light.output_scale * over * facing, vec3<f32>(HALF_MAX)), alpha_v * facing);
    }
    let mine_rim_on = glow_program > 22.5 && glow_program < 23.5;
    var mine_normal = n;
    var mine_alpha = 1.0;
    if mine_rim_on {
        let mine_map = material_sample(MATERIAL_PAD_MASK, in.texcoord);
        mine_normal = hull_normal(mine_map, n, in.tangent);
        mine_alpha = mine_map.a;
    }
    let mine_angle = max(clamp(1.0 - dot(to_eye, mine_normal), 0.0, 1.0), 1.0e-6);
    let mine_power = 4.0 * clamp(1.0 - glow_tint_offset.w, 0.0, 1.0) + 1.0;
    let mine_divisor = vec3<f32>(1.0) - 0.9 * glow_tint_offset.rgb;
    let mine_rim = select(
        vec3<f32>(0.0),
        vec3<f32>(pow(mine_angle, mine_power)) / max(mine_divisor, vec3<f32>(1.0e-6)),
        mine_divisor > vec3<f32>(0.0),
    );
    if mine_rim_on {
        return vec4<f32>(min(scene.light.output_scale * mine_rim, vec3<f32>(HALF_MAX)), mine_alpha);
    }
    // **HD's Detonator bomb colour and brightness** (`Emissive::program` mode 24,
    // `oag_mesh::mesh::rcs::bomb_tint`): the entry's tint times its offset, unlit, through the
    // `/2` the fogged programs end in. **Chosen, not measured**: how the programs combine the
    // colour with their normal map and environment picture is unread; the brightness is their
    // last multiply.
    if glow_program > 23.5 && glow_program < 24.5 {
        return vec4<f32>(
            min(scene.light.output_scale * glow_tint_offset.rgb * glow_tint_offset.w, vec3<f32>(HALF_MAX)),
            1.0,
        );
    }
    // **HD's Detonator cannon gauge** (`Emissive::program` mode 27, `oag_mesh::mesh::rcs::bullet_gauge`):
    // the segment picture at `v + 0.5 * F`, `F` the entry's offset (`StandardBullets`), coloured by
    // its tint (`StandardColour`) where it is white, and the picture unshifted (`PowerBullets` is 0,
    // `PowerColour` white) where it is not. Unlit, through the fogged programs' `/2`
    // (`docs/ghidra/functions/ps3-hdfury-eu/detonator-ammo-gauge.md`, "The segment program").
    if glow_program > 26.5 && glow_program < 27.5 {
        let gauge = material_sample(
            MATERIAL_ALBEDO,
            vec2<f32>(in.texcoord.x, in.texcoord.y + 0.5 * glow_tint_offset.w),
        ).rgb;
        let power = material_sample(MATERIAL_ALBEDO, in.texcoord.xy).rgb;
        let segments = gauge * glow_tint_offset.rgb * gauge.r + power * (1.0 - gauge.r);
        return vec4<f32>(min(scene.light.output_scale * segments, vec3<f32>(HALF_MAX)), 1.0);
    }
    // **HD's Detonator cannon bolt** (`Emissive::program` mode 30, `oag_mesh::mesh::rcs::cannon_bolt`,
    // block `@0x1b70`; docs/rendering/hd-unlit-programs.md, "The Detonator cannon bolt"):
    //
    //     noise = tex0(uv1).a;  c = tex0(uv1 + 0.8 noise + 5 t)     uv1 is `texcoord`
    //     ring  = tex1(uv0).a                                       uv0 is `lightmap_texcoord.xy`
    //     F     = fB + (1 - fB) sat(1 - N.V)^5                      fB the entry's offset
    //     out   = (c.rgb + (1 - F)^3 (0.0619202, 0.744629, 1), 5 c.a ring)
    //
    // Unlit, through the fogged programs' `/2` (`output_scale`, 1 on 2048 and Omega, whose
    // programs end without it); the fog is the caller's. Blended `SrcAlpha`/`One` by the material's
    // state. The tint is the title's own, in the entry. The entry's `program[1]` is 1 on 2048 and
    // Omega, whose colour tap is read as decoded (2048's program squares it, Omega's picture is
    // sRGB-typed). On HD it is decoded as the other weapon glows' (chosen, not measured: this
    // draw's unit-0 gamma nibble is unread). The alpha is held to `1` as an 8-bit target would
    // hold it (chosen, not measured).
    if glow_program > 29.5 && glow_program < 30.5 {
        let bolt_noise = material_sample(MATERIAL_ALBEDO, in.texcoord).a;
        let bolt_tap = material_sample(
            MATERIAL_ALBEDO,
            in.texcoord + vec2<f32>(0.8 * bolt_noise + 5.0 * scene.time.x),
        );
        let bolt_c = mix(bolt_tap.rgb, decode(bolt_tap.rgb), max(encoded_out, row_scale.w) * linear_out);
        let bolt_ring = material_sample(MATERIAL_LIGHTMAP, in.lightmap_texcoord.xy).a;
        let bolt_rim = clamp(1.0 - dot(to_eye, n), 0.0, 1.0);
        let bolt_rim_5 = bolt_rim * bolt_rim * bolt_rim * bolt_rim * bolt_rim;
        let bolt_base = glow_tint_offset.w;
        let bolt_keep = clamp(1.0 - (bolt_base + (1.0 - bolt_base) * bolt_rim_5), 0.0, 1.0);
        let bolt_rgb = bolt_c + bolt_keep * bolt_keep * bolt_keep * glow_tint_offset.rgb;
        return vec4<f32>(
            min(scene.light.output_scale * bolt_rgb, vec3<f32>(HALF_MAX)),
            min(5.0 * bolt_tap.a * bolt_ring, 1.0),
        );
    }
    if !(rim_glow_on || rim_edge_on) {
        // **Only where the colour returned is still the generic combine's**, which is
        // what `sun_removed` was computed against. The facing-ramp sheen, the magstrip
        // floor, the light cone, the flame and the absorb shell each replace it with
        // their own program, and subtracting the generic sun term from one of those
        // would take out light it never contained: they leave `SunTerms::known` clear and
        // `shadowed` falls back to its multiply for them. Their own `ShadowToAlpha`
        // programs are unread.
        let generic_combine = !ramp_sheen
            && !((in.slots & 262144u) != 0u && zone_program == 0.0)
            && (in.slots & 524288u) == 0u
            && flame_shading == 0.0
            && absorb_shading == 0.0;
        if generic_combine && rig_on != 0.0 {
            (*sun).delta = rig_on * in.lit_light.x * sun_removed;
            (*sun).known = 1.0;
            (*sun).after_fog = rig_on * in.lit_light.x * zone_after;
            (*sun).redraw = rig_on * in.lit_light.x * zone_redraw;
        }
        // `grade-sky`: the sky draw only, and only when a track module asked, so every
        // other pixel keeps the path above untouched. The authored picture is flattened
        // toward an overcast grey, darker and slightly cool, keeping its own structure.
        if uniforms.sky_grey > 0.0 {
            let luma = dot(composed.rgb, vec3<f32>(0.299, 0.587, 0.114));
            let overcast = vec3<f32>(luma * SKY_OVERCAST_LUMA) + SKY_OVERCAST_TINT;
            return vec4<f32>(mix(composed.rgb, overcast, uniforms.sky_grey), composed.a);
        }
        // **HD's Detonator plasma shells** (`Emissive::program` modes 28 and 29,
        // `oag_mesh::mesh::rcs::hull_charge`): the program's alpha is `MOV H0.w, Charge`, so the
        // draw's alpha is the entry's offset (`Charge1`, `Charge2`).
        if glow_program > 27.5 && glow_program < 29.5 {
            return vec4<f32>(composed.rgb, glow_tint_offset.w);
        }
        return composed;
    }
    let rim_noise = textureSample(
        albedo,
        albedo_sampler,
        vec2<f32>(in.texcoord.x, in.texcoord.y + 0.0001 * scene.time.x),
    ).a;
    let rim_tap = in.texcoord + vec2<f32>(0.2 * rim_noise + 0.4 * scene.time.x);
    // **The colour tap is sRGB-decoded on fetch on HD's race target** for both rim
    // programs: the LeachBall's (and the Missile head's) and the Plasma head's unit 0
    // read gamma nibble 7 (`0x60710101`); the noise tap's alpha is raw
    // (docs/rendering/hd-unlit-programs.md, "The weapon units, read live").
    let rim_texel = textureSample(albedo, albedo_sampler, rim_tap).rgb;
    let rim_c = mix(rim_texel, decode(rim_texel), encoded_out * linear_out);
    let rim_angle = clamp(1.0 - dot(to_eye, n), 0.0, 1.0);
    // `LG2`/`MUL 5`/`EX2` in the program: `rim^5`, multiplied out so `0^5` is
    // `0` rather than whatever `pow(0, 5)` answers on a given backend.
    let rim_5 = rim_angle * rim_angle * rim_angle * rim_angle * rim_angle;
    // RIM_GLOW: `MAD_SAT -x, 0.9, 0.9`, then `^5` again.
    let rim_fade_base = clamp(0.9 - 0.9 * rim_5, 0.0, 1.0);
    let rim_fade = rim_fade_base * rim_fade_base * rim_fade_base * rim_fade_base * rim_fade_base;
    // `c / (1 - c)` through three `RCP`s of half registers. A texel at 255
    // makes that `1 / 0`; it is held at `HALF_MAX`.
    let rim_expand = min(vec3<f32>(1.0) / (vec3<f32>(1.0) - rim_c), vec3<f32>(HALF_MAX));
    let rim_glow = vec4<f32>(rim_fade * rim_c * rim_expand, rim_fade);
    // RIM_EDGE: `MUL H1.w, rim^5, 1000`, times the tap; the alpha is the
    // material's `0x7611a2d8`, which `rim_glow::classify` only routes at 1.0.
    let rim_edge = vec4<f32>(1000.0 * rim_5 * rim_c, 1.0);
    return select(select(composed, rim_glow, rim_glow_on), rim_edge, rim_edge_on);
}
