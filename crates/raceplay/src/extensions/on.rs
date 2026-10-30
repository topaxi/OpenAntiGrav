use oag_ext_abi::{Command, Event, MAX_EVENTS, Pickup};

use crate::extension_effects::Fired;
use oag_modhost::{Module, Observation};

use super::Report;
use crate::extension_pickup::pickup;
use crate::sim::RaceSim;

#[derive(Debug, Clone)]
pub(super) struct Slot {
    mod_id: String,
    module: Module,
    pub(super) blob: Vec<u8>,
    faulted: Option<(u64, String)>,
    fuel_used: u64,
    fuel_peak: u64,
    call_us: f32,
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
    /// The module that owns the mod pickup kind (its index plus one, 0 for none):
    /// the first to give, clear or roll it. Another module doing so faults.
    owner: u8,
    /// The `pickup-roll` rule (weapon, every) and how many grants of that weapon
    /// it has counted.
    roll: Option<(oag_tables::weapons::Weapon, u8)>,
    rolled: u32,
    /// `pickup-fire` / `pickup-absorbed` raised since the last call, one bit a slot.
    fires: u32,
    absorbs: u32,
    /// The track as the modules read it.
    region: super::track::Region,
    /// Each speed pad's race-wide `scale-pad`, held until replaced.
    pad_scale: Vec<f32>,
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
    /// The race-wide outputs of this tick.
    pub(super) sky: super::Sky,
}

impl On {
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

    /// The look of `slot`'s mod pickup this tick ([`crate::extension_pickup::resolve`]).
    pub(super) fn pickup_look(&self, slot: usize) -> Option<oag_tables::pickup_look::PickupLook> {
        crate::extension_pickup::resolve(&self.hud, slot, self.owner, &self.pickups, |m| {
            &self.slots[m].mod_id
        })
    }

    pub(super) fn new(
        modules: Vec<(String, Module)>,
        region: super::track::Region,
        strings: Vec<super::Strings>,
        pickups: Vec<Option<oag_tables::pickup_look::PickupLook>>,
    ) -> Self {
        Self {
            slots: modules
                .into_iter()
                .map(|(mod_id, module)| Slot {
                    blob: vec![0; module.limits().state_bytes as usize],
                    mod_id,
                    module,
                    faulted: None,
                    fuel_used: 0,
                    fuel_peak: 0,
                    call_us: 0.0,
                })
                .collect(),
            walls: 0,
            hit_amount: [0.0; oag_gameplay::MAX_SHIPS],
            hit_weapon: [None; oag_gameplay::MAX_SHIPS],
            gave: 0,
            boost: [1.0; oag_gameplay::MAX_SHIPS],
            owner: 0,
            roll: None,
            rolled: 0,
            fires: 0,
            absorbs: 0,
            pad_scale: vec![1.0; region.speed_pads],
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
            sky: super::Sky::default(),
        }
    }

    #[cfg(test)]
    pub(super) fn slot_blob(&self, index: usize) -> Vec<u8> {
        self.slots[index].blob.clone()
    }

    pub(super) fn rolls(&mut self, weapon: oag_tables::weapons::Weapon) -> bool {
        let Some((rolled, every)) = self.roll else {
            return false;
        };
        if weapon != rolled {
            return false;
        }
        self.rolled += 1;
        self.rolled.is_multiple_of(u32::from(every))
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

    pub(super) fn pickup_used(&mut self, slot: usize, absorbed: bool) {
        if absorbed {
            self.absorbs |= 1 << slot;
        } else {
            self.fires |= 1 << slot;
        }
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
        for (amount, weapon) in self.hit_amount.iter().zip(&self.hit_weapon) {
            hasher.write_f32(*amount);
            hasher.write_u8(weapon.map_or(0, |weapon| weapon as u8 + 1));
        }
        for boost in self.boost {
            hasher.write_f32(boost);
        }
        hasher.write_u8(self.owner);
        match self.roll {
            None => hasher.write_u8(0),
            Some((weapon, every)) => {
                hasher.write_u8(weapon as u8 + 1);
                hasher.write_u8(every);
            }
        }
        hasher.write_u32(self.rolled);
        hasher.write_u32(self.fires);
        hasher.write_u32(self.absorbs);
        for scale in self.pad_scale.iter().chain(&self.slot_pad_scale) {
            hasher.write_f32(*scale);
        }
        for entered in self.speed_entered.iter().chain(&self.weapon_entered) {
            hasher.write_u32(entered.map_or(0, |pad| u32::from(pad) + 1));
        }
        for mask in self.inside.iter().chain(&self.reported) {
            hasher.write_u64(*mask);
        }
        self.prefer.hash_into(hasher);
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

/// `give-pickup`, `clear-pickup` and `pickup-roll` from module `index`: the first
/// module to send one owns the mod pickup kind for the race, and another one sending
/// one faults. A race without weapons gives nothing and rolls nothing.
fn pickup_command(sim: &mut RaceSim, index: usize, command: Command) {
    let tick = sim.world.tick;
    let on = &mut sim.extensions.on;
    let me = index as u8 + 1;
    if on.owner != 0 && on.owner != me {
        let owner = on.slots[usize::from(on.owner - 1)].mod_id.clone();
        let module = &mut on.slots[index];
        if module.faulted.is_none() {
            let why = format!("the mod pickup belongs to mod {owner}");
            log::warn!(
                "mod {}: {why}, at tick {tick}; off for the rest of the race",
                module.mod_id
            );
            module.faulted = Some((tick, why));
        }
        return;
    }
    let hands_out = matches!(
        command,
        Command::GivePickup { .. } | Command::PickupRoll { .. }
    );
    if hands_out && on.pickups[index].is_none() {
        let module = &mut on.slots[index];
        if module.faulted.is_none() {
            let why = "it hands out its own pickup but its oag-mod.toml has no [pickup] table \
                       (icon and colour, each naming its source)";
            log::warn!(
                "mod {}: {why}, at tick {tick}; off for the rest of the race",
                module.mod_id
            );
            module.faulted = Some((tick, why.to_string()));
        }
        return;
    }
    on.owner = me;
    if !sim.weapons_on {
        return;
    }
    let extension = oag_tables::weapons::Weapon::Extension;
    match command {
        // The slot's `last` stays what it was: the repeat rule is about the title's
        // own weapons.
        Command::GivePickup { slot } => {
            let pickup = &mut sim.world.ships[slot.index()].pickup;
            if pickup.weapon.is_none() {
                pickup.weapon = Some(extension);
            }
        }
        Command::ClearPickup { slot } => {
            let pickup = &mut sim.world.ships[slot.index()].pickup;
            if pickup.weapon == Some(extension) {
                pickup.take();
            }
        }
        Command::PickupRoll { weapon, every } => {
            let weapon = crate::extension_pickup::weapon(weapon);
            let on = &mut sim.extensions.on;
            if on.roll != Some((weapon, every)) {
                on.roll = Some((weapon, every));
                on.rolled = 0;
            }
        }
        _ => {}
    }
}

/// What happened since the last call, per slot in slot order: a wall contact, a
/// fire, an absorb, the speed pad and the weapon pad entered, the triggers left and
/// the triggers entered (each in index order). Past `MAX_EVENTS` the rest are
/// dropped.
fn events(on: &On) -> Vec<Event> {
    (0..oag_gameplay::MAX_SHIPS)
        .filter_map(oag_ext_abi::Slot::new)
        .flat_map(|slot| {
            let i = slot.index();
            let bit = |mask: u32| mask & slot.bit() != 0;
            let (inside, before) = (on.inside[i], on.reported[i]);
            let triggers = |mask: u64| (0..64_u16).filter(move |t| mask & (1 << t) != 0);
            [
                bit(on.walls).then_some(Event::WallContact { slot }),
                bit(on.fires).then_some(Event::PickupFire { slot }),
                bit(on.absorbs).then_some(Event::PickupAbsorbed { slot }),
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
        .take(MAX_EVENTS)
        .collect()
}

pub(super) fn tick(sim: &mut RaceSim, inputs: &oag_gameplay::PlayerInputs) {
    if sim.extensions.on.slots.is_empty() {
        return;
    }
    let crafts = crafts(sim, inputs);
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
    let events = events(on);
    on.walls = 0;
    on.hit_amount = [0.0; oag_gameplay::MAX_SHIPS];
    on.hit_weapon = [None; oag_gameplay::MAX_SHIPS];
    on.fires = 0;
    on.absorbs = 0;
    on.speed_entered = [None; oag_gameplay::MAX_SHIPS];
    on.weapon_entered = [None; oag_gameplay::MAX_SHIPS];
    on.reported = on.inside;
    on.prefer = super::route::Preferred::default();
    on.hud.clear();
    on.sky = super::Sky::default();
    // This tick's alone: a tick that draws nothing (a replayed one, a server's) drops
    // them rather than piling them up.
    on.effects.clear();
    let region = on.region.clone();
    let observation = Observation {
        tick,
        active,
        events: &events,
        crafts: &crafts,
        track: &region.bytes,
        speed_pads: region.speed_pads,
        routes: region.routes,
    };
    let mut commands = Vec::new();
    // Every module's checked commands, in id order: applied once all have run, so
    // each saw the same observation.
    let mut applied = Vec::new();
    for (index, slot) in on.slots.iter_mut().enumerate() {
        if slot.faulted.is_some() {
            slot.fuel_used = 0;
            continue;
        }
        // Timed for the overlay only; nothing simulated reads it.
        let started = web_time::Instant::now();
        let outcome = slot
            .module
            .tick(&observation, &mut slot.blob, &mut commands);
        slot.call_us = started.elapsed().as_secs_f32() * 1.0e6;
        let cost = match outcome {
            Ok(cost) => {
                applied.extend(commands.drain(..).map(|command| (index, command)));
                cost
            }
            Err((fault, cost)) => {
                log::warn!(
                    "mod {}: {fault} at tick {tick}; off for the rest of the race",
                    slot.mod_id
                );
                slot.faulted = Some((tick, fault.to_string()));
                cost
            }
        };
        slot.fuel_used = cost.fuel_used;
        slot.fuel_peak = slot.fuel_peak.max(cost.fuel_used);
    }
    for (index, command) in applied {
        match command {
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
            Command::ShieldDelta { slot, amount } => {
                super::shield::delta(sim, slot.index(), amount)
            }
            // Race-wide and held: the later module in id order wins.
            Command::ScalePad { pad, factor } => {
                sim.extensions.on.pad_scale[usize::from(pad)] = factor;
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
            Command::GivePickup { .. }
            | Command::ClearPickup { .. }
            | Command::PickupRoll { .. } => pickup_command(sim, index, command),
            Command::PreferRoute { slot, route } => {
                sim.extensions.on.prefer.set(slot.index(), route);
            }
        }
    }
}
