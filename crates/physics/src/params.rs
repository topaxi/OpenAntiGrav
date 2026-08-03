//! The physics parameter set, one block per ship and speed class.
//!
//! Every field here is a tunable the original reads from
//! `Data\Ships\<Team>\handlingstats.xml` on the player's own disc. Nothing in
//! this crate ships a value: [`Handling::ZERO`] exists so a test can name the
//! two or three fields it cares about, and everything real is loaded.
//!
//! # Why this is not the format's own type
//!
//! `oag_formats::handling` parses the XML, cameras and front-end bars included,
//! and its types describe the *document*. This one describes what the force law
//! consumes, and it exists separately so that `oag-physics` depends on nothing
//! but `oag-core` - a schema change cannot reach the simulation without someone
//! deliberately updating the mapping in `oag-gameplay`. The field names are kept
//! identical to the XML attribute names on purpose, so the mapping is
//! inspectable rather than clever.
//!
//! The schema is documented in `docs/formats/handling-stats.md`; how each field
//! is consumed is in `docs/physics/README.md` and
//! `docs/ghidra/functions/psp-pulse/engine.md`, which is the authority where the
//! two disagree. Per `docs/architecture/adr/0006-no-copyrighted-content.md` no
//! values are reproduced here or anywhere else in the repository.
//!
//! # This is the in-memory form, already scaled
//!
//! **Four fields are pre-scaled by the original's XML loader**, so the number the
//! craft code reads is not the number in the file. `oag-physics` holds the
//! *scaled* form, the same value the original's force law consumes, and the force
//! law here is written against that:
//!
//! | Field | Stored as | Consequence |
//! | --- | --- | --- |
//! | [`Engine::amount`] | `xml * 0.001` | |
//! | [`Brakes::amount`] | `xml * -0.01` | **negative in memory**; the brake force is applied along `+unit(v)` and the sign lives in the parameter |
//! | [`Airbrake::amount`] | `xml * 0.0001` | |
//! | [`Airbrake::slidegrip`] | `xml * 0.0001` | an XML 0..100 becomes 0..0.01, which is what makes the grip coefficient reach exactly zero at full airbrake |
//!
//! Confidence 88, from the parsers themselves
//! (`HandlingXml_ParseEngine` at `0x0883945c` and friends).
//!
//! **Applying those factors is `oag-gameplay`'s job, not this crate's and not
//! `oag_formats::handling`'s**, which deliberately returns the document's raw
//! values. Applying them twice is the most likely integration bug in this area,
//! and it would be silent: the ship would simply be sluggish.

/// Engine response. `<Engine accelcap amount falloff gain turbo/>`.
///
/// Offsets `0xb8..0xcc` of the class block; see
/// `docs/ghidra/functions/psp-pulse/engine.md`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Engine {
    /// Base of the thrust ceiling: `cap = 0.5 * speed + accelcap`.
    pub accelcap: f32,
    /// Thrust magnitude, **already scaled by `0.001`**.
    pub amount: f32,
    /// **Dead.** Parsed at `+0xc0` and consumed by nothing.
    ///
    /// `Ship_UpdateEngine` computes a throttle ramp in exactly the shape the
    /// airbrakes use, stores it at `craft+0x2b8`, and then overwrites it with the
    /// raw input at `0x0884c728` before computing thrust from the raw value. Both
    /// branches of the ramp land on that store. Verified in disassembly rather
    /// than in the decompiler, confidence 85. Kept here because it really is in
    /// the data, and because reintroducing the ramp is the obvious mistake: the
    /// resulting throttle lag reads as "feels close enough".
    pub falloff: f32,
    /// **Dead**, at `+0xb8`. See [`Self::falloff`].
    pub gain: f32,
    /// Added to thrust while the turbo flags and mode allow it.
    pub turbo: f32,
}

/// Braking response. `<Brakes amount falloff gain/>`.
///
/// Offsets `0xac..0xb8`. There is **no brake axis**: the brake engages when both
/// airbrake inputs are positive at once, and it is applied only while grounded.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Brakes {
    /// Brake magnitude, **already scaled by `-0.01` and so negative**.
    ///
    /// The force is applied along `+unit(velocity)`, so the sign is carried here.
    /// Negating at the use site as well would accelerate under braking.
    pub amount: f32,
    /// Per-second decay of the brake ramp. **Live**, unlike the engine's.
    pub falloff: f32,
    /// Per-second rise of the brake ramp, toward a ceiling of 100.
    pub gain: f32,
}

/// Steering response. `<Turning amount falloff gain/>`.
///
/// Offsets `0xcc..0xd8`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Turning {
    /// Yaw authority. Feeds body-local yaw directly, with **no speed factor**, so
    /// steering authority at a standstill is not zero.
    pub amount: f32,
    /// Per-second rate while the steering state moves back toward centre.
    ///
    /// Asymmetric with [`Self::gain`] by intent rather than by sign: the pair gives
    /// a fast bite and a slow return, or the reverse.
    pub falloff: f32,
    /// Per-second rate while the steering state moves toward a larger-magnitude
    /// target.
    pub gain: f32,
}

/// Airbrake response. `<Airbrake amount drag falloff gain turn slidegrip sideshift/>`.
///
/// All seven names were recovered twice independently, from the XML and from the
/// parameter block at `+0xd8`, which is what retired the three the static
/// reading had left unresolved.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Airbrake {
    /// **A lateral force gain, not a drag.** Scales the sideways force from an
    /// airbrake imbalance. **Already scaled by `0.0001`.**
    pub amount: f32,
    /// Feeds the forward slide term. Whether it accelerates or decelerates is
    /// unresolved; see `docs/physics/README.md`.
    pub drag: f32,
    /// Per-second decay toward the analog input.
    pub falloff: f32,
    /// Per-second rise toward the analog input.
    pub gain: f32,
    /// Feeds body-local angular acceleration directly.
    pub turn: f32,
    /// Percent of lateral grip retained at full airbrake, **already scaled by
    /// `0.0001`** and so held on `0.0..=0.01`.
    ///
    /// The scaling is what makes the grip coefficient's `(0.01 - slidegrip)` reach
    /// exactly zero at an XML value of 100. Confidence 90, arithmetic rather than
    /// interpretation.
    pub slidegrip: f32,
    /// One-shot lateral impulse, applied straight to the body.
    pub sideshift: f32,
}

/// Antigravity and suspension. `<Antigrav grip_air grip_ground landing_rebound rebound rebound_jump_time ride_height/>`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Antigrav {
    /// Lateral grip coefficient while airborne.
    pub grip_air: f32,
    /// Lateral grip coefficient while grounded.
    pub grip_ground: f32,
    /// Replaces [`Self::rebound`] for the first 0.2 s after touchdown.
    pub landing_rebound: f32,
    /// Scales suspension **damping only**, never the spring.
    pub rebound: f32,
    /// Unused by the recovered force law; carried because the XML has it.
    pub rebound_jump_time: f32,
    /// The raycast length for the hover probes, **and the primary term of the
    /// hover spring's target height**.
    ///
    /// `docs/physics/README.md` originally said this "never appears in the force
    /// law". `docs/ghidra/functions/psp-pulse/engine.md` traced the offset chain and
    /// showed otherwise, at confidence 88: the parser stores it at `+0x94`,
    /// `craft+0x70` points at it, and `Ship_UpdateCraft` builds the spring target
    /// `craft+0x2f0` from it. See [`crate::hover::target_height`].
    pub ride_height: f32,
}

/// Mass and the three gravity values. `<Physical flight_gravity mass normal_gravity track_gravity/>`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Physical {
    /// Gravity applied while airborne, blended in by `1 - grounded`.
    pub flight_gravity: f32,
    /// Ship mass, at `+0xf4`.
    ///
    /// **Every force term in the original reads the mass at `body+0x374` instead**,
    /// and how the two relate was not traced. This crate reads this field for the
    /// hover spring, per `docs/physics/README.md`, and [`crate::ship::Body::mass`]
    /// for gravity, per `docs/ghidra/functions/psp-pulse/engine.md`. Keeping the two
    /// equal is the integration layer's job.
    pub mass: f32,
    /// Gravity along world down while grounded, additionally scaled by the
    /// per-class factor in [`crate::forces::Environment::class_gravity_scale`] -
    /// which is authored under an attribute named `airborne`, and does not scale
    /// the airborne term. Class-block offset `+0xf8`.
    pub normal_gravity: f32,
    /// Gravity along the track's own up axis, which is what makes inversions work.
    ///
    /// **It appears only in the hover spring's calibration**, never in the gravity
    /// force itself: the inline gravity term blends `normal_gravity` against
    /// `flight_gravity` and does not mention this one.
    pub track_gravity: f32,
}

/// Pitch response. `<pitch pitch_air pitch_ground pitch_damping antigrav_height_adjust/>`.
///
/// Offsets `0x104..0x114`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Pitch {
    /// Pitch authority while airborne. A plain gain on the input axis, not a rate:
    /// there is no ramp and no state.
    pub pitch_air: f32,
    /// Pitch authority while grounded.
    pub pitch_ground: f32,
    /// Pitch-axis angular damping.
    ///
    /// **Consumed by `Ship_ApplyAngularDamping`, not by the pitch term.** It is the
    /// only per-ship component of the angular damping triple; yaw and roll are hard
    /// `-5.0` and `-2.0` for every craft in the game.
    pub pitch_damping: f32,
    /// Offset applied to the hover target height, **probably**.
    ///
    /// An additive offset does sit in the hover target chain, at `craft+0x74`, and
    /// nothing was found that writes this field there; a search for readers of
    /// `+0x110` in the craft path found none. That is a weak negative (it does not
    /// cover VFPU loads or a cached copy), so "parsed but never consumed" is a
    /// hypothesis at confidence 50 rather than a finding. [`crate::hover::target_height`]
    /// feeds it into that slot and says so.
    pub antigrav_height_adjust: f32,
}

/// Hull dimensions and the shield pool. From `<Misc/>`, which is per ship rather
/// than per speed class.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Dimensions {
    /// Hull height, and so the box half-extent used for contact generation.
    pub height: f32,
    /// Hull length. Also sets where the two hover probes sit.
    pub length: f32,
    /// Hull width.
    pub width: f32,
    /// Shield pool.
    pub shield: f32,
    /// Shield pool on the easier difficulties.
    pub easyshield: f32,
    /// Fore/aft mass bias.
    pub weight_distribution: f32,
}

/// The speed-pad boost, per speed class. From `<GlobalClass><SpeedupPads/>`.
///
/// The one member of [`Handling`] that does **not** come from the ship's own
/// `handlingstats.xml`: it lives in the engine-wide `Data\XML\HandlingStats.xml`
/// instead, and is per speed class rather than per team. It is here anyway
/// because it is a tunable the force law reads for one ship in one class, which
/// is what [`Handling`] is; where the number was authored is the format's
/// business, not this crate's.
///
/// Neither value is pre-scaled - see `oag_gameplay::handling::SCALED_FIELDS`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SpeedupPads {
    /// The force magnitude the boost settles at, `0x08b36bc0[class]`.
    pub amount: f32,
    /// How long the boost lasts, in seconds, `0x08b36bd0[class]`.
    pub time: f32,
}

/// Everything the force law reads for one ship in one speed class.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Handling {
    /// `<Engine/>`.
    pub engine: Engine,
    /// `<Brakes/>`.
    pub brakes: Brakes,
    /// `<Turning/>`.
    pub turning: Turning,
    /// `<Airbrake/>`.
    pub airbrake: Airbrake,
    /// `<Antigrav/>`.
    pub antigrav: Antigrav,
    /// `<Physical/>`.
    pub physical: Physical,
    /// `<pitch/>`.
    pub pitch: Pitch,
    /// `<Misc/>`, which is shared by all four of a ship's classes.
    pub dimensions: Dimensions,
    /// `<GlobalClass><SpeedupPads/>`, which is shared by all eight teams.
    pub speedup_pads: SpeedupPads,
}

impl Handling {
    /// Every tunable zero.
    ///
    /// Not a playable ship. It exists so a unit test can start from a known
    /// nothing and set only the two or three fields whose behaviour it is
    /// pinning, which keeps the test readable and keeps invented constants out
    /// of the repository.
    pub const ZERO: Self = Self {
        engine: Engine {
            accelcap: 0.0,
            amount: 0.0,
            falloff: 0.0,
            gain: 0.0,
            turbo: 0.0,
        },
        brakes: Brakes {
            amount: 0.0,
            falloff: 0.0,
            gain: 0.0,
        },
        turning: Turning {
            amount: 0.0,
            falloff: 0.0,
            gain: 0.0,
        },
        airbrake: Airbrake {
            amount: 0.0,
            drag: 0.0,
            falloff: 0.0,
            gain: 0.0,
            turn: 0.0,
            slidegrip: 0.0,
            sideshift: 0.0,
        },
        antigrav: Antigrav {
            grip_air: 0.0,
            grip_ground: 0.0,
            landing_rebound: 0.0,
            rebound: 0.0,
            rebound_jump_time: 0.0,
            ride_height: 0.0,
        },
        physical: Physical {
            flight_gravity: 0.0,
            mass: 0.0,
            normal_gravity: 0.0,
            track_gravity: 0.0,
        },
        pitch: Pitch {
            pitch_air: 0.0,
            pitch_ground: 0.0,
            pitch_damping: 0.0,
            antigrav_height_adjust: 0.0,
        },
        dimensions: Dimensions {
            height: 0.0,
            length: 0.0,
            width: 0.0,
            shield: 0.0,
            easyshield: 0.0,
            weight_distribution: 0.0,
        },
        speedup_pads: SpeedupPads {
            amount: 0.0,
            time: 0.0,
        },
    };
}

/// The four speed classes, in the order the XML lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpeedClass {
    /// Slowest.
    Venom,
    /// Second.
    Flash,
    /// Third.
    Rapier,
    /// Fastest.
    Phantom,
}

impl SpeedClass {
    /// All four, slowest first.
    pub const ALL: [Self; 4] = [Self::Venom, Self::Flash, Self::Rapier, Self::Phantom];

    /// The `name` attribute the XML uses.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Venom => "VENOM",
            Self::Flash => "FLASH",
            Self::Rapier => "RAPIER",
            Self::Phantom => "PHANTOM",
        }
    }

    /// Parses the XML's `name` attribute, case-insensitively.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|class| class.as_str().eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_class_names_round_trip() {
        for class in SpeedClass::ALL {
            assert_eq!(SpeedClass::from_name(class.as_str()), Some(class));
        }
        assert_eq!(SpeedClass::from_name("venom"), Some(SpeedClass::Venom));
        assert_eq!(SpeedClass::from_name("nonsense"), None);
    }

    /// `ride_height` looking like a target height is the single most likely
    /// misreading of this parameter set, so the correction is a test rather than
    /// only a doc comment.
    #[test]
    fn zero_handling_is_not_a_ship() {
        assert_eq!(Handling::ZERO.physical.mass, 0.0);
        assert_eq!(Handling::ZERO.antigrav.ride_height, 0.0);
    }
}
