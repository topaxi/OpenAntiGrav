//! The scene's ribbon effects built from [`RibbonTextures`]: the HD-lineage
//! magstrip arc wake (`magstrip_wake`) and Wipeout HD's Rocket smoke, each
//! `None` where its textures did not load. Grouped so the scene builds,
//! uploads and draws them through one seat.

use oag_fx::beam::pipeline::Style;
use oag_fx::rocket_smoke::{CAPACITY, FINS, MAX_TRAILS, OpacityRamp};
use oag_title::weapons::MissileTrail;

use crate::Race;
use crate::rocket_smoke::RibbonTextures;

/// Vertices the smoke's buffer holds: every ribbon full, six per fin per
/// segment.
const SMOKE_VERTICES: usize = MAX_TRAILS * (CAPACITY - 1) * FINS * 6;

/// Vertices the Missile ribbons' buffer holds: both pools full, every chain at its
/// builder's capacity, six vertices a segment. The specs are the title's, so this is
/// sized from them when the title builds the ribbons.
fn missile_vertices(trail: &MissileTrail) -> usize {
    let per_element = |spec: oag_title::weapons::FatLineSpec| {
        spec.chains as usize * (spec.capacity as usize).saturating_sub(1) * 6
    };
    crate::missile_smoke::POOL * (per_element(trail.stub) + per_element(trail.helix))
}

/// Vertices the explosion streaks' buffer holds: the pool full, every element at its
/// builder's capacity, six vertices a segment.
fn streak_vertices(streak: &oag_title::weapons::BlastStreak) -> usize {
    streak.pool
        * streak.line.chains as usize
        * (streak.line.capacity as usize).saturating_sub(1)
        * 6
}

#[derive(Debug)]
pub(super) struct Ribbons {
    pub(super) magstrip: Option<super::magstrip_wake::Magstrip>,
    rocket_smoke: Option<RocketSmoke>,
    missile_smoke: Option<MissileSmoke>,
    missile_streaks: Option<MissileSmoke>,
    leach_strip: Option<std::cell::RefCell<oag_fx::beam::Pipeline>>,
}

#[derive(Debug)]
struct RocketSmoke {
    pipeline: std::cell::RefCell<oag_fx::beam::Pipeline>,
    ramp: OpacityRamp,
}

#[derive(Debug)]
struct MissileSmoke {
    pipeline: std::cell::RefCell<oag_fx::beam::Pipeline>,
    ramp: OpacityRamp,
}

pub(super) fn build(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    format: wgpu::TextureFormat,
    store: oag_gpu::formats::SceneStore,
    textures: &RibbonTextures,
    sample_count: u32,
) -> Ribbons {
    let magstrip = super::magstrip_wake::build(
        device,
        queue,
        format,
        store,
        textures.magstrip.as_ref(),
        sample_count,
    );
    let rocket_smoke = textures.rocket_smoke.as_ref().map(|assets| RocketSmoke {
        pipeline: std::cell::RefCell::new(oag_fx::beam::Pipeline::with_style(
            device,
            queue,
            format,
            store,
            &assets.texture,
            sample_count,
            super::mesh_render::Velocity::Write,
            Style::rocket_smoke(SMOKE_VERTICES),
        )),
        ramp: assets.ramp.clone(),
    });
    let missile_smoke = textures.missile_smoke.as_ref().map(|assets| MissileSmoke {
        pipeline: std::cell::RefCell::new(oag_fx::beam::Pipeline::with_style(
            device,
            queue,
            format,
            store,
            &assets.texture,
            sample_count,
            super::mesh_render::Velocity::Write,
            Style::fat_line(missile_vertices(&assets.trail)),
        )),
        ramp: assets.ramp.clone(),
    });
    let missile_streaks = textures.missile_smoke.as_ref().and_then(|assets| {
        let (texture, streak) = assets
            .streak_texture
            .as_ref()
            .zip(assets.trail.blast_streak)?;
        Some(MissileSmoke {
            pipeline: std::cell::RefCell::new(oag_fx::beam::Pipeline::with_style(
                device,
                queue,
                format,
                store,
                texture,
                sample_count,
                super::mesh_render::Velocity::Write,
                Style::fat_line(streak_vertices(&streak)),
            )),
            ramp: assets.ramp.clone(),
        })
    });
    let leach_strip = textures.leach_strip.as_ref().map(|assets| {
        std::cell::RefCell::new(oag_fx::beam::Pipeline::with_noise(
            device,
            queue,
            format,
            store,
            &assets.glow,
            Some(&assets.noise),
            sample_count,
            super::mesh_render::Velocity::Write,
            Style::leach_strip(crate::leach_strip::VERTICES),
        ))
    });
    Ribbons {
        magstrip,
        rocket_smoke,
        missile_smoke,
        missile_streaks,
        leach_strip,
    }
}

impl Ribbons {
    /// The pipelines in the order the effects draw them: the wake's contact quads under its
    /// arcs, then the Missile's additive ribbons (pools 0 and 2, the stub and the helix) and
    /// the explosion's streaks (pool 1), the Rocket's smoke over them
    /// (`RibbonEffects_Render` blends pools 0, 2, 1, 5 additively before 3 and 4), then HD's
    /// Leach strip.
    pub(super) fn cells(&self) -> [Option<&std::cell::RefCell<oag_fx::beam::Pipeline>>; 6] {
        let (contact, atlas) = self
            .magstrip
            .as_ref()
            .map_or((None, None), |m| (Some(m.contact()), Some(m.atlas())));
        [
            contact,
            atlas,
            self.missile_smoke.as_ref().map(|smoke| &smoke.pipeline),
            self.missile_streaks.as_ref().map(|smoke| &smoke.pipeline),
            self.rocket_smoke.as_ref().map(|smoke| &smoke.pipeline),
            self.leach_strip.as_ref(),
        ]
    }
}

impl super::Scene {
    /// Uploads this frame's ribbons. The smoke's facing fade needs the eye,
    /// read out of the view matrix's inverse.
    pub(super) fn upload_ribbons(
        &self,
        race: &Race,
        queue: &dyn oag_gpu::writes::Writes,
        vp: &[[f32; 4]; 4],
    ) {
        self.upload_magstrip_wake(race, queue, vp);
        if let Some(smoke) = &self.ribbons.rocket_smoke {
            let eye = race.view().inverse().w_axis.truncate();
            let vertices = race.rocket_smoke_vertices(&smoke.ramp);
            smoke
                .pipeline
                .borrow_mut()
                .upload_with_eye(queue, vp, eye.to_array(), &vertices);
        }
        if let Some(smoke) = &self.ribbons.missile_smoke {
            let eye = race.view().inverse().w_axis.truncate();
            let vertices = race.missile_smoke_vertices(eye, &smoke.ramp);
            smoke
                .pipeline
                .borrow_mut()
                .upload_with_eye(queue, vp, eye.to_array(), &vertices);
        }
        if let Some(streaks) = &self.ribbons.missile_streaks {
            let eye = race.view().inverse().w_axis.truncate();
            let vertices = race.missile_streak_vertices(eye, &streaks.ramp);
            streaks
                .pipeline
                .borrow_mut()
                .upload_with_eye(queue, vp, eye.to_array(), &vertices);
        }
        if let Some(strip) = &self.ribbons.leach_strip {
            let eye = race.view().inverse().w_axis.truncate();
            strip.borrow_mut().upload_with_clock(
                queue,
                vp,
                eye.to_array(),
                // The offset and the flyby are the world meshes' epoch; the strip is a weapon's.
                self.anim_clock.get() - race.world_clock_offset() - race.flyby_seconds(),
                &race.leach_strip_vertices(),
            );
        }
    }
}
