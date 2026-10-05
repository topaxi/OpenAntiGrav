//! `Weather`'s mist overlay: the opacity `Weather_Update` (`0x088f1e58`) eases
//! on the cover edges and the layers [`oag_fx::mist`] steps with the
//! camera. See `docs/ghidra/functions/psp-pulse-usa/weather.md`, "The mist
//! overlay".
//!
//! - **The opacity** starts at `Alpha` (`Weather_Construct` sets `+0x450` and
//!   its target `+0x454` both). Entering or staying under cover the target
//!   becomes `0`, leaving it `Alpha` - both only while `MistInside` is `0` -
//!   and `Weather_UpdateWind` eases it `+= (target - it) * 0.1` once per whole
//!   `1/60` s.
//! - **The drift** is `-(0.3 * wind) * DriftMistMult`, world units a second,
//!   taken out of the camera's move before the layers see it.
//! - **The layers** step only while the opacity is above zero, and nothing
//!   draws otherwise.
//!
//! **The first frame steps nothing**, chosen, not measured: the original's
//! sampler compares against whatever its node held before the first call, a
//! one-off jump that only moves the layers' phases by an arbitrary amount.

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_fx::mist::{self, Config as Layout, Layers, Motion};
use oag_fx::psys::field::Frame;
use oag_tables::trackstartup::Weather as Config;

use super::lens::Edge;

/// How far the opacity moves toward its target a step (`node+0x458`,
/// `0x3ccccccd`).
const EASE: f32 = 0.1;

/// The step the ease counts in, seconds (`0.016666668`).
const STEP: f32 = 0.016_666_668;

/// Seed for the layers' random UV offsets. Any constant: render state, never
/// hashed, and its own so the weather's particle draws do not shift.
const SEED: u64 = 0x6d15_7a11;

/// One race's mist.
#[derive(Debug, Clone)]
pub struct Mist {
    layout: Layout,
    alpha: f32,
    drift_mult: f32,
    holds_inside: bool,
    opacity: f32,
    target: f32,
    layers: Layers,
    rng: Rng,
}

impl Mist {
    /// From the circuit's `<Weather>` element, or nothing when it names no
    /// `Tex`.
    pub fn new(config: &Config) -> Option<Self> {
        config.tex.as_ref()?;
        let mut rng = Rng::new(SEED);
        Some(Self {
            layout: Layout {
                tex_scale: config.tex_scale,
                aspect: config.aspect_ratio,
                display_scale: config.display_scale,
            },
            alpha: config.alpha,
            drift_mult: config.drift_mist_mult,
            // `Xml_AttributeAsInt`: the "0.2" Outpost 7 authors reads `0`, and
            // its covered start section was read live with the target at `0`.
            holds_inside: config.mist_inside as i32 != 0,
            opacity: config.alpha,
            target: config.alpha,
            layers: Layers::new(&mut rng),
            rng,
        })
    }

    /// The opacity now, `node+0x450`.
    #[must_use]
    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    /// A section edge: the target `Weather_Update` sets on it.
    pub fn edge(&mut self, edge: Edge) {
        if self.holds_inside {
            return;
        }
        self.target = match edge {
            Edge::Entered | Edge::Held => 0.0,
            Edge::Left => self.alpha,
        };
    }

    /// One frame: the ease, then the layers' step from `previous` to `camera`.
    /// `wind_modifier` is `0.3 *` the wind, what the env effect is handed.
    pub fn advance(
        &mut self,
        dt: f32,
        previous: Option<Frame>,
        camera: Frame,
        wind_modifier: Vec3,
    ) {
        for _ in 0..(dt / STEP) as u32 {
            self.opacity += (self.target - self.opacity) * EASE;
        }
        let Some(previous) = previous else {
            return;
        };
        if self.opacity <= 0.0 {
            return;
        }
        let drift = -wind_modifier * self.drift_mult;
        let motion = Motion::between(&previous, &camera, drift, dt);
        self.layers
            .step(&self.layout, &motion, self.opacity, &mut self.rng);
    }

    /// This frame's two quads, or `None` while the opacity is not above zero.
    /// `tan_half_fov` is the drawn camera's vertical half-angle tangent.
    #[must_use]
    pub fn vertices(&self, tan_half_fov: f32) -> Option<[mist::Vertex; 12]> {
        if self.opacity <= 0.0 {
            return None;
        }
        let quads = self.layers.quads(self.layout.display_scale * tan_half_fov);
        Some(mist::vertices(&quads, self.layers.alphas()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(mist_inside: f32) -> Config {
        Config {
            tex: Some(r"Data\Tex\ScreenFX\Mist.mip".into()),
            alpha: 0.3,
            tex_scale: 3.1,
            display_scale: 0.8,
            aspect_ratio: 0.5625,
            drift_mist_mult: 6.0,
            mist_inside,
            ..Config::default()
        }
    }

    #[test]
    fn cover_fades_it_out_and_the_open_back_in() {
        let mut mist = Mist::new(&config(0.0)).unwrap();
        assert_eq!(mist.opacity(), 0.3);
        mist.edge(Edge::Entered);
        let camera = Frame::IDENTITY;
        mist.advance(1.0 / 60.0, None, camera, Vec3::ZERO);
        assert!((mist.opacity() - 0.27).abs() < 1e-6, "{}", mist.opacity());
        for _ in 0..200 {
            mist.advance(1.0 / 60.0, Some(camera), camera, Vec3::ZERO);
        }
        assert!(mist.opacity() < 1e-6);
        mist.edge(Edge::Left);
        mist.advance(1.0 / 60.0, Some(camera), camera, Vec3::ZERO);
        assert!(mist.opacity() > 0.029);
    }

    #[test]
    fn mist_inside_holds_it_through_cover() {
        let mut mist = Mist::new(&config(1.0)).unwrap();
        mist.edge(Edge::Entered);
        let camera = Frame::IDENTITY;
        for _ in 0..30 {
            mist.advance(1.0 / 60.0, Some(camera), camera, Vec3::ZERO);
        }
        assert_eq!(mist.opacity(), 0.3);
        assert!(mist.vertices(0.6249).is_some());
    }

    #[test]
    fn a_fractional_mist_inside_reads_as_zero() {
        let mut mist = Mist::new(&config(0.2)).unwrap();
        mist.edge(Edge::Entered);
        let camera = Frame::IDENTITY;
        mist.advance(1.0 / 60.0, None, camera, Vec3::ZERO);
        assert!(mist.opacity() < 0.3, "Outpost 7's 0.2 is the integer 0");
    }

    #[test]
    fn no_texture_no_mist() {
        let mut bare = config(0.0);
        bare.tex = None;
        assert!(Mist::new(&bare).is_none());
    }
}
