//! The engine-wide `<Global>` block and the per-speed-class tables under it.
//!
//! Split out of [`super`] under the 1,000-line rule, plus the name-keyed
//! lookups [`Global::extra`] documents.

use super::{Error, Node, Result, SpeedClass, Zone, child, descendant, fexml, number};

/// The speed-pad boost, per speed class. `<SpeedupPads amount time/>`.
///
/// Under `<Handling><Global><GlobalClass name="...">`. `Xml_ReadGlobalSettings`
/// (`0x0883a970`) stores both attributes **verbatim** into the per-class tables
/// at `0x08b36bc0` (`amount`) and `0x08b36bd0` (`time`), indexed by
/// `g_handling_parse_class`. Confidence **90**.
///
/// **Neither is pre-scaled**, unlike the four in
/// `oag_gameplay::handling::SCALED_FIELDS`. The factor of ten is the force law's
/// own; see [`oag_physics::forces::evaluate`]'s speed-pad term.
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

/// The weapon-pad re-trigger cooldown, per speed class.
/// `<WeaponPad refresh_time elimination_refresh_time/>`.
///
/// `Xml_ReadGlobalSettings` (`0x0883aa14`) stores `refresh_time` at
/// `0x08b34328` and `elimination_refresh_time` at `0x08b34338`, per class and
/// verbatim. `WeaponPads_TestCraft` (`0x0888727c`) stamps the hit pad with one
/// (the Eliminator table under mode `8`) and `WeaponPad_UpdateRefreshTimer`
/// (`0x0892c034`) counts it down at `dt`. Confidence **90**; see
/// `docs/ghidra/functions/psp-pulse-usa/pads.md`.
///
/// **A debounce, not a pickup respawn.** Both PSP discs author
/// `refresh_time="0.55"` for every class; a craft clears a roughly 9.6-unit pad
/// in well under that, so one crossing is one pickup.
/// `elimination_refresh_time` is `0.05`, fitting Eliminator's freer weapons.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WeaponPad {
    /// Seconds before a pad can be triggered again, in the ordinary modes.
    pub refresh_time: f32,
    /// The same, in Eliminator.
    ///
    /// An [`Option`] because the difference is measured: Pulse authors both
    /// attributes and **Pure only `refresh_time`**
    /// (`crates/pure/tests/handling_schema_ground_truth.rs`), and Pure has no
    /// Eliminator. A zero default would look like authored zero.
    pub elimination_refresh_time: Option<f32>,
}

impl WeaponPad {
    const ELEMENT: &'static str = "WeaponPad";

    fn from_node(node: &Node) -> Result<Self> {
        Ok(Self {
            refresh_time: number(node, Self::ELEMENT, "refresh_time")?,
            elimination_refresh_time: match node.value("elimination_refresh_time") {
                Some(_) => Some(number(node, Self::ELEMENT, "elimination_refresh_time")?),
                None => None,
            },
        })
    }
}

/// The per-speed-class gravity scale. `<GravityMul airborne/>`.
///
/// `Xml_ReadGlobalSettings` (`0x0883a970`) stores it verbatim into
/// `g_class_gravity_scale` (`0x08ab0dcc`), indexed by `g_handling_parse_class`.
/// Confidence **90**.
///
/// # `airborne` scales the *grounded* term
///
/// Settled instruction by instruction. The table has three references: this
/// write and two reads in `Ship_UpdateCraft`'s gravity term at
/// `0x08849b40`/`0x08849b48`, which builds four VFPU pairs and multiplies them:
///
/// ```text
/// C600 = (class+0xf8, class+0xfc) = (normal_gravity, flight_gravity)
/// C610 = (g_class_gravity_scale[class], 1.0)      ; viim.s S611, 1
/// C620 = (mass, mass)
/// C630 = (craft+0x2b0, 1 - craft+0x2b0)           ; vocp.s S631, S630
/// worldForce.y += -(C600 * C610 * C620 * C630).x + .y
/// ```
///
/// Lane 0 is `normal_gravity`, multiplied by `grounded`, and takes the scale.
/// Lane 1 (airborne) is multiplied by a literal `1.0`. So despite the name this
/// scales weight **on the ground**. `HandlingXml_ParsePhysical` (`0x08838f50`)
/// stores `normal_gravity` to `+0xf8` and `flight_gravity` to `+0xfc` with a
/// `0x80` per-class stride, matching `sll a0, a0, 0x7` at the gravity site.
///
/// Whether "airborne" is unimplemented intent or a renaming nobody propagated is
/// **not** answered; only which term the number reaches.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GravityMul {
    /// The multiplier on `<Physical normal_gravity>`, despite this name; see the
    /// type's docs.
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

/// Four of `<Special>`'s five attributes.
///
/// The element carries `roll_cost roll_speed roll_turbotime speedpad_jump
/// turbo_jump`; `turbo_jump` is left until something needs it (probably not the
/// barrel roll's; see `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`).
/// The other four are the **barrel roll**'s tunables, read by
/// `Xml_ReadGlobalSettings` (`0x0883a970`) into one contiguous `.bss` run,
/// confidence **90**. Both PSP pressings author:
///
/// ```text
/// <Special roll_cost="8" roll_speed="1.5" roll_turbotime="0.5"
///          speedpad_jump="0.1" turbo_jump="0.1"/>
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Special {
    /// How far the speed-pad boost tilts toward the hull's up axis while
    /// pitch-up is held. `0.1` on both discs.
    ///
    /// `g_speedpad_jump` (`0x08b36bec`); `Ship_ApplySpeedupPad` (`0x08848f9c`)
    /// adds `craft+0x160 * g_speedpad_jump` to the pad's push direction. **Not a
    /// jump**: the direction is not renormalised, so `0.1` is a 5.71-degree tilt
    /// and 0.5 % more force, mostly absorbed by the hover spring. See
    /// `oag_physics::engine::speedup_pad`. Stored **unscaled**, like
    /// `<SpeedupPads>`.
    pub speedpad_jump: f32,
    /// What a completed barrel roll costs, as a **percentage** of shield
    /// capacity. `8` on both discs.
    ///
    /// `Ship_BarrelRollCost` (`0x08840770`) reads `g_roll_cost` (`0x08b36bf4`)
    /// and returns `roll_cost * 0.01 * stats_base[skill_level].shield`, the same
    /// expression `Ship_SetShield` (`0x0883e6f4`) clamps writes to
    /// (`oag_physics::params::Dimensions::shield`). The gesture arms only if
    /// this is strictly less than the current shield. See
    /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
    pub roll_cost: f32,
    /// How fast the roll's signed phase ramps toward `+/-1.0`, per second.
    /// `1.5` on both discs, so a full roll takes `0.667 s`. `g_roll_speed`
    /// (`0x08b36bf0`).
    pub roll_speed: f32,
    /// How long the landing payout holds once a completed roll touches down, in
    /// seconds. `0.5` on both discs. `g_roll_turbotime` (`0x08b36bf8`).
    ///
    /// While it runs the original holds `craft+0x1c0 & 0x400`: a **1.5x lateral
    /// grip multiplier**, the hover spring's damping scalar forced to `1.0`, and
    /// the uncapped turbo add `oag_physics::ship::ShipState::turbo_timer` models
    /// for the bit's other source. Confidence 90; see the `input-bindings.md`
    /// page above.
    pub roll_turbotime: f32,
}

impl Special {
    const ELEMENT: &'static str = "Special";

    fn from_node(node: &Node) -> Result<Self> {
        Ok(Self {
            speedpad_jump: number(node, Self::ELEMENT, "speedpad_jump")?,
            roll_cost: number(node, Self::ELEMENT, "roll_cost")?,
            roll_speed: number(node, Self::ELEMENT, "roll_speed")?,
            roll_turbotime: number(node, Self::ELEMENT, "roll_turbotime")?,
        })
    }
}

/// `<StartBoost windowStart windowEnd stallEnd overallDuration stallMul normalMul
/// boostMul/>`: the launch boost's window and three multipliers.
///
/// `Xml_ReadBoostSettings` (`0x088390b4`) matches the seven names literally and
/// stores each unscaled into `0x08ab0d70..0x08ab0d88`; `Ship_UpdateStartBoost`
/// (`0x0883fdec`) and `FUN_0882773c` read them (`docs/physics/launch-boost.md`).
/// Confidence **90** for names and order; the law was watched live on five
/// launches.
///
/// **Absent on Pure**, so [`Global`] holds it as an `Option`: the element's
/// presence says a title has a launch boost. Values are read from the player's
/// disc and reproduced nowhere in this repository.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StartBoost {
    /// Seconds after GO at which the "perfect" window opens (`0x08ab0d70`).
    pub window_start: f32,
    /// Seconds after GO at which the perfect window closes (`0x08ab0d74`).
    pub window_end: f32,
    /// Seconds after GO at which the stall window closes (`0x08ab0d78`).
    pub stall_end: f32,
    /// How long the multiplier is held after GO, in seconds (`0x08ab0d7c`).
    pub overall_duration: f32,
    /// The multiplier for a thrust that first landed before the perfect window or
    /// in the stall window after it (`0x08ab0d80`).
    pub stall_mul: f32,
    /// The multiplier before any grade is earned, and for a thrust that first
    /// landed after the stall window (`0x08ab0d84`).
    pub normal_mul: f32,
    /// The multiplier for a thrust that first landed inside the perfect window
    /// (`0x08ab0d88`).
    pub boost_mul: f32,
}

impl StartBoost {
    const ELEMENT: &'static str = "StartBoost";

    fn from_node(node: &Node) -> Result<Self> {
        let e = Self::ELEMENT;
        Ok(Self {
            window_start: number(node, e, "windowStart")?,
            window_end: number(node, e, "windowEnd")?,
            stall_end: number(node, e, "stallEnd")?,
            overall_duration: number(node, e, "overallDuration")?,
            stall_mul: number(node, e, "stallMul")?,
            normal_mul: number(node, e, "normalMul")?,
            boost_mul: number(node, e, "boostMul")?,
        })
    }
}

/// A `<GlobalClass>` whose `name` is outside [`SpeedClass`]: [`super::Class`]'s
/// `raw_name` one level up. `VECTOR` is the only one any measured disc carries.
#[derive(Debug, Clone, PartialEq)]
pub struct ForeignGlobalClass {
    /// The `name` attribute exactly as written.
    pub name: String,
    /// `<SpeedupPads/>` for this rung.
    pub speedup_pads: SpeedupPads,
    /// `<GravityMul/>` for this rung.
    pub gravity_mul: GravityMul,
    /// `<WeaponPad/>` for this rung.
    pub weapon_pad: WeaponPad,
}

/// The engine-wide `<Global>` block, out of [`GLOBAL_ENTRY`].
///
/// Only parts with a consumer are decoded. The three camera pitch modifiers and
/// `<CameraSideOffset>` are read by `Xml_ReadGlobalSettings` and left alone
/// (named in `docs/ghidra/functions/psp-pulse-usa/engine.md`); of `<Special>`
/// only `turbo_jump` is unread; see [`Special`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Global {
    /// `<Zone/>`.
    pub zone: Zone,
    /// `<Special/>`, four of five attributes.
    pub special: Special,
    /// `<StartBoost/>`, `None` where the file authors none (Pure).
    pub start_boost: Option<StartBoost>,
    /// `<GlobalClass><SpeedupPads/></GlobalClass>`, indexed by [`SpeedClass`].
    pub speedup_pads: [SpeedupPads; 4],
    /// `<GlobalClass><GravityMul/></GlobalClass>`, indexed by [`SpeedClass`].
    pub gravity_mul: [GravityMul; 4],
    /// `<GlobalClass><WeaponPad/></GlobalClass>`, indexed by [`SpeedClass`].
    pub weapon_pads: [WeaponPad; 4],
    /// The `<GlobalClass>` blocks whose `name` is outside [`SpeedClass`], in
    /// document order.
    ///
    /// Retaining them does not contradict [`global_classes`]' skip: that
    /// argument is about the four **arrays**, and nothing changes what they
    /// return for Pulse's rungs. But the document authors this rung with three
    /// whole elements, so the parser keeps them. **Which rungs a title offers is
    /// `oag_title::SpeedClasses`' business**, measured per title from its
    /// per-team `handlingstats.xml`: Pulse's author four `<Class>` blocks and no
    /// `VECTOR`, Pure's five. "Pulse discards `VECTOR`" thereby becomes a title
    /// fact with evidence, as [ADR-0022] asks.
    ///
    /// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
    pub extra: Vec<ForeignGlobalClass>,
}

impl Global {
    /// The speed-pad tunables for one class.
    #[must_use]
    pub fn speedup_pads(&self, class: SpeedClass) -> SpeedupPads {
        self.speedup_pads[class as usize]
    }

    /// The weapon-pad cooldown for one class.
    #[must_use]
    pub fn weapon_pads(&self, class: SpeedClass) -> WeaponPad {
        self.weapon_pads[class as usize]
    }

    /// The gravity scale for one class.
    #[must_use]
    pub fn gravity_mul(&self, class: SpeedClass) -> GravityMul {
        self.gravity_mul[class as usize]
    }

    /// The whole `<GlobalClass>` for a rung named as the document spells it,
    /// case-insensitively.
    ///
    /// For titles whose ladder is not Pulse's, and the only way to reach
    /// [`Self::extra`]; it answers Pulse's four out of the arrays too.
    ///
    /// `None` means **this file does not author that rung**, not zeroes, and not
    /// that a neighbour's numbers will do: the caller must say so, not
    /// substitute.
    #[must_use]
    pub fn class_named(&self, name: &str) -> Option<(SpeedupPads, GravityMul, WeaponPad)> {
        if let Some(class) = SpeedClass::from_name(name) {
            return Some((
                self.speedup_pads[class as usize],
                self.gravity_mul[class as usize],
                self.weapon_pads[class as usize],
            ));
        }
        self.extra
            .iter()
            .find(|block| block.name.eq_ignore_ascii_case(name))
            .map(|block| (block.speedup_pads, block.gravity_mul, block.weapon_pad))
    }
}

/// The archive entry holding `<Global>`, engine-wide rather than per team.
///
/// A literal in the executable at `0x08a8a270`, passed to `Handling_ParseStats`
/// by `0x0894f6a8`, so the WAD lookup is an exact `oag_formats::wad::hash_name`
/// hit rather than a mined candidate.
pub const GLOBAL_ENTRY: &str = r"Data\XML\HandlingStats.xml";

/// Reads `<Handling><Global>` out of an **already expanded** document.
///
/// `Ok(None)` when there is no `<Global>` at all (every shipped per-team file).
/// Once present, every part [`Global`] names is required: a mode configured
/// with three-quarters of its numbers is worse than one with none.
pub fn parse_global(expanded: &str) -> Result<Option<Global>> {
    let root = fexml::parse(expanded);
    let handling = descendant(&root, "Handling").ok_or(Error::MissingElement {
        element: "Handling",
    })?;
    let Ok(global) = child(handling, "Global") else {
        return Ok(None);
    };
    let (speedup_pads, gravity_mul, weapon_pads, extra) = global_classes(global)?;
    Ok(Some(Global {
        zone: Zone::from_node(child(global, Zone::ELEMENT)?)?,
        special: Special::from_node(child(global, Special::ELEMENT)?)?,
        start_boost: child(global, StartBoost::ELEMENT)
            .ok()
            .map(StartBoost::from_node)
            .transpose()?,
        speedup_pads,
        gravity_mul,
        weapon_pads,
        extra,
    }))
}

/// The four arrays [`Global`] holds, plus the rungs they cannot hold.
type GlobalClasses = (
    [SpeedupPads; 4],
    [GravityMul; 4],
    [WeaponPad; 4],
    Vec<ForeignGlobalClass>,
);

/// Collects `<GlobalClass><SpeedupPads/></GlobalClass>` into an array indexed by
/// [`SpeedClass`], plus the rungs that array cannot hold.
///
/// Same one-pass shape as [`super::classes`], with **one deliberate difference:
/// an unrecognised `name` is not an [`Error::UnknownClass`]**. Every disc
/// measured (Pulse's two pressings, Pure's two) authors **five** blocks, `VECTOR`
/// first, so erroring would fail all four.
///
/// The original discards `VECTOR` by accident: `Xml_ReadGlobalSettings`
/// (`0x0883a970`) matches `name` against a four-entry table and on no match
/// leaves `g_handling_parse_class` at the last match (a global, never reset), so
/// `VECTOR`'s numbers land in another class's slot and are overwritten by the
/// four that follow. Confidence **88**. **This holds only while `VECTOR` is
/// first**; authored last it would corrupt `PHANTOM` in the original and not
/// here, which is worth knowing, not emulating.
///
/// An unrecognised block is kept in [`Global::extra`] rather than dropped,
/// because **Pure's engine offers this rung**: losing the numbers would leave a
/// `VECTOR` race with no authored pads, gravity or cooldown and only an invented
/// stand-in to borrow. Which rungs a title *offers* is
/// `oag_title::SpeedClasses`' decision.
fn global_classes(global: &Node) -> Result<GlobalClasses> {
    const ELEMENT: &str = "GlobalClass";
    let mut found: [Option<(SpeedupPads, GravityMul, WeaponPad)>; 4] = [None; 4];
    let mut extra: Vec<ForeignGlobalClass> = Vec::new();

    for node in global.children_named(ELEMENT) {
        let name = node.value("name").ok_or(Error::MissingAttribute {
            element: "GlobalClass",
            attribute: "name",
        })?;
        let name = name.trim();
        let Some(class) = SpeedClass::from_name(name) else {
            // A rung outside the four: read it if whole, **drop it silently if
            // not**, so an unrecognised `name` can never fail a file (an
            // unmeasured disc's malformed foreign block must not reject a
            // `<Global>` whose four real rungs parse). An unreadable rung is the
            // same honest absence as one never authored.
            if let Ok(block) = read_class_block(node) {
                extra.push(ForeignGlobalClass {
                    name: name.to_string(),
                    speedup_pads: block.0,
                    gravity_mul: block.1,
                    weapon_pad: block.2,
                });
            }
            continue;
        };

        let slot = &mut found[class as usize];
        if slot.is_some() {
            return Err(Error::DuplicateGlobalClass { class });
        }
        *slot = Some(read_class_block(node)?);
    }

    for class in SpeedClass::ALL {
        if found[class as usize].is_none() {
            return Err(Error::MissingGlobalClass { class });
        }
    }

    let found = found.map(|block| block.expect("every slot filled above"));
    Ok((
        found.map(|(pads, _, _)| pads),
        found.map(|(_, mul, _)| mul),
        found.map(|(_, _, weapon)| weapon),
        extra,
    ))
}

/// The three elements every `<GlobalClass>` carries, recognised rung or not.
fn read_class_block(node: &Node) -> Result<(SpeedupPads, GravityMul, WeaponPad)> {
    Ok((
        SpeedupPads::from_node(child(node, SpeedupPads::ELEMENT)?)?,
        GravityMul::from_node(child(node, GravityMul::ELEMENT)?)?,
        WeaponPad::from_node(child(node, WeaponPad::ELEMENT)?)?,
    ))
}

/// [`parse_global`] over raw archive bytes, expanding them if needed, as
/// [`from_blob`] does.
pub fn global_from_blob(data: &[u8]) -> Result<Option<Global>> {
    if fexml::is_fexml(data) {
        parse_global(&fexml::expand(data)?)
    } else {
        parse_global(std::str::from_utf8(data).map_err(|_| fexml::Error::NotText)?)
    }
}
