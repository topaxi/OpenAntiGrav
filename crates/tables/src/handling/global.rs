//! The engine-wide `<Global>` block and the per-speed-class tables under it.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change, followed by
//! the name-keyed lookups [`Global::extra`] documents.

use super::{Error, Node, Result, SpeedClass, Zone, child, descendant, fexml, number};

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

/// The weapon-pad re-trigger cooldown, per speed class.
/// `<WeaponPad refresh_time elimination_refresh_time/>`.
///
/// Sits beside [`SpeedupPads`] under `<GlobalClass>`. `Xml_ReadGlobalSettings`
/// (`0x0883aa14`) stores `refresh_time` into the per-class table at `0x08b34328`
/// and `elimination_refresh_time` into `0x08b34338`, both indexed by
/// `g_handling_parse_class` and both verbatim. `WeaponPads_TestCraft`
/// (`0x0888727c`) stamps the pad it hit with one of the two - the Eliminator
/// table under mode `8`, otherwise the ordinary one - and
/// `WeaponPad_UpdateRefreshTimer` (`0x0892c034`) counts it back down at `dt` a
/// tick. Confidence **90**; see
/// `docs/ghidra/functions/psp-pulse-usa/pads.md`.
///
/// # It is a debounce, not a pickup respawn
///
/// Worth saying because the attribute's name suggests otherwise. Both shipped
/// PSP discs author `refresh_time="0.55"` for every class, and a craft at racing
/// speed clears a pad about 9.6 units long in well under that - so what the
/// timer buys is that one crossing is one pickup, not that a collected pad goes
/// away for a while. `elimination_refresh_time` is `0.05`, an order of magnitude
/// shorter, which fits Eliminator handing weapons out far more freely.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WeaponPad {
    /// Seconds before a pad can be triggered again, in the ordinary modes.
    pub refresh_time: f32,
    /// The same, in Eliminator.
    ///
    /// An [`Option`] where the rest of this module makes a missing attribute an
    /// error, because the difference is measured rather than defensive: Pulse
    /// authors both attributes and **Pure authors only `refresh_time`** - see
    /// `crates/pure/tests/handling_schema_ground_truth.rs`, which pins exactly
    /// that difference, and Pure has no Eliminator to configure. Defaulting it
    /// to zero would be indistinguishable from a disc that authored zero.
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

/// Four of `<Special>`'s five attributes.
///
/// The element carries five - `roll_cost roll_speed roll_turbotime speedpad_jump
/// turbo_jump` - and `turbo_jump` is left alone until something needs it, the
/// way the rest of `<Global>` is; see `docs/ghidra/functions/psp-pulse-usa/
/// input-bindings.md`'s barrel-roll section for why it is probably not this
/// mechanic's.
///
/// The other four are the **barrel roll**'s tunables, all read from
/// `Xml_ReadGlobalSettings` (`0x0883a970`) into one contiguous `.bss` run,
/// confidence **90**. Both shipped PSP pressings author the same line:
///
/// ```text
/// <Special roll_cost="8" roll_speed="1.5" roll_turbotime="0.5"
///          speedpad_jump="0.1" turbo_jump="0.1"/>
/// ```
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
    /// What a completed barrel roll costs, as a **percentage** of the ship's
    /// shield capacity. `8` on both shipped discs.
    ///
    /// `Ship_BarrelRollCost` (`0x08840770`) reads `g_roll_cost` (`0x08b36bf4`)
    /// and returns `roll_cost * 0.01 * stats_base[skill_level].shield` - the
    /// **identical** expression `Ship_SetShield` (`0x0883e6f4`) clamps every
    /// shield write to, i.e. `oag_physics::params::Dimensions::shield` in this
    /// crate. The gesture arms nothing unless this is strictly less than the
    /// current shield. See `docs/ghidra/functions/psp-pulse-usa/
    /// input-bindings.md`.
    pub roll_cost: f32,
    /// How fast the barrel roll's signed phase ramps toward `+/-1.0`, in units
    /// per second. `1.5` on both shipped discs, so a full roll takes `0.667 s`.
    ///
    /// `g_roll_speed` (`0x08b36bf0`).
    pub roll_speed: f32,
    /// How long the landing payout holds, in seconds, once a completed roll
    /// touches down. `0.5` on both shipped discs.
    ///
    /// `g_roll_turbotime` (`0x08b36bf8`). While it runs the original's
    /// `craft+0x1c0 & 0x400` is held, which is a **1.5x lateral grip
    /// multiplier**, the hover spring's damping scalar forced to `1.0`, and the
    /// same uncapped turbo add `oag_physics::ship::ShipState::turbo_timer`
    /// already models for the other source of that bit. Confidence 90; see
    /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
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

/// The engine-wide `<Global>` block, out of [`GLOBAL_ENTRY`].
///
/// Only the parts this project has a consumer for are decoded. The three camera
/// pitch modifiers and `<CameraSideOffset>` are read by the original's
/// `Xml_ReadGlobalSettings` and are deliberately left alone here;
/// they are named in `docs/ghidra/functions/psp-pulse-usa/engine.md` and can be
/// added when something needs them.
///
/// Of `<Special>`'s five attributes, `turbo_jump` is the only one left unread -
/// see [`Special`].
/// `<StartBoost windowStart windowEnd stallEnd overallDuration stallMul normalMul
/// boostMul/>` - the launch boost's window and its three multipliers.
///
/// `Xml_ReadBoostSettings` (`0x088390b4`) compares the seven attribute names
/// literally and stores each through `Xml_AttributeAsFloat` into
/// `0x08ab0d70..0x08ab0d88` unscaled; `Ship_UpdateStartBoost` (`0x0883fdec`) and
/// `FUN_0882773c` read them, see `docs/physics/launch-boost.md`. Confidence
/// **90** for the names and the order, from the parse function; the law that
/// uses them was watched live on five launches.
///
/// **Absent on Pure**, whose `<Global>` has no such element, so [`Global`] holds
/// it as an `Option`: the presence of the element is what says a title has a
/// launch boost at all. Values are read from the player's own disc at runtime
/// and are not reproduced anywhere in this repository.
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
    /// The multiplier for a thrust that first landed before the perfect window
    /// or in the stall window after it (`0x08ab0d80`).
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

/// A `<GlobalClass>` whose `name` is outside [`SpeedClass`].
///
/// The counterpart of [`super::Class`]'s `raw_name`, one level up: the same rung
/// authored in the engine-wide file rather than in a team's. `VECTOR` is the only
/// one any measured disc carries.
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
/// Only the parts this project has a consumer for are decoded. The three camera
/// pitch modifiers and `<CameraSideOffset>` are read by the original's
/// `Xml_ReadGlobalSettings` and are deliberately left alone here;
/// they are named in `docs/ghidra/functions/psp-pulse-usa/engine.md` and can be
/// added when something needs them.
///
/// Of `<Special>`'s five attributes, `turbo_jump` is the only one left unread -
/// see [`Special`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Global {
    /// `<Zone/>`.
    pub zone: Zone,
    /// `<Special/>`, of which one attribute is decoded.
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
    /// # Why retaining these is not a contradiction of [`global_classes`]' skip
    ///
    /// [`global_classes`] explains at length that dropping an unrecognised
    /// `<GlobalClass>` reproduces what the original does with `VECTOR`, and
    /// every disc measured so far - Pulse's two pressings *and* Pure's two -
    /// authors five blocks with `VECTOR` first. That argument is about the four
    /// **arrays**, and it still holds: nothing here changes what
    /// [`Self::speedup_pads`], [`Self::gravity_mul`] or [`Self::weapon_pads`]
    /// return for any of Pulse's four rungs.
    ///
    /// What changes is that the block is no longer thrown away. The parser's job
    /// is to describe the document, and the document authors this rung with
    /// three fully populated elements. **Which rungs a title actually offers is
    /// `oag_title::SpeedClasses`' business, one layer up** - and it is measured
    /// per title, from each title's own per-team `handlingstats.xml`. Pulse's
    /// per-team files author four `<Class>` blocks and no `VECTOR`, so Pulse's
    /// ladder is four and nothing ever asks this vector for its `VECTOR` entry.
    /// Pure's author five, so Pure's does.
    ///
    /// So "Pulse discards `VECTOR`" stops being a parser accident preserved by
    /// hand and becomes a title fact with evidence behind it - which is the
    /// direction [ADR-0022] asks these decisions to move. The one behaviour this
    /// removes is nothing: a rung no ladder names is a rung no menu draws.
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

    /// The whole `<GlobalClass>` for a rung named the way the document spells
    /// it, matched case-insensitively.
    ///
    /// This is the lookup a title whose ladder is not Pulse's needs, and the
    /// only one that can reach [`Self::extra`]. It answers for Pulse's four as
    /// well, out of the arrays, so a caller holding nothing but a name never has
    /// to know which storage a rung landed in.
    ///
    /// `None` means **this file does not author that rung** - not that it
    /// authors zeroes, and emphatically not that some neighbouring rung's
    /// numbers will do. A caller that gets `None` has to say so rather than
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
/// by `0x0894f6a8`. Being a literal means the WAD lookup is an exact
/// `oag_formats::wad::hash_name` hit rather than a mined candidate.
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
/// The same one-pass shape as [`super::classes`], and for the same reasons, with
/// **one deliberate difference: an unrecognised `name` does not become an
/// [`Error::UnknownClass`]**. That is the only place in this module where an
/// unknown name is not an error, so it is worth saying why.
///
/// Every disc measured so far - Pulse's two pressings and Pure's two - authors
/// **five** `<GlobalClass>` blocks, `VECTOR` first, then the four speed classes.
/// Erroring here would fail on all four discs.
///
/// The original discards `VECTOR` too, and by accident rather than by design.
/// `Xml_ReadGlobalSettings` (`0x0883a970`) matches `name` against a four-entry
/// table and, on no match, leaves `g_handling_parse_class` holding whatever the
/// last match left there - it is a global and is never reset per element. So
/// `VECTOR`'s numbers land in some other class's slot and are then overwritten by
/// the four blocks that follow it, because `VECTOR` is authored first.
/// Confidence **88**.
///
/// **This holds only while `VECTOR` is first.** Authored last it would corrupt
/// `PHANTOM` in the original and not here, which is a difference worth knowing
/// about rather than one worth emulating.
///
/// # Why the block is now kept rather than dropped
///
/// It used to `continue` past an unrecognised name, which reproduced the
/// original's outcome for the four arrays. It still does reproduce it: the
/// arrays are unchanged and no rung of Pulse's ladder reads a foreign block. But
/// the numbers are now handed back in [`Global::extra`] instead of being thrown
/// away, because **Pure's engine offers this rung and needs them**. Losing them
/// here would leave a `VECTOR` race with no authored speed pads, gravity scale
/// or weapon-pad cooldown, and the only thing left to do would be to borrow
/// another rung's - the invented stand-in this project forbids.
///
/// The decision about which rungs a title *offers* moved to
/// `oag_title::SpeedClasses`, where it is measured per title. See
/// [`Global::extra`].
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
            // A rung outside the four. Read it if it is whole and **drop it
            // silently if it is not**, which keeps the property this function
            // has always had: an unrecognised `name` can never fail a file.
            // Erroring instead would let an unmeasured disc's malformed foreign
            // block reject a `<Global>` whose four real rungs parse perfectly.
            // A rung that cannot be read is a rung no ladder can offer - the
            // same honest absence as one that was never authored.
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
