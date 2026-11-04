//! Gathering a frame's particle geometry, by layer: the additive and alpha-over lists
//! the pass draws and the underlay that draws before the alpha-over one
//! ([`super::EmitterSpec::under_alpha`]).

use oag_core::math::Vec3;
use oag_mesh::mesh::GpuVertex;

use super::{
    Blend, ColourMode, Effect, EmitterSpec, Particle, Render, Stage, System, billboard,
    channel_sample, guard, streak,
};

impl System {
    /// This frame's geometry, split by blend class: `(additive, alpha_over)`,
    /// the two GE configurations the original's state selector switches
    /// between. `right` and `up` come from the camera; streaks follow the
    /// streak-quad builder at `0x08916820`.
    #[must_use]
    pub fn vertices(
        &self,
        effect: &Effect,
        right: Vec3,
        up: Vec3,
    ) -> (Vec<GpuVertex>, Vec<GpuVertex>) {
        let mut additive = Vec::new();
        let mut alpha_over = Vec::new();
        self.extend_vertices(&mut additive, &mut alpha_over, effect, right, up, None);
        (additive, alpha_over)
    }

    /// The additive geometry of the emitters flagged [`EmitterSpec::under_alpha`], which
    /// [`Self::extend_vertices`] leaves out: drawn before the alpha-over layer.
    pub fn extend_underlay_vertices(
        &self,
        out: &mut Vec<GpuVertex>,
        effect: &Effect,
        right: Vec3,
        up: Vec3,
        guard: Option<&guard::GuardBand>,
    ) {
        self.extend_layer(true, out, &mut Vec::new(), effect, right, up, guard);
    }

    /// [`Self::vertices`], appended to two lists the caller owns - the
    /// renderer's form (a returned pair per effect allocated 1.99 MB a frame
    /// with rockets in the air). `guard` is the GE's screen-range cull, or
    /// `None` for a source it was not measured on.
    pub fn extend_vertices(
        &self,
        additive: &mut Vec<GpuVertex>,
        alpha_over: &mut Vec<GpuVertex>,
        effect: &Effect,
        right: Vec3,
        up: Vec3,
        guard: Option<&guard::GuardBand>,
    ) {
        self.extend_layer(false, additive, alpha_over, effect, right, up, guard);
    }

    /// A particle's age, half size, alpha and palette colour this frame.
    fn look(&self, spec: &EmitterSpec, particle: &Particle) -> (f32, f32, f32, [f32; 4]) {
        let age = 1.0 - (particle.life / particle.max_life).clamp(0.0, 1.0);
        let half = channel_sample(&spec.size, age, particle.size_sample)
            * self.drawn_scale(spec, particle);
        let alpha = channel_sample(&spec.alpha, age, particle.alpha_sample) / spec.colour_divisor;
        let colour = match spec.colour_mode {
            ColourMode::RandomEntry => spec.palette[usize::from(particle.colour_index)],
            // `palette[(int)(age * 255.999)]`, the original's own
            // index; the alpha byte comes from the channel either way.
            ColourMode::OverLife => spec.palette[(age * 255.999) as usize & 0xff],
        };
        (age, half, alpha, colour)
    }

    /// This frame's quads of the emitters a mod's shader draws, which [`Self::extend_vertices`]
    /// leaves out, each onto the list of its shader's place in `out`. Only a mod's file names
    /// a shader, and it can only name it on a billboard, so nothing else ever lands here.
    pub fn extend_shaded(
        &self,
        out: &mut Vec<(usize, Vec<GpuVertex>)>,
        effect: &Effect,
        right: Vec3,
        up: Vec3,
        guard: Option<&guard::GuardBand>,
    ) {
        for particle in &self.particles {
            if !particle.alive() {
                continue;
            }
            let spec = &effect.emitters[usize::from(particle.spec)];
            let Some(shader) = spec.shader else { continue };
            let (age, half, alpha, colour) = self.look(spec, particle);
            let rgb = [colour[0], colour[1], colour[2]];
            let corners = billboard(spec, particle, age, half, right, up, rgb, alpha);
            if guard::drops(guard, &effect.name, spec.template, &corners) {
                continue;
            }
            let place = usize::from(shader);
            match out.iter_mut().find(|(at, _)| *at == place) {
                Some((_, list)) => list.extend_from_slice(&corners),
                None => out.push((place, corners.to_vec())),
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn extend_layer(
        &self,
        underlay: bool,
        additive: &mut Vec<GpuVertex>,
        alpha_over: &mut Vec<GpuVertex>,
        effect: &Effect,
        right: Vec3,
        up: Vec3,
        guard: Option<&guard::GuardBand>,
    ) {
        for particle in &self.particles {
            if !particle.alive() {
                continue;
            }
            let spec = &effect.emitters[usize::from(particle.spec)];
            if spec.shader.is_some() {
                continue;
            }
            let look = self.look(spec, particle);
            let (age, half, alpha, colour) = look;
            if spec.under_alpha != underlay {
                continue;
            }
            let out: &mut Vec<GpuVertex> = match spec.blend {
                Blend::Additive => additive,
                Blend::AlphaOver => alpha_over,
                Blend::Distort => continue,
            };
            let rgb = [colour[0], colour[1], colour[2]];
            if let Render::Streak { .. } = spec.render {
                streak::extend(
                    out,
                    spec,
                    particle,
                    half,
                    [rgb[0], rgb[1], rgb[2], alpha],
                    (right, up),
                    guard,
                );
                continue;
            }
            let corners = billboard(spec, particle, age, half, right, up, rgb, alpha);
            if guard::drops(guard, &effect.name, spec.template, &corners) {
                continue;
            }
            out.extend_from_slice(&corners);
        }
    }
}

impl Stage {
    /// Every playing instance's geometry, split by blend class the same way
    /// [`System::vertices`] splits one.
    #[must_use]
    pub fn vertices(&self, right: Vec3, up: Vec3) -> (Vec<GpuVertex>, Vec<GpuVertex>) {
        let mut additive = Vec::new();
        let mut alpha_over = Vec::new();
        self.extend_vertices(&mut additive, &mut alpha_over, right, up, None);
        (additive, alpha_over)
    }

    /// [`Self::vertices`], appended to two lists the caller owns - the form
    /// the renderer uses, for the reason [`System::extend_vertices`] gives.
    pub fn extend_vertices(
        &self,
        additive: &mut Vec<GpuVertex>,
        alpha_over: &mut Vec<GpuVertex>,
        right: Vec3,
        up: Vec3,
        guard: Option<&guard::GuardBand>,
    ) {
        for instance in &self.instances {
            let Some(effect) = instance.effect.as_deref() else {
                continue;
            };
            instance
                .system
                .extend_vertices(additive, alpha_over, effect, right, up, guard);
        }
    }

    /// [`System::extend_shaded`] for every playing instance.
    pub fn extend_shaded(
        &self,
        out: &mut Vec<(usize, Vec<GpuVertex>)>,
        right: Vec3,
        up: Vec3,
        guard: Option<&guard::GuardBand>,
    ) {
        for instance in &self.instances {
            let Some(effect) = instance.effect.as_deref() else {
                continue;
            };
            instance.system.extend_shaded(out, effect, right, up, guard);
        }
    }

    /// [`System::extend_underlay_vertices`] for every playing instance.
    pub fn extend_underlay_vertices(
        &self,
        out: &mut Vec<GpuVertex>,
        right: Vec3,
        up: Vec3,
        guard: Option<&guard::GuardBand>,
    ) {
        for instance in &self.instances {
            let Some(effect) = instance.effect.as_deref() else {
                continue;
            };
            instance
                .system
                .extend_underlay_vertices(out, effect, right, up, guard);
        }
    }
}
