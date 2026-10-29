//! Pulse's rain and snow: what `Weather_Update` (`0x088f1e58`) does with the
//! effect `Weather_Construct` (`0x088f184c`) spawns off a circuit's
//! `TrackStartup` `<Weather>` element.
//!
//! **Where the weather is.** Out in the open it rides the camera: the effect's
//! box is `depth` units in front of the lens and holds still in the world as
//! the craft drives through it, so rain streaks away from where the camera is
//! heading. In a *covered* section - one with a `weatherPos` anchor, see
//! [`oag_vex::weather`] - the same effect instead stays where the anchor is,
//! in the world: rain falls through the gap the artist put the anchor at and
//! nowhere else.
//!
//! **The switch is on a section change only.** `Weather_Update` compares the
//! section the camera published this frame with last frame's and does
//! nothing while they match, so the mode is held state, entered on the frame a
//! section edge is crossed. A section below zero, which is what both start as,
//! counts as covered; the first frame therefore leaves "covered" for an open
//! section and enters it for a covered one.
//!
//! **The lens** `ScreenPsys` (`WO_RAIN_LENS`, Fort Gale) is [`super::lens`].
//!
//! **The mist overlay** `WeatherMist_Construct` (`0x088fa0a0`) builds from
//! `Tex` and its sizes is [`super::mist`], drawn by `oag_fx::mist`.
//!
//! **Not played, and said so:**
//! - The manager's clock behind the noise: its units are taken as seconds, the
//!   race clock, and the noise table is a seeded one - see [`super::noise`].
//! - `Weather_Update` is skipped in Zone, and here the plan is not built there.

use std::sync::Arc;

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_fx::psys::field::{Anchor, FieldSpec, Frame};
use oag_fx::psys::{self, Effect, System};
use oag_mesh::mesh::GpuVertex;
use oag_tables::trackstartup::{Weather as Config, effect_name};
use oag_vex::weather::{self as covered, Anchor as CoveredAnchor};

use super::lens::{Edge, Lens};
use super::mist::Mist;
use super::splash::Splashes;
use super::spray::Spray;
use super::wind::Wind;

/// Seed for the weather's own spawn draws, wind table and phases. Any
/// constant: render state, never hashed.
const SEED: u64 = 0x7a1e_b01d;

/// Seconds of rain it takes the enhanced weather's road to go fully wet. Chosen,
/// not measured: the original has no wet road.
pub const WET_RAMP_SECONDS: f32 = 40.0;

/// How wet the road is after `clock` seconds of weather, `0.0..=1.0`: rising
/// linearly over [`WET_RAMP_SECONDS`] for rain, never for snow. **Not decided by
/// where the camera is**: a roofed road is kept dry per draw at load
/// ([`dry_covered_road`]), so the open road seen from inside a tunnel is wet
/// and the roofed road seen from outside is not.
#[must_use]
pub fn wetness(clock: f32, snow: bool) -> f32 {
    if snow {
        return 0.0;
    }
    (clock / WET_RAMP_SECONDS).clamp(0.0, 1.0)
}

/// Marks every road vertex of a covered section dry: clears
/// [`oag_mesh::mesh::ROAD_MARK`] on the vertices of each draw call whose scene
/// node a covered section governs (`oag_vex::pvs::governing_sections`, the
/// rule the PVS places draws by). A section the file declares no anchor for
/// stays open. Returns how many vertices it cleared. Chosen, not measured:
/// the original has no wet road.
pub fn dry_covered_road(
    model: &mut oag_mesh::mesh::Model,
    governing: &[Option<u8>],
    anchors: &[CoveredAnchor],
) -> usize {
    let mask = covered::covered_mask(anchors);
    if mask == 0 {
        return 0;
    }
    let lists = [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ];
    let mut ranges = Vec::new();
    for draw in lists.into_iter().flatten() {
        let section = draw
            .node
            .and_then(|node| governing.get(node as usize).copied().flatten());
        if section.is_some_and(|id| covered::covered(mask, i32::from(id))) {
            ranges.push(draw.range.clone());
        }
    }
    clear_road_marks(&mut model.vertices, &model.indices, &ranges)
}

/// Clears the road mark of every vertex the index `ranges` reference, and
/// counts the marks it cleared.
fn clear_road_marks(
    vertices: &mut [oag_mesh::mesh::GpuVertex],
    indices: &[u32],
    ranges: &[std::ops::Range<u32>],
) -> usize {
    let mut cleared = 0;
    for range in ranges {
        let Some(indices) = indices.get(range.start as usize..range.end as usize) else {
            continue;
        };
        for &index in indices {
            if let Some(vertex) = vertices.get_mut(index as usize)
                && vertex.texcoord2[0] != 0.0
            {
                vertex.texcoord2[0] = 0.0;
                cleared += 1;
            }
        }
    }
    cleared
}

/// What a circuit asks for and where it is covered, read at load.
#[derive(Debug, Clone, PartialEq)]
pub struct Setup {
    /// The circuit's `<Weather>` element.
    pub config: Config,
    /// Its `weatherPos` anchors, one per covered section.
    pub anchors: Vec<CoveredAnchor>,
    /// `Tex` decoded, the mist overlay's texture; `None` when the circuit
    /// names none or it did not decode, and then the mist draws nothing.
    pub mist_texture: Option<oag_fx::exhaust::FlareTexture>,
    /// Where this project's own splash art sits on the particle sheet, when
    /// it was found and placed; `None` and the enhanced weather draws no
    /// splash (see [`super::splash`]).
    pub splash_cell: Option<[f32; 4]>,
    /// A track module's rain rather than the circuit's own: idle until a module
    /// sends `set-weather` ([`Weather::drive`]), then played at that strength, drawn
    /// with the enhanced effects whatever the player's setting. `false` for a
    /// circuit's authored weather, which plays from the first frame as it always did.
    pub driven: bool,
}

/// The weather of a race: inert unless the circuit authors one.
#[derive(Debug, Clone)]
pub struct Weather {
    setup: Option<Setup>,
    mask: u64,
    effect: Option<Arc<Effect>>,
    resolved: bool,
    /// Boxed: a pool is some 25 KB, and a `Race` is moved by value.
    pool: Box<System>,
    /// The sections published last frame and this frame, `DAT_08ab10a4` and
    /// `DAT_08ab10a0`.
    sections: (i32, i32),
    /// Whether the effect sits at its anchor rather than on the camera: the
    /// original's instance flag `0x200000`.
    anchored: bool,
    anchor: Frame,
    previous: Option<Frame>,
    wind: Wind,
    lens: Lens,
    mist: Option<Mist>,
    clock: f32,
    rng: Rng,
    /// The enhanced weather's splashes: empty and never touched at
    /// `original`. Own random stream, so they never move the drops'.
    splashes: Splashes,
    /// The enhanced weather's road spray: same rules, own stream.
    spray: Spray,
    /// The last frame's wind step, a drop's move in the world a tick.
    step: Vec3,
    /// A driven setup's strength this tick, `0.0..=1.0`; `1.0` and never read for an
    /// authored weather.
    strength: f32,
    /// The effect's authored emissions per tick, to scale by [`Self::strength`].
    emission: Option<Vec<(u32, u32)>>,
}

impl Default for Weather {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Weather {
    /// Nothing plays until the first [`Self::advance`].
    #[must_use]
    pub fn new(setup: Option<Setup>) -> Self {
        let driven = setup.as_ref().is_some_and(|s| s.driven);
        let mut rng = Rng::new(SEED);
        let config = setup.as_ref().map(|s| &s.config);
        let wind = Wind::new(
            config.map_or(0.0, |c| c.wind_base),
            config.map_or(0.0, |c| c.wind_range),
            config.map_or(0.0, |c| c.drift_y),
            &mut rng,
        );
        let mask = setup
            .as_ref()
            .map_or(0, |s| covered::covered_mask(&s.anchors));
        let mist = setup
            .as_ref()
            .filter(|s| s.mist_texture.is_some())
            .and_then(|s| Mist::new(&s.config));
        Self {
            setup,
            mask,
            effect: None,
            resolved: false,
            pool: Box::new(System::new()),
            sections: (-1, -1),
            anchored: false,
            anchor: Frame::IDENTITY,
            previous: None,
            wind,
            lens: Lens::new(None),
            mist,
            clock: 0.0,
            rng,
            splashes: Splashes::default(),
            spray: Spray::default(),
            step: Vec3::ZERO,
            strength: if driven { 0.0 } else { 1.0 },
            emission: None,
        }
    }

    /// Whether a track module drives this weather rather than the circuit.
    #[must_use]
    pub fn is_driven(&self) -> bool {
        self.setup.as_ref().is_some_and(|s| s.driven)
    }

    /// Whether a driven weather is raining now: the enhanced effects are forced on for
    /// it. Always `false` for a circuit's authored weather.
    #[must_use]
    pub fn is_driven_active(&self) -> bool {
        self.is_driven() && self.strength > 0.0
    }

    /// Whether anything of this weather is drawn and run: an authored weather always,
    /// a driven one while its strength is above zero.
    fn is_active(&self) -> bool {
        !self.is_driven() || self.strength > 0.0
    }

    /// Sets a driven weather's strength for this tick (`set-weather`, `0.0` when none
    /// was sent). The weather, once started, keeps its pool and clock; at `0.0` it
    /// neither runs nor draws.
    pub fn drive(&mut self, strength: f32) {
        if self.is_driven() {
            self.strength = strength.clamp(0.0, 1.0);
            self.lens.set_gain(self.strength);
            if let Some(mist) = &mut self.mist {
                mist.set_gain(self.strength);
            }
        }
    }

    /// Whether the circuit authors weather.
    #[must_use]
    pub fn is_authored(&self) -> bool {
        self.setup.is_some()
    }

    /// Whether the env effect loaded and is playing.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.effect.is_some() && self.pool.is_running()
    }

    /// The section last published, or `-1` before the first.
    #[must_use]
    pub fn section(&self) -> i32 {
        self.sections.1
    }

    /// Whether it sits at its anchor, a covered section's.
    #[must_use]
    pub fn is_anchored(&self) -> bool {
        self.anchored
    }

    /// The raindrops on the glass.
    #[must_use]
    pub fn lens(&self) -> &Lens {
        &self.lens
    }

    /// The mist overlay, when the circuit authors one and its texture decoded.
    #[must_use]
    pub fn mist(&self) -> Option<&Mist> {
        self.mist.as_ref().filter(|_| self.is_active())
    }

    /// The mist overlay's decoded texture, for the renderer.
    #[must_use]
    pub fn mist_texture(&self) -> Option<&oag_fx::exhaust::FlareTexture> {
        self.setup.as_ref()?.mist_texture.as_ref()
    }

    /// Where the splash art sits on the particle sheet.
    #[must_use]
    pub fn splash_cell(&self) -> Option<[f32; 4]> {
        self.setup.as_ref()?.splash_cell
    }

    /// Whether the env effect is the snow rather than the rain.
    #[must_use]
    pub fn is_snow(&self) -> bool {
        self.setup
            .as_ref()
            .and_then(|s| s.config.env_psys.as_deref())
            .is_some_and(|path| effect_name(path) == oag_title::engine_effects::SNOW_EFFECT)
    }

    /// Whether `section` is a covered one: the original's own test
    /// (`Section_IsCovered`), read here for a hit's or a draw's section rather
    /// than the camera's. A section below zero counts as covered, as there.
    #[must_use]
    pub fn section_is_covered(&self, section: i32) -> bool {
        covered::covered(self.mask, section)
    }

    /// How wet the enhanced weather's road is now, `0.0..=1.0`; see [`wetness`].
    /// `0.0` for a circuit with no playing weather.
    #[must_use]
    pub fn wet_road(&self) -> f32 {
        if !self.is_playing() || !self.is_active() {
            return 0.0;
        }
        wetness(self.clock, self.is_snow()) * self.strength
    }

    /// The enhanced weather's splashes.
    #[must_use]
    pub fn splashes(&self) -> &Splashes {
        &self.splashes
    }

    /// The enhanced weather's road spray.
    #[must_use]
    pub fn spray(&self) -> &Spray {
        &self.spray
    }

    /// The spray, to age and add to.
    pub fn spray_mut(&mut self) -> &mut Spray {
        &mut self.spray
    }

    /// The splashes, to age and add to.
    pub fn splashes_mut(&mut self) -> &mut Splashes {
        &mut self.splashes
    }

    /// The last move of every drop in the open, as a world segment from where
    /// it was a tick ago to where it is drawn: what a splash is found on.
    ///
    /// Empty under cover (the original's anchored box: no hits there), before
    /// the pool plays, and for a circuit with no field. The pool is not touched.
    #[must_use]
    pub fn drop_segments(&self) -> Vec<(Vec3, Vec3)> {
        let (Some(effect), Some(previous), false, true) =
            (&self.effect, self.previous, self.anchored, self.is_active())
        else {
            return Vec::new();
        };
        if effect.field.is_none() || !self.pool.is_running() {
            return Vec::new();
        }
        let anchor = Anchor::Camera {
            previous,
            current: previous,
        };
        self.pool
            .world_positions(&anchor, effect.view_depth)
            .map(|at| (at - self.step, at))
            .collect()
    }

    /// Appends the splashes' geometry, when their art is placed.
    pub fn extend_splash_vertices(
        &self,
        additive: &mut Vec<GpuVertex>,
        alpha_over: &mut Vec<GpuVertex>,
        right: Vec3,
        up: Vec3,
    ) {
        if let Some(cell) = self.splash_cell().filter(|_| self.is_active()) {
            self.splashes
                .extend_vertices(additive, alpha_over, right, up, cell);
            self.spray.extend_vertices(alpha_over, right, up, cell);
        }
    }

    /// The pool, for tests.
    #[must_use]
    pub fn pool(&self) -> &System {
        &self.pool
    }

    /// One frame: `camera` is where the lens is, `section` the one the craft is in.
    pub fn advance(&mut self, effects: &psys::Library, dt: f32, camera: Frame, section: i32) {
        let Some(setup) = &self.setup else {
            return;
        };
        if !self.is_active() {
            return;
        }
        if !self.resolved {
            self.resolved = true;
            self.effect = setup
                .config
                .env_psys
                .as_deref()
                .and_then(|path| effects.get(effect_name(path)))
                .cloned();
            if let Some(effect) = &self.effect {
                self.pool.ignite(effect, Vec3::ZERO, 1.0);
                self.emission = Some(effect.emitters.iter().map(|e| e.per_emission).collect());
            }
            let gain = self.lens.gain();
            self.lens = Lens::new(
                setup
                    .config
                    .screen_psys
                    .as_deref()
                    .and_then(|path| effects.get(effect_name(path))),
            );
            self.lens.set_gain(gain);
        }
        self.clock += dt;
        self.sections = (self.sections.1, section);
        if self.sections.0 != self.sections.1 {
            let was = covered::covered(self.mask, self.sections.0);
            let now = covered::covered(self.mask, self.sections.1);
            if now {
                self.anchored = true;
                let at = setup
                    .anchors
                    .iter()
                    .find(|a| i32::from(a.section) == section);
                if let Some(anchor) = at {
                    self.anchor = frame_of(&anchor.world);
                }
            } else if was {
                self.anchored = false;
            }
            let edge = match (was, now) {
                (false, true) => Some(Edge::Entered),
                (true, true) => Some(Edge::Held),
                (true, false) => Some(Edge::Left),
                (false, false) => None,
            };
            if let Some(edge) = edge {
                self.lens.edge(edge, self.clock);
                if let Some(mist) = &mut self.mist {
                    mist.edge(edge);
                }
            }
        }
        self.wind.advance(dt, self.clock);
        if let Some(mist) = &mut self.mist {
            let wind = self.wind.modifier();
            mist.advance(dt, self.previous, camera, wind);
        }
        self.lens
            .advance(dt, self.clock, camera, self.wind.vector());
        if let Some(authored) = self.effect.clone()
            && let Some(field) = authored.field
        {
            let driven;
            let effect: &Effect = match &self.emission {
                Some(emission) if self.is_driven() => {
                    let mut scaled = (*authored).clone();
                    for (emitter, (low, high)) in scaled.emitters.iter_mut().zip(emission) {
                        let by = |n: u32| (n as f32 * self.strength).round() as u32;
                        emitter.per_emission = (by(*low), by(*high));
                    }
                    driven = scaled;
                    &driven
                }
                _ => &authored,
            };
            let spec = FieldSpec {
                half_extent: field.half_extent,
                depth: effect.view_depth,
                wind: self.wind.modifier(),
            };
            let anchor = self.anchor_for(camera);
            self.step = spec.wind;
            self.pool
                .advance_field(effect, dt, &anchor, &spec, &mut self.rng);
        }
        self.previous = Some(camera);
    }

    fn anchor_for(&self, camera: Frame) -> Anchor {
        if self.anchored {
            Anchor::World(self.anchor)
        } else {
            Anchor::Camera {
                previous: self.previous.unwrap_or(camera),
                current: camera,
            }
        }
    }

    /// Appends this frame's geometry, with the camera as it is drawn.
    pub fn extend_vertices(
        &self,
        additive: &mut Vec<GpuVertex>,
        alpha_over: &mut Vec<GpuVertex>,
        camera: Frame,
        right: Vec3,
        up: Vec3,
    ) {
        if !self.is_active() {
            return;
        }
        self.lens
            .extend_vertices(additive, alpha_over, camera, right, up);
        let Some(effect) = &self.effect else {
            return;
        };
        if effect.field.is_none() {
            return;
        }
        let anchor = if self.anchored {
            Anchor::World(self.anchor)
        } else {
            Anchor::Camera {
                previous: camera,
                current: camera,
            }
        };
        self.pool
            .in_world(&anchor, effect.view_depth)
            .extend_vertices(additive, alpha_over, effect, right, up, None);
    }
}

/// A row-major matrix's rows as a [`Frame`].
fn frame_of(world: &[f32; 16]) -> Frame {
    let row = |at: usize| Vec3::new(world[at], world[at + 1], world[at + 2]);
    Frame {
        position: row(12),
        right: row(0),
        up: row(4),
        back: row(8),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rain_wets_the_road_over_time_and_snow_does_not() {
        assert_eq!(wetness(0.0, false), 0.0);
        let early = wetness(10.0, false);
        let later = wetness(30.0, false);
        assert!(0.0 < early && early < later && later < 1.0);
        assert_eq!(wetness(WET_RAMP_SECONDS * 3.0, false), 1.0);
        assert_eq!(wetness(100.0, true), 0.0, "snow leaves the road dry");
    }

    #[test]
    fn only_the_vertices_of_a_covered_draw_lose_the_road_mark() {
        let road = oag_mesh::mesh::GpuVertex {
            texcoord2: [oag_mesh::mesh::ROAD_MARK, 0.0],
            ..bytemuck::Zeroable::zeroed()
        };
        let mut vertices = [road; 6];
        let indices = [0, 1, 2, 3, 4, 5];
        let cleared = clear_road_marks(&mut vertices, &indices, std::slice::from_ref(&(3..6)));
        assert_eq!(cleared, 3);
        let marked: Vec<bool> = vertices.iter().map(|v| v.texcoord2[0] != 0.0).collect();
        assert_eq!(marked, [true, true, true, false, false, false]);
    }

    #[test]
    fn a_circuit_with_no_weather_is_dry() {
        assert_eq!(Weather::default().wet_road(), 0.0);
    }
}
