//! `handlingstats.xml`: every ship-handling tunable the game reads at runtime.
//!
//! Ship handling is **data, not code**: one file per team at
//! `Data\Ships\<Team>\handlingstats.xml` in `Data.wad` holds the whole
//! parameter set (five camera rigs, airbrake animation, hull dimensions, the
//! four front-end bars, then one `<Class>` block per speed class). The physics
//! code defines the *shape* of the model; these files supply every constant.
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
//! # A second file with the same root element
//!
//! `Handling_ParseStats` (`0x0883a2f0`) has **two** callers:
//!
//! - `0x088c291c` builds `%s\handlingstats.xml` per team, the schema above.
//! - `0x0894f6a8` passes [`GLOBAL_ENTRY`], `Data\XML\HandlingStats.xml`, once at
//!   system-root creation.
//!
//! The parser walks `<Handling>` for `<Stats>` *and* `<Global>` (handed to
//! `Xml_ReadGlobalSettings`, `0x0883a970`), which carries engine-wide statics:
//! Zone mode's speed law, speed-pad and weapon-pad tunables, per-class gravity,
//! the start boost, three camera pitch modifiers.
//!
//! **Measured**: `which_top_level_elements_handlingstats_carries` reads all
//! sixteen per-team files (eight teams on each of PSP and PS2) and every one
//! holds `<Stats>` only. So [`parse`] does not look for `<Global>`;
//! [`global_from_blob`] does. Only `<Zone>`, and `<SpeedupPads>` and
//! `<GravityMul>` under `<GlobalClass>`, are decoded; the rest is named on
//! [`Global`] and `docs/ghidra/functions/psp-pulse-usa/engine.md`.
//!
//! The files are [shortened XML](crate::fexml); [`from_blob`] expands and parses.
//!
//! # Nothing here defaults
//!
//! Every element and attribute the schema lists is **required**, and a missing
//! one is an [`Error`], not a zero: a `mass` quietly arriving as `0.0` produces
//! a ship that behaves oddly and reads as a tuning bug. `"nan"` and `"inf"`,
//! which `f32`'s parser accepts, are rejected too.
//!
//! The `<Class>` blocks go into a fixed-length array indexed by [`SpeedClass`]:
//! a fifth block, a duplicate, an unknown name and a missing one are each a
//! distinct error, and no map iteration order can reach the simulation.
//!
//! # No values live here
//!
//! Per `docs/architecture/adr/0006-no-copyrighted-content.md` this module
//! describes the schema only; the numbers are read from the player's own disc,
//! and test fixtures use invented values. Pulse's eight-team roster moved to
//! `oag_pulse::race::TEAMS` (2026-09-01) as an ADR-0022 title fact.
//!
//! Evidence and confidence: `docs/formats/handling-stats.md`; the subset the
//! force law consumes: `oag_physics::Handling`.

use std::fmt;

use crate::fexml::{self, Node};

mod global;

pub use global::{
    ForeignGlobalClass, GLOBAL_ENTRY, Global, GravityMul, Special, SpeedupPads, StartBoost,
    WeaponPad, global_from_blob, parse_global,
};

/// The four speed classes, in the order the XML lists them.
///
/// Separate from `oag_physics::SpeedClass`: this describes the document, that
/// the simulation's input, and `oag-physics` depends on `oag-core` alone.
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
    /// All four, slowest first; the position is the discriminant, so
    /// [`Stats::classes`] is indexable by class.
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
        element: &'static str,
    },
    /// A required attribute is absent. Never defaulted: see the module docs.
    MissingAttribute {
        element: &'static str,
        attribute: &'static str,
    },
    /// An attribute is present but is not a finite number.
    /// An attribute is present but not a finite number (unparseable, or `NaN`/
    /// infinity, which the simulation must not see).
    NotANumber {
        element: &'static str,
        attribute: &'static str,
        value: String,
    },
    /// A `<Class name>` that is not one of the four speed classes.
    UnknownClass {
        name: String,
    },
    /// Two `<Class>` blocks claim the same speed class.
    DuplicateClass {
        class: SpeedClass,
    },
    /// One of the four speed classes has no `<Class>` block.
    MissingClass {
        class: SpeedClass,
    },
    /// Two `<GlobalClass>` blocks claim the same speed class.
    DuplicateGlobalClass {
        class: SpeedClass,
    },
    /// One of the four speed classes has no `<GlobalClass>` block.
    MissingGlobalClass {
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

pub type Result<T> = std::result::Result<T, Error>;

mod cameras;
mod names;

pub use cameras::{AirbrakeGraphics, BonnetCamera, Camera, ExternalCamera};
pub use names::{SHIP_DIR, entry_name, entry_name_in};

/// Hull dimensions and the shield pool. `<Misc/>`. Per ship, shared by all four
/// `<Class>` blocks.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Misc {
    /// Hull height, and so the box half-extent used for contact generation.
    pub height: f32,
    /// Hull length. Also sets where the two hover probes sit.
    pub length: f32,
    /// Shield pool. **A bulk default for all three difficulty slots**; see
    /// [`Misc::shield_for`].
    pub shield: f32,
    /// Shield pool on easy, overriding [`Self::shield`] for that slot.
    pub easyshield: Option<f32>,
    /// Shield pool on medium. **No shipped file authors it**; the PS2 parser
    /// accepts it, so this does.
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
    /// **Three slots, not two.** `HandlingXml_ParseMisc` (`0x0014db08` in PS2
    /// `SCES_547.48`) writes the three to `0x84`/`0x88`/`0x8c` and plain `shield`
    /// to all three; PSP `Ship_SetShield` indexes that range as `0x84 + skill *
    /// 4`. Writer on one binary, reader on the other: confidence 88,
    /// `docs/ghidra/functions/psp-pulse-usa/shield.md`.
    ///
    /// Out-of-range skill clamps to hard: the index comes from a race option and
    /// the original reads three words whatever is in it.
    #[must_use]
    pub fn shield_for(&self, skill: u8) -> f32 {
        match skill {
            0 => self.easyshield.unwrap_or(self.shield),
            1 => self.mediumshield.unwrap_or(self.shield),
            _ => self.hardshield.unwrap_or(self.shield),
        }
    }
}

/// Zone mode's speed law and reward. `<Zone start increment recharge/>`.
///
/// Under `<Handling><Global>`, a sibling of `<Stats>`. `Handling_ParseStats`
/// (`0x0883a2f0`) dispatches `<Global>` to `Xml_ReadGlobalSettings`
/// (`0x0883a970`), where these are read. Confidence **84**.
///
/// The speeds land in `g_autospeed_base` (`0x08b36be0`) and `g_autospeed_step`
/// (`0x08b36be4`), which `Ship_UpdateEngine`'s four-corner branch reads as `base
/// + step * (float)(uint32)zone`. `recharge` goes to `0x08b34360` and is added
/// to the shield when a zone is completed without contact.
///
/// **Global despite living in a per-team file.** All eight teams carry a copy
/// and nothing makes them agree; a disagreement is a fact worth surfacing, not
/// averaging.
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
/// **Presentation only**: nothing in the simulation may read them.
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
/// All seven names were recovered twice independently, from this XML and from
/// the parameter block at `+0xd8` in the PSP executable.
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
    /// The raycast length for the hover probes. **Not a target height**: it
    /// never appears in the force law; the settled height is emergent.
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
/// Names also recovered twice: the PSP block at `+0xf8` holds `normal_gravity`,
/// `flight_gravity`, `track_gravity` in that order.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Physical {
    /// Gravity applied while airborne.
    pub flight_gravity: f32,
    /// Ship mass.
    pub mass: f32,
    /// Gravity along world down.
    pub normal_gravity: f32,
    /// Gravity along the track's own up axis, which makes inversions work.
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
/// The element name is lower case on PSP and capitalised on PS2, the only
/// element the releases spell differently. Lookup is case-insensitive.
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
/// Not `Copy`: [`Self::raw_name`] carries a rung with no enum variant (Pure
/// ships a fifth class below Pulse's slowest).
#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    /// Which of Pulse's four this is, or `None` for a rung outside that ladder.
    pub name: Option<SpeedClass>,
    /// The `name` attribute as written; the only handle on a rung [`SpeedClass`]
    /// cannot name.
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
    /// `<pitch/>`, when the file authors one.
    ///
    /// **`None` on Pure, which omits it from every `<Class>` on both pressings**
    /// (`crates/pure/tests/handling_schema_ground_truth.rs`). A block a file does
    /// not author is a fact about that file; `Option` states it rather than
    /// returning a zeroed struct that reads like authored data.
    ///
    /// Unlike `fe`, this is **not** presentation: it feeds `oag_physics::Pitch`.
    /// What a ship with no pitch does is not decided here:
    /// `oag_gameplay::handling::handling_for` substitutes a stand-in and says
    /// which, because a number is a claim about a *title*. See [ADR-0022].
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
            // `transpose` propagates a parse error, so a typo cannot pass as an
            // older schema.
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
    /// `None` on Pure, which omits the element. Presentation only. Found by
    /// pointing the parser at a real Pure file; the full delta is in
    /// `docs/formats/pure-status.md` and
    /// `crates/pure/tests/handling_schema_ground_truth.rs`.
    pub fe: Option<Fe>,
    /// The `<Class>` blocks, in ladder order, slowest first.
    ///
    /// Look one up with [`Self::class`], which matches on name; indexing by
    /// `SpeedClass as usize` would pick the wrong block in a file whose ladder is
    /// not Pulse's.
    ///
    /// A `Vec` because Pure ships **five** classes, one below Pulse's slowest
    /// (`docs/formats/pure-status.md`). Order is built in ladder order, never
    /// from a hasher; the count is the file's business, so [`Self::class`]
    /// returns an `Option`. Every Pulse file yields four, checkable with
    /// [`Self::has_pulse_class_ladder`].
    pub classes: Vec<Class>,
}

impl Stats {
    /// The block for one of Pulse's four speed classes, if this file has it.
    ///
    /// `None` where the ladder does not reach that far: a different generation
    /// of the format, not a broken file.
    #[must_use]
    pub fn class(&self, class: SpeedClass) -> Option<&Class> {
        self.classes.iter().find(|block| block.name == Some(class))
    }

    /// The block for a rung named as the document spells it, case-insensitively.
    ///
    /// For ladders that are not Pulse's: [`Self::class`] cannot ask for Pure's
    /// `VECTOR` (below `VENOM`, in every race team's file), which is parsed and
    /// kept (see [`Class::raw_name`]). It answers for Pulse's four identically,
    /// since [`Class::from_node`] writes the canonical [`SpeedClass::as_str`]
    /// spelling into `raw_name`.
    ///
    /// `None` means **this file does not author that rung**, not an invitation to
    /// fall back on a neighbour.
    #[must_use]
    pub fn class_named(&self, name: &str) -> Option<&Class> {
        self.classes
            .iter()
            .find(|block| block.raw_name.eq_ignore_ascii_case(name))
    }

    /// The rungs this file authors, in ladder order, as a readable list, for
    /// when [`Self::class_named`] returns `None`: a message naming only the
    /// missing rung sends the reader to the wrong file.
    #[must_use]
    pub fn ladder(&self) -> String {
        self.classes
            .iter()
            .map(|block| block.raw_name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Whether this file carries exactly Pulse's four-class ladder (every Pulse
    /// file does; Pure's carry five).
    #[must_use]
    pub fn has_pulse_class_ladder(&self) -> bool {
        self.classes.len() == SpeedClass::ALL.len()
    }
}

/// Reads an **already expanded** `handlingstats.xml`; use [`from_blob`] for raw
/// archive bytes.
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

/// Reads an archive blob, expanding it first if needed.
///
/// PSP stores the file as [shortened XML](crate::fexml) with a per-file `<code>`
/// dictionary; PS2 stores plain text beginning `<?xml`, which expanding would
/// fail on. [`fexml::is_fexml`] tells them apart from the leading bytes
/// (`docs/formats/fexml.md`), so callers pass no flag.
pub fn from_blob(data: &[u8]) -> Result<Stats> {
    if fexml::is_fexml(data) {
        parse(&fexml::expand(data)?)
    } else {
        parse(std::str::from_utf8(data).map_err(|_| fexml::Error::NotText)?)
    }
}

/// Collects the `<Class>` blocks, one pass over what the document has, so an
/// extra block, a duplicate and an unknown name are each caught.
fn classes(stats: &Node) -> Result<Vec<Class>> {
    let mut found: [Option<Class>; 4] = [None, None, None, None];
    // Blocks whose `name` is not one of Pulse's four. Pure's fifth class lives
    // here: an unrecognised rung is a different ladder, not a corrupt file.
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

    // A file naming *some* of Pulse's ladder must name all of it; one naming
    // none is another generation's ladder and is kept whole.
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
    // Extra rungs follow the recognised ladder so `SpeedClass`'s discriminant
    // stays a valid index into the front.
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
/// [`fexml::parse`] returns a synthetic `#document` root, and real files have
/// been seen with and without a wrapper around `<Handling>`.
fn descendant<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    if node.name.eq_ignore_ascii_case(name) {
        return Some(node);
    }
    node.children.iter().find_map(|c| descendant(c, name))
}

/// An attribute a *later* schema added, so its absence is a fact about the
/// file's generation, not a defect.
///
/// `Ok(None)` only when the attribute is absent; a present unparseable value is
/// still an error.
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

/// Reads one attribute as a finite `f32`. `"nan"` and `"inf"` parse but are
/// rejected, or they would surface later as a state hash that will not reproduce.
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
