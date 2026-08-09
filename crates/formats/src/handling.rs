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
//! for the same reason.
//!
//! See `docs/formats/handling-stats.md` for the evidence and the confidence
//! scores, and `oag_physics::Handling` for the subset the force law consumes.

use std::fmt;

use crate::fexml::{self, Node};

/// The eight playable teams, one `handlingstats.xml` each.
///
/// All eight files were located by hashing candidate names built from this list;
/// none of them appears as a string in the executable, because the loader builds
/// the path from a template. See `docs/formats/fexml.md`.
pub const TEAMS: [&str; 8] = [
    "AG_Systems",
    "Assegai",
    "EGX",
    "Feisar",
    "Goteki",
    "Piranha",
    "Qirex",
    "Triakis",
];

/// The archive entry name for a team's handling stats.
///
/// Assembled the way the loader assembles it, with backslashes, which is what
/// [`wad::hash_name`](crate::wad::hash_name) needs to find the entry.
#[must_use]
pub fn entry_name(team: &str) -> String {
    format!(r"Data\Ships\{team}\handlingstats.xml")
}

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

/// A cockpit camera rig. `<InternalCamera/>` and `<BackwardCamera/>`.
///
/// The two share a shape: the backward camera is the same rig looking the other
/// way, which is why it carries a `headtilt` the bonnet camera does not.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Camera {
    /// Field of view.
    pub fov: f32,
    /// How far the view tilts with the ship's roll.
    pub headtilt: f32,
    /// Eye offset along the ship's up axis.
    pub height: f32,
    /// Eye offset along the ship's forward axis.
    pub length: f32,
    /// Fixed pitch applied to the rig.
    pub pitch: f32,
}

impl Camera {
    fn from_node(node: &Node, element: &'static str) -> Result<Self> {
        Ok(Self {
            fov: number(node, element, "fov")?,
            headtilt: number(node, element, "headtilt")?,
            height: number(node, element, "height")?,
            length: number(node, element, "length")?,
            pitch: number(node, element, "pitch")?,
        })
    }
}

/// The bonnet camera. `<BonnetCamera fov height length pitch/>`.
///
/// Deliberately not a [`Camera`]: it has no `headtilt`, and folding the two
/// together would mean inventing a default for an attribute the document does
/// not have.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BonnetCamera {
    /// Field of view.
    pub fov: f32,
    /// Eye offset along the ship's up axis.
    pub height: f32,
    /// Eye offset along the ship's forward axis.
    pub length: f32,
    /// Fixed pitch applied to the rig.
    pub pitch: f32,
}

impl BonnetCamera {
    const ELEMENT: &'static str = "BonnetCamera";

    fn from_node(node: &Node) -> Result<Self> {
        let e = Self::ELEMENT;
        Ok(Self {
            fov: number(node, e, "fov")?,
            height: number(node, e, "height")?,
            length: number(node, e, "length")?,
            pitch: number(node, e, "pitch")?,
        })
    }
}

/// A chase camera. `<ExternalCameraFar/>` and `<ExternalCameraClose/>`.
///
/// Two spring constants rather than one, split horizontal and vertical, which is
/// what lets the camera lag behind in a corner without bouncing over crests.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ExternalCamera {
    /// Field of view.
    pub fov: f32,
    /// Height of the point the camera aims at, relative to the ship.
    pub lookat_height: f32,
    /// Distance along the ship's forward axis of the point it aims at.
    pub lookat_length: f32,
    /// Height of the camera itself, relative to the ship.
    pub pos_height: f32,
    /// Distance behind the ship.
    pub pos_length: f32,
    /// Spring stiffness in the horizontal plane.
    pub spring_horiz: f32,
    /// Spring stiffness vertically.
    pub spring_vert: f32,
}

impl ExternalCamera {
    fn from_node(node: &Node, element: &'static str) -> Result<Self> {
        Ok(Self {
            fov: number(node, element, "fov")?,
            lookat_height: number(node, element, "lookat_height")?,
            lookat_length: number(node, element, "lookat_length")?,
            pos_height: number(node, element, "pos_height")?,
            pos_length: number(node, element, "pos_length")?,
            spring_horiz: number(node, element, "spring_horiz")?,
            spring_vert: number(node, element, "spring_vert")?,
        })
    }
}

/// How far and how fast the airbrake flaps move. `<AirbrakeGraphics/>`.
///
/// Presentation only: the force law reads [`Airbrake`], not this.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AirbrakeGraphics {
    /// Deflection at full input.
    pub amount: f32,
    /// Rate the flap returns at.
    pub down_speed: f32,
    /// Rate the flap deploys at.
    pub up_speed: f32,
}

impl AirbrakeGraphics {
    const ELEMENT: &'static str = "AirbrakeGraphics";

    fn from_node(node: &Node) -> Result<Self> {
        let e = Self::ELEMENT;
        Ok(Self {
            amount: number(node, e, "amount")?,
            down_speed: number(node, e, "down_speed")?,
            up_speed: number(node, e, "up_speed")?,
        })
    }
}

/// Hull dimensions and the shield pool. `<Misc/>`.
///
/// Per ship rather than per speed class, so all four `<Class>` blocks share it.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Misc {
    /// Hull height, and so the box half-extent used for contact generation.
    pub height: f32,
    /// Hull length. Also sets where the two hover probes sit.
    pub length: f32,
    /// Shield pool.
    pub shield: f32,
    /// Shield pool on the easier difficulties, which is what implies the pool is
    /// difficulty-scaled rather than the incoming damage.
    pub easyshield: Option<f32>,
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
            width: number(node, e, "width")?,
            weight_distribution: optional_number(node, e, "weight_distribution")?,
        })
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

/// The speed-pad boost, per speed class. `<SpeedupPads amount time/>`.
///
/// Lives under `<Handling><Global><GlobalClass name="...">`, beside
/// `<WeaponPad>` and `<GravityMul>`. `Xml_ReadGlobalSettings` (`0x0883a970`)
/// reads both attributes with `Xml_AttributeAsFloat` and stores them **verbatim**
/// into the two per-class tables at `0x08b36bc0` (`amount`) and `0x08b36bd0`
/// (`time`), indexed by `g_handling_parse_class`. Confidence **90**.
///
/// **Neither attribute is pre-scaled**, unlike the four in
/// `oag_gameplay::handling::SCALED_FIELDS`: the store is the parser's return
/// value with nothing in between. The factor of ten in the force law is the force
/// law's own; see [`oag_physics::forces::evaluate`]'s speed-pad term.
///
/// [`oag_physics::forces::evaluate`]: https://docs.rs/oag-physics
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SpeedupPads {
    /// The force magnitude the boost settles at.
    pub amount: f32,
    /// How long the boost lasts, in seconds, from the last tick inside the pad.
    pub time: f32,
}

impl SpeedupPads {
    const ELEMENT: &'static str = "SpeedupPads";

    fn from_node(node: &Node) -> Result<Self> {
        let e = Self::ELEMENT;
        Ok(Self {
            amount: number(node, e, "amount")?,
            time: number(node, e, "time")?,
        })
    }
}

/// The per-speed-class gravity scale. `<GravityMul airborne/>`.
///
/// Sits beside [`SpeedupPads`] under `<GlobalClass>`, and
/// `Xml_ReadGlobalSettings` (`0x0883a970`) stores it verbatim into
/// `g_class_gravity_scale` (`0x08ab0dcc`), indexed by `g_handling_parse_class`.
/// Confidence **90**.
///
/// # The attribute is named `airborne` and it scales the *grounded* term
///
/// This is the one thing about this element worth reading twice, and it was
/// settled instruction by instruction rather than inferred. The table has exactly
/// three references: this write, and two reads in `Ship_UpdateCraft`'s gravity
/// term at `0x08849b40`/`0x08849b48`. That term builds four VFPU pairs and
/// multiplies them together:
///
/// ```text
/// C600 = (class+0xf8, class+0xfc) = (normal_gravity, flight_gravity)
/// C610 = (g_class_gravity_scale[class], 1.0)      ; viim.s S611, 1
/// C620 = (mass, mass)
/// C630 = (craft+0x2b0, 1 - craft+0x2b0)           ; vocp.s S631, S630
/// worldForce.y += -(C600 * C610 * C620 * C630).x + .y
/// ```
///
/// Lane 0 carries `normal_gravity`, is multiplied by `grounded`, **and is the
/// lane the scale lands on**. Lane 1 - the airborne one - is multiplied by a
/// literal `1.0`. So despite the attribute's name, this scales how heavy a craft
/// is **on the ground**, and the air term is unscaled.
///
/// The two class-block offsets are not read off this page's guess either:
/// `HandlingXml_ParsePhysical` (`0x08838f50`) stores `normal_gravity` to `+0xf8`
/// and `flight_gravity` to `+0xfc`, with a `0x80` stride per class that matches
/// the `sll a0, a0, 0x7` at the gravity site.
///
/// Whether "airborne" describes an intent the code does not implement, or a
/// renaming nobody propagated, is **not** answered here. What is established is
/// which term the number reaches.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GravityMul {
    /// The multiplier on `<Physical normal_gravity>`, despite this name.
    ///
    /// See the type's docs: it rides the grounded lane of the gravity term.
    pub airborne: f32,
}

impl GravityMul {
    const ELEMENT: &'static str = "GravityMul";

    fn from_node(node: &Node) -> Result<Self> {
        Ok(Self {
            airborne: number(node, Self::ELEMENT, "airborne")?,
        })
    }
}

/// The one attribute of `<Special>` this project reads.
///
/// The element carries five - `roll_cost roll_speed roll_turbotime speedpad_jump
/// turbo_jump` - and the other four are left alone until something needs them,
/// the way the rest of `<Global>` is. Only `speedpad_jump` has a consumer.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Special {
    /// How far the speed-pad boost tilts toward the hull's up axis while the
    /// pitch-up input is held. `0.1` on both shipped discs, read live.
    ///
    /// `Xml_ReadGlobalSettings` (`0x0883a970`) puts it in `g_speedpad_jump`
    /// (`0x08b36bec`), and `Ship_ApplySpeedupPad` (`0x08848f9c`) adds
    /// `craft+0x160 * g_speedpad_jump` to the pad's own push direction on the
    /// gated branch. It is **not** a jump: the direction is not renormalised, so
    /// `0.1` is a 5.71-degree tilt and 0.5 % more force, and the hover spring
    /// absorbs most of that. See `oag_physics::engine::speedup_pad`.
    ///
    /// Stored **unscaled**, like `<SpeedupPads>` and unlike
    /// `oag_gameplay::handling::SCALED_FIELDS`: the parse writes
    /// `Xml_AttributeAsFloat`'s return straight into the global.
    pub speedpad_jump: f32,
}

impl Special {
    const ELEMENT: &'static str = "Special";

    fn from_node(node: &Node) -> Result<Self> {
        Ok(Self {
            speedpad_jump: number(node, Self::ELEMENT, "speedpad_jump")?,
        })
    }
}

/// The engine-wide `<Global>` block, out of [`GLOBAL_ENTRY`].
///
/// Only the parts this project has a consumer for are decoded. `<WeaponPad>`, the
/// three camera pitch modifiers, `<CameraSideOffset>` and `<StartBoost>` are all
/// read by the original's `Xml_ReadGlobalSettings` and are deliberately left
/// alone here; they are named in
/// `docs/ghidra/functions/psp-pulse-usa/engine.md` and can be added when
/// something needs them. `<WeaponPad>` fills two more per-class tables and has no
/// consumer, because nothing hands out a weapon yet.
///
/// Of `<Special>`'s five attributes only `speedpad_jump` is read - see
/// [`Special`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Global {
    /// `<Zone/>`.
    pub zone: Zone,
    /// `<Special/>`, of which one attribute is decoded.
    pub special: Special,
    /// `<GlobalClass><SpeedupPads/></GlobalClass>`, indexed by [`SpeedClass`].
    pub speedup_pads: [SpeedupPads; 4],
    /// `<GlobalClass><GravityMul/></GlobalClass>`, indexed by [`SpeedClass`].
    pub gravity_mul: [GravityMul; 4],
}

impl Global {
    /// The speed-pad tunables for one class.
    #[must_use]
    pub fn speedup_pads(&self, class: SpeedClass) -> SpeedupPads {
        self.speedup_pads[class as usize]
    }

    /// The gravity scale for one class.
    #[must_use]
    pub fn gravity_mul(&self, class: SpeedClass) -> GravityMul {
        self.gravity_mul[class as usize]
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
/// The element name is lower case in the document where every sibling is
/// capitalised. Element lookup is case-insensitive, so that costs nothing, but it
/// is worth knowing before searching an expanded file for `<Pitch`.
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
    /// `<pitch/>`.
    pub pitch: Pitch,
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
            pitch: Pitch::from_node(child(node, Pitch::ELEMENT)?)?,
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
    /// `<FE/>`: the ship-select bars. Presentation only.
    pub fe: Fe,
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
        fe: Fe::from_node(child(stats, Fe::ELEMENT)?)?,
        classes: classes(stats)?,
    })
}

/// The archive entry holding `<Global>`, engine-wide rather than per team.
///
/// A literal in the executable at `0x08a8a270`, passed to `Handling_ParseStats`
/// by `0x0894f6a8`. Being a literal means the WAD lookup is an exact
/// [`wad::hash_name`](crate::wad::hash_name) hit rather than a mined candidate.
pub const GLOBAL_ENTRY: &str = r"Data\XML\HandlingStats.xml";

/// Reads `<Handling><Global>` out of an **already expanded** document.
///
/// `Ok(None)` when the document parses but carries no `<Global>` at all - which
/// is what all sixteen shipped per-team files do, and is not an error. Once
/// `<Global>` *is* present every part [`Global`] names is required, for the reason
/// the module docs give: a mode configured with three-quarters of its numbers is
/// worse than one configured with none.
pub fn parse_global(expanded: &str) -> Result<Option<Global>> {
    let root = fexml::parse(expanded);
    let handling = descendant(&root, "Handling").ok_or(Error::MissingElement {
        element: "Handling",
    })?;
    let Ok(global) = child(handling, "Global") else {
        return Ok(None);
    };
    let (speedup_pads, gravity_mul) = global_classes(global)?;
    Ok(Some(Global {
        zone: Zone::from_node(child(global, Zone::ELEMENT)?)?,
        special: Special::from_node(child(global, Special::ELEMENT)?)?,
        speedup_pads,
        gravity_mul,
    }))
}

/// Collects `<GlobalClass><SpeedupPads/></GlobalClass>` into an array indexed by
/// [`SpeedClass`].
///
/// The same one-pass shape as [`classes`], and for the same reasons, with **one
/// deliberate difference: an unrecognised `name` is skipped rather than an
/// [`Error::UnknownClass`]**. That is the only place in this module where an
/// unknown name is not an error, so it is worth saying why.
///
/// Both shipped discs author **five** `<GlobalClass>` blocks - `VECTOR` first,
/// then the four speed classes - while every per-team file authors exactly four
/// `<Class>` blocks and no `VECTOR`. Erroring here would fail on real data.
///
/// The original discards it too, and by accident rather than by design.
/// `Xml_ReadGlobalSettings` (`0x0883a970`) matches `name` against a four-entry
/// table and, on no match, leaves `g_handling_parse_class` holding whatever the
/// last match left there - it is a global and is never reset per element. So
/// `VECTOR`'s numbers land in some other class's slot and are then overwritten by
/// the four blocks that follow it, because `VECTOR` is authored first. Skipping it
/// reproduces the outcome without reproducing the accident. Confidence **88**.
///
/// **This holds only while `VECTOR` is first.** Authored last it would corrupt
/// `PHANTOM` in the original and not here, which is a difference worth knowing
/// about rather than one worth emulating.
fn global_classes(global: &Node) -> Result<([SpeedupPads; 4], [GravityMul; 4])> {
    const ELEMENT: &str = "GlobalClass";
    let mut found: [Option<(SpeedupPads, GravityMul)>; 4] = [None; 4];

    for node in global.children_named(ELEMENT) {
        let name = node.value("name").ok_or(Error::MissingAttribute {
            element: "GlobalClass",
            attribute: "name",
        })?;
        let Some(class) = SpeedClass::from_name(name.trim()) else {
            continue;
        };

        let slot = &mut found[class as usize];
        if slot.is_some() {
            return Err(Error::DuplicateGlobalClass { class });
        }
        *slot = Some((
            SpeedupPads::from_node(child(node, SpeedupPads::ELEMENT)?)?,
            GravityMul::from_node(child(node, GravityMul::ELEMENT)?)?,
        ));
    }

    for class in SpeedClass::ALL {
        if found[class as usize].is_none() {
            return Err(Error::MissingGlobalClass { class });
        }
    }

    let found = found.map(|block| block.expect("every slot filled above"));
    Ok((found.map(|(pads, _)| pads), found.map(|(_, mul)| mul)))
}

/// [`parse_global`] over raw archive bytes, expanding them if they need it.
///
/// The same PSP-shortened / PS2-plaintext dispatch [`from_blob`] documents.
pub fn global_from_blob(data: &[u8]) -> Result<Option<Global>> {
    if fexml::is_fexml(data) {
        parse_global(&fexml::expand(data)?)
    } else {
        parse_global(std::str::from_utf8(data).map_err(|_| fexml::Error::NotText)?)
    }
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
mod tests {
    use super::*;

    /// Every number in these fixtures is **invented**: a plain 1, 2, 3 ... in
    /// document order. Per
    /// `docs/architecture/adr/0006-no-copyrighted-content.md` no shipped tuning
    /// value may appear anywhere in this repository, tests included, and
    /// counting from one is about as obviously synthetic as data gets.
    const HEADER: &str = concat!(
        r#"<InternalCamera fov="1" headtilt="2" height="3" length="4" pitch="5"/>"#,
        r#"<BackwardCamera fov="6" headtilt="7" height="8" length="9" pitch="10"/>"#,
        r#"<BonnetCamera fov="11" height="12" length="13" pitch="14"/>"#,
        r#"<ExternalCameraFar fov="15" lookat_height="16" lookat_length="17""#,
        r#" pos_height="18" pos_length="19" spring_horiz="20" spring_vert="21"/>"#,
        r#"<ExternalCameraClose fov="22" lookat_height="23" lookat_length="24""#,
        r#" pos_height="25" pos_length="26" spring_horiz="27" spring_vert="28"/>"#,
        r#"<AirbrakeGraphics amount="29" down_speed="30" up_speed="31"/>"#,
        r#"<Misc height="32" length="33" shield="34" easyshield="35" width="36""#,
        r#" weight_distribution="37"/>"#,
        r#"<FE speed="38" thrust="39" handling="40" shield="41"/>"#,
    );

    /// The seven per-class blocks, again with invented values.
    const CLASS_BODY: &str = concat!(
        r#"<Engine accelcap="1" amount="2" falloff="3" gain="4" turbo="5"/>"#,
        r#"<Brakes amount="6" falloff="7" gain="8"/>"#,
        r#"<Turning amount="9" falloff="10" gain="11"/>"#,
        r#"<Airbrake amount="12" drag="13" falloff="14" gain="15" turn="16""#,
        r#" slidegrip="17" sideshift="18"/>"#,
        r#"<Antigrav grip_air="19" grip_ground="20" landing_rebound="21""#,
        r#" rebound="22" rebound_jump_time="23" ride_height="24"/>"#,
        r#"<Physical flight_gravity="25" mass="26" normal_gravity="27" track_gravity="28"/>"#,
        r#"<pitch pitch_air="29" pitch_ground="30" pitch_damping="31""#,
        r#" antigrav_height_adjust="32"/>"#,
    );

    /// Attributes the fixture carries, which is also the count the schema lists:
    /// 41 in the header plus `team`, and 32 per class plus its `name`.
    const FIXTURE_ATTRIBUTES: usize = 42 + 4 * 33;

    fn class_block(name: &str) -> String {
        format!(r#"<Class name="{name}">{CLASS_BODY}</Class>"#)
    }

    fn document(classes: &[&str]) -> String {
        let blocks: String = classes.iter().map(|c| class_block(c)).collect();
        format!(r#"<Handling><Stats team="Testers">{HEADER}{blocks}</Stats></Handling>"#)
    }

    fn all_four() -> String {
        document(&["VENOM", "FLASH", "RAPIER", "PHANTOM"])
    }

    /// Every ` name="value"` span in `doc`, so a test can remove them one at a
    /// time. Crude on purpose: the fixture is the only input it ever sees.
    fn attribute_spans(doc: &str) -> Vec<std::ops::Range<usize>> {
        let bytes = doc.as_bytes();
        let mut out = Vec::new();
        let mut at = 0usize;

        while let Some(eq) = doc[at..].find('=').map(|i| i + at) {
            let Some(open) = doc[eq + 1..].find('"').map(|i| i + eq + 1) else {
                break;
            };
            let Some(close) = doc[open + 1..].find('"').map(|i| i + open + 1) else {
                break;
            };
            let mut start = eq;
            while start > 0 && !bytes[start - 1].is_ascii_whitespace() && bytes[start - 1] != b'<' {
                start -= 1;
            }
            out.push(start..close + 1);
            at = close + 1;
        }

        out
    }

    #[test]
    fn parses_a_whole_document() {
        let stats = parse(&all_four()).expect("well-formed fixture");
        assert_eq!(stats.team, "Testers");
        assert_eq!(stats.internal_camera.fov, 1.0);
        assert_eq!(stats.backward_camera.headtilt, 7.0);
        assert_eq!(stats.bonnet_camera.pitch, 14.0);
        assert_eq!(stats.external_camera_far.spring_vert, 21.0);
        assert_eq!(stats.external_camera_close.lookat_height, 23.0);
        assert_eq!(stats.airbrake_graphics.up_speed, 31.0);
        assert_eq!(stats.misc.weight_distribution, Some(37.0));
        assert_eq!(stats.fe.shield, 41.0);
    }

    #[test]
    fn every_class_block_is_read_and_indexed_by_its_own_name() {
        let stats = parse(&all_four()).expect("well-formed fixture");
        for (index, class) in SpeedClass::ALL.into_iter().enumerate() {
            assert_eq!(stats.classes[index].name, Some(class), "slot {index}");
            assert_eq!(stats.class(class).expect("four rungs").name, Some(class));
        }
        let phantom = stats.class(SpeedClass::Phantom).expect("four rungs");
        assert_eq!(phantom.engine.turbo, 5.0);
        assert_eq!(phantom.brakes.gain, 8.0);
        assert_eq!(phantom.turning.falloff, 10.0);
        assert_eq!(phantom.airbrake.sideshift, Some(18.0));
        assert_eq!(phantom.antigrav.ride_height, 24.0);
        assert_eq!(phantom.physical.mass, 26.0);
        assert_eq!(phantom.pitch.antigrav_height_adjust, 32.0);
    }

    /// The invariant that matters most. A silently absent attribute defaulting
    /// to zero would be a physics bug that reads as a tuning problem, so every
    /// one of them is removed in turn and every removal must be a typed error.
    #[test]
    fn a_missing_attribute_is_an_error_not_a_default() {
        let doc = all_four();
        let spans = attribute_spans(&doc);
        assert_eq!(
            spans.len(),
            FIXTURE_ATTRIBUTES,
            "the fixture should carry every attribute the schema lists"
        );

        for span in spans {
            // The three attributes Pulse *added*. Their absence is a fact about
            // an earlier schema, not a defect, so they are exempt here and
            // pinned separately by
            // `the_three_pulse_era_attributes_are_absent_rather_than_missing`.
            let text = &doc[span.clone()];
            if ["easyshield", "weight_distribution", "sideshift"]
                .iter()
                .any(|added| text.trim_start().starts_with(added))
            {
                continue;
            }

            let mut broken = doc.clone();
            broken.replace_range(span.clone(), "");
            let result = parse(&broken);
            assert!(
                matches!(result, Err(Error::MissingAttribute { .. })),
                "removing {} was tolerated: {result:?}",
                &doc[span]
            );
        }
    }

    #[test]
    fn a_missing_element_is_an_error() {
        let doc = all_four();
        for element in [
            "InternalCamera",
            "BackwardCamera",
            "BonnetCamera",
            "ExternalCameraFar",
            "ExternalCameraClose",
            "AirbrakeGraphics",
            "Misc",
            "FE",
            "Engine",
            "Brakes",
            "Turning",
            "Airbrake",
            "Antigrav",
            "Physical",
            "pitch",
        ] {
            let open = format!("<{element} ");
            let at = doc.find(&open).expect("element in the fixture");
            let end = doc[at..].find("/>").expect("self-closing") + at + 2;
            let mut broken = doc.clone();
            broken.replace_range(at..end, "");
            assert_eq!(
                parse(&broken),
                Err(Error::MissingElement { element }),
                "dropping <{element}> was tolerated"
            );
        }
    }

    #[test]
    /// A file that names *some* of Pulse's ladder must name all of it.
    ///
    /// Narrowed rather than relaxed: the check now applies only where the file
    /// has shown it is using Pulse's ladder, so a partial Pulse file is still an
    /// error while a different generation's ladder is not.
    fn a_partly_present_pulse_ladder_is_an_error() {
        assert_eq!(
            parse(&document(&["VENOM", "FLASH", "RAPIER"])),
            Err(Error::MissingClass {
                class: SpeedClass::Phantom
            })
        );
        assert_eq!(
            parse(&document(&["FLASH", "RAPIER", "PHANTOM"])),
            Err(Error::MissingClass {
                class: SpeedClass::Venom
            })
        );
        assert_eq!(
            parse(&document(&["VENOM"])),
            Err(Error::MissingClass {
                class: SpeedClass::Flash
            })
        );
    }

    /// The three attributes Pulse added read as `None` when absent, and the
    /// exemption goes no further than those three.
    ///
    /// `None` is "this schema predates the field", which is why it is not an
    /// error - and why a *present but broken* value still is. Collapsing those
    /// two would let a typo pass as an older file.
    #[test]
    fn the_three_pulse_era_attributes_are_absent_rather_than_missing() {
        let doc = all_four();
        for (attribute, value) in [
            ("easyshield", "35"),
            ("weight_distribution", "37"),
            ("sideshift", "18"),
        ] {
            let stripped = doc.replace(&format!(r#" {attribute}="{value}""#), "");
            assert_ne!(stripped, doc, "the fixture should carry {attribute}");
            let stats = parse(&stripped)
                .unwrap_or_else(|e| panic!("removing {attribute} should not be an error: {e:?}"));
            let read = match attribute {
                "easyshield" => stats.misc.easyshield,
                "weight_distribution" => stats.misc.weight_distribution,
                _ => {
                    stats
                        .class(SpeedClass::Venom)
                        .expect("four rungs")
                        .airbrake
                        .sideshift
                }
            };
            assert_eq!(read, None, "{attribute} should read as absent");
        }

        // Present but unparseable is still an error, for the same three.
        let broken = doc.replace(r#"easyshield="35""#, r#"easyshield="oops""#);
        assert!(
            matches!(parse(&broken), Err(Error::NotANumber { .. })),
            "a broken easyshield should not pass as an older schema"
        );
    }

    /// A rung outside Pulse's four is **kept**, not rejected.
    ///
    /// This used to be `Error::UnknownClass`, and that strictness was what made
    /// this parser refuse Pure's `handlingstats.xml` outright: Pure ships a
    /// fifth speed class below Pulse's slowest. An unrecognised rung is a
    /// different ladder, not a corrupt file.
    ///
    /// What is still guaranteed: Pulse's four keep their `SpeedClass`
    /// discriminants as indices into the front of the vector, so the extra rung
    /// cannot displace them.
    #[test]
    fn a_class_name_outside_pulses_ladder_is_kept_after_the_four() {
        let doc = document(&["VENOM", "FLASH", "RAPIER", "PHANTOM", "SUPERSONIC"]);
        let stats = parse(&doc).expect("a fifth rung is a different ladder, not an error");

        assert_eq!(stats.classes.len(), 5);
        assert!(!stats.has_pulse_class_ladder());
        for class in SpeedClass::ALL {
            assert_eq!(
                stats
                    .class(class)
                    .expect("Pulse's four are still here")
                    .name,
                Some(class),
                "{class} moved when the fifth rung was added"
            );
        }
        assert_eq!(stats.classes[4].name, None);
        assert_eq!(stats.classes[4].raw_name, "SUPERSONIC");
    }

    /// A file naming *none* of Pulse's ladder is another generation's, and is
    /// kept whole rather than reported as four missing classes.
    #[test]
    fn a_ladder_with_no_pulse_rung_at_all_is_kept_whole() {
        let doc = document(&["ALPHA", "BETA"]);
        let stats = parse(&doc).expect("a wholly different ladder still parses");
        assert_eq!(stats.classes.len(), 2);
        assert!(!stats.has_pulse_class_ladder());
        assert_eq!(stats.class(SpeedClass::Venom), None);
    }

    #[test]
    fn a_duplicated_class_is_an_error() {
        let doc = document(&["VENOM", "FLASH", "RAPIER", "PHANTOM", "FLASH"]);
        assert_eq!(
            parse(&doc),
            Err(Error::DuplicateClass {
                class: SpeedClass::Flash
            })
        );
    }

    #[test]
    fn a_non_numeric_attribute_is_an_error() {
        let doc = all_four().replace(r#"mass="26""#, r#"mass="quite heavy""#);
        assert_eq!(
            parse(&doc),
            Err(Error::NotANumber {
                element: "Physical",
                attribute: "mass",
                value: "quite heavy".to_string(),
            })
        );
    }

    /// `"nan"` and `"inf"` parse fine as `f32`, and either one would poison every
    /// state hash downstream of it without ever failing a parse.
    #[test]
    fn a_non_finite_attribute_is_an_error() {
        for text in ["nan", "NaN", "inf", "-inf", "infinity"] {
            let doc = all_four().replace(r#"mass="26""#, &format!(r#"mass="{text}""#));
            assert!(
                matches!(parse(&doc), Err(Error::NotANumber { .. })),
                "{text} was accepted as a mass"
            );
        }
    }

    /// `<Values>` is an attribute carrier for its parent throughout this format,
    /// so a block may be written either way round.
    #[test]
    fn attributes_may_arrive_on_a_values_carrier() {
        let doc = all_four().replace(
            r#"<Brakes amount="6" falloff="7" gain="8"/>"#,
            r#"<Brakes><Values amount="6" falloff="7" gain="8"/></Brakes>"#,
        );
        let stats = parse(&doc).expect("a carrier is equivalent");
        assert_eq!(
            stats
                .class(SpeedClass::Venom)
                .expect("four rungs")
                .brakes
                .amount,
            6.0
        );
    }

    /// The files on disc are shortened, so the expander and the schema have to
    /// join up. Only the outer three element names are shortened here; names
    /// absent from a dictionary pass through untouched, which is what keeps the
    /// fixture readable.
    #[test]
    fn reads_a_shortened_blob() {
        let doc = all_four()
            .replace("<Handling>", "<h>")
            .replace("</Handling>", "</h>")
            .replace("<Stats ", "<s ")
            .replace("</Stats>", "</s>")
            .replace("<Class ", "<c ")
            .replace("</Class>", "</c>");
        let blob = format!(r#"<code hs="Handling" ss="Stats" cs="Class"></code>{doc}"#);

        assert_eq!(
            from_blob(blob.as_bytes()).expect("expands and parses"),
            parse(&all_four()).expect("well-formed fixture")
        );
    }

    /// The PS2 release ships this file as plain text beginning `<?xml`, so a blob
    /// with no `<code>` dictionary is not an error: it is the other platform.
    #[test]
    fn reads_a_plain_unshortened_document() {
        let plain = format!("<?xml version=\"1.0\"?>{}", all_four());
        assert_eq!(
            from_blob(plain.as_bytes()).expect("plain XML needs no expansion"),
            parse(&all_four()).expect("well-formed fixture")
        );
    }

    #[test]
    fn rejects_a_blob_that_is_not_text() {
        assert_eq!(
            from_blob(&[0xff, 0xfe, 0xff]),
            Err(Error::Expand(fexml::Error::NotText))
        );
    }

    #[test]
    fn a_document_with_no_handling_element_is_an_error() {
        assert_eq!(
            parse("<Screen name=\"Top\"></Screen>"),
            Err(Error::MissingElement {
                element: "Handling"
            })
        );
        assert_eq!(
            parse("<Handling></Handling>"),
            Err(Error::MissingElement { element: "Stats" })
        );
    }

    #[test]
    fn speed_class_names_round_trip_and_index_their_own_slot() {
        for (index, class) in SpeedClass::ALL.into_iter().enumerate() {
            assert_eq!(SpeedClass::from_name(class.as_str()), Some(class));
            assert_eq!(class as usize, index, "{class} is not in slot {index}");
        }
        assert_eq!(SpeedClass::from_name("venom"), Some(SpeedClass::Venom));
        assert_eq!(SpeedClass::from_name("SUPERSONIC"), None);
    }

    #[test]
    fn entry_names_are_built_the_way_the_loader_builds_them() {
        assert_eq!(entry_name("Feisar"), r"Data\Ships\Feisar\handlingstats.xml");
        assert_eq!(TEAMS.len(), 8);
    }

    /// A `<Global>` document with `classes` as its `<GlobalClass>` names, in the
    /// order given. Every number is invented and none is the game's; the two
    /// `<SpeedupPads>` attributes count from the block's position so a test can
    /// tell the slots apart.
    fn global_document(classes: &[&str]) -> String {
        let blocks: String = classes
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let n = i + 1;
                format!(
                    r#"<GlobalClass name="{name}"><SpeedupPads amount="{n}" time="{}"/>"#,
                    n * 10
                ) + &format!(r#"<GravityMul airborne="{}"/></GlobalClass>"#, n * 100)
            })
            .collect();
        format!(
            r#"<Handling><Global><Zone start="1" increment="2" recharge="3"/>{SPECIAL}{blocks}</Global></Handling>"#
        )
    }

    /// `<Special>` as [`global_document`] authors it. All five attributes, so
    /// the fixture is the shape the disc's own file is rather than only the one
    /// attribute [`Special`] reads; only `speedpad_jump` is asserted on.
    const SPECIAL: &str = r#"<Special roll_cost="4" roll_speed="5" roll_turbotime="6" speedpad_jump="7" turbo_jump="8"/>"#;

    const FOUR: [&str; 4] = ["VENOM", "FLASH", "RAPIER", "PHANTOM"];

    #[test]
    fn a_per_team_file_carries_no_global_block_and_that_is_not_an_error() {
        assert_eq!(parse_global(&all_four()), Ok(None));
    }

    #[test]
    fn each_global_class_lands_in_its_own_slot() {
        let global = parse_global(&global_document(&FOUR))
            .expect("parses")
            .expect("has a <Global>");
        assert_eq!(global.zone.start, 1.0);
        // The one `<Special>` attribute with a consumer, read off the same
        // `<Global>` as the rest and distinct from every other number here.
        assert_eq!(global.special.speedpad_jump, 7.0);
        for (index, class) in SpeedClass::ALL.into_iter().enumerate() {
            let n = (index + 1) as f32;
            assert_eq!(
                global.speedup_pads(class),
                SpeedupPads {
                    amount: n,
                    time: n * 10.0
                }
            );
            // Read from the same block, so a reader that took the pads from one
            // `<GlobalClass>` and the gravity from another would show up here.
            assert_eq!(
                global.gravity_mul(class),
                GravityMul {
                    airborne: n * 100.0
                }
            );
        }
    }

    /// The finding this reader is shaped around: both shipped discs author a
    /// fifth `<GlobalClass name="VECTOR">` **first**, and it must neither be an
    /// error nor shift the four that follow into the wrong slots. Position
    /// indexing would put every class one slot out and produce a boost that is
    /// wrong by a plausible-looking amount on all four.
    #[test]
    fn an_unrecognised_global_class_is_skipped_without_shifting_the_others() {
        let with_vector = ["VECTOR", "VENOM", "FLASH", "RAPIER", "PHANTOM"];
        let shifted = parse_global(&global_document(&with_vector))
            .expect("parses")
            .expect("has a <Global>");
        assert_eq!(shifted.speedup_pads(SpeedClass::Venom).amount, 2.0);
        assert_eq!(shifted.speedup_pads(SpeedClass::Phantom).amount, 5.0);

        // And the same four names without it keep the same *relative* order, so
        // the assertion above is about the skip rather than about the numbering.
        let without = parse_global(&global_document(&FOUR))
            .expect("parses")
            .expect("has a <Global>");
        assert_eq!(without.speedup_pads(SpeedClass::Venom).amount, 1.0);
        assert_eq!(without.speedup_pads(SpeedClass::Phantom).amount, 4.0);
    }

    #[test]
    fn a_global_block_missing_a_speed_class_is_an_error() {
        assert_eq!(
            parse_global(&global_document(&["VENOM", "FLASH", "RAPIER"])),
            Err(Error::MissingGlobalClass {
                class: SpeedClass::Phantom
            })
        );
        assert_eq!(
            parse_global(&global_document(&[
                "VENOM", "VENOM", "FLASH", "RAPIER", "PHANTOM"
            ])),
            Err(Error::DuplicateGlobalClass {
                class: SpeedClass::Venom
            })
        );
    }

    /// `<Global>` present but incomplete is an error rather than `Ok(None)`,
    /// which is the tightening this reader makes over the `<Zone>`-only one it
    /// replaced. Half a configuration is worse than none: the boost would be
    /// silently absent on whichever class lost its block.
    #[test]
    fn a_global_block_missing_its_zone_or_its_pads_is_an_error() {
        let no_zone =
            global_document(&FOUR).replace(r#"<Zone start="1" increment="2" recharge="3"/>"#, "");
        assert_eq!(
            parse_global(&no_zone),
            Err(Error::MissingElement { element: "Zone" })
        );

        let no_special = global_document(&FOUR).replace(SPECIAL, "");
        assert_eq!(
            parse_global(&no_special),
            Err(Error::MissingElement { element: "Special" })
        );

        let no_pads = global_document(&FOUR).replace(r#"<SpeedupPads amount="1" time="10"/>"#, "");
        assert_eq!(
            parse_global(&no_pads),
            Err(Error::MissingElement {
                element: "SpeedupPads"
            })
        );

        let no_gravity = global_document(&FOUR).replace(r#"<GravityMul airborne="100"/>"#, "");
        assert_eq!(
            parse_global(&no_gravity),
            Err(Error::MissingElement {
                element: "GravityMul"
            })
        );
    }
}
