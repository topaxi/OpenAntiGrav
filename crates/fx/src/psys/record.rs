//! One emitter record into the [`EmitterSpec`] the player runs.

use oag_core::math::Vec3;
use oag_pob::{self as pob, ChannelMode};

use super::{
    Blend, ColourMode, ColourScale, Direction, EmitterSpec, Error, Render, sample,
    spawn::Spawn,
    sprite::{Atlas, Sprite},
    streak::StreakDraw,
};
use super::{frames::FrameAdvance, guard::DrawClass, playback, roll, turbulence};

impl EmitterSpec {
    /// How long one instance of this emitter runs, in ticks - infinite
    /// under [`EmitterSpec::looping`], which is what stops the schedule
    /// ever expiring on its own.
    #[must_use]
    pub fn run_ticks(&self) -> f32 {
        if self.looping {
            f32::INFINITY
        } else {
            self.duration_ticks
        }
    }

    /// Translates one parsed record.
    pub(super) fn from_record(
        record: &pob::Emitter,
        scale: ColourScale,
        sprite: Option<Sprite>,
    ) -> Result<Self, Error> {
        let emitter = || record.name.clone();

        let render = match record.draw_class() {
            Some(1..=3) => Render::Billboard,
            Some(6 | 7) => Render::Streak {
                from_spawn: record.flags & pob::flags::STREAK_FROM_SPAWN != 0,
            },
            _ => {
                return Err(Error::UnknownDrawClass {
                    emitter: emitter(),
                    render_mode: record.render_mode,
                });
            }
        };
        let blend = match record.blend_class {
            2 => Blend::Additive,
            1 | 3 => Blend::AlphaOver,
            8 => Blend::Distort,
            other => {
                return Err(Error::UnknownBlendClass {
                    emitter: emitter(),
                    blend_class: other,
                });
            }
        };
        for channel in [
            &record.size,
            &record.alpha,
            &record.rotation_speed,
            &record.emission_scale,
        ] {
            if let ChannelMode::Unknown(mode) = channel.mode {
                return Err(Error::UnknownChannelMode {
                    emitter: emitter(),
                    mode,
                });
            }
        }

        let jitter = record.cone_degrees.to_radians();
        let hemisphere = record.shape == 7;
        let direction = match (record.shape, record.velocity_mode) {
            (4 | 7, 2) => Direction::Tangent { hemisphere },
            (4 | 7, _) => Direction::Radial { hemisphere },
            (_, 1) => Direction::Aimed {
                elevation: record.elevation,
                azimuth: record.azimuth,
                jitter,
            },
            _ => Direction::Cone { half_angle: jitter },
        };

        let mut palette = Box::new([[0.0f32; 4]; 256]);
        let divisor = scale.divisor();
        for (out, entry) in palette.iter_mut().zip(record.colours.iter()) {
            *out = [
                f32::from(entry[0]) / divisor,
                f32::from(entry[1]) / divisor,
                f32::from(entry[2]) / divisor,
                f32::from(entry[3]) / divisor,
            ];
        }

        let drag = record.drag_per_tick().unwrap_or([1.0; 3]);
        let interval = (
            record.interval_ticks.0.max(1) as u32,
            record.interval_ticks.1.max(record.interval_ticks.0).max(1) as u32,
        );
        let per_emission = (
            record.per_emission.0.max(0) as u32,
            record.per_emission.1.max(record.per_emission.0).max(0) as u32,
        );

        Ok(Self {
            name: record.name.clone(),
            duration_ticks: record.duration_ticks.max(0.0),
            looping: record.looping(),
            interval_ticks: interval,
            per_emission,
            lifetime_ticks: sample::lifetime_ticks(record),
            speed_per_tick: record.speed_per_tick,
            direction,
            drag_per_tick: Vec3::new(drag[0], drag[1], drag[2]),
            gravity_per_tick2: if record.gravity_enabled() {
                record.gravity_per_tick2
            } else {
                0.0
            },
            turbulence: turbulence::Turbulence::from_record(record),
            live_cap: record.live_cap.max(0) as usize,
            size: record.size.clone(),
            alpha: record.alpha.clone(),
            colour_divisor: divisor,
            palette,
            colour_mode: ColourMode::authored(record.colour_mode),
            authored_colour_mode: record.colour_mode,
            render,
            draw_class: DrawClass::of(record.draw_class()),
            streak: StreakDraw::of(record.draw_class(), record.aspect),
            blend,
            distort_strength: record.distort_strength,
            colour_scale: record.colour_scale,
            near_cap: record.near_cap,
            authored_frame_range: record.frame_range,
            frame_range: None,
            authored_size_ramp: record.size_span.filter(|_| {
                let wanted = pob::flags::SIZE_RAMP | pob::flags::SIZE_RAMP_OVER_RUN;
                record.flags & wanted == wanted && record.flags & pob::flags::LOOPING == 0
            }),
            size_ramp: None,
            particle_child: record.particle_child,
            death_child: record.death_child,
            spawn_probability: record.child_spawn_probability.clamp(0.0, 1.0),
            velocity_inherit: record.child_velocity_inherit,
            spawn: Spawn::of(record),
            emission_scale: record.emission_scale.clone(),
            extent_animation: record
                .attribute_animations
                .iter()
                .find(|animation| animation.selector == 2)
                .map(|animation| animation.channel.clone()),
            playback: playback::Playback::of(record),
            sprite,
            atlas: Atlas::of(record),
            frames: FrameAdvance::of(record, Atlas::of(record).frames()),
            sheet_rect: None,
            short_run: true,
            under_alpha: false,
            shader: None,
            world_space: record.flags & pob::flags::WORLD_SPACE != 0,
            template: false,
            rotation: roll::Rotation::of_emitter(record),
        })
    }
}
