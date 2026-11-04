//! The reply: one typed method per command, refusing what the host would fault on.

use oag_ext_abi::{
    Colour, Command, Cue, Distance, Effect, HudElement, HudPlace, HudStr, Icon, MAX_COMMANDS,
    MAX_EMITS_PER_TICK, MAX_HUD_EXTRA, MAX_HUD_ITEMS, MAX_HUD_SHAKE, MAX_PAD_FACTOR,
    MAX_PROBE_RADIUS, MAX_SPEED_FACTOR, MAX_THRUST_FACTOR, MAX_TRANSIT_TICKS, POOL_AT, POOL_LEN,
    Pickup, REPLY_AT, Rgba, Sections, SoundMode, Weapon, WeatherKind,
};

use crate::Craft;

/// Why a [`Reply`] method sent nothing. The host would have faulted the module for the
/// race on the same command; here the author decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// The reply already holds `MAX_COMMANDS` commands.
    Full,
    /// A second `scale_thrust`, `shield_delta`, `scale_slot_pads`, `prefer_route` or
    /// Shield command (`give_shield`, `hold_shield`, `clear_shield`) for one craft, or a
    /// second `scale_pad` for one pad, this tick.
    Repeated,
    /// A value out of its range: a factor outside `[0, MAX_THRUST_FACTOR]` or
    /// `[0, MAX_PAD_FACTOR]`, a pad or route the track does not have, a shield
    /// change past the craft's maximum, a non-finite number, a roll period of 0.
    Range,
    /// More than `MAX_HUD_ITEMS` `hud_text` and `hud_icon` items for one craft this
    /// tick, or a string pool already full.
    HudItems,
    /// `give_pickup` to a craft holding something, or `clear_pickup` from one not
    /// holding the mod's own pickup.
    Pickup,
    /// `hud_tint` of the pickup element: a mod pickup's colour is `pickup_look`'s.
    PickupTint,
    /// More than `MAX_EMITS_PER_TICK` `emit`s this tick: the host faults the module on the
    /// next one.
    Emits,
    /// More than `MAX_FIRE_EFFECTS_PER_TICK` `fire_effect` and `fire_mod_effect` commands
    /// this tick: the host faults the module on the next one.
    Effects,
    /// More than `MAX_BEAMS_PER_TICK` `draw_beam` commands this tick: the host faults the
    /// module on the next one.
    Beams,
}

/// This tick's commands. The count is written when the tick returns.
#[derive(Debug)]
pub struct Reply<'a> {
    reply: &'a mut [u8],
    count: usize,
    thrust: u32,
    shield: u32,
    shield_cmd: u32,
    slot_pads: u32,
    pads: [u32; oag_ext_abi::track::MAX_ENTITIES / 32],
    speed_pads: usize,
    routes: usize,
    route_cmd: u32,
    pool_used: usize,
    items: [u8; oag_ext_abi::MAX_CRAFTS],
    emits: usize,
    effects: usize,
    beams: usize,
    /// `transit`, `scale-speed`, `probe-clearance` sent, one bit a slot each.
    motion: [u32; 3],
}

impl<'a> Reply<'a> {
    pub(crate) fn new(reply: &'a mut [u8], speed_pads: usize, routes: usize) -> Self {
        Self {
            reply,
            count: 0,
            thrust: 0,
            shield: 0,
            shield_cmd: 0,
            slot_pads: 0,
            pads: [0; oag_ext_abi::track::MAX_ENTITIES / 32],
            speed_pads,
            routes,
            route_cmd: 0,
            pool_used: 0,
            items: [0; oag_ext_abi::MAX_CRAFTS],
            emits: 0,
            effects: 0,
            beams: 0,
            motion: [0; 3],
        }
    }

    pub(crate) fn finish(self) {
        Sections::put_count(self.reply, self.count);
    }

    /// Commands sent so far this tick.
    #[must_use]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Whether none has been sent.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn push(&mut self, command: &Command) -> Result<(), Refused> {
        if self.count >= MAX_COMMANDS {
            return Err(Refused::Full);
        }
        Sections::put_command(self.reply, self.count, command);
        self.count += 1;
        Ok(())
    }

    /// `scale-thrust`: this tick, the craft's engine output times `factor`, in
    /// `[0, MAX_THRUST_FACTOR]`. Once per craft per tick; re-sent each tick it holds.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Repeated`], [`Refused::Range`].
    pub fn scale_thrust(&mut self, craft: &Craft<'_>, factor: f32) -> Result<(), Refused> {
        if !(0.0..=MAX_THRUST_FACTOR).contains(&factor) {
            return Err(Refused::Range);
        }
        let bit = craft.slot().bit();
        if self.thrust & bit != 0 {
            return Err(Refused::Repeated);
        }
        self.push(&Command::ScaleThrust {
            slot: craft.slot(),
            factor,
        })?;
        self.thrust |= bit;
        Ok(())
    }

    /// `shield-delta`: add `amount` to the craft's energy (negative takes it, through
    /// the damage path, and can wreck). `|amount|` at most [`Craft::shield_max`]. Once
    /// per craft per tick.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Repeated`], [`Refused::Range`].
    pub fn shield_delta(&mut self, craft: &Craft<'_>, amount: f32) -> Result<(), Refused> {
        if !amount.is_finite() || amount.abs() > craft.shield_max() {
            return Err(Refused::Range);
        }
        let bit = craft.slot().bit();
        if self.shield & bit != 0 {
            return Err(Refused::Repeated);
        }
        self.push(&Command::ShieldDelta {
            slot: craft.slot(),
            amount,
        })?;
        self.shield |= bit;
        Ok(())
    }

    /// One motion command (`which`: 0 transit, 1 scale-speed, 2 probe) per craft a tick.
    fn motion(&mut self, which: usize, craft: &Craft<'_>, command: Command) -> Result<(), Refused> {
        let bit = craft.slot().bit();
        if self.motion[which] & bit != 0 {
            return Err(Refused::Repeated);
        }
        self.push(&command)?;
        self.motion[which] |= bit;
        Ok(())
    }

    /// `transit`: take the craft out of the world for `ticks` ticks (frozen, not drawn,
    /// not collided with, not hit, its controls ignored), then put it at `distance` on
    /// (forward, under a lap) at `exit_factor` times the speed it has now. Laps count
    /// through the jump; a finish in it is timed at the exit tick. A `transit-end` event
    /// comes on the exit tick's call. The host faults the module if the distance does not
    /// resolve on this track (no course, a lap or more, a trigger it does not have).
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Repeated`]; [`Refused::Range`] for a craft not
    /// racing or already in a transit, `ticks` outside `1..=MAX_TRANSIT_TICKS`,
    /// `exit_factor` outside `[0, MAX_SPEED_FACTOR]`, or a distance that cannot be sent.
    pub fn transit(
        &mut self,
        craft: &Craft<'_>,
        ticks: u16,
        distance: Distance,
        exit_factor: f32,
    ) -> Result<(), Refused> {
        let (kind, amount) = distance.split();
        if !craft.is_racing()
            || craft.in_transit()
            || ticks == 0
            || ticks > MAX_TRANSIT_TICKS
            || !(0.0..=MAX_SPEED_FACTOR).contains(&exit_factor)
            || Distance::join(kind, amount) != Some(distance)
        {
            return Err(Refused::Range);
        }
        let command = Command::Transit {
            slot: craft.slot(),
            ticks,
            distance,
            exit_factor,
        };
        self.motion(0, craft, command)
    }

    /// `scale-speed`: multiply the craft's velocity by `factor`, in
    /// `[0, MAX_SPEED_FACTOR]`, once, at this call. Once per craft per tick.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Repeated`], [`Refused::Range`].
    pub fn scale_speed(&mut self, craft: &Craft<'_>, factor: f32) -> Result<(), Refused> {
        if !(0.0..=MAX_SPEED_FACTOR).contains(&factor) {
            return Err(Refused::Range);
        }
        let command = Command::ScaleSpeed {
            slot: craft.slot(),
            factor,
        };
        self.motion(1, craft, command)
    }

    /// `probe-clearance`: ask whether a half disc of `radius` (`(0, MAX_PROBE_RADIUS]`)
    /// standing across the track at `distance` on fits without touching its geometry.
    /// The answer is a `clearance` event at the next call (fits, and how far on it
    /// probed); a distance that does not resolve answers "does not fit". Once per craft
    /// per tick.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Repeated`], [`Refused::Range`].
    pub fn probe_clearance(
        &mut self,
        craft: &Craft<'_>,
        distance: Distance,
        radius: f32,
    ) -> Result<(), Refused> {
        let (kind, amount) = distance.split();
        if !(radius > 0.0 && radius <= MAX_PROBE_RADIUS)
            || Distance::join(kind, amount) != Some(distance)
        {
            return Err(Refused::Range);
        }
        let command = Command::ProbeClearance {
            slot: craft.slot(),
            distance,
            radius,
        };
        self.motion(2, craft, command)
    }

    /// `fire-mod-effect-ahead`: the mod's own effect at place `index` of its `[effects]`,
    /// played at the course point `distance` ahead of the craft and left there for the
    /// effect's own lifetime: a sprite of the mod's texture standing in the world. Counts
    /// toward `MAX_FIRE_EFFECTS_PER_TICK`. Drawn, never hashed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Effects`], [`Refused::Range`] for a distance that
    /// is not one the host accepts.
    pub fn fire_mod_effect_ahead(
        &mut self,
        craft: &Craft<'_>,
        index: u16,
        distance: Distance,
    ) -> Result<(), Refused> {
        let (kind, amount) = distance.split();
        if Distance::join(kind, amount) != Some(distance) {
            return Err(Refused::Range);
        }
        self.room_for_effect()?;
        self.push(&Command::FireModEffectAhead {
            slot: craft.slot(),
            index,
            distance,
        })?;
        self.effects += 1;
        Ok(())
    }

    /// `screen-fill`: wash the craft's view with `rgba` this tick; re-send it each tick
    /// it should hold. The slot's own view, never hashed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`].
    pub fn screen_fill(&mut self, craft: &Craft<'_>, rgba: Rgba) -> Result<(), Refused> {
        self.push(&Command::ScreenFill {
            slot: craft.slot(),
            rgba,
            shader: None,
        })
    }

    /// `screen-fill` through the shader at place `shader` of the mod's `[shaders]` (name
    /// order), `rgba` being the colour it is handed. One that did not load draws nothing.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Range`] past `MAX_SHADER_INDEX`.
    pub fn screen_fill_shaded(
        &mut self,
        craft: &Craft<'_>,
        rgba: Rgba,
        shader: u16,
    ) -> Result<(), Refused> {
        if shader > oag_ext_abi::MAX_SHADER_INDEX {
            return Err(Refused::Range);
        }
        self.push(&Command::ScreenFill {
            slot: craft.slot(),
            rgba,
            shader: Some(shader),
        })
    }

    /// `draw-beam`: a ribbon from `craft` to the craft in `to` this tick, flat `rgba` or
    /// through the shader at that place of the mod's `[shaders]`; re-send it each tick it
    /// should hold. Draws nothing while either craft is out of the world. Never hashed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Beams`], [`Refused::Range`] for a beam to `craft`
    /// itself, a `half_width` outside `(0, MAX_BEAM_HALF_WIDTH]`, or a shader past
    /// `MAX_SHADER_INDEX`.
    pub fn draw_beam(
        &mut self,
        craft: &Craft<'_>,
        to: oag_ext_abi::Slot,
        rgba: Rgba,
        half_width: f32,
        shader: Option<u16>,
    ) -> Result<(), Refused> {
        let width = half_width > 0.0 && half_width <= oag_ext_abi::MAX_BEAM_HALF_WIDTH;
        let index = shader.is_none_or(|shader| shader <= oag_ext_abi::MAX_SHADER_INDEX);
        if to == craft.slot() || !width || !index {
            return Err(Refused::Range);
        }
        if self.beams >= oag_ext_abi::MAX_BEAMS_PER_TICK {
            return Err(Refused::Beams);
        }
        self.push(&Command::DrawBeam {
            slot: craft.slot(),
            to,
            rgba,
            half_width,
            shader,
        })?;
        self.beams += 1;
        Ok(())
    }

    /// `set-shader-param`: scalar `index` (its place in the shader's header) of the shader
    /// at place `shader` of the mod's `[shaders]`, in `craft`'s view, this tick; unset, the
    /// header's default. Never hashed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Range`] for a value that is not finite or an `index`
    /// from `MAX_SHADER_PARAMS` up.
    pub fn set_shader_param(
        &mut self,
        craft: &Craft<'_>,
        shader: u16,
        index: u8,
        value: f32,
    ) -> Result<(), Refused> {
        if !value.is_finite() || index >= oag_ext_abi::MAX_SHADER_PARAMS {
            return Err(Refused::Range);
        }
        self.push(&Command::SetShaderParam {
            slot: craft.slot(),
            shader,
            index,
            value,
        })
    }

    /// `fov-offset`: widen (positive) or narrow the craft's view by `degrees`, within
    /// `MAX_FOV_OFFSET_DEG` either way, this tick; re-send it each tick it should hold.
    /// Never hashed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Range`].
    pub fn fov_offset(&mut self, craft: &Craft<'_>, degrees: f32) -> Result<(), Refused> {
        if !(-oag_ext_abi::MAX_FOV_OFFSET_DEG..=oag_ext_abi::MAX_FOV_OFFSET_DEG).contains(&degrees)
        {
            return Err(Refused::Range);
        }
        self.push(&Command::FovOffset {
            slot: craft.slot(),
            degrees,
        })
    }

    /// `give-pickup`: the mod's own pickup to a craft holding nothing.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Pickup`].
    pub fn give_pickup(&mut self, craft: &Craft<'_>) -> Result<(), Refused> {
        if craft.pickup() != Pickup::None {
            return Err(Refused::Pickup);
        }
        self.push(&Command::GivePickup { slot: craft.slot() })
    }

    /// `clear-pickup`: take the mod's own pickup from the craft holding it.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Pickup`].
    pub fn clear_pickup(&mut self, craft: &Craft<'_>) -> Result<(), Refused> {
        if craft.pickup() != Pickup::Ours {
            return Err(Refused::Pickup);
        }
        self.push(&Command::ClearPickup { slot: craft.slot() })
    }

    /// `pickup-roll`: race-wide, every `every`-th time a weapon pad hands out `weapon`,
    /// it hands out the mod's own pickup instead. Held until replaced; sending the same
    /// rule again keeps its count.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Range`] for `every` 0.
    pub fn pickup_roll(&mut self, weapon: Weapon, every: u8) -> Result<(), Refused> {
        if every == 0 {
            return Err(Refused::Range);
        }
        self.push(&Command::PickupRoll { weapon, every })
    }

    /// `scale-pad{pad, factor}`: race-wide, from now on speed pad `pad` (its index
    /// among the track's speed pads) pushes `factor` times its own push, `factor` in
    /// `[0, MAX_PAD_FACTOR]`. Held until replaced; once per pad per tick.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Repeated`], [`Refused::Range`] for a factor out
    /// of range or a pad the track does not have.
    pub fn scale_pad(&mut self, pad: u16, factor: f32) -> Result<(), Refused> {
        let index = usize::from(pad);
        if !(0.0..=MAX_PAD_FACTOR).contains(&factor) || index >= self.speed_pads {
            return Err(Refused::Range);
        }
        let (word, bit) = (index / 32, 1 << (index % 32));
        if self.pads[word] & bit != 0 {
            return Err(Refused::Repeated);
        }
        self.push(&Command::ScalePad { pad, factor })?;
        self.pads[word] |= bit;
        Ok(())
    }

    /// `scale-pad{slot, factor}`: this tick, every speed pad pushes the craft `factor`
    /// times as hard, on top of the pad's own factor. At 0 the pad gives no push, no
    /// plume and no sound; [`Event::SpeedPad`](crate::Event::SpeedPad) still fires.
    /// Once per craft per tick; re-sent each tick it holds.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Repeated`], [`Refused::Range`].
    pub fn scale_slot_pads(&mut self, craft: &Craft<'_>, factor: f32) -> Result<(), Refused> {
        if !(0.0..=MAX_PAD_FACTOR).contains(&factor) {
            return Err(Refused::Range);
        }
        let bit = craft.slot().bit();
        if self.slot_pads & bit != 0 {
            return Err(Refused::Repeated);
        }
        self.push(&Command::ScaleSlotPads {
            slot: craft.slot(),
            factor,
        })?;
        self.slot_pads |= bit;
        Ok(())
    }

    /// `give-shield`: start the title's own Shield on the craft, as firing a held Shield
    /// pickup does. One already running is not refreshed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Repeated`] for a second shield command for the
    /// craft this tick.
    pub fn give_shield(&mut self, craft: &Craft<'_>) -> Result<(), Refused> {
        self.shield_command(craft, Command::GiveShield { slot: craft.slot() })
    }

    /// `hold-shield`: the craft's running Shield does not run out this tick. Re-send it
    /// each tick it should hold; a Shield not held ends within two ticks.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Repeated`].
    pub fn hold_shield(&mut self, craft: &Craft<'_>) -> Result<(), Refused> {
        self.shield_command(craft, Command::HoldShield { slot: craft.slot() })
    }

    /// `clear-shield`: end the craft's running Shield before this tick's step.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Repeated`].
    pub fn clear_shield(&mut self, craft: &Craft<'_>) -> Result<(), Refused> {
        self.shield_command(craft, Command::ClearShield { slot: craft.slot() })
    }

    /// `prefer-route`: at its next fork the craft's driver takes route `route` (an index
    /// into [`Track::route`](crate::Track::route)) instead of the one its coin drew, if
    /// the route leaves that fork and the driver can drive it; the coin is still thrown.
    /// Re-sent each tick it should hold: a preference dropped before the fork leaves
    /// the coin's draw. Moves the driver only, so a person steering ignores it.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Repeated`], [`Refused::Range`] for a route the
    /// track does not have.
    pub fn prefer_route(&mut self, craft: &Craft<'_>, route: u16) -> Result<(), Refused> {
        if usize::from(route) >= self.routes {
            return Err(Refused::Range);
        }
        let bit = craft.slot().bit();
        if self.route_cmd & bit != 0 {
            return Err(Refused::Repeated);
        }
        self.push(&Command::PreferRoute {
            slot: craft.slot(),
            route,
        })?;
        self.route_cmd |= bit;
        Ok(())
    }

    /// `emit`: send `value` on `channel` ([`channel!`](crate::channel)) about the craft.
    /// Every module, this one included, receives an [`Event::Mod`](crate::Event::Mod) for
    /// it on the next tick. The channel must be one the mod's `oag-mod.toml` publishes
    /// (the host faults the module for an undeclared one, which the SDK cannot see), at
    /// most [`MAX_EMITS_PER_TICK`] a tick.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Emits`], [`Refused::Range`] for a non-finite value.
    pub fn emit(&mut self, craft: &Craft<'_>, channel: u32, value: f32) -> Result<(), Refused> {
        if !value.is_finite() {
            return Err(Refused::Range);
        }
        if self.emits >= MAX_EMITS_PER_TICK {
            return Err(Refused::Emits);
        }
        self.push(&Command::Emit {
            slot: craft.slot(),
            channel,
            value,
        })?;
        self.emits += 1;
        Ok(())
    }

    fn shield_command(&mut self, craft: &Craft<'_>, command: Command) -> Result<(), Refused> {
        let bit = craft.slot().bit();
        if self.shield_cmd & bit != 0 {
            return Err(Refused::Repeated);
        }
        self.push(&command)?;
        self.shield_cmd |= bit;
        Ok(())
    }

    /// `play-cue`: one of the title's own sounds from the craft. Heard, never hashed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`].
    pub fn play_cue(&mut self, craft: &Craft<'_>, cue: Cue) -> Result<(), Refused> {
        self.push(&Command::PlayCue {
            slot: craft.slot(),
            cue,
        })
    }

    /// `play-mod-sound`: the mod's own sound at place `index` of its `[sounds]` (name
    /// order) from the craft, once, as a loop to start, or as a loop to stop
    /// ([`SoundMode`]). One the mod lacks or that did not load plays nothing, and the
    /// load report says so. Heard, never hashed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`].
    pub fn play_mod_sound(
        &mut self,
        craft: &Craft<'_>,
        index: u16,
        mode: SoundMode,
    ) -> Result<(), Refused> {
        self.push(&Command::PlayModSound {
            slot: craft.slot(),
            index,
            mode,
        })
    }

    /// `fire-effect`: one of the title's own particle effects at the craft's nozzle.
    /// Drawn, never hashed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Effects`] past `MAX_FIRE_EFFECTS_PER_TICK` a tick.
    pub fn fire_effect(&mut self, craft: &Craft<'_>, effect: Effect) -> Result<(), Refused> {
        self.room_for_effect()?;
        self.push(&Command::FireEffect {
            slot: craft.slot(),
            effect,
        })?;
        self.effects += 1;
        Ok(())
    }

    fn room_for_effect(&self) -> Result<(), Refused> {
        if self.effects >= oag_ext_abi::MAX_FIRE_EFFECTS_PER_TICK {
            return Err(Refused::Effects);
        }
        Ok(())
    }

    /// `fire-effect` with the effect at place `index` of the mod's own `[effects]`
    /// (name order): a parametric TOML the manifest declares, played at the craft's
    /// nozzle. One the mod lacks or that did not load draws nothing, and the load
    /// report says so. Drawn, never hashed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Effects`] past `MAX_FIRE_EFFECTS_PER_TICK` a tick
    /// (counting [`Self::fire_effect`] too), or [`Refused::Range`] for an index past 15
    /// bits.
    pub fn fire_mod_effect(&mut self, craft: &Craft<'_>, index: u16) -> Result<(), Refused> {
        if index >= 1 << 15 {
            return Err(Refused::Range);
        }
        self.room_for_effect()?;
        self.push(&Command::FireModEffect {
            slot: craft.slot(),
            index,
        })?;
        self.effects += 1;
        Ok(())
    }

    /// `hud-tint`: this tick, recolour the craft's HUD `element`. Drawn, never hashed;
    /// a title without the element draws nothing.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::PickupTint`] for the pickup element (use
    /// [`Self::pickup_look`]).
    pub fn hud_tint(
        &mut self,
        craft: &Craft<'_>,
        element: HudElement,
        rgba: Rgba,
    ) -> Result<(), Refused> {
        if element == HudElement::Pickup {
            return Err(Refused::PickupTint);
        }
        self.push(&Command::HudTint {
            slot: craft.slot(),
            element,
            rgba,
        })
    }

    /// `hud-shake`: this tick, jitter the craft's HUD `element` by up to `amplitude`
    /// HUD pixels, in `[0, MAX_HUD_SHAKE]`. Drawn, never hashed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Range`].
    pub fn hud_shake(
        &mut self,
        craft: &Craft<'_>,
        element: HudElement,
        amplitude: f32,
    ) -> Result<(), Refused> {
        if !(0.0..=MAX_HUD_SHAKE).contains(&amplitude) {
            return Err(Refused::Range);
        }
        self.push(&Command::HudShake {
            slot: craft.slot(),
            element,
            amplitude,
        })
    }

    /// `hud-bar-extra`: this tick, the craft's bar `element` also shows `extra` times
    /// its value on top (0.9 for a turbo of x1.9), the added share blending into
    /// `rgba`. `extra` in `(0, MAX_HUD_EXTRA]`. Drawn, never hashed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Range`].
    pub fn hud_bar_extra(
        &mut self,
        craft: &Craft<'_>,
        element: HudElement,
        extra: f32,
        rgba: Rgba,
    ) -> Result<(), Refused> {
        if !(extra > 0.0 && extra <= MAX_HUD_EXTRA) {
            return Err(Refused::Range);
        }
        self.push(&Command::HudBarExtra {
            slot: craft.slot(),
            element,
            extra,
            rgba,
        })
    }

    /// `hud-text`: this tick, `text` (at most `MAX_HUD_TEXT` bytes) on the craft's HUD
    /// at `place`, in the title's HUD font. Drawn, never hashed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Range`] for a string too long or a placement out
    /// of range, [`Refused::HudItems`] past `MAX_HUD_ITEMS` for the craft or the pool.
    pub fn hud_text(
        &mut self,
        craft: &Craft<'_>,
        place: HudPlace,
        rgba: Rgba,
        text: &str,
    ) -> Result<(), Refused> {
        self.text(craft, place, rgba, text, false)
    }

    /// `hud-text` with the text named by string id `key` in the mod's own
    /// `strings/<language>.toml`: the host shows the entry of the game's language, or
    /// English's when the language lacks it, or nothing when no file has it.
    ///
    /// # Errors
    ///
    /// As [`Self::hud_text`].
    pub fn hud_string(
        &mut self,
        craft: &Craft<'_>,
        place: HudPlace,
        rgba: Rgba,
        key: &str,
    ) -> Result<(), Refused> {
        self.text(craft, place, rgba, key, true)
    }

    fn text(
        &mut self,
        craft: &Craft<'_>,
        place: HudPlace,
        rgba: Rgba,
        text: &str,
        own: bool,
    ) -> Result<(), Refused> {
        let text = HudStr::new(text).ok_or(Refused::Range)?;
        self.push_pooled(
            craft,
            place,
            &Command::HudText {
                slot: craft.slot(),
                place,
                rgba,
                text,
                own,
            },
            text,
        )
    }

    /// `hud-icon`: this tick, the title's HUD sprite `sprite` on the craft's HUD at
    /// `place`, `rgba` its modulating colour. Drawn, never hashed.
    ///
    /// # Errors
    ///
    /// As [`Self::hud_text`].
    pub fn hud_icon(
        &mut self,
        craft: &Craft<'_>,
        place: HudPlace,
        rgba: Rgba,
        sprite: &str,
    ) -> Result<(), Refused> {
        self.icon(craft, place, rgba, sprite, false)
    }

    /// `hud-icon` with the image named by `name` in the mod's own `[icons]`: a PNG the
    /// manifest declares, drawn at its own size times the placement's. One the mod
    /// lacks or the host could not read draws nothing, and the load report says so.
    ///
    /// # Errors
    ///
    /// As [`Self::hud_text`].
    pub fn hud_mod_icon(
        &mut self,
        craft: &Craft<'_>,
        place: HudPlace,
        rgba: Rgba,
        name: &str,
    ) -> Result<(), Refused> {
        self.icon(craft, place, rgba, name, true)
    }

    fn icon(
        &mut self,
        craft: &Craft<'_>,
        place: HudPlace,
        rgba: Rgba,
        sprite: &str,
        own: bool,
    ) -> Result<(), Refused> {
        let sprite = HudStr::new(sprite).ok_or(Refused::Range)?;
        self.push_pooled(
            craft,
            place,
            &Command::HudIcon {
                slot: craft.slot(),
                place,
                rgba,
                sprite,
                own,
            },
            sprite,
        )
    }

    fn push_pooled(
        &mut self,
        craft: &Craft<'_>,
        place: HudPlace,
        command: &Command,
        string: HudStr,
    ) -> Result<(), Refused> {
        let max = oag_ext_abi::hud::MAX_HUD_OFFSET;
        if HudPlace::from_bits(place.to_bits()) != Some(place)
            || place.x.abs() > max
            || place.y.abs() > max
        {
            return Err(Refused::Range);
        }
        let slot = craft.slot().index();
        if usize::from(self.items[slot]) >= MAX_HUD_ITEMS {
            return Err(Refused::HudItems);
        }
        self.push_string(command, string.as_str())?;
        self.items[slot] += 1;
        Ok(())
    }

    /// A command whose record names `text` in the reply's pool.
    fn push_string(&mut self, command: &Command, text: &str) -> Result<(), Refused> {
        let bytes = text.as_bytes();
        let at = self.pool_used;
        if at + 1 + bytes.len() > POOL_LEN {
            return Err(Refused::HudItems);
        }
        if self.count >= MAX_COMMANDS {
            return Err(Refused::Full);
        }
        let base = POOL_AT - REPLY_AT + at;
        if let Some(entry) = self.reply.get_mut(base..base + 1 + bytes.len()) {
            entry[0] = bytes.len() as u8;
            entry[1..].copy_from_slice(bytes);
        }
        let record = command.encode_pooled(at as u16);
        let start = 4 + self.count * oag_ext_abi::COMMAND_LEN;
        if let Some(out) = self.reply.get_mut(start..start + oag_ext_abi::COMMAND_LEN) {
            out.copy_from_slice(&record);
        }
        self.count += 1;
        self.pool_used += 1 + bytes.len();
        Ok(())
    }

    /// `pickup-look`: this tick, the look of the craft's held mod pickup: the `icon`
    /// and the backdrop `colour`, each naming its source (the title's own, or the
    /// mod's). Re-send it each tick it should hold; without one the manifest's
    /// `[pickup]` is the look. The title's fade, overshoot and size pulse play on it.
    /// Drawn, never hashed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::HudItems`] when the string pool is full.
    pub fn pickup_look(
        &mut self,
        craft: &Craft<'_>,
        icon: Icon,
        colour: Colour,
    ) -> Result<(), Refused> {
        let command = Command::PickupLook {
            slot: craft.slot(),
            icon,
            colour,
        };
        match icon {
            Icon::Title(_) => self.push(&command),
            Icon::Mod(name) => self.push_string(&command, name.as_str()),
        }
    }

    /// `set-weather`: race-wide, play the host title's own `kind` of weather at
    /// `strength` in `[0, 1]`, drawn with the enhanced weather whatever the player's
    /// setting. Re-send it each tick it should hold. Drawn, never hashed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Range`] outside `[0, 1]` or not a number.
    pub fn set_weather(&mut self, kind: WeatherKind, strength: f32) -> Result<(), Refused> {
        if !(0.0..=1.0).contains(&strength) {
            return Err(Refused::Range);
        }
        self.push(&Command::SetWeather { kind, strength })
    }

    /// `set-sun`: race-wide, scale the sun's draw and the lens flare's alpha by
    /// `intensity` in `[0, 1]` (0 = both off). The sun as a light is untouched.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Range`] outside `[0, 1]` or not a number.
    pub fn set_sun(&mut self, intensity: f32) -> Result<(), Refused> {
        if !(0.0..=1.0).contains(&intensity) {
            return Err(Refused::Range);
        }
        self.push(&Command::SetSun { intensity })
    }

    /// `grade-sky`: race-wide, mix the sky toward a grey, rainy colour by `grey` in
    /// `[0, 1]` (0 = the authored sky).
    ///
    /// # Errors
    ///
    /// [`Refused::Full`], [`Refused::Range`] outside `[0, 1]` or not a number.
    pub fn grade_sky(&mut self, grey: f32) -> Result<(), Refused> {
        if !(0.0..=1.0).contains(&grey) {
            return Err(Refused::Range);
        }
        self.push(&Command::GradeSky { grey })
    }
}
