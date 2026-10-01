//! `handlingstats.xml`: every ship-handling tunable the game reads at runtime.
//!
//! Ship handling in Pulse is **data, not code**. There is one file per team at
//! `Data\Ships\<Team>\handlingstats.xml` inside `Data.wad`, and it carries the
//! whole parameter set: five camera rigs, the airbrake animation, hull
//! dimensions, the four front-end bars, and then one `<Class>` block per speed
//! class holding engine, brakes, steering, airbrake, antigravity, mass and
//! pitch. The physics code defines the *shape* of the model; these files supply
//! every constant in it.
//!
//! ```text
//! <Handling>
//!   <Stats team="...">
//!     <InternalCamera/> <BackwardCamera/> <BonnetCamera/>
//!     <ExternalCameraFar/> <ExternalCameraClose/>
//!     <AirbrakeGraphics/> <Misc/> <FE/>
//!     <Class name="VENOM|FLASH|RAPIER|PHANTOM">
//!       <Engine/> <Brakes/> <Turning/> <Airbrake/>
//!       <Antigrav/> <Physical/> <pitch/>
//!     </Class>            x4
//!   </Stats>
//! </Handling>
//! ```
//!
//! # There is a second file with the same root element
//!
//! `Handling_ParseStats` (`0x0883a2f0`) has **two** callers, and they pass
//! different files:
//!
//! - `0x088c291c` builds `%s\handlingstats.xml` per team - the schema above.
//! - `0x0894f6a8` passes the literal [`GLOBAL_ENTRY`],
//!   `Data\XML\HandlingStats.xml`, once at system-root creation.
//!
//! Both go through the same parser, which walks `<Handling>`'s children looking
//! for `<Stats>` *and* for `<Global>`, handing the latter to
//! `Xml_ReadGlobalSettings` (`0x0883a970`). `<Global>` carries engine-wide
//! statics: Zone mode's speed law, the speed-pad and weapon-pad tunables,
//! per-class gravity, the start boost and three camera pitch modifiers.
//!
//! **Which file carries which is measured, not assumed.** The ground-truth test
//! `which_top_level_elements_handlingstats_carries` reads all sixteen shipped
//! per-team files - eight teams on each of the PSP and PS2 discs - and every one
//! holds `<Stats>` and nothing else. So `<Global>` belongs to the global file
//! alone, and [`parse`] does not look for it. [`global_from_blob`] reads it.
//!
//! Only the parts this project has a consumer for are decoded out of `<Global>`:
//! `<Zone>`, and `<SpeedupPads>` and `<GravityMul>` under `<GlobalClass>`. The rest is named on
//! [`Global`] and in `docs/ghidra/functions/psp-pulse-usa/engine.md`, and can be added
//! when something needs it.
//!
//! The files are stored as [shortened XML](crate::fexml), so they go through
//! [`fexml::expand`] before they can be read: [`from_blob`] does both steps and
//! is what a caller holding an archive entry wants.
//!
//! # Nothing here defaults
//!
//! Every element and every attribute the schema lists is **required**, and a
//! missing one is an [`Error`] rather than a zero. This is the one design
//! decision in the module worth arguing for: a `mass` that quietly arrived as
//! `0.0` does not crash, it produces a ship that behaves oddly, and that reads
//! as a tuning problem or a bug in the force law. It would be looked for in the
//! wrong place for a long time. The same goes for `"nan"` and `"inf"`, which
//! Rust's `f32` parser accepts quite happily, so non-finite values are rejected
//! too.
//!
//! For the same reason the four `<Class>` blocks are collected into a
//! fixed-length array indexed by [`SpeedClass`]: a fifth block, a duplicate, an
//! unknown class name and a missing one are each a distinct error, and there is
//! no map whose iteration order could reach the simulation.
//!
//! # No values live here
//!
//! Per `docs/architecture/adr/0006-no-copyrighted-content.md` this module
//! describes the schema only. The names of the fields are a description of the
//! format; the numbers are the game's design data and are read at runtime from
//! the player's own disc. The unit-test fixtures use obviously invented values
//! for the same reason. **Pulse's own eight-team roster used to sit here
//! too**, an ADR-0022 title fact rather than a format one, and moved to
//! `oag_pulse::race::TEAMS` on 2026-09-01.
//!
//! See `docs/formats/handling-stats.md` for the evidence and the confidence
//! scores, and `oag_physics::Handling` for the subset the force law consumes.

use std::fmt;

use crate::fexml::{self, Node};

mod global;

pub use global::{
    ForeignGlobalClass, GLOBAL_ENTRY, Global, GravityMul, Special, SpeedupPads, StartBoost,
    WeaponPad, global_from_blob, parse_global,
};

/// The four speed classes, in the order the XML lists them.
///
/// A separate type from `oag_physics::SpeedClass` for the same reason the
/// parameter blocks are: this one describes the document, that one describes what
/// the simulation consumes, and `oag-physics` depends on nothing but `oag-core`.
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
    /// All four, slowest first. The position in this array is the discriminant,
    /// which is what makes [`Stats::classes`] indexable by class.
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

impl fmt::Display for SpeedClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Something wrong with a `handlingstats.xml` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The blob could not be expanded out of its shortened form.
    Expand(fexml::Error),
    /// A required element is absent.
    MissingElement {
        /// The element that was looked for.
        element: &'static str,
    },
    /// A required attribute is absent. Never defaulted: see the module docs.
    MissingAttribute {
        /// The element it should have been on.
        element: &'static str,
        /// The attribute that was looked for.
        attribute: &'static str,
    },
    /// An attribute is present but is not a finite number.
    ///
    /// Covers both text that will not parse and text that parses to `NaN` or an
    /// infinity, which `f32`'s parser accepts and the simulation must not see.
    NotANumber {
        /// The element it was on.
        element: &'static str,
        /// The attribute it was on.
        attribute: &'static str,
        /// What was found, so the error names the offending text.
        value: String,
    },
    /// A `<Class name>` that is not one of the four speed classes.
    UnknownClass {
        /// What was found.
        name: String,
    },
    /// Two `<Class>` blocks claim the same speed class.
    DuplicateClass {
        /// The class named twice.
        class: SpeedClass,
    },
    /// One of the four speed classes has no `<Class>` block.
    MissingClass {
        /// The class with no block.
        class: SpeedClass,
    },
    /// Two `<GlobalClass>` blocks claim the same speed class.
    DuplicateGlobalClass {
        /// The class named twice.
        class: SpeedClass,
    },
    /// One of the four speed classes has no `<GlobalClass>` block.
    MissingGlobalClass {
        /// The class with no block.
        class: SpeedClass,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expand(e) => write!(f, "expanding shortened XML: {e}"),
            Self::MissingElement { element } => write!(f, "no <{element}> element"),
            Self::MissingAttribute { element, attribute } => {
                write!(f, "<{element}> has no {attribute} attribute")
            }
            Self::NotANumber {
                element,
                attribute,
                value,
            } => write!(
                f,
                "<{element}> {attribute}=\"{value}\" is not a finite number"
            ),
            Self::UnknownClass { name } => write!(f, "unknown speed class \"{name}\""),
            Self::DuplicateClass { class } => write!(f, "two <Class> blocks named {class}"),
            Self::MissingClass { class } => write!(f, "no <Class> block for {class}"),
            Self::DuplicateGlobalClass { class } => {
                write!(f, "two <GlobalClass> blocks named {class}")
            }
            Self::MissingGlobalClass { class } => write!(f, "no <GlobalClass> block for {class}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Expand(e) => Some(e),
            _ => None,
        }
    }
}

impl From<fexml::Error> for Error {
    fn from(value: fexml::Error) -> Self {
        Self::Expand(value)
    }
}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

mod cameras;
mod names;

pub use cameras::{AirbrakeGraphics, BonnetCamera, Camera, ExternalCamera};
pub use names::{SHIP_DIR, entry_name, entry_name_in};

/// Hull dimensions and the shield pool. `<Misc/>`.
///
/// Per ship rather than per speed class, so all four `<Class>` blocks share it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Misc {
    /// Hull height, and so the box half-extent used for contact generation.
    pub height: f32,
    /// Hull length. Also sets where the two hover probes sit.
    pub length: f32,
    /// Shield pool. **A bulk default for all three difficulty slots**, not a
    /// fourth value beside them - see [`Misc::shield_for`].
    pub shield: f32,
    /// Shield pool on the easy skill level, overriding [`Self::shield`] for that
    /// slot alone.
    pub easyshield: Option<f32>,
    /// Shield pool on the medium skill level. **No shipped file authors this**;
    /// the PS2 parser accepts it, so the decoder does too.
    pub mediumshield: Option<f32>,
    /// Shield pool on the hard skill level. Also authored nowhere.
    pub hardshield: Option<f32>,
    /// Hull width.
    pub width: f32,
    /// Fore/aft mass bias.
    pub weight_distribution: Option<f32>,
}

impl Misc {
    const ELEMENT: &'static str = "Misc";

    fn from_node(node: &Node) -> Result<Self> {
        let e = Self::ELEMENT;
        Ok(Self {
            height: number(node, e, "height")?,
            length: number(node, e, "length")?,
            shield: number(node, e, "shield")?,
            easyshield: optional_number(node, e, "easyshield")?,
            mediumshield: optional_number(node, e, "mediumshield")?,
            hardshield: optional_number(node, e, "hardshield")?,
            width: number(node, e, "width")?,
            weight_distribution: optional_number(node, e, "weight_distribution")?,
        })
    }

    /// The pool for a skill level, `0` easy through `2` hard.
    ///
    /// **Three slots, not two.** `HandlingXml_ParseMisc` (`0x0014db08` in the PS2
    /// `SCES_547.48`) writes `easyshield`/`mediumshield`/`hardshield` to
    /// `0x84`/`0x88`/`0x8c` on the stats base and plain `shield` to all three at
    /// once, and the PSP's `Ship_SetShield` indexes exactly that range as
    /// `0x84 + skill * 4`. A writer on one binary and a reader on the other,
    /// which is what puts this at confidence 88 -
    /// `docs/ghidra/functions/psp-pulse-usa/shield.md`.
    ///
    /// Out-of-range skill levels clamp to hard rather than panicking: the index
    /// comes from a race option, and the original reads three words whatever is
    /// in it.
    #[must_use]
    pub fn shield_for(&self, skill: u8) -> f32 {
        match skill {
            0 => self.easyshield.unwrap_or(self.shield),
            1 => self.mediumshield.unwrap_or(self.shield),
            _ => self.hardshield.unwrap_or(self.shield),
        }
    }
}

/// Zone mode's speed law and its reward. `<Zone start increment recharge/>`.
///
/// Lives under `<Handling><Global>`, beside `<Special>`, `<GlobalClass>` and
/// `<StartBoost>` - a sibling of `<Stats>`, not a child of it. `Handling_ParseStats`
/// (`0x0883a2f0`) dispatches the `<Global>` subtree to `Xml_ReadGlobalSettings`
/// (`0x0883a970`), which is where these three attributes are read. Confidence
/// **84**.
///
/// The two speed values land in `g_autospeed_base` (`0x08b36be0`) and
/// `g_autospeed_step` (`0x08b36be4`), which `Ship_UpdateEngine`'s four-corner
/// branch reads as `base + step * (float)(uint32)zone`. `recharge` goes to
/// `0x08b34360` and is added to the shield when a zone is completed without
/// touching anything.
///
/// **Global despite living in a per-team file.** All eight teams carry a copy;
/// which one the game reads is whichever ship it happened to load. Nothing here
/// makes the copies agree, and if they ever disagree that is a fact about the
/// disc worth surfacing rather than averaging away.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Zone {
    /// The target speed at zone zero.
    pub start: f32,
    /// Added to that target for every zone survived.
    pub increment: f32,
    /// Shield restored for a zone completed with no contact.
    pub recharge: f32,
}

impl Zone {
    const ELEMENT: &'static str = "Zone";

    fn from_node(node: &Node) -> Result<Self> {
        let e = Self::ELEMENT;
        Ok(Self {
            start: number(node, e, "start")?,
            increment: number(node, e, "increment")?,
            recharge: number(node, e, "recharge")?,
        })
    }
}

/// The four bars on the ship-select screen. `<FE speed thrust handling shield/>`.
///
/// **Presentation only.** They need not agree with the physics, and nothing in
/// the simulation may read them: a ship whose bar says it is fast is not
/// thereby fast.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Fe {
    /// The speed bar.
    pub speed: f32,
    /// The thrust bar.
    pub thrust: f32,
    /// The handling bar.
    pub handling: f32,
    /// The shield bar.
    pub shield: f32,
}

impl Fe {
    const ELEMENT: &'static str = "FE";

    fn from_node(node: &Node) -> Result<Self> {
        let e = Self::ELEMENT;
        Ok(Self {
            speed: number(node, e, "speed")?,
            thrust: number(node, e, "thrust")?,
            handling: number(node, e, "handling")?,
            shield: number(node, e, "shield")?,
        })
    }
}

/// Engine response. `<Engine accelcap amount falloff gain turbo/>`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Engine {
    /// Ceiling on accumulated acceleration.
    pub accelcap: f32,
    /// Thrust magnitude.
    pub amount: f32,
    /// Per-second decay of the thrust ramp.
    pub falloff: f32,
    /// Per-second rise of the thrust ramp.
    pub gain: f32,
    /// Turbo multiplier.
    pub turbo: f32,
}

impl Engine {
    const ELEMENT: &'static str = "Engine";

    fn from_node(node: &Node) -> Result<Self> {
        let e = Self::ELEMENT;
        Ok(Self {
            accelcap: number(node, e, "accelcap")?,
            amount: number(node, e, "amount")?,
            falloff: number(node, e, "falloff")?,
            gain: number(node, e, "gain")?,
            turbo: number(node, e, "turbo")?,
        })
    }
}

/// Braking response. `<Brakes amount falloff gain/>`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Brakes {
    /// Brake magnitude.
    pub amount: f32,
    /// Per-second decay of the brake ramp.
    pub falloff: f32,
    /// Per-second rise of the brake ramp.
    pub gain: f32,
}

impl Brakes {
    const ELEMENT: &'static str = "Brakes";

    fn from_node(node: &Node) -> Result<Self> {
        let e = Self::ELEMENT;
        Ok(Self {
            amount: number(node, e, "amount")?,
            falloff: number(node, e, "falloff")?,
            gain: number(node, e, "gain")?,
        })
    }
}

/// Steering response. `<Turning amount falloff gain/>`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Turning {
    /// Yaw authority.
    pub amount: f32,
    /// Per-second decay of the steering ramp.
    pub falloff: f32,
    /// Per-second rise of the steering ramp.
    pub gain: f32,
}

impl Turning {
    const ELEMENT: &'static str = "Turning";

    fn from_node(node: &Node) -> Result<Self> {
        let e = Self::ELEMENT;
        Ok(Self {
            amount: number(node, e, "amount")?,
            falloff: number(node, e, "falloff")?,
            gain: number(node, e, "gain")?,
        })
    }
}

/// Airbrake response. `<Airbrake amount drag falloff gain turn slidegrip sideshift/>`.
///
/// All seven names were recovered twice independently, from this XML and from the
/// parameter block at `+0xd8` in the PSP executable, which is what retired the
/// three the static reading had left unresolved. Two sources agreeing on a set of
/// names is much stronger than either alone.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Airbrake {
    /// A lateral force gain, **not** a drag, despite the name.
    pub amount: f32,
    /// Feeds the forward slide term.
    pub drag: f32,
    /// Per-second decay toward the analog input.
    pub falloff: f32,
    /// Per-second rise toward the analog input.
    pub gain: f32,
    /// Feeds body-local angular acceleration directly.
    pub turn: f32,
    /// Percent of lateral grip retained at full airbrake.
    pub slidegrip: f32,
    /// One-shot lateral impulse.
    pub sideshift: Option<f32>,
}

impl Airbrake {
    const ELEMENT: &'static str = "Airbrake";

    fn from_node(node: &Node) -> Result<Self> {
        let e = Self::ELEMENT;
        Ok(Self {
            amount: number(node, e, "amount")?,
            drag: number(node, e, "drag")?,
            falloff: number(node, e, "falloff")?,
            gain: number(node, e, "gain")?,
            turn: number(node, e, "turn")?,
            slidegrip: number(node, e, "slidegrip")?,
            sideshift: optional_number(node, e, "sideshift")?,
        })
    }
}

/// Antigravity and suspension.
/// `<Antigrav grip_air grip_ground landing_rebound rebound rebound_jump_time ride_height/>`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Antigrav {
    /// Lateral grip coefficient while airborne.
    pub grip_air: f32,
    /// Lateral grip coefficient while grounded.
    pub grip_ground: f32,
    /// Replaces [`Self::rebound`] for a short window after touchdown.
    pub landing_rebound: f32,
    /// Scales suspension damping only, never the spring.
    pub rebound: f32,
    /// Unused by the recovered force law; carried because the XML has it.
    pub rebound_jump_time: f32,
    /// The raycast length for the hover probes.
    ///
    /// **Not a target height.** It never appears in the force law; the height the
    /// ship settles at is emergent.
    pub ride_height: f32,
}

impl Antigrav {
    const ELEMENT: &'static str = "Antigrav";

    fn from_node(node: &Node) -> Result<Self> {
        let e = Self::ELEMENT;
        Ok(Self {
            grip_air: number(node, e, "grip_air")?,
            grip_ground: number(node, e, "grip_ground")?,
            landing_rebound: number(node, e, "landing_rebound")?,
            rebound: number(node, e, "rebound")?,
            rebound_jump_time: number(node, e, "rebound_jump_time")?,
            ride_height: number(node, e, "ride_height")?,
        })
    }
}

/// Mass and the three gravity values.
/// `<Physical flight_gravity mass normal_gravity track_gravity/>`.
///
/// These four names were also recovered twice: the block at `+0xf8` in the PSP
/// executable holds `normal_gravity`, `flight_gravity` and `track_gravity` in
/// that order.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Physical {
    /// Gravity applied while airborne.
    pub flight_gravity: f32,
    /// Ship mass.
    pub mass: f32,
    /// Gravity along world down.
    pub normal_gravity: f32,
    /// Gravity along the track's own up axis, which is what makes inversions
    /// work.
    pub track_gravity: f32,
}

impl Physical {
    const ELEMENT: &'static str = "Physical";

    fn from_node(node: &Node) -> Result<Self> {
        let e = Self::ELEMENT;
        Ok(Self {
            flight_gravity: number(node, e, "flight_gravity")?,
            mass: number(node, e, "mass")?,
            normal_gravity: number(node, e, "normal_gravity")?,
            track_gravity: number(node, e, "track_gravity")?,
        })
    }
}

/// Pitch response. `<pitch pitch_air pitch_ground pitch_damping antigrav_height_adjust/>`.
///
/// The element name is lower case on PSP, where every sibling is capitalised -
/// and capitalised on PS2, which is the only element the two releases spell
/// differently. Element lookup is case-insensitive, so that costs nothing, but it
/// is worth knowing before searching an expanded file for one spelling.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Pitch {
    /// Pitch authority while airborne.
    pub pitch_air: f32,
    /// Pitch authority while grounded.
    pub pitch_ground: f32,
    /// Pitch rate damping.
    pub pitch_damping: f32,
    /// Offset applied to the hover target height.
    pub antigrav_height_adjust: f32,
}

impl Pitch {
    const ELEMENT: &'static str = "pitch";

    fn from_node(node: &Node) -> Result<Self> {
        let e = Self::ELEMENT;
        Ok(Self {
            pitch_air: number(node, e, "pitch_air")?,
            pitch_ground: number(node, e, "pitch_ground")?,
            pitch_damping: number(node, e, "pitch_damping")?,
            antigrav_height_adjust: number(node, e, "antigrav_height_adjust")?,
        })
    }
}

/// One speed class's worth of tunables: a whole `<Class>` block.
///
/// No longer `Copy`, because [`Self::raw_name`] has to carry a rung this
/// project has no enum variant for: Pure ships a fifth class below Pulse's
/// slowest, and dropping its name to keep the type `Copy` would throw away the
/// one fact about it worth recording.
#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    /// Which of Pulse's four this is, or `None` for a rung outside that ladder.
    pub name: Option<SpeedClass>,
    /// The `name` attribute exactly as written, which is the only handle on a
    /// rung [`SpeedClass`] cannot name.
    pub raw_name: String,
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
    /// `<pitch/>`, when the file authors one.
    ///
    /// **`None` on Pure, which omits the element from every `<Class>` block on
    /// both pressings** - enumerated with the rest of that title's schema delta
    /// by `crates/pure/tests/handling_schema_ground_truth.rs`. The same shape as
    /// [`Stats::fe`] and for the same reason: a block a file does not author is
    /// a fact about that file, and `Option` is how this parser states it rather
    /// than handing back a zeroed struct that reads like authored data.
    ///
    /// Unlike `fe`, this one is **not** presentation - it feeds
    /// `oag_physics::Pitch`, so somebody has to decide what a ship with no
    /// authored pitch response does. That decision is deliberately not made
    /// here: `oag_gameplay::handling::handling_for` substitutes a stand-in and
    /// says which, because picking a number is a claim about a *title* and this
    /// crate must not make one. See [ADR-0022].
    ///
    /// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
    pub pitch: Option<Pitch>,
}

impl Class {
    const ELEMENT: &'static str = "Class";

    fn from_node(name: SpeedClass, node: &Node) -> Result<Self> {
        Self::parse(Some(name), name.as_str(), node)
    }

    /// A `<Class>` block whose `name` is outside [`SpeedClass`].
    fn from_node_named(raw_name: &str, node: &Node) -> Result<Self> {
        Self::parse(None, raw_name, node)
    }

    fn parse(name: Option<SpeedClass>, raw_name: &str, node: &Node) -> Result<Self> {
        Ok(Self {
            name,
            raw_name: raw_name.to_string(),
            engine: Engine::from_node(child(node, Engine::ELEMENT)?)?,
            brakes: Brakes::from_node(child(node, Brakes::ELEMENT)?)?,
            turning: Turning::from_node(child(node, Turning::ELEMENT)?)?,
            airbrake: Airbrake::from_node(child(node, Airbrake::ELEMENT)?)?,
            antigrav: Antigrav::from_node(child(node, Antigrav::ELEMENT)?)?,
            physical: Physical::from_node(child(node, Physical::ELEMENT)?)?,
            // Absent from every Pure `<Class>`. A *malformed* one still fails:
            // `transpose` propagates a parse error and only a missing element
            // becomes `None`, so a typo'd attribute cannot arrive here disguised
            // as an older schema.
            pitch: node
                .children_named(Pitch::ELEMENT)
                .next()
                .map(Pitch::from_node)
                .transpose()?,
        })
    }
}

/// One team's whole `handlingstats.xml`.
#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    /// The `<Stats team>` attribute, as written.
    pub team: String,
    /// `<InternalCamera/>`: the cockpit view.
    pub internal_camera: Camera,
    /// `<BackwardCamera/>`: the look-behind view.
    pub backward_camera: Camera,
    /// `<BonnetCamera/>`.
    pub bonnet_camera: BonnetCamera,
    /// `<ExternalCameraFar/>`.
    pub external_camera_far: ExternalCamera,
    /// `<ExternalCameraClose/>`.
    pub external_camera_close: ExternalCamera,
    /// `<AirbrakeGraphics/>`.
    pub airbrake_graphics: AirbrakeGraphics,
    /// `<Misc/>`, shared by all four classes.
    pub misc: Misc,
    /// `<FE/>`: the ship-select bars, when the file has them.
    ///
    /// `None` on Pure, which omits the element entirely. Presentation only, so
    /// nothing in the simulation notices its absence.
    ///
    /// It was the fourth schema difference found, and the first one no survey
    /// had predicted: it turned up by pointing the parser at a real Pure file,
    /// which is the argument for `oag-pure` existing at all. Pure's whole
    /// element set has since been enumerated in one pass against both discs -
    /// see `docs/formats/pure-status.md` and
    /// `crates/pure/tests/handling_schema_ground_truth.rs` - so this is now a
    /// listed difference rather than a surprise.
    pub fe: Option<Fe>,
    /// The `<Class>` blocks, in ladder order, slowest first.
    ///
    /// Look one up with [`Self::class`], which matches on the rung's own name;
    /// indexing by `SpeedClass as usize` would find the wrong block in a file
    /// whose ladder is not Pulse's.
    ///
    /// A `Vec` rather than a fixed array, and reluctantly: "exactly four, one
    /// each" was a property of the type until Pure turned out to ship **five**
    /// speed classes, one below Pulse's slowest (`docs/formats/pure-status.md`).
    /// The ordering guarantee survives - it is built in ladder order and never
    /// from a hasher - but the count is now the file's business rather than the
    /// type's, so [`Self::class`] returns an `Option`.
    ///
    /// Every Pulse file still yields exactly four, which
    /// [`Self::has_pulse_class_ladder`] states as a checkable claim rather than
    /// an assumption.
    pub classes: Vec<Class>,
}

impl Stats {
    /// The block for one of Pulse's four speed classes, if this file has it.
    ///
    /// `None` where the file's ladder does not reach that far, which is not a
    /// defect: a schema with a different ladder is a different generation of the
    /// format, not a broken file.
    #[must_use]
    pub fn class(&self, class: SpeedClass) -> Option<&Class> {
        self.classes.iter().find(|block| block.name == Some(class))
    }

    /// The block for a rung named the way the document spells it, matched
    /// case-insensitively.
    ///
    /// The lookup for a ladder that is not Pulse's. [`Self::class`] can only ask
    /// for a rung [`SpeedClass`] has a variant for, and Pure authors one it does
    /// not: `VECTOR`, below `VENOM`, in every race team's file. That block is
    /// parsed and kept - see [`Class::raw_name`] - and this is how it is
    /// reached.
    ///
    /// It answers for Pulse's four identically to [`Self::class`], because
    /// [`Class::from_node`] writes the canonical [`SpeedClass::as_str`] spelling
    /// into `raw_name` rather than the document's literal one. So a caller
    /// holding nothing but a name never has to know whether the rung is one of
    /// the four.
    ///
    /// `None` means **this file does not author that rung**. It is not an
    /// invitation to fall back on a neighbouring one.
    #[must_use]
    pub fn class_named(&self, name: &str) -> Option<&Class> {
        self.classes
            .iter()
            .find(|block| block.raw_name.eq_ignore_ascii_case(name))
    }

    /// The rungs this file authors, in ladder order, as a readable list.
    ///
    /// For the one thing a caller does when [`Self::class_named`] returns
    /// `None`: say what the file *does* carry. A message naming only the rung
    /// that was missing sends the reader to the wrong file.
    #[must_use]
    pub fn ladder(&self) -> String {
        self.classes
            .iter()
            .map(|block| block.raw_name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Whether this file carries exactly Pulse's four-class ladder.
    ///
    /// Every shipped Pulse `handlingstats.xml` does. Pure's do not - they carry
    /// five, the extra one below `Venom`.
    #[must_use]
    pub fn has_pulse_class_ladder(&self) -> bool {
        self.classes.len() == SpeedClass::ALL.len()
    }
}

/// Reads an **already expanded** `handlingstats.xml`.
///
/// Use [`from_blob`] for bytes straight out of an archive; those are shortened
/// and this would not find a single element.
pub fn parse(expanded: &str) -> Result<Stats> {
    let root = fexml::parse(expanded);
    let handling = descendant(&root, "Handling").ok_or(Error::MissingElement {
        element: "Handling",
    })?;
    let stats = child(handling, "Stats")?;

    Ok(Stats {
        team: stats
            .value("team")
            .ok_or(Error::MissingAttribute {
                element: "Stats",
                attribute: "team",
            })?
            .trim()
            .to_string(),
        internal_camera: Camera::from_node(child(stats, "InternalCamera")?, "InternalCamera")?,
        backward_camera: Camera::from_node(child(stats, "BackwardCamera")?, "BackwardCamera")?,
        bonnet_camera: BonnetCamera::from_node(child(stats, BonnetCamera::ELEMENT)?)?,
        external_camera_far: ExternalCamera::from_node(
            child(stats, "ExternalCameraFar")?,
            "ExternalCameraFar",
        )?,
        external_camera_close: ExternalCamera::from_node(
            child(stats, "ExternalCameraClose")?,
            "ExternalCameraClose",
        )?,
        airbrake_graphics: AirbrakeGraphics::from_node(child(stats, AirbrakeGraphics::ELEMENT)?)?,
        misc: Misc::from_node(child(stats, Misc::ELEMENT)?)?,
        fe: match stats.children_named(Fe::ELEMENT).next() {
            Some(node) => Some(Fe::from_node(node)?),
            None => None,
        },
        classes: classes(stats)?,
    })
}

/// Reads an archive blob, expanding it first if it needs it.
///
/// The two shipped releases store this file differently and both are handled
/// here, because which one a caller is holding is a property of the disc rather
/// than of the caller:
///
/// - **PSP** stores it as [shortened XML](crate::fexml) with a per-file `<code>`
///   dictionary, which has to be expanded before a single element name is
///   recognisable.
/// - **PS2** stores it as plain text beginning `<?xml`, and expanding that would
///   fail for want of a dictionary.
///
/// The leading bytes say which, per `docs/formats/fexml.md`, so the dispatch is
/// [`fexml::is_fexml`] rather than a flag the caller has to pass and can get
/// wrong.
pub fn from_blob(data: &[u8]) -> Result<Stats> {
    if fexml::is_fexml(data) {
        parse(&fexml::expand(data)?)
    } else {
        parse(std::str::from_utf8(data).map_err(|_| fexml::Error::NotText)?)
    }
}

/// Collects the four `<Class>` blocks into an array indexed by [`SpeedClass`].
///
/// One pass over the blocks the document actually has, which is what lets an
/// extra block, a duplicate and an unknown name each be caught. Looking the four
/// names up instead would silently ignore all three.
fn classes(stats: &Node) -> Result<Vec<Class>> {
    let mut found: [Option<Class>; 4] = [None, None, None, None];
    // Blocks whose `name` is not one of Pulse's four. Pure's fifth class lives
    // here rather than being rejected: an unrecognised rung is a different
    // ladder, not a corrupt file, and `UnknownClass` was previously the reason
    // this parser refused Pure's `handlingstats.xml` outright.
    let mut extra: Vec<Class> = Vec::new();

    for node in stats.children_named(Class::ELEMENT) {
        let name = node.value("name").ok_or(Error::MissingAttribute {
            element: Class::ELEMENT,
            attribute: "name",
        })?;
        let name = name.trim();
        let Some(class) = SpeedClass::from_name(name) else {
            extra.push(Class::from_node_named(name, node)?);
            continue;
        };

        let slot = &mut found[class as usize];
        if slot.is_some() {
            return Err(Error::DuplicateClass { class });
        }
        *slot = Some(Class::from_node(class, node)?);
    }

    // A file that names *some* of Pulse's ladder must name all of it; a file
    // that names none of it is another generation's ladder and is kept whole.
    let named = found.iter().filter(|slot| slot.is_some()).count();
    if named == 0 {
        return Ok(extra);
    }
    // Reported in `ALL` order so the message does not depend on document order.
    for class in SpeedClass::ALL {
        if found[class as usize].is_none() {
            return Err(Error::MissingClass { class });
        }
    }

    let mut out: Vec<Class> = found
        .into_iter()
        .map(|class| class.expect("every slot filled above"))
        .collect();
    // The extra rungs sit after the recognised ladder, so `SpeedClass`'s
    // discriminant stays a valid index into the front of the vector.
    out.extend(extra);
    Ok(out)
}

/// The first child element with this name, or a typed error.
fn child<'a>(parent: &'a Node, element: &'static str) -> Result<&'a Node> {
    parent
        .children_named(element)
        .next()
        .ok_or(Error::MissingElement { element })
}

/// The first element with this name anywhere in the tree.
///
/// Needed because [`fexml::parse`] returns a synthetic `#document` root and the
/// real files have been seen with and without a wrapping element around
/// `<Handling>`.
fn descendant<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    if node.name.eq_ignore_ascii_case(name) {
        return Some(node);
    }
    node.children.iter().find_map(|c| descendant(c, name))
}

/// Reads one attribute as a finite `f32`.
///
/// `f32`'s parser accepts `"nan"` and `"inf"`, both of which would propagate
/// silently through the whole simulation, so they are rejected here rather than
/// found later in a state hash that will not reproduce.
/// An attribute a *later* schema added, so its absence is a fact about the
/// file's generation rather than a defect in it.
///
/// `Ok(None)` only for an attribute that is not there at all. A present but
/// unparseable value is still an error: "Pure does not have this field" and
/// "this number is broken" must not collapse into one answer.
fn optional_number(
    node: &Node,
    element: &'static str,
    attribute: &'static str,
) -> Result<Option<f32>> {
    if node.value(attribute).is_none() {
        return Ok(None);
    }
    number(node, element, attribute).map(Some)
}

fn number(node: &Node, element: &'static str, attribute: &'static str) -> Result<f32> {
    let raw = node
        .value(attribute)
        .ok_or(Error::MissingAttribute { element, attribute })?;

    let bad = || Error::NotANumber {
        element,
        attribute,
        value: raw.to_string(),
    };
    let parsed: f32 = raw.trim().parse().map_err(|_| bad())?;
    if parsed.is_finite() {
        Ok(parsed)
    } else {
        Err(bad())
    }
}

#[cfg(test)]
mod tests;
