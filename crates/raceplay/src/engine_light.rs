//! A craft's engine light: one vertex light per craft, behind its nozzle,
//! brightening with the boost. Wipeout HD, 2048 and Omega all carry it.
//!
//! **Omega's and 2048's is `EngineFlare_SubmitVertexLight` (`0x0173e350`,
//! `0x812726d6`), the HD law below with two more colours and a wider reach**:
//! a 2048-era craft's `(3.0, 0.5, 0.1)` at range 30 (`Radius`), the mag strip's
//! `(1.5, 0.1, 3.0)` over a strip in a 2048 mode, a wander of 0.2 and 1.0
//! scaled by `1 - boost_blend`, and no light at a range of 0.01 or less
//! (`docs/ghidra/functions/ps4-omega-eu/engine-light.md`, confidence 85, the
//! falloff 60 on Omega and 80 on 2048). The numbers are title data
//! (`oag_title::flare::EngineLightLaw`); [`record`] is the arithmetic. The
//! receivers' shader is HD's `SVC1` combine, taken by inheritance: Omega's and
//! 2048's own fragment combine of the light was not read.
//!
//! **HD's is `EngineFlare_SubmitSpuLight` (`0x0029ff28` in `ps3-hdfury-eu`),
//! read in full** - `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "The
//! captured buffer's producer is found", confidence 85. Once per craft per
//! frame it submits one candidate to `SpuLight_AddCandidate`:
//!
//! ```text
//! anchor   = flare_node.position - flare_node.z_axis * Distance
//! position = anchor + (U(-0.1, 0.1), U(-0.1, 0.1), 0)
//! D        = Radius + U(-0.1, 0.1)
//! w        = 1.0
//! base     = Fury skin ? (40, 10, 4) : (4, 10, 40)
//! colour   = base * (1 + 10 * boost_blend)
//! ```
//!
//! where `Distance`/`Radius` are the ship directory's `EngineLightData.xml`
//! (`oag_livery::engine_light`), the flare node is the `Engine Flare`
//! locator under the craft's own world matrix, the Fury-skin byte is the
//! one `Ship_SetFuryTrailFlag` writes (the same flag that turns the trail
//! red - `RaceView::hd_trail_red`), and `boost_blend` is the sum
//! `EngineFlare_PlaceShapes` scales `EF_Main`/`EF_Boost` by - the boost snap
//! and its `*0.8`-per-substep decay, `oag_fx::exhaust::hd::Flame`. The
//! jitter is a `vec4` the original rewrites every `EngineFlare_Update` tick
//! from its uniform RNG (`FUN_0028c660`), `(x, y, 0, D)`, range `0x3dcccccd`
//! = 0.1 either side.
//!
//! Every one of those numbers is corroborated by the live capture in
//! `data/traces/hd-spu-light-companion/`: 40 records, all `w = 1.0`, 21
//! exactly `(40, 10, 4)`, the rest `(40, 10, 4) * (1 + 10 * blend)` with
//! `blend` at the snap or on its decay tail, and `D` in `0.62`-`2.05` =
//! `Radius` 0.7-2.0 plus the jitter.
//!
//! # What is this project's
//!
//! - **The RNG.** All randomness here goes through [`oag_core::Rng`], one
//!   stream per craft seeded off [`ENGINE_LIGHT_SEED`], drawn three times per
//!   60 Hz tick. The original draws per `EngineFlare_Update` call; how many
//!   of those run per frame was not counted, and the jitter is noise either
//!   way. **Chosen, not measured.**
//! - **Not the blue-to-orange transition tint.** `EngineFlare_SubmitSpuLight`
//!   has a second branch, selected by a countdown at `flare+0x240` and a flag
//!   at `+0x2b4`, that fades `(4, 10, 40)` to `(80, 10, 0)`; what arms it is
//!   unread, so it stays unwired rather than fired on a guess. A capture
//!   showed it once, on a different boot.
//!
//! # Who receives it, and why the floor is not it (the hull binding is measured)
//!
//! [`Race::hd_engine_lights`] is bound into the `Scene` uniform of the track
//! chunks **and of the craft**. Bound to the track alone it changed zero
//! pixels of an Amphiseum frame, and the numbers say it never can at ride
//! height: a craft rides at `ride_height * 0.75` = 4.1 units (Feisar) above
//! the floor while `D` is 0.7-2.0 and the light sits at or inside the nozzle
//! (`Distance` runs to -0.4). **The original's own records agree**, measured
//! rather than inferred: every one of the 40 live-captured light positions in
//! `data/traces/hd-spu-light-companion/` sits 2.98-4.41 units (mean 4.03)
//! from the nearest collision triangle of the circuit it was captured on,
//! against 2.85-4.53 (mean 4.09) for this project's lights on the same
//! circuit over a 1,200-tick race.
//! Not one record on either side is within its own `D` of the floor. (That
//! circuit is **Talon's Junction**, not the Amphiseum renderer.md's capture
//! entry names: scored against all sixteen circuits' collision soups,
//! `hd_engine_light_which_circuit.rs`, Talon's Junction puts 40 of 40
//! records 3-4.4 units off a surface and Amphiseum 3 of 40 within 5.)
//!
//! So at rest the only surface within `D` is the craft's own engine housing,
//! which is what a per-ship `Distance` reads as being tuned for, and what
//! the hull *can* take: `scripts/ps3-sho.py svc-twins <image>
//! materials/ships` finds 74 ship materials compiled with `SVC1` twins
//! (1,776 pairs, every `RigidBody` class included, all 888 vertex blocks
//! carrying the `(255, 128)` decode and the `0x868f8229` attribute). The
//! picture agrees: the Fury housings take the warm tint the reference
//! capture shows, and boosted (`1 + 10 * blend`, a 440-unit light a hand's
//! breadth from the housing) the whole rear of the hull washes warm.
//!
//! **The runtime half is read live (confidence 90).** The
//! original selects `SVC1` for the hull. `Ship_ReloadModelForSkin`
//! registers the hull `ship.vex` with flag word `0x1ccb`
//! (`0x000dc044`), and `ModelRecord_Create` (`0x003f0348`) stores it at
//! `record+0xe4`. `0x1ccb` carries the `0x800` bit that `Ship_DrawModels`
//! (`0x003ea368`) gates on. On RPCS3 in a Talon's Junction race, the
//! player's hull record read `0x1ccb`. A breakpoint on the gate's taken
//! branch (`0x003eb368`, just before `li r5,0x800`) stopped 10 times with
//! the player's hull record in `r19`, at cruise and with a boosted light
//! in the visible list. See `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`,
//! "The hull's `record+0xe4 & 0x800` bit is set at load". So the craft
//! binding below is the original's. It is also what whitens the housing
//! rim and inner rear panels at a boost, and **the original does the same**
//! (confidence 85). Its flare node matches our locator on
//! `Assegai`, `Assegai_c1` and `Harimau`. An injected light lights the same
//! panels on both sides. In a real boosted race frame, holding the player's
//! light at its rest colour removes the original's own white rim (Assegai)
//! and white inner channel (Harimau). See renderer.md, "The original washes
//! `Assegai` and `Harimau` at a real boost the way ours does".
//!
//! The track floor and walls light up only when a craft is
//! within `D` of them: landings, wall scrapes and banked sections.
//!
//! # The circuit's switch does not reach the hull (read live)
//!
//! `Lighting.Enable spu vertex lights` is authored `0` on Metropia and Sol 2.
//! The original's hull still draws its `SVC1` variant there, and the
//! attribute-4 light buffer of the hull's chunks holds the engine light: on
//! Sol 2 (Harimau classic, at rest) the bay chunks' records decode to a
//! blue-dominant `0.1 : 0.25 : 1` (raw `(15, 37, 148)` at exponent byte 133),
//! the literal `(4, 10, 40)`'s ratio. So the switch gates the track's lights
//! (and ours: [`Race::hd_engine_lights`]) and not the craft's
//! ([`Race::hd_craft_engine_lights`]). Evidence:
//! `docs/rendering/hd-ship-materials.md`, "The engine bay".

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_gameplay::MAX_SHIPS;
use oag_mesh::mesh_render::SpuLight;

use super::Race;
use oag_livery::engine_light::EngineLight;

/// Seed for the engine light's jitter draws. Distinct from every other seed
/// in `race.rs` for the same determinism reason those are distinct from
/// each other: three floats a tick that the simulation must never see, and
/// that must not shift `EXHAUST_SEED`'s stream, which captures already pin.
pub const ENGINE_LIGHT_SEED: u64 = 0x5_9a_2b_04;

/// The per-craft seed: [`ENGINE_LIGHT_SEED`] plus the slot, on the same
/// SplitMix64-decorrelated terms as `race::effects::exhaust_seed`.
pub(super) fn engine_light_seed(slot: usize) -> u64 {
    ENGINE_LIGHT_SEED + slot as u64
}

/// The falloff exponent every record carries: `0x008b2ef4 = 1.0`.
const EXPONENT: f32 = 1.0;

/// The least range the original submits (`0x01863ae8`, `0.01`).
const LEAST_RANGE: f32 = 0.01;

/// One craft's wander this tick, in the original's own lane order
/// `(x, y, 0, D)`, each a uniform draw in `[-1, 1)`; the law's
/// [`oag_title::flare::Wander`] scales it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(super) struct Jitter {
    pub x: f32,
    pub y: f32,
    pub d: f32,
}

impl Jitter {
    /// Three draws of `U(-1, 1)`.
    pub(super) fn draw(rng: &mut Rng) -> Self {
        let mut uniform = || rng.next_f32() * 2.0 - 1.0;
        Self {
            x: uniform(),
            y: uniform(),
            d: uniform(),
        }
    }
}

/// What a craft's light reads off the race this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Craft {
    /// The Fury skin's flag (`RaceView::hd_trail_red`).
    pub fury: bool,
    /// Whether the craft's mag-floor probe hit this tick
    /// (`mag_floor_fx::Shown::on`).
    pub over_strip: bool,
    /// `EngineFlare_PlaceShapes`' boost blend.
    pub boost_blend: f32,
    /// This tick's draws.
    pub jitter: Jitter,
}

/// The record one craft submits, from its authored light, its live pose and
/// state - the arithmetic of the original's submit function with nothing
/// added - or `None` when its range is at or under [`LEAST_RANGE`].
///
/// `model` is the craft's world matrix; `nozzle` and `axis` are the flare
/// locator's position and Z axis in the craft's model space, so
/// `model * (nozzle - axis * Distance)` is the original's
/// `node.row3 - node.row2 * dist` on the node's world matrix - the craft's
/// `0.75` scale included, exactly as the row carries it. `Radius` is **not**
/// scaled: the live `D` spans `Radius +- 0.1` unscaled (0.62-2.05 against
/// 0.7-2.0), which is what pins that.
///
/// Base: the 2048-era craft's, else the skin's. Over a strip the base is the
/// strip's and the gain is scaled. The wander is scaled by `1 - blend` where
/// the law says the boost calms it.
pub(super) fn record(
    model: oag_core::math::Mat4,
    nozzle: Vec3,
    light: EngineLight,
    craft: Craft,
) -> Option<SpuLight> {
    let law = light.law;
    let tint = match law.era_2048 {
        Some(era) if light.era_2048 => era.tint,
        _ if craft.fury => law.fury,
        _ => law.classic,
    };
    let (base, gain) = match law.mag_strip {
        Some(strip) if light.strip_circuit && craft.over_strip => {
            (strip.colour, tint.gain * strip.gain_scale)
        }
        _ => (tint.colour, tint.gain),
    };
    let gain = craft.boost_blend * gain + 1.0;
    let calm = if law.wander.calmed_by_boost {
        1.0 - craft.boost_blend
    } else {
        1.0
    };
    let jitter = craft.jitter;
    let anchor = model.transform_point3(nozzle - light.axis * light.data.distance);
    let position = anchor
        + Vec3::new(
            jitter.x * law.wander.position * calm,
            jitter.y * law.wander.position * calm,
            0.0,
        );
    let range = light.data.radius + jitter.d * law.wander.range * calm;
    (range > LEAST_RANGE).then_some(SpuLight {
        position: [position.x, position.y, position.z, EXPONENT],
        colour: [base[0] * gain, base[1] * gain, base[2] * gain, range],
    })
}

impl Race {
    /// Redraws one craft's jitter for this tick - the original's
    /// `EngineFlare_Update` rewrite of `flare+0x250`. Called from the exhaust
    /// tick, on a title with an engine light law.
    pub(super) fn advance_engine_light_jitter(&mut self, slot: usize) {
        self.view.engine_light_jitter[slot] = Jitter::draw(&mut self.view.engine_light_rng[slot]);
    }

    /// This frame's engine lights as the **track** takes them: one per active
    /// craft with an authored engine light, or empty on a title with none, on a circuit that
    /// switches the SPU vertex lights off (`Setup::spu_vertex_lights`), and for
    /// every craft without one. The hull takes them whatever the circuit says:
    /// [`Self::hd_craft_engine_lights`].
    #[must_use]
    pub fn hd_engine_lights(&self) -> Vec<SpuLight> {
        if !self.view.spu_vertex_lights {
            return Vec::new();
        }
        self.hd_craft_engine_lights()
    }

    /// The engine lights as the **craft** takes them: the circuit's
    /// `Enable spu vertex lights` does not reach the hull. Sol 2 and Metropia
    /// author `0` and the original's hull still reads a blue `(4, 10, 40)`
    /// light buffer there (see this module's "The circuit's switch").
    #[must_use]
    pub fn hd_craft_engine_lights(&self) -> Vec<SpuLight> {
        if !self.view.hd_trail_active {
            return Vec::new();
        }
        (0..MAX_SHIPS)
            .filter(|&slot| self.sim.ship_active(slot))
            .filter_map(|slot| {
                let light = self.view.engine_lights.get(slot).copied().flatten()?;
                let nozzle = self.nozzle_local_of(slot)?;
                record(
                    self.ship_model_matrix_of(slot),
                    nozzle,
                    light,
                    Craft {
                        fury: self.view.hd_trail_red[slot] > 0.5,
                        over_strip: self.view.mag_floor_fx[slot].on,
                        boost_blend: self.view.hd_flame[slot].boost_blend(),
                        jitter: self.view.engine_light_jitter[slot],
                    },
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_core::math::Mat4;
    use oag_title::flare::{EngineLightLaw, Flare};

    const NO_JITTER: Jitter = Jitter {
        x: 0.0,
        y: 0.0,
        d: 0.0,
    };

    const REST: Craft = Craft {
        fury: false,
        over_strip: false,
        boost_blend: 0.0,
        jitter: NO_JITTER,
    };

    /// The law a title ships, so a title whose `engine_light` goes back to
    /// `None` fails here rather than passing on constants this file typed.
    fn law_of(title: &oag_title::Title) -> EngineLightLaw {
        let Flare::PerTeam(authored) = title.flare else {
            panic!("{}: no per-team flare", title.name);
        };
        *authored
            .engine_light
            .unwrap_or_else(|| panic!("{}: no engine light", title.name))
    }

    fn light(law: EngineLightLaw, distance: f32, radius: f32) -> EngineLight {
        EngineLight {
            law,
            era_2048: false,
            strip_circuit: false,
            data: oag_tables::enginelight::EngineLightData { distance, radius },
            axis: Vec3::Z,
        }
    }

    fn colour(light: EngineLight, craft: Craft) -> [f32; 4] {
        record(Mat4::IDENTITY, Vec3::ZERO, light, craft)
            .expect("a light in range")
            .colour
    }

    /// A Fury craft at rest is the literal the capture held 21 times.
    #[test]
    fn a_fury_engine_at_rest_is_forty_ten_four() {
        let law = law_of(oag_hd::TITLE);
        let craft = Craft { fury: true, ..REST };
        assert_eq!(colour(light(law, 0.0, 1.0), craft), [40.0, 10.0, 4.0, 1.0]);
        let at = record(Mat4::IDENTITY, Vec3::ZERO, light(law, 0.0, 1.0), craft).unwrap();
        assert_eq!(at.position, [0.0, 0.0, 0.0, 1.0]);
    }

    /// The snap: `blend = 1.0` is `(440, 110, 44)`, the capture's `k = 11`.
    #[test]
    fn the_boost_snap_is_eleven_times_the_base() {
        let law = law_of(oag_hd::TITLE);
        let craft = Craft {
            fury: true,
            boost_blend: 1.0,
            ..REST
        };
        assert_eq!(
            colour(light(law, 0.0, 1.0), craft),
            [440.0, 110.0, 44.0, 1.0]
        );
    }

    /// A classic skin swaps the red and blue lanes, on every title that has the base.
    #[test]
    fn a_classic_hd_era_hull_is_blue_on_every_title() {
        for title in [oag_hd::TITLE, oag_omega::TITLE, oag_2048::TITLE] {
            let law = law_of(title);
            assert_eq!(
                colour(light(law, 0.0, 1.0), REST),
                [4.0, 10.0, 40.0, 1.0],
                "{}",
                title.name
            );
            let fury = Craft { fury: true, ..REST };
            assert_eq!(
                colour(light(law, 0.0, 1.0), fury)[..3],
                [40.0, 10.0, 4.0],
                "{}",
                title.name
            );
        }
    }

    /// A 2048-era craft is the dim red light with Radius 30's reach, whatever its skin;
    /// HD, which has no such craft, never takes the branch.
    #[test]
    fn a_2048_era_craft_is_three_half_a_tenth_at_its_own_radius() {
        for title in [oag_omega::TITLE, oag_2048::TITLE] {
            let mut craft = light(law_of(title), 3.0, 30.0);
            craft.era_2048 = true;
            assert_eq!(colour(craft, REST), [3.0, 0.5, 0.1, 30.0], "{}", title.name);
            let fury = Craft { fury: true, ..REST };
            assert_eq!(colour(craft, fury)[..3], [3.0, 0.5, 0.1], "{}", title.name);
        }
        let mut hd = light(law_of(oag_hd::TITLE), 3.0, 30.0);
        hd.era_2048 = true;
        assert_eq!(colour(hd, REST)[..3], [4.0, 10.0, 40.0]);
    }

    /// Over a mag strip on a gated circuit the colour swaps and the boost gain is a fifth.
    #[test]
    fn a_mag_strip_swaps_the_colour_and_scales_the_gain() {
        for title in [oag_omega::TITLE, oag_2048::TITLE] {
            let mut hull = light(law_of(title), 0.0, 1.0);
            hull.strip_circuit = true;
            let over = Craft {
                over_strip: true,
                ..REST
            };
            assert_eq!(colour(hull, over)[..3], [1.5, 0.1, 3.0], "{}", title.name);
            // HD-era gain 10 * 0.2 = 2: `1 + 2 * 0.5`.
            let boosted = Craft {
                boost_blend: 0.5,
                ..over
            };
            assert_eq!(
                colour(hull, boosted)[..3],
                [3.0, 0.2, 6.0],
                "{}",
                title.name
            );
            // Not over the strip, or on a circuit the mode gate refuses: the skin's colour.
            assert_eq!(colour(hull, REST)[..3], [4.0, 10.0, 40.0]);
            hull.strip_circuit = false;
            assert_eq!(colour(hull, over)[..3], [4.0, 10.0, 40.0]);
        }
        assert!(law_of(oag_hd::TITLE).mag_strip.is_none());
    }

    /// `Distance` slides the anchor back along the locator's own axis, and
    /// the craft's matrix scale rides on that slide but not on `Radius`.
    #[test]
    fn distance_slides_along_the_axis_through_the_craft_matrix_and_radius_does_not() {
        let model = Mat4::from_scale(Vec3::splat(0.75));
        let at = record(
            model,
            Vec3::new(0.0, 0.0, -4.0),
            light(law_of(oag_hd::TITLE), 2.0, 1.5),
            Craft { fury: true, ..REST },
        )
        .unwrap();
        // (0, 0, -4) - (0, 0, 1) * 2 = (0, 0, -6), scaled by 0.75.
        assert_eq!(at.position, [0.0, 0.0, -4.5, 1.0]);
        assert_eq!(at.colour[3], 1.5);
    }

    /// The wander lands on `x`, `y` and `D` and never on `z`, at each title's own size.
    #[test]
    fn the_wander_moves_x_y_and_range_only_by_the_titles_size() {
        let jitter = Jitter {
            x: 0.5,
            y: -0.5,
            d: 1.0,
        };
        let craft = Craft { jitter, ..REST };
        for (title, position, range) in [
            (oag_hd::TITLE, 0.05, 0.1),
            (oag_omega::TITLE, 0.1, 1.0),
            (oag_2048::TITLE, 0.1, 1.0),
        ] {
            let at = record(
                Mat4::IDENTITY,
                Vec3::ZERO,
                light(law_of(title), 0.0, 2.0),
                craft,
            )
            .unwrap();
            assert_eq!(
                at.position,
                [position, -position, 0.0, 1.0],
                "{}",
                title.name
            );
            assert_eq!(at.colour[3], 2.0 + range, "{}", title.name);
        }
    }

    /// A boost calms the wander on Omega and 2048 and leaves HD's whole.
    #[test]
    fn a_full_boost_calms_the_wander_where_the_law_says_so() {
        let craft = Craft {
            boost_blend: 1.0,
            jitter: Jitter {
                x: 1.0,
                y: 1.0,
                d: 1.0,
            },
            ..REST
        };
        for title in [oag_omega::TITLE, oag_2048::TITLE] {
            let at = record(
                Mat4::IDENTITY,
                Vec3::ZERO,
                light(law_of(title), 0.0, 2.0),
                craft,
            )
            .unwrap();
            assert_eq!(at.position, [0.0, 0.0, 0.0, 1.0], "{}", title.name);
            assert_eq!(at.colour[3], 2.0, "{}", title.name);
        }
        let at = record(
            Mat4::IDENTITY,
            Vec3::ZERO,
            light(law_of(oag_hd::TITLE), 0.0, 2.0),
            craft,
        )
        .unwrap();
        assert_eq!(at.position, [0.1, 0.1, 0.0, 1.0]);
        assert_eq!(at.colour[3], 2.1);
    }

    /// Omega's wander of one unit takes a Radius 1.0 light to nothing, and the original
    /// submits nothing at or under 0.01.
    #[test]
    fn a_range_at_or_under_a_hundredth_submits_nothing() {
        let craft = Craft {
            jitter: Jitter {
                x: 0.0,
                y: 0.0,
                d: -1.0,
            },
            ..REST
        };
        let law = law_of(oag_omega::TITLE);
        assert!(record(Mat4::IDENTITY, Vec3::ZERO, light(law, 0.0, 1.0), craft).is_none());
        assert!(record(Mat4::IDENTITY, Vec3::ZERO, light(law, 0.0, 1.005), craft).is_none());
        assert!(record(Mat4::IDENTITY, Vec3::ZERO, light(law, 0.0, 1.02), craft).is_some());
    }

    /// Every draw stays inside the unit range the law scales.
    #[test]
    fn draws_stay_within_one() {
        let mut rng = Rng::new(engine_light_seed(3));
        for _ in 0..1000 {
            let j = Jitter::draw(&mut rng);
            for v in [j.x, j.y, j.d] {
                assert!((-1.0..=1.0).contains(&v), "{v}");
            }
        }
    }
}
