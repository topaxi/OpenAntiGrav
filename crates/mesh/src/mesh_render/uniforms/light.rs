//! The authored light rig a circuit's own settings file states, as the uniform
//! the mesh shader reads. Split out of `uniforms.rs` for its length.

/// The authored light rig a circuit's own settings file states.
///
/// # Why this exists
///
/// **`mesh.wesl`'s two-light rig is a stand-in and says so**, with invented
/// directions chosen so geometry reads clearly. Wipeout HD authors the real
/// thing in plain text, one file per circuit - see
/// `oag_tables::envsettings` - and `CLAUDE.md`'s rule about not inventing
/// what the assets already author applies directly.
///
/// # What is the disc's and what is this project's
///
/// **The combination is the disc's**, read out of the circuit materials' own
/// fragment microcode
/// (`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`) rather than assumed:
///
/// ```text
/// light  = ambient + prelit_scale * lightmap^prelit_power
///        + sun * max(dot(N, L), 0) * lightmap.a
/// colour = diffuse * light
///        + sun * specular_scale * pow(max(dot(H, N), 0), 32)
///          * max(dot(N, L), 0) * lightmap.a * diffuse.a
/// ```
///
/// The lightmap's alpha gating the direct sun - a baked shadow mask - and the
/// prelit power curve are the two halves this project used to replace with a
/// plain multiply. **The magnitudes are the disc's too, and the arithmetic is
/// done in linear light** as the RSX does - samples sRGB-decoded, lit, the
/// result saturated and encoded back into the gamma target - per
/// [ADR-0026](../../../docs/architecture/adr/0026-hd-authored-lighting-is-linear.md),
/// which narrows ADR-0020 to the titles its GE argument is about. The
/// saturate is this project's stand-in for HD's unread exposure stage and is
/// judged against an rpcs3 reference frame; the per-vertex additive term the
/// original's vertex programs interpolate on top (dynamic lights among them)
/// is not reproduced.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Light {
    /// Unit direction **towards** the light, which is what a normal dots with.
    pub direction: [f32; 3],
    /// `1.0` to use this rig, `0.0` for `mesh.wesl`'s stand-in.
    pub enabled: f32,
    /// Constant ambient, passed through as authored - see [`Light::authored`].
    pub ambient: [f32; 3],
    /// The output scale the title's lit fragment programs end on: `0.5` where
    /// every lit program's last instruction is `/2` (HD's), `1.0` otherwise.
    /// Applied to the whole lit sum, specular and glows included, and, through
    /// the fog colour the loader halves with it, to the fog share too - see
    /// [`Light::with_output_scale`].
    pub output_scale: f32,
    /// The sun's colour, magnitude and all - see [`Light::authored`]. **Not**
    /// divided down to a hue: that reduction was this project's own before
    /// `76d0f58e` read the circuit's combining equation off its own
    /// microcode; the magnitude is the disc's now, same as the direction.
    pub sun: [f32; 3],
    /// `1.0` where the prelit term is Wipeout: Omega Collection's
    /// `scale * pow(lightmap, power) + bias` on the raw atlas, with no constant
    /// ambient on a lightmapped draw - see [`Light::with_nova_prelit`]. `0.0`
    /// everywhere else, which is every title but Omega.
    pub nova: f32,
    /// `Lighting.Prelit ambient colour scale`, applied exactly where the
    /// circuit's own fragment microcode applies it:
    /// `prelit = scale * lightmap^power`. See [`Light::authored`] for what
    /// of the equation is read and what is this project's.
    pub prelit_scale: [f32; 3],
    /// `Lighting.Sun specular scale`, weighting the read specular term
    /// `pow(max(dot(H, N), 0), 32) * max(dot(N, L), 0) * lightmap.a`.
    /// The exponent 32 is an **inline** microcode constant, not a patched
    /// parameter, so it lives in `mesh.wesl` rather than here.
    pub specular_scale: f32,
    /// `Lighting.Prelit ambient colour power` - the exponent in the same
    /// prelit term.
    pub prelit_power: [f32; 3],
    /// The `bias` of [`Light::with_nova_prelit`], added to the powed lightmap.
    /// Read only where [`Self::nova`] is set; `0.0` on every other title.
    pub prelit_bias: f32,
    /// Pulse's GE light list for a craft's hull, or [`crate::mesh_render::HullLights::OFF`].
    /// Read by `vs_main` alone, and only for a lit vertex - see
    /// [`crate::mesh_render::HullLights`].
    pub hull: crate::mesh_render::HullLights,
}

impl Light {
    /// This rig with [`Self::hull`] switched off, for every draw that is not a
    /// craft's hull.
    #[must_use]
    pub fn without_hull(self) -> Self {
        Self {
            hull: crate::mesh_render::HullLights::OFF,
            ..self
        }
    }

    /// The stand-in rig: what every title but HD binds.
    ///
    /// A value rather than an unbound group, for the reason [`super::Fog::off`] is
    /// one: WGSL has no optional bindings.
    #[must_use]
    pub fn stand_in() -> Self {
        Self {
            direction: [0.0, 1.0, 0.0],
            enabled: 0.0,
            ambient: [0.0; 3],
            output_scale: 1.0,
            sun: [0.0; 3],
            nova: 0.0,
            prelit_scale: [1.0; 3],
            specular_scale: 0.0,
            prelit_power: [1.0; 3],
            prelit_bias: 0.0,
            hull: crate::mesh_render::HullLights::OFF,
        }
    }

    /// The rig a circuit authors.
    ///
    /// `direction` must already be normalised;
    /// `oag_tables::envsettings::EnvSettings::direction` does it and answers
    /// `None` for the degenerate triples four circuits write, which is why this
    /// takes a direction rather than a settings file.
    ///
    /// Every value is passed through as authored - the equation these feed is
    /// the one read out of the circuit's own microcode, and reducing a term of
    /// it would un-read it. Where the sum leaves the range this target holds,
    /// the target saturates; that clamp is the documented stand-in for HD's
    /// missing tonemap stage - see the type-level docs.
    #[must_use]
    pub fn authored(
        direction: [f32; 3],
        colour: [f32; 3],
        ambient: [f32; 3],
        prelit_scale: [f32; 3],
        prelit_power: [f32; 3],
        specular_scale: f32,
    ) -> Self {
        Self {
            direction,
            enabled: 1.0,
            ambient,
            output_scale: 1.0,
            sun: colour,
            nova: 0.0,
            prelit_scale,
            specular_scale,
            // A power of zero would turn an unsampled black lightmap texel
            // into full white; the floor keeps the curve monotone in the
            // lightmap without changing any authored value (the corpus
            // authors 1.0 to 2.0).
            prelit_power: std::array::from_fn(|i| prelit_power[i].max(1e-3)),
            prelit_bias: 0.0,
            hull: crate::mesh_render::HullLights::OFF,
        }
    }

    /// This rig with the title's lit-program output scale (`0.5` for HD).
    #[must_use]
    pub fn with_output_scale(self, output_scale: f32) -> Self {
        Self {
            output_scale,
            ..self
        }
    }

    /// This rig with Wipeout: Omega Collection's prelit combination, read out of
    /// its circuit pixel shaders (`ps4-omega-eu/lightmap-prelit.md` under
    /// `docs/ghidra/functions/`), in place of HD's:
    ///
    /// ```text
    /// prelit = scale * pow(lightmap.rgb, power) + bias        (raw atlas, no clamp)
    /// light  = prelit + sun * max(dot(N, L), 0) * lightmap.a  (no constant ambient)
    /// ```
    ///
    /// `(scale, bias, power)` is the circuit's `Lighting.Nova prelit scale bias
    /// power` triple - three scalars, unlike HD's per-channel vectors - or the
    /// executable's own default when the file omits it. Applies to draws that
    /// carry a lightmap; every other draw keeps the constant ambient.
    ///
    /// **The power is floored at `1e-3`**, as [`Light::authored`] floors it: an
    /// unsampled black texel would otherwise read as full white. No authored
    /// triple is that low (the corpus runs 1 to 12).
    #[must_use]
    pub fn with_nova_prelit(self, scale: f32, bias: f32, power: f32) -> Self {
        Self {
            nova: 1.0,
            prelit_scale: [scale; 3],
            prelit_power: [power.max(1e-3); 3],
            prelit_bias: bias,
            ..self
        }
    }
}
