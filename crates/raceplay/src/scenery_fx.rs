//! The particle effects a circuit places itself: the welders' sparks and the
//! steam vents its `.vex` authors as `ParticleSystem` nodes.
//!
//! **No simulation event, by design.** The original spawns each one when the
//! circuit loads and keeps it on its node for the whole race
//! ([`oag_vex::placed_psys`] for the read), so there is nothing for the
//! simulation to decide: this is scenery, owned by the view and advanced on
//! the same clock the circuit's animated geometry runs on (`tick / 60`, see
//! `race::scene::frame`).
//!
//! **Its own [`psys::Stage`].** Circuit seven places eighteen steam vents,
//! every one attached for the whole race; on the shared stage they would
//! leave weapon detonations a handful of slots. A second pool keeps the two
//! apart. Chosen, not measured: the original's particle manager is one pool.
//!
//! **Not done, and said so:**
//! - The original hands the instance the node's whole world matrix; a
//!   [`psys::System`] takes a position and an `up`, so the emitter frame's
//!   rotation about its own `+Y` is not carried.
//! - Nothing is culled. Whether the original's draw slot (`0x08915fd0`) skips a
//!   node outside the visible sections is unread; every placed effect here is
//!   simulated and drawn wherever the camera is. Chosen, not measured.
//! - Pulse PSP only, the one source the spawn was confirmed live on - see
//!   `race::load::pulse_psp::finish`.
//!
//! **The weather is here too**, in [`weather`]: a circuit's rain or snow is the
//! other thing it places on its own, from its `TrackStartup` rather than its
//! `.vex` nodes, and it rides the same pool of view-side state.

pub mod lens;
pub mod mist;
mod noise;
pub mod splash;
pub mod weather;
mod wind;

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_fx::psys;
use oag_vex::placed_psys::Placed;

pub use weather::Weather;

/// What a circuit places on its own scenery, as read at load.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Plan {
    /// The `ParticleSystem` nodes of its `.vex`.
    pub placed: Vec<Placed>,
    /// Its `<Weather>` element and anchors.
    pub weather: Option<weather::Setup>,
    /// Its sun's lens flare: the `lensflare` node and the atlas, on a title whose
    /// `Looks::lens_flare` applies.
    ///
    /// Boxed: the plan is copied by value through `start` in an unoptimised build, where the
    /// race tests' stacks have no room for a bigger one.
    pub lens_flare: Option<Box<oag_fx::lens_flare::Setup>>,
    /// The PSP bloom's composite lands before the HUD, on a title whose
    /// `Looks::bloom_under_hud` applies.
    pub bloom_under_hud: bool,
    /// Towards a track file's sun, when it may cast (ADR-0076): the `mapped`
    /// shadows' direction in place of the host title's own axis. `None` on
    /// every disc circuit.
    pub sun: Option<[f32; 3]>,
    /// A track file's sun turning over time (ADR-0076 decision 10): it turns
    /// [`Self::sun`] and, when [`Self::flare_turns`], the flare's direction.
    pub sun_turn: Option<crate::foreign::art::motion::SunTurn>,
    /// The flare follows the turning sun.
    pub flare_turns: bool,
}

/// Seed for the scenery effects' own spawn parameters. Any constant: render
/// state, never hashed.
const SEED: u64 = 0x5ce7_e4f5;

/// Severity every placed instance plays at. `PsysNode_Init` writes no
/// severity, so the instance keeps the default every other untuned spawn here
/// plays at too.
const SEVERITY: f32 = 1.0;

/// The circuit's placed effects, and the pool they play in.
#[derive(Debug, Clone)]
pub struct SceneryFx {
    weather: Weather,
    lens_flare: Option<Box<oag_fx::lens_flare::Setup>>,
    bloom_under_hud: bool,
    sun: Option<[f32; 3]>,
    sun_turn: Option<crate::foreign::art::motion::SunTurn>,
    flare_turns: bool,
    placed: Vec<Placed>,
    playing: Vec<Option<psys::Playing>>,
    stage: psys::Stage,
    rng: Rng,
}

impl Default for SceneryFx {
    fn default() -> Self {
        Self::new(Plan::default())
    }
}

impl SceneryFx {
    /// Nothing playing yet; the first [`Self::advance`] starts every effect
    /// the library holds.
    #[must_use]
    pub fn new(plan: Plan) -> Self {
        let Plan {
            placed,
            weather,
            lens_flare,
            bloom_under_hud,
            sun,
            sun_turn,
            flare_turns,
        } = plan;
        Self {
            lens_flare,
            bloom_under_hud,
            sun,
            sun_turn,
            flare_turns,
            weather: Weather::new(weather),
            playing: vec![None; placed.len()],
            placed,
            stage: psys::Stage::new(),
            rng: Rng::new(SEED),
        }
    }

    /// Moves every instance onto its node at `seconds` and runs the pool one
    /// step. An effect the library did not load is skipped, and stays
    /// undrawn rather than stood in for.
    pub fn advance(&mut self, effects: &psys::Library, dt: f32, seconds: f32) {
        for (placed, playing) in self.placed.iter().zip(&mut self.playing) {
            let world = placed.world_at(seconds);
            let at = Vec3::new(world[12], world[13], world[14]);
            let up = Vec3::new(world[4], world[5], world[6]);
            match playing {
                Some(handle) => {
                    self.stage.follow(*handle, at);
                    self.stage.orient(*handle, up);
                }
                None => {
                    let Some(effect) = effects.get(&placed.name) else {
                        continue;
                    };
                    *playing = self.stage.attach(effect, at, SEVERITY);
                    if let Some(handle) = playing {
                        self.stage.orient(*handle, up);
                    }
                }
            }
        }
        self.stage.advance(dt, &mut self.rng);
    }

    /// The circuit's sun flare, when its title draws one and the circuit authors it.
    #[must_use]
    pub fn lens_flare(&self) -> Option<&oag_fx::lens_flare::Setup> {
        self.lens_flare.as_deref()
    }

    /// Towards the track's own sun at `seconds` on the animation clock, where
    /// a track file authors one that casts.
    #[must_use]
    pub fn sun(&self, seconds: f32) -> Option<[f32; 3]> {
        let rest = self.sun?;
        Some(
            self.sun_turn
                .as_ref()
                .map_or(rest, |turn| turn.direction_at(seconds)),
        )
    }

    /// The flare's direction at `seconds`: its own, or the turning sun's.
    #[must_use]
    pub fn lens_flare_direction(&self, seconds: f32) -> Option<[f32; 3]> {
        let setup = self.lens_flare.as_deref()?;
        Some(match (&self.sun_turn, self.flare_turns) {
            (Some(turn), true) => turn.direction_at(seconds),
            _ => setup.direction,
        })
    }

    /// Whether the bloom's composite lands before the HUD (`Looks::bloom_under_hud`)
    /// rather than over it.
    #[must_use]
    pub fn bloom_under_hud(&self) -> bool {
        self.bloom_under_hud
    }

    /// The circuit's weather.
    #[must_use]
    pub fn weather(&self) -> &Weather {
        &self.weather
    }

    /// Runs the weather one step - see [`weather`].
    pub fn advance_weather(
        &mut self,
        effects: &psys::Library,
        dt: f32,
        camera: psys::field::Frame,
        section: i32,
    ) {
        self.weather.advance(effects, dt, camera, section);
    }

    /// The weather, to age and add splashes to.
    pub(crate) fn weather_mut(&mut self) -> &mut Weather {
        &mut self.weather
    }

    /// [`psys::Stage::hold_still`] on the pool, at the start of a tick path.
    pub fn hold_still(&mut self) {
        self.stage.hold_still();
    }

    /// The pool, for the renderer and for tests.
    #[must_use]
    pub fn stage(&self) -> &psys::Stage {
        &self.stage
    }

    /// The placements this race carries.
    #[must_use]
    pub fn placed(&self) -> &[Placed] {
        &self.placed
    }

    /// How many placements are playing now.
    #[must_use]
    pub fn playing_count(&self) -> usize {
        self.playing.iter().flatten().count()
    }
}

impl super::Race {
    /// The weather's one step: where the camera is and the section the drawn
    /// craft is in, as `cam+0x1e8` publishes it through `FUN_08878644`.
    ///
    /// A craft the spline cannot place keeps the section it last published -
    /// chosen, not measured: the original's field is always a real section.
    pub(super) fn advance_weather(&mut self, dt: f32) {
        let section = self
            .station_camera_section()
            .unwrap_or_else(|| self.section_of_slot(self.player_slot()));
        let section = if section == oag_render::pvs::UNPLACED {
            self.view.scenery_fx.weather().section()
        } else {
            i32::from(section)
        };
        let camera = self.camera_frame();
        self.view
            .scenery_fx
            .advance_weather(&self.view.effects, dt, camera, section);
        self.advance_splashes(dt);
    }

    /// The enhanced weather's one step: ages the splashes and throws one
    /// where a drop's last move crossed a floor. **Nothing runs at `original`**
    /// beyond clearing what a switch back left behind, so that frame is the
    /// original's. See [`splash`].
    fn advance_splashes(&mut self, dt: f32) {
        if !self.view.weather.is_enhanced() {
            if !self.view.scenery_fx.weather().splashes().is_empty() {
                self.view.scenery_fx.weather_mut().splashes_mut().clear();
            }
            return;
        }
        let weather = self.view.scenery_fx.weather();
        let snow = weather.is_snow();
        let hits: Vec<(Vec3, Vec3)> = weather
            .drop_segments()
            .into_iter()
            .filter_map(|(from, to)| self.floor_crossing(from, to))
            .filter(|&(at, _)| !self.point_is_under_cover(at))
            .collect();
        let splashes = self.view.scenery_fx.weather_mut().splashes_mut();
        splashes.advance(dt);
        for (at, normal) in hits {
            splashes.spawn(at, normal, snow);
        }
    }

    /// Whether `point`, a hit on the track, lies in a covered section: the
    /// section of the spline sample nearest it, tested with the original's
    /// own mask (`Section_IsCovered`), wherever the camera is. **A point the
    /// spline cannot place (off the track) counts as open**: no cover is
    /// evidence for it. Chosen, not measured.
    pub fn point_is_under_cover(&self, point: Vec3) -> bool {
        let Some((index, _, _)) = self.sim.spline.nearest(point) else {
            return false;
        };
        let section = self.section_of(index, point);
        section != oag_render::pvs::UNPLACED
            && self
                .view
                .scenery_fx
                .weather()
                .section_is_covered(i32::from(section))
    }

    /// Where the segment `from..to` crosses a floor from above, and that
    /// floor's normal: the same read-only cast the blob shadows make.
    fn floor_crossing(&self, from: Vec3, to: Vec3) -> Option<(Vec3, Vec3)> {
        use oag_physics::collide::{Ray, Raycaster};
        let along = to - from;
        let length = along.length();
        if length <= f32::EPSILON {
            return None;
        }
        let direction = along / length;
        let mut hits = [None; 4];
        let found = self.sim.collision.raycast_all(
            Ray::new(from, direction, length),
            None,
            false,
            &mut hits,
        );
        hits.iter()
            .flatten()
            .take(found)
            .filter(|hit| hit.surface.is_hoverable() && hit.normal.dot(direction) < 0.0)
            .min_by(|a, b| a.distance.total_cmp(&b.distance))
            .map(|hit| (hit.point, hit.normal))
    }

    /// The lens as a world frame: where it is and where its axes point.
    pub(super) fn camera_frame(&self) -> psys::field::Frame {
        let world = self.view().inverse();
        psys::field::Frame {
            position: world.w_axis.truncate(),
            right: world.x_axis.truncate(),
            up: world.y_axis.truncate(),
            back: world.z_axis.truncate(),
        }
    }
}
