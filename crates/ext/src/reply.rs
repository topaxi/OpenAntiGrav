//! The reply: one typed method per command, refusing what the host would fault on.

use oag_ext_abi::{
    Command, Cue, Effect, HudElement, HudPlace, HudStr, MAX_COMMANDS, MAX_HUD_EXTRA, MAX_HUD_ITEMS,
    MAX_HUD_SHAKE, MAX_PAD_FACTOR, MAX_THRUST_FACTOR, POOL_AT, POOL_LEN, Pickup, REPLY_AT, Rgba,
    Sections, Weapon, WeatherKind,
};

use crate::Craft;

/// Why a [`Reply`] method sent nothing. The host would have faulted the module for the
/// race on the same command; here the author decides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// The reply already holds `MAX_COMMANDS` commands.
    Full,
    /// A second `scale_thrust`, `shield_delta`, `scale_slot_pads` or Shield command
    /// (`give_shield`, `hold_shield`, `clear_shield`) for one craft, or a
    /// second `scale_pad` for one pad, this tick.
    Repeated,
    /// A value out of its range: a factor outside `[0, MAX_THRUST_FACTOR]` or
    /// `[0, MAX_PAD_FACTOR]`, a pad the track does not have, a shield
    /// change past the craft's maximum, a non-finite number, a roll period of 0.
    Range,
    /// More than `MAX_HUD_ITEMS` `hud_text` and `hud_icon` items for one craft this
    /// tick, or a string pool already full.
    HudItems,
    /// `give_pickup` to a craft holding something, or `clear_pickup` from one not
    /// holding the mod's own pickup.
    Pickup,
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
    pool_used: usize,
    items: [u8; oag_ext_abi::MAX_CRAFTS],
}

impl<'a> Reply<'a> {
    pub(crate) fn new(reply: &'a mut [u8], speed_pads: usize) -> Self {
        Self {
            reply,
            count: 0,
            thrust: 0,
            shield: 0,
            shield_cmd: 0,
            slot_pads: 0,
            pads: [0; oag_ext_abi::track::MAX_ENTITIES / 32],
            speed_pads,
            pool_used: 0,
            items: [0; oag_ext_abi::MAX_CRAFTS],
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

    /// `fire-effect`: one of the title's own particle effects at the craft's nozzle.
    /// Drawn, never hashed.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`].
    pub fn fire_effect(&mut self, craft: &Craft<'_>, effect: Effect) -> Result<(), Refused> {
        self.push(&Command::FireEffect {
            slot: craft.slot(),
            effect,
        })
    }

    /// `hud-tint`: this tick, recolour the craft's HUD `element`. Drawn, never hashed;
    /// a title without the element draws nothing.
    ///
    /// # Errors
    ///
    /// [`Refused::Full`].
    pub fn hud_tint(
        &mut self,
        craft: &Craft<'_>,
        element: HudElement,
        rgba: Rgba,
    ) -> Result<(), Refused> {
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

    /// `hud-text` with the text named by `key` in the mod's own `[strings]` tables: the
    /// host shows the entry of the game's language, or English's when the language lacks
    /// it, or nothing when no table has it.
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
        let bytes = string.as_str().as_bytes();
        let at = self.pool_used;
        if usize::from(self.items[slot]) >= MAX_HUD_ITEMS || at + 1 + bytes.len() > POOL_LEN {
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
        self.items[slot] += 1;
        Ok(())
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
