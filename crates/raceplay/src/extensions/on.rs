use oag_ext_abi::{Command, Event, MAX_EVENTS, Pickup};

use crate::extension_effects::Fired;
use oag_modhost::{Module, Observation};

use super::{Drops, Report};
use crate::extension_pickup::pickup;
use crate::sim::RaceSim;

#[derive(Debug, Clone)]
pub(super) struct Slot {
    pub(super) mod_id: String,
    /// `channel_key` of the mod's id: a `mod` event's `from`.
    sender: u32,
    /// The channels its `emit` may name.
    publishes: Vec<(u32, String)>,
    /// `[extension] drives_ai_pickup`.
    drives_ai_pickup: bool,
    module: Module,
    pub(super) blob: Vec<u8>,
    faulted: Option<(u64, String)>,
    fuel_used: u64,
    fuel_peak: u64,
    call_us: f32,
}

impl Slot {
    /// Turns the module off for the race, once: the first fault stands.
    fn fault(&mut self, tick: u64, why: String) {
        if self.faulted.is_none() {
            log::warn!(
                "mod {}: {why}, at tick {tick}; off for the rest of the race",
                self.mod_id
            );
            self.faulted = Some((tick, why));
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct On {
    pub(super) slots: Vec<Slot>,
    /// One bit per slot that touched a wall since the last call: the event buffer,
    /// at most one `wall-contact` per craft per tick, raised in slot order.
    walls: u32,
    /// Each slot's engine scale above 1.0 for this tick's step
    /// (`Environment::extension_thrust`), set at the call and taken by the step.
    pub(super) boost: [f32; oag_gameplay::MAX_SHIPS],
    /// Each module's pickup kind, in module order: a mod that declares a `[pickup]` has one,
    /// by its place among those, and only it gives, clears or rolls it.
    module_kind: Vec<Option<oag_tables::weapons::ModKind>>,
    /// The module of each kind, in kind order.
    kind_module: Vec<usize>,
    /// Each module's `pickup-roll` rule (weapon, every) and how many grants of that weapon
    /// it has counted.
    roll: Vec<Option<(oag_tables::weapons::Weapon, u8)>>,
    rolled: Vec<u32>,
    /// One bit per slot whose held mod pickup came from a `pickup-roll` (the grant's
    /// weapon is the craft's `last`), not from a `give-pickup`: what a fault of the
    /// owner turns it back into.
    from_roll: u32,
    /// `pickup-fire` / `pickup-absorbed` raised since the last call, one bit a slot, per
    /// kind: the event goes to the module of the kind the craft held when it was used.
    pub(super) fires: [u32; oag_tables::weapons::ModKind::MAX],
    absorbs: [u32; oag_tables::weapons::ModKind::MAX],
    /// The `mod` events raised since the last call (the modules' `emit`s and the track's
    /// triggers'), delivered at the next one: race state, hashed.
    mail: Vec<super::channel::Mail>,
    /// The track as the modules read it.
    pub(super) region: super::track::Region,
    /// Each speed pad's race-wide `scale-pad`, held until replaced.
    pad_scale: Vec<f32>,
    /// The module (index plus one, 0 for none) whose `scale-pad` holds each pad.
    pad_by: Vec<u8>,
    /// Each slot's `scale-pad` this tick.
    slot_pad_scale: [f32; oag_gameplay::MAX_SHIPS],
    /// What weapons asked of each slot since the last call (a hit a Shield absorbed
    /// included) and the weapon of the first: the `hit` event's buffer, raised after
    /// the call like `walls`.
    hit_amount: [f32; oag_gameplay::MAX_SHIPS],
    hit_weapon: [Option<oag_tables::weapons::Weapon>; oag_gameplay::MAX_SHIPS],
    /// One bit per slot a `give-shield` started this tick, for the shell's rise
    /// (an output, never hashed).
    pub(super) gave: u32,
    /// The speed pad and the weapon pad each slot entered since the last call.
    speed_entered: [Option<u16>; oag_gameplay::MAX_SHIPS],
    weapon_entered: [Option<u16>; oag_gameplay::MAX_SHIPS],
    /// The triggers each slot is inside now, and was inside at the last call: the
    /// difference is the `trigger-enter` and `trigger-exit` events.
    inside: [u64; oag_gameplay::MAX_SHIPS],
    reported: [u64; oag_gameplay::MAX_SHIPS],
    /// Each slot's `prefer-route` this tick (`super::route`), held for the step.
    pub(super) prefer: super::route::Preferred,
    /// Outputs, never hashed: this tick's `hud-*` commands, and the
    /// `fire-effect`s the view has still to draw.
    pub(super) hud: Vec<(usize, Command)>,
    /// Each module's own text, in the race's language.
    strings: Vec<super::Strings>,
    /// Each module's `[pickup]`, in module order.
    pickups: Vec<Option<oag_tables::pickup_look::PickupLook>>,
    pub(super) effects: Vec<(usize, crate::extension_effects::Fired)>,
    /// The `play-mod-sound` loops running, so a fault can stop them.
    pub(super) sound_loops: super::sound::Loops,
    /// Each slot's `screen-fill` this tick (the later module wins) and `fov-offset` this
    /// tick (modules add): outputs, never hashed.
    /// A fill's shader is the sending module and the place in its `[shaders]`.
    pub(super) fill: [Option<(oag_ext_abi::Rgba, Option<(usize, u16)>)>; oag_gameplay::MAX_SHIPS],
    pub(super) fov: [f32; oag_gameplay::MAX_SHIPS],
    /// This tick's `draw-beam`s and `set-shader-param`s: outputs, never hashed.
    pub(super) beams: Vec<(usize, super::Beam)>,
    pub(super) shader_params: Vec<ShaderParam>,
    /// Events dropped past a cap, counted for the report: an output, never hashed.
    pub(super) drops: Drops,
    /// The race-wide outputs of this tick.
    pub(super) sky: super::Sky,
    /// The `[[pickup.odds]]` the pads draw with: each declaring module, its rows, the title.
    odds: Vec<(usize, Vec<oag_tables::pickup_look::OddsRow>, String)>,
    /// The race's title key, written into every observation.
    title: u32,
    /// The slots whose `transit` ended this tick, before the call: their `transit-end`
    /// is in this call's events.
    pub(super) transit_ends: u32,
    /// This call's `transit` and `probe-clearance` commands with their module's index,
    /// for `Race` to run once the call is done.
    pub(super) motion: Vec<(usize, Command)>,
    /// Each slot's `probe-clearance` answer (fits, units ahead), for the next call's
    /// `clearance` event: race state, hashed.
    pub(super) clearance: [Option<(bool, f32)>; oag_gameplay::MAX_SHIPS],
}

/// A `set-shader-param` of this tick.
#[derive(Debug, Clone, Copy)]
pub(super) struct ShaderParam {
    pub(super) module: usize,
    pub(super) slot: usize,
    pub(super) shader: u16,
    pub(super) index: u8,
    pub(super) value: f32,
}

impl On {
    /// The mod module `module` belongs to.
    pub(super) fn module_id(&self, module: usize) -> &str {
        &self.slots[module].mod_id
    }

    /// The `hud-*` commands of `slot` as the HUD's own vocabulary. A later command for
    /// the same element replaces an earlier one (modules run in id order).
    #[cfg(feature = "view")]
    pub(super) fn hud_outputs(&self, slot: usize, tick: u64) -> oag_hud::ext_draws::Outputs {
        use oag_ext_abi::{Anchor as A, HudElement as E, Rgba};
        use oag_hud::ext_draws::{Anchor, BarExtra, Element, Item, Outputs, What};
        let element = |e: E| match e {
            E::Speed => Element::Speed,
            E::Thrust => Element::Thrust,
            E::Energy => Element::Energy,
            E::Pickup => Element::Pickup,
            E::Position => Element::Position,
            E::Lap => Element::Lap,
            E::Time => Element::Time,
        };
        let index = |e: E| element(e) as usize;
        let colour = |Rgba { r, g, b, a }: Rgba| [r, g, b, a].map(|c| f32::from(c) / 255.0);
        let anchor = |a: A| match a {
            A::TopLeft => Anchor::TopLeft,
            A::Top => Anchor::Top,
            A::TopRight => Anchor::TopRight,
            A::Left => Anchor::Left,
            A::Center => Anchor::Center,
            A::Right => Anchor::Right,
            A::BottomLeft => Anchor::BottomLeft,
            A::Bottom => Anchor::Bottom,
            A::BottomRight => Anchor::BottomRight,
        };
        let mut out = Outputs {
            tick,
            slot: slot as u8,
            ..Outputs::default()
        };
        for &(module, command) in self
            .hud
            .iter()
            .filter(|(_, c)| c.slot().map(|s| s.index()) == Some(slot))
        {
            match command {
                Command::HudTint {
                    element: e, rgba, ..
                } => out.tint[index(e)] = Some(colour(rgba)),
                Command::PickupLook { .. } => {
                    out.pickup_look =
                        crate::extension_pickup::from_command(&command, &self.slots[module].mod_id);
                }
                Command::HudShake {
                    element: e,
                    amplitude,
                    ..
                } => out.shake[index(e)] = amplitude,
                Command::HudBarExtra {
                    element: e,
                    extra,
                    rgba,
                    ..
                } => {
                    out.bar_extra[index(e)] = Some(BarExtra {
                        extra,
                        color: colour(rgba),
                    });
                }
                Command::HudText {
                    place,
                    rgba,
                    text,
                    own,
                    ..
                } => {
                    let shown = if own {
                        super::resolve_string(self.strings.get(module), text.as_str())
                    } else {
                        Some(text.as_str().to_owned())
                    };
                    out.items.extend(shown.map(|shown| Item {
                        anchor: anchor(place.anchor),
                        x: f32::from(place.x),
                        y: f32::from(place.y),
                        scale: place.scale(),
                        color: colour(rgba),
                        what: What::Text(shown),
                    }));
                }
                Command::HudIcon {
                    place,
                    rgba,
                    sprite,
                    own,
                    ..
                } => out.items.push(Item {
                    anchor: anchor(place.anchor),
                    x: f32::from(place.x),
                    y: f32::from(place.y),
                    scale: place.scale(),
                    color: colour(rgba),
                    what: if own {
                        What::ModIcon(super::mod_icon_src(
                            &self.slots[module].mod_id,
                            sprite.as_str(),
                        ))
                    } else {
                        What::Icon(sprite.as_str().to_owned())
                    },
                }),
                _ => {}
            }
        }
        out
    }

    /// The look of `slot`'s mod pickup of `kind` this tick ([`crate::extension_pickup::resolve`]).
    pub(super) fn pickup_look(
        &self,
        slot: usize,
        kind: oag_tables::weapons::ModKind,
    ) -> Option<oag_tables::pickup_look::PickupLook> {
        let module = *self.kind_module.get(kind.index())?;
        crate::extension_pickup::resolve(&self.hud, slot, module, &self.pickups, |m| {
            &self.slots[m].mod_id
        })
    }

    /// The race's title key, for every observation.
    pub(super) fn with_title(mut self, title: u32) -> Self {
        self.title = title;
        self
    }

    pub(super) fn new(
        modules: Vec<(String, Module)>,
        region: super::track::Region,
        strings: Vec<super::Strings>,
        pickups: Vec<Option<oag_tables::pickup_look::PickupLook>>,
        drives_ai: Vec<bool>,
        publishes: Vec<Vec<(u32, String)>>,
        odds: Vec<(usize, Vec<oag_tables::pickup_look::OddsRow>, String)>,
    ) -> Self {
        let mut kind_module = Vec::new();
        let module_kind = pickups
            .iter()
            .enumerate()
            .map(|(module, look)| {
                let kind = look
                    .as_ref()
                    .and_then(|_| oag_tables::weapons::ModKind::new(kind_module.len()))?;
                kind_module.push(module);
                Some(kind)
            })
            .collect();
        let modules_len = modules.len();
        Self {
            title: 0,
            module_kind,
            kind_module,
            odds,
            slots: modules
                .into_iter()
                .zip(publishes)
                .zip(drives_ai)
                .map(|(((mod_id, module), publishes), drives_ai_pickup)| Slot {
                    blob: vec![0; module.limits().state_bytes as usize],
                    sender: oag_ext_abi::channel_key(&mod_id),
                    publishes,
                    drives_ai_pickup,
                    mod_id,
                    module,
                    faulted: None,
                    fuel_used: 0,
                    fuel_peak: 0,
                    call_us: 0.0,
                })
                .collect(),
            walls: 0,
            mail: Vec::new(),
            hit_amount: [0.0; oag_gameplay::MAX_SHIPS],
            hit_weapon: [None; oag_gameplay::MAX_SHIPS],
            gave: 0,
            boost: [1.0; oag_gameplay::MAX_SHIPS],
            roll: vec![None; modules_len],
            rolled: vec![0; modules_len],
            from_roll: 0,
            fires: [0; oag_tables::weapons::ModKind::MAX],
            absorbs: [0; oag_tables::weapons::ModKind::MAX],
            pad_scale: vec![1.0; region.speed_pads],
            pad_by: vec![0; region.speed_pads],
            region,
            slot_pad_scale: [1.0; oag_gameplay::MAX_SHIPS],
            speed_entered: [None; oag_gameplay::MAX_SHIPS],
            weapon_entered: [None; oag_gameplay::MAX_SHIPS],
            inside: [0; oag_gameplay::MAX_SHIPS],
            reported: [0; oag_gameplay::MAX_SHIPS],
            prefer: super::route::Preferred::default(),
            hud: Vec::new(),
            strings,
            pickups,
            effects: Vec::new(),
            beams: Vec::new(),
            shader_params: Vec::new(),
            sound_loops: Vec::new(),
            fill: [None; oag_gameplay::MAX_SHIPS],
            fov: [0.0; oag_gameplay::MAX_SHIPS],
            drops: Drops::default(),
            sky: super::Sky::default(),
            transit_ends: 0,
            motion: Vec::new(),
            clearance: [None; oag_gameplay::MAX_SHIPS],
        }
    }

    #[cfg(test)]
    pub(super) fn slot_blob(&self, index: usize) -> Vec<u8> {
        self.slots[index].blob.clone()
    }

    /// One row per module that declares odds for `class`, in kind order, for the pad's
    /// draw: not a module that has faulted.
    pub(super) fn pickup_odds(
        &self,
        class: &str,
    ) -> Vec<(oag_tables::weapons::Weapon, oag_tables::weapons::PickupOdds)> {
        self.odds
            .iter()
            .filter(|(module, _, _)| self.slots[*module].faulted.is_none())
            .filter_map(|(module, rows, title)| {
                let kind = self.module_kind[*module]?;
                let odds = oag_tables::pickup_look::odds_for(rows, title, class)?;
                Some((oag_tables::weapons::Weapon::Extension(kind), odds))
            })
            .collect()
    }

    /// The first module, in id order, whose `pickup-roll` turns a grant of `weapon` into
    /// its own pickup. Every rolling module counts a grant of the weapon it rolls on, up to
    /// the one that turns it.
    pub(super) fn rolls(
        &mut self,
        slot: usize,
        weapon: oag_tables::weapons::Weapon,
    ) -> Option<oag_tables::weapons::ModKind> {
        for module in 0..self.roll.len() {
            let Some((rolled, every)) = self.roll[module] else {
                continue;
            };
            if weapon != rolled {
                continue;
            }
            self.rolled[module] += 1;
            if self.rolled[module].is_multiple_of(u32::from(every)) {
                self.from_roll |= 1 << slot;
                return self.module_kind[module];
            }
        }
        None
    }

    pub(super) fn pad_factor(&self, slot: usize, pad: usize) -> f32 {
        if self.slots.is_empty() {
            return 1.0;
        }
        self.pad_scale.get(pad).copied().unwrap_or(1.0) * self.slot_pad_scale[slot]
    }

    pub(super) fn pad_entered(&mut self, slot: usize, pad: usize, weapon: bool) {
        if self.slots.is_empty() {
            return;
        }
        let entered = if weapon {
            &mut self.weapon_entered
        } else {
            &mut self.speed_entered
        };
        entered[slot] = u16::try_from(pad).ok();
    }

    pub(super) fn test_triggers(&mut self, slot: usize, sweep: &[oag_core::math::Vec3]) {
        if self.slots.is_empty() || self.region.triggers.is_empty() {
            return;
        }
        self.inside[slot] = self
            .region
            .triggers
            .iter()
            .enumerate()
            .filter(|(_, b)| sweep.iter().any(|p| b.contains(p.to_array())))
            .fold(0, |mask, (i, _)| mask | 1 << i);
    }

    /// The module of the pickup `kind` decides when an AI craft uses it: the engine
    /// raises no `pickup-fire` for one.
    pub(super) fn guest_drives_ai_pickup(&self, kind: oag_tables::weapons::ModKind) -> bool {
        self.kind_module
            .get(kind.index())
            .and_then(|module| self.slots.get(*module))
            .is_some_and(|slot| slot.drives_ai_pickup)
    }

    /// Queues module `module`'s own effect `index` for the view at `at`.
    pub(super) fn fire_effect_at(
        &mut self,
        module: usize,
        slot: usize,
        index: u16,
        at: oag_core::math::Vec3,
        up: oag_core::math::Vec3,
    ) {
        let mod_id = self.slots[module].mod_id.clone();
        self.effects.push((
            slot,
            crate::extension_effects::Fired::OwnAt {
                mod_id,
                index,
                at,
                up,
            },
        ));
    }

    pub(super) fn pickup_used(
        &mut self,
        slot: usize,
        kind: oag_tables::weapons::ModKind,
        absorbed: bool,
    ) {
        let raised = if absorbed {
            &mut self.absorbs
        } else {
            &mut self.fires
        };
        raised[kind.index()] |= 1 << slot;
    }

    /// Folds a step's weapon hits into the `hit` event buffer: per slot the amounts
    /// sum and the first weapon stays. A hit with no weapon (nothing asked) adds
    /// nothing.
    pub(super) fn weapon_hits(&mut self, hits: &[oag_weapons::projectile::WeaponHit]) {
        if self.slots.is_empty() {
            return;
        }
        for (slot, hit) in hits.iter().enumerate().take(oag_gameplay::MAX_SHIPS) {
            if let Some(weapon) = hit.weapon.filter(|_| hit.amount > 0.0) {
                self.hit_amount[slot] += hit.amount;
                self.hit_weapon[slot].get_or_insert(weapon);
            }
        }
    }

    /// A craft entered a trigger that sends `channel`: queued for the next call, from
    /// the track.
    pub(super) fn mail_from_track(&mut self, slot: usize, channel: u32) {
        if !self.slots.is_empty() {
            self.mail.push(super::channel::Mail {
                slot: slot as u8,
                from: oag_ext_abi::TRACK_SENDER,
                channel,
                value: 1.0,
            });
        }
    }

    pub(super) fn wall_contact(&mut self, slot: usize) {
        if !self.slots.is_empty() {
            self.walls |= 1 << slot;
        }
    }

    pub(super) fn hash_into(&self, hasher: &mut oag_core::hash::StateHasher) {
        if self.slots.is_empty() {
            return;
        }
        hasher.write_u32(self.slots.len() as u32);
        hasher.write_u32(self.walls);
        super::channel::hash_mail(&self.mail, hasher);
        for (amount, weapon) in self.hit_amount.iter().zip(&self.hit_weapon) {
            hasher.write_f32(*amount);
            hasher.write_u8(weapon.map_or(0, |weapon| weapon.index() as u8 + 1));
        }
        for boost in self.boost {
            hasher.write_f32(boost);
        }
        for (roll, rolled) in self.roll.iter().zip(&self.rolled) {
            match roll {
                None => hasher.write_u8(0),
                Some((weapon, every)) => {
                    hasher.write_u8(weapon.index() as u8 + 1);
                    hasher.write_u8(*every);
                }
            }
            hasher.write_u32(*rolled);
        }
        hasher.write_u32(self.from_roll);
        for kind in 0..self.kind_module.len() {
            hasher.write_u32(self.fires[kind]);
            hasher.write_u32(self.absorbs[kind]);
        }
        for scale in self.pad_scale.iter().chain(&self.slot_pad_scale) {
            hasher.write_f32(*scale);
        }
        hasher.write(&self.pad_by);
        for entered in self.speed_entered.iter().chain(&self.weapon_entered) {
            hasher.write_u32(entered.map_or(0, |pad| u32::from(pad) + 1));
        }
        for mask in self.inside.iter().chain(&self.reported) {
            hasher.write_u64(*mask);
        }
        self.prefer.hash_into(hasher);
        for answer in self.clearance {
            match answer {
                None => hasher.write_u8(0),
                Some((fits, ahead)) => {
                    hasher.write_u8(1 + u8::from(fits));
                    hasher.write_f32(ahead);
                }
            }
        }
        for slot in &self.slots {
            hasher.write(&slot.blob);
            match &slot.faulted {
                None => hasher.write_u8(0),
                Some((tick, _)) => {
                    hasher.write_u8(1);
                    hasher.write_u64(*tick);
                }
            }
        }
    }

    pub(super) fn reports(&self) -> Vec<Report> {
        self.slots
            .iter()
            .map(|slot| Report {
                mod_id: slot.mod_id.clone(),
                fuel_used: slot.fuel_used,
                fuel_peak: slot.fuel_peak,
                fuel_budget: slot.module.limits().fuel,
                call_us: slot.call_us,
                faulted: slot.faulted.clone(),
            })
            .collect()
    }
}

/// `play-cue`'s sounds as the engine's cues.
fn cue(cue: oag_ext_abi::Cue) -> oag_gameplay::cue::Cue {
    use oag_ext_abi::Cue as Abi;
    use oag_gameplay::cue::Cue;
    match cue {
        Abi::SpeedupPad => Cue::SpeedupPad,
        Abi::Turbo => Cue::Turbo,
        Abi::Collision => Cue::Collision,
        Abi::Absorb => Cue::Absorb,
        Abi::Shield => Cue::Shield,
        Abi::ShieldActive => Cue::ShieldActive,
    }
}

/// `fire-effect`'s effects as the title's triggers.
fn effect(effect: oag_ext_abi::Effect) -> oag_title::effects::Trigger {
    use oag_title::effects::Trigger;
    match effect {
        oag_ext_abi::Effect::CollisionSpark => Trigger::CollisionSpark,
        oag_ext_abi::Effect::HitSpark => Trigger::HitSpark,
    }
}

/// The observation's craft records: copies of the world, never a pointer into it.
fn crafts(sim: &RaceSim, inputs: &oag_gameplay::PlayerInputs) -> Vec<oag_ext_abi::Craft> {
    sim.world
        .ships
        .iter()
        .enumerate()
        .map(|(slot, ship)| {
            let person = sim.world.person_flown(slot);
            let fire_held = person
                && inputs
                    .get(slot)
                    .buttons
                    .is_held(oag_gameplay::input::Button::Square);
            // Racing and past the start: a craft on the grid has no thrust yet.
            let racing = ship.physics.craft_state == oag_physics::damage::CraftState::Racing
                && !oag_race::RaceState::thrust_gated(sim.world.tick);
            oag_ext_abi::Craft {
                pickup: ship.pickup.weapon.map_or(Pickup::None, pickup),
                person,
                fire_held,
                racing,
                shielded: ship.physics.shield_pickup_timer > 0.0,
                in_transit: ship.transit.is_some(),
                lap: u8::try_from(ship.standing.lap).unwrap_or(u8::MAX),
                laps_target: sim
                    .world
                    .laps_target()
                    .map_or(0, |n| u8::try_from(n).unwrap_or(u8::MAX)),
                shield: ship.physics.shield,
                shield_max: ship.handling.dimensions.shield,
                speed_kmh: ship.physics.body.linear_velocity.length()
                    * oag_core::math::SPEED_TO_KMH,
            }
        })
        .collect()
}

/// Where each craft sits in each other craft's own frame: the dot of the offset between
/// their bodies with the observer's forward, right and up axes, in `f32`. A slot with no craft
/// has no row and no column.
fn aiming(sim: &RaceSim) -> oag_ext_abi::Aiming {
    let mut aiming = oag_ext_abi::Aiming::default();
    let ships = &sim.world.ships;
    for (observer, from) in ships.iter().enumerate().filter(|(_, ship)| ship.active) {
        let body = &from.physics.body;
        let (forward, right, up) = (body.forward(), body.right(), body.up());
        for (target, to) in ships.iter().enumerate().filter(|(_, ship)| ship.active) {
            if target == observer {
                continue;
            }
            let offset = to.physics.body.position - body.position;
            aiming.set(
                observer,
                target,
                oag_ext_abi::Aim::from_offset(
                    offset.dot(forward),
                    offset.dot(right),
                    offset.dot(up),
                ),
            );
        }
    }
    aiming
}

/// Each slot's distance along the circuit and place. The place is the one the HUD shows
/// (`RaceSim::places`); an empty slot has none.
fn tracking(sim: &RaceSim) -> Vec<oag_ext_abi::Tracking> {
    let places = sim.places();
    sim.world
        .ships
        .iter()
        .zip(places)
        .map(|(ship, place)| oag_ext_abi::Tracking {
            progress: ship.standing.progress,
            place: if ship.active { place } else { 0 },
        })
        .collect()
}

/// `give-pickup`, `clear-pickup` and `pickup-roll` from module `index`, already checked
/// by [`refusal`]: each acts on the module's own pickup kind. A race without weapons gives
/// nothing and rolls nothing.
fn pickup_command(sim: &mut RaceSim, index: usize, command: Command) {
    let may = match command {
        Command::ClearPickup { slot } => sim.may_use_pickup(slot.index()),
        _ => true,
    };
    let on = &mut sim.extensions.on;
    let Some(kind) = on.module_kind[index] else {
        return;
    };
    if !sim.weapons_on {
        return;
    }
    let extension = oag_tables::weapons::Weapon::Extension(kind);
    match command {
        // The slot's `last` stays what it was: the repeat rule is about the title's
        // own weapons.
        Command::GivePickup { slot } => {
            let pickup = &mut sim.world.ships[slot.index()].pickup;
            if pickup.weapon.is_none() {
                pickup.weapon = Some(extension);
                on.from_roll &= !(1 << slot.index());
            }
        }
        // A guest never uses a pickup for a craft that may not (`Race::may_use_pickup`).
        Command::ClearPickup { .. } if !may => {}
        Command::ClearPickup { slot } => {
            let pickup = &mut sim.world.ships[slot.index()].pickup;
            if pickup.weapon == Some(extension) {
                pickup.take();
            }
        }
        Command::PickupRoll { weapon, every } => {
            let weapon = crate::extension_pickup::weapon(weapon);
            if on.roll[index] != Some((weapon, every)) {
                on.roll[index] = Some((weapon, every));
                on.rolled[index] = 0;
            }
        }
        _ => {}
    }
}

/// Why module `index`'s commands this tick are refused, if any is: an `emit` past the
/// cap or on a channel its mod does not publish, or a pickup command from a module whose
/// `oag-mod.toml` describes no pickup of its own. Checked over the whole reply before any of it lands, so a refused
/// reply lands nothing (the module's other commands included).
fn refusal(on: &On, index: usize, commands: &[Command]) -> Option<String> {
    let module = &on.slots[index];
    let mut emits = 0;
    for command in commands {
        match command {
            Command::Emit { channel, .. } => {
                emits += 1;
                if emits > oag_ext_abi::MAX_EMITS_PER_TICK {
                    return Some(format!(
                        "emit past the cap of {} a tick",
                        oag_ext_abi::MAX_EMITS_PER_TICK
                    ));
                }
                if !module.publishes.iter().any(|(key, _)| key == channel) {
                    let declared: Vec<&str> =
                        module.publishes.iter().map(|(_, n)| n.as_str()).collect();
                    return Some(format!(
                        "emit on channel {channel:#010x}, which its oag-mod.toml [channels] publishes does not name (it publishes {declared:?})"
                    ));
                }
            }
            Command::GivePickup { .. }
            | Command::ClearPickup { .. }
            | Command::PickupRoll { .. }
                if on.module_kind[index].is_none() =>
            {
                return Some(
                    "it gives, rolls or clears its own pickup but its oag-mod.toml has no \
                     [pickup] table (icon and colour, each naming its source)"
                        .to_string(),
                );
            }
            _ => {}
        }
    }
    None
}

/// Takes back everything module `index` holds for the rest of the race, as it faults:
/// the pads it scaled go back to 1.0, and if it has a mod pickup, the roll ends and
/// the pickups crafts still hold of its kind become the title's again. A pickup that came from a
/// roll is the weapon the pad drew (the craft's `last`), so the AI and the person can
/// use it as if the module had never run; one that was given has no such weapon and is
/// taken away. (Mail the module already sent stays: one tick of events.)
fn release(sim: &mut RaceSim, index: usize) {
    super::sound::release(sim, index);
    let on = &mut sim.extensions.on;
    let me = index as u8 + 1;
    for (scale, by) in on.pad_scale.iter_mut().zip(&mut on.pad_by) {
        if *by == me {
            *scale = 1.0;
            *by = 0;
        }
    }
    let Some(kind) = on.module_kind[index] else {
        return;
    };
    on.roll[index] = None;
    on.rolled[index] = 0;
    let from_roll = on.from_roll;
    let extension = Some(oag_tables::weapons::Weapon::Extension(kind));
    for (slot, ship) in sim.world.ships.iter_mut().enumerate() {
        if ship.pickup.weapon != extension {
            continue;
        }
        on.from_roll &= !(1 << slot);
        // Only a title weapon is handed back: a stale roll bit must not return another mod's
        // kind (drawn by its odds, so the craft's `last`) that the AI could never use up.
        let handed_back = ship
            .pickup
            .last
            .filter(|last| from_roll & (1 << slot) != 0 && last.mod_kind().is_none());
        match handed_back {
            Some(drawn) => ship.pickup.weapon = Some(drawn),
            None => {
                ship.pickup.take();
            }
        }
    }
}

/// What happened since the last call, per slot in slot order: a wall contact, a
/// fire, an absorb, the speed pad and the weapon pad entered, the triggers left and
/// the triggers entered (each in index order). Past `MAX_EVENTS` the rest are
/// dropped. The `mod` events (`mail`, sorted) come first, so a crowd of engine events
/// cannot crowd out a mod's.
fn events(
    on: &On,
    mail: &[super::channel::Mail],
    kind: Option<oag_tables::weapons::ModKind>,
) -> (Vec<Event>, usize) {
    let mut all: Vec<Event> = mail
        .iter()
        .filter_map(|m| m.event())
        .chain(engine_events(on, kind))
        .collect();
    let dropped = all.len().saturating_sub(MAX_EVENTS);
    all.truncate(MAX_EVENTS);
    (all, dropped)
}

fn engine_events(
    on: &On,
    kind: Option<oag_tables::weapons::ModKind>,
) -> impl Iterator<Item = Event> + '_ {
    (0..oag_gameplay::MAX_SHIPS)
        .filter_map(oag_ext_abi::Slot::new)
        .flat_map(move |slot| {
            let i = slot.index();
            let bit = |mask: u32| mask & slot.bit() != 0;
            let (inside, before) = (on.inside[i], on.reported[i]);
            let triggers = |mask: u64| (0..64_u16).filter(move |t| mask & (1 << t) != 0);
            [
                bit(on.transit_ends).then_some(Event::TransitEnd { slot }),
                on.clearance[i].map(|(fits, ahead)| Event::Clearance { slot, fits, ahead }),
                bit(on.walls).then_some(Event::WallContact { slot }),
                kind.filter(|kind| bit(on.fires[kind.index()]))
                    .map(|_| Event::PickupFire { slot }),
                kind.filter(|kind| bit(on.absorbs[kind.index()]))
                    .map(|_| Event::PickupAbsorbed { slot }),
                on.speed_entered[i].map(|pad| Event::SpeedPad { slot, pad }),
                on.weapon_entered[i].map(|pad| Event::WeaponPad { slot, pad }),
                on.hit_weapon[i].and_then(|weapon| match pickup(weapon) {
                    Pickup::Weapon(weapon) => Some(Event::Hit {
                        slot,
                        amount: on.hit_amount[i],
                        weapon,
                    }),
                    _ => None,
                }),
            ]
            .into_iter()
            .flatten()
            .chain(
                triggers(before & !inside).map(move |trigger| Event::TriggerExit { slot, trigger }),
            )
            .chain(
                triggers(inside & !before)
                    .map(move |trigger| Event::TriggerEnter { slot, trigger }),
            )
        })
}

/// A module's `emit` (checked by [`refusal`]): queued for every module's next call.
fn emit(sim: &mut RaceSim, index: usize, (slot, channel, value): (oag_ext_abi::Slot, u32, f32)) {
    let tick = sim.world.tick;
    let on = &mut sim.extensions.on;
    let from = on.slots[index].sender;
    on.mail.push(super::channel::Mail {
        slot: slot.index() as u8,
        from,
        channel,
        value,
    });
    sim.extensions.note_fired(channel, value, tick);
}

/// Whether module `index` is off for the race.
pub(super) fn faulted(sim: &RaceSim, index: usize) -> bool {
    sim.extensions.on.slots[index].faulted.is_some()
}

/// Faults module `index` for a refusal the race found after the call (a `transit` it
/// could not start), taking back what it holds, as a refused reply does.
pub(super) fn fault(sim: &mut RaceSim, index: usize, why: String) {
    let tick = sim.world.tick;
    sim.extensions.on.slots[index].fault(tick, why);
    release(sim, index);
}

pub(super) fn tick(sim: &mut RaceSim, inputs: &oag_gameplay::PlayerInputs, transited: u32) {
    if sim.extensions.on.slots.is_empty() {
        return;
    }
    sim.extensions.on.transit_ends = transited;
    let crafts = crafts(sim, inputs);
    let tracking = tracking(sim);
    let aiming = aiming(sim);
    let tick = sim.world.tick;
    let active = sim
        .world
        .ships
        .iter()
        .enumerate()
        .fold(0_u32, |mask, (slot, ship)| {
            mask | u32::from(ship.active) << slot
        });
    let on = &mut sim.extensions.on;
    // A boost the step never took (a craft that did not step) does not carry over.
    on.boost = [1.0; oag_gameplay::MAX_SHIPS];
    on.slot_pad_scale = [1.0; oag_gameplay::MAX_SHIPS];
    let queued = std::mem::take(&mut on.mail);
    let mail = super::channel::sorted(&queued);
    // Each module's own view: the events of its own pickup kind alone, and the crafts
    // holding its kind as `Pickup::Ours`.
    let mut engine_dropped = 0;
    let views: Vec<(Vec<Event>, Vec<oag_ext_abi::Craft>)> = on
        .module_kind
        .iter()
        .map(|&kind| {
            let (events, dropped) = events(on, &mail, kind);
            engine_dropped = engine_dropped.max(dropped);
            let own = kind.map(|kind| Pickup::Other(kind.index() as u8));
            let crafts = crafts
                .iter()
                .map(|craft| oag_ext_abi::Craft {
                    pickup: if Some(craft.pickup) == own {
                        Pickup::Ours
                    } else {
                        craft.pickup
                    },
                    ..*craft
                })
                .collect();
            (events, crafts)
        })
        .collect();
    let (mods_dropped, track_dropped) = super::channel::dropped(&queued);
    on.drops
        .note(tick, mods_dropped, track_dropped, engine_dropped);
    on.walls = 0;
    on.transit_ends = 0;
    on.clearance = [None; oag_gameplay::MAX_SHIPS];
    on.hit_amount = [0.0; oag_gameplay::MAX_SHIPS];
    on.hit_weapon = [None; oag_gameplay::MAX_SHIPS];
    on.fires = [0; oag_tables::weapons::ModKind::MAX];
    on.absorbs = [0; oag_tables::weapons::ModKind::MAX];
    on.speed_entered = [None; oag_gameplay::MAX_SHIPS];
    on.weapon_entered = [None; oag_gameplay::MAX_SHIPS];
    on.reported = on.inside;
    on.prefer = super::route::Preferred::default();
    on.hud.clear();
    on.sky = super::Sky::default();
    // This tick's alone: a tick that draws nothing (a replayed one, a server's) drops
    // them rather than piling them up.
    on.effects.clear();
    on.fill = [None; oag_gameplay::MAX_SHIPS];
    on.beams.clear();
    on.shader_params.clear();
    on.fov = [0.0; oag_gameplay::MAX_SHIPS];
    let region = on.region.clone();
    let course_length = sim.course.as_ref().map_or(0.0, oag_race::Course::length);
    let title = on.title;
    let mut commands = Vec::new();
    // Every module's checked commands, in id order, with the blob it had before the call:
    // applied once all have run, so each saw the same observation.
    let mut applied = Vec::new();
    let mut faulted = Vec::new();
    for (index, slot) in on.slots.iter_mut().enumerate() {
        if slot.faulted.is_some() {
            slot.fuel_used = 0;
            continue;
        }
        let prior = slot.blob.clone();
        let (events, crafts) = &views[index];
        let observation = Observation {
            tick,
            active,
            events,
            crafts,
            course: oag_ext_abi::Course {
                length: course_length,
                tracking: &tracking,
                title,
                aiming: Some(&aiming),
            },
            track: &region.bytes,
            speed_pads: region.speed_pads,
            routes: region.routes,
        };
        // Timed for the overlay only; nothing simulated reads it.
        let started = web_time::Instant::now();
        let outcome = slot
            .module
            .tick(&observation, &mut slot.blob, &mut commands);
        slot.call_us = started.elapsed().as_secs_f32() * 1.0e6;
        let cost = match outcome {
            Ok(cost) => {
                applied.push((index, prior, std::mem::take(&mut commands)));
                cost
            }
            Err((fault, cost)) => {
                slot.fault(tick, fault.to_string());
                faulted.push(index);
                cost
            }
        };
        slot.fuel_used = cost.fuel_used;
        slot.fuel_peak = slot.fuel_peak.max(cost.fuel_used);
    }
    for index in faulted {
        release(sim, index);
    }
    for (index, prior, commands) in applied {
        if let Some(why) = refusal(&sim.extensions.on, index, &commands) {
            let slot = &mut sim.extensions.on.slots[index];
            slot.fault(tick, why);
            // The faulted call's new blob is not kept, as for a fault in the call itself.
            slot.blob = prior;
            release(sim, index);
            continue;
        }
        for command in commands {
            apply(sim, index, command);
        }
    }
}

fn apply(sim: &mut RaceSim, index: usize, command: Command) {
    match command {
        Command::Emit {
            slot,
            channel,
            value,
        } => emit(sim, index, (slot, channel, value)),
        // Below 1.0 the Leach Beam's path, multiplied so a drain is kept; above it
        // the mod's own engine scale, multiplied so two modules compose. Both are
        // consumed by this tick's step.
        Command::ScaleThrust { slot, factor } if factor <= 1.0 => {
            sim.world.ships[slot.index()].pending_thrust_scale *= factor;
        }
        Command::ScaleThrust { slot, factor } => {
            let boost = &mut sim.extensions.on.boost[slot.index()];
            *boost = (*boost * factor).min(oag_ext_abi::MAX_THRUST_FACTOR);
        }
        Command::ShieldDelta { slot, amount } => super::shield::delta(sim, slot.index(), amount),
        // Race-wide and held until replaced (the later module in id order wins) or its
        // module faults, which puts it back.
        Command::ScalePad { pad, factor } => {
            let on = &mut sim.extensions.on;
            on.pad_scale[usize::from(pad)] = factor;
            on.pad_by[usize::from(pad)] = index as u8 + 1;
        }
        // Per slot, this tick: two modules compose, bounded as a factor is.
        Command::ScaleSlotPads { slot, factor } => {
            let scale = &mut sim.extensions.on.slot_pad_scale[slot.index()];
            *scale = (*scale * factor).min(oag_ext_abi::MAX_PAD_FACTOR);
        }
        Command::PlayCue { slot, cue } => {
            sim.cues.push(oag_gameplay::cue::CueEvent::new(
                self::cue(cue),
                slot.index(),
            ));
        }
        Command::PlayModSound {
            slot,
            index: sound,
            mode,
        } => {
            super::sound::apply(sim, index, slot.index() as u8, sound, mode);
        }
        Command::FireEffect { slot, effect } => sim
            .extensions
            .on
            .effects
            .push((slot.index(), Fired::Title(self::effect(effect)))),
        Command::FireModEffect { slot, index: own } => {
            let mod_id = sim.extensions.on.slots[index].mod_id.clone();
            sim.extensions
                .on
                .effects
                .push((slot.index(), Fired::Own { mod_id, index: own }));
        }
        Command::ScreenFill { slot, rgba, shader } => {
            sim.extensions.on.fill[slot.index()] =
                Some((rgba, shader.map(|shader| (index, shader))));
        }
        Command::DrawBeam {
            slot,
            to,
            rgba,
            half_width,
            shader,
        } => sim.extensions.on.beams.push((
            index,
            super::Beam {
                from: slot.index(),
                to: to.index(),
                rgba: super::unit(rgba),
                half_width,
                shader,
            },
        )),
        Command::SetShaderParam {
            slot,
            shader,
            index: scalar,
            value,
        } => sim.extensions.on.shader_params.push(ShaderParam {
            module: index,
            slot: slot.index(),
            shader,
            index: scalar,
            value,
        }),
        Command::FovOffset { slot, degrees } => {
            let fov = &mut sim.extensions.on.fov[slot.index()];
            *fov = (*fov + degrees).clamp(
                -oag_ext_abi::MAX_FOV_OFFSET_DEG,
                oag_ext_abi::MAX_FOV_OFFSET_DEG,
            );
        }
        Command::PickupLook { .. }
        | Command::HudTint { .. }
        | Command::HudShake { .. }
        | Command::HudBarExtra { .. }
        | Command::HudText { .. }
        | Command::HudIcon { .. } => sim.extensions.on.hud.push((index, command)),
        Command::SetWeather { strength, .. } => {
            sim.extensions.on.sky.rain = Some(strength);
        }
        Command::SetSun { intensity } => sim.extensions.on.sky.sun = intensity,
        Command::GradeSky { grey } => sim.extensions.on.sky.grey = grey,
        Command::GiveShield { slot } => super::shield::give(sim, slot.index()),
        Command::HoldShield { slot } => super::shield::hold(sim, slot.index()),
        Command::ClearShield { slot } => {
            sim.world.ships[slot.index()].physics.shield_pickup_timer = 0.0;
        }
        Command::GivePickup { .. } | Command::ClearPickup { .. } | Command::PickupRoll { .. } => {
            pickup_command(sim, index, command);
        }
        Command::PreferRoute { slot, route } => {
            sim.extensions.on.prefer.set(slot.index(), route);
        }
        // Once, at the call: two modules compose.
        Command::ScaleSpeed { slot, .. } if !sim.may_use_pickup(slot.index()) => {}
        Command::ScaleSpeed { slot, factor } => {
            sim.world.ships[slot.index()].physics.body.linear_velocity *= factor;
        }
        // Run by `Race` once every module's commands have landed (`crate::transit`).
        Command::Transit { .. }
        | Command::ProbeClearance { .. }
        | Command::FireModEffectAhead { .. } => {
            sim.extensions.on.motion.push((index, command));
        }
    }
}

/// The kind the `held`-th pickup-declaring module gets, or why there is none: a race has at
/// most [`oag_tables::weapons::ModKind::MAX`] mod pickups.
pub(super) fn pickup_kind(held: usize) -> Result<oag_tables::weapons::ModKind, String> {
    oag_tables::weapons::ModKind::new(held).ok_or_else(|| {
        format!(
            "a race runs at most {} mod pickups",
            oag_tables::weapons::ModKind::MAX
        )
    })
}
