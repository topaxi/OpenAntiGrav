//! The sound-effects fade a Pulse race opens with: snapped to silence, then a linear ramp back.
//!
//! `RaceMode_UpdateIntro` calls `Audio_SetSfxFadeTarget(0)` in its first call, which drops the
//! effects fade to zero, and `Audio_StartSfxFadeIn(0.02, ...)` (`0x0893a958`) when the lock counter
//! reads `0x28`, after which `Audio_UpdateGroupVolumes` adds `0.02` once a frame until it reaches
//! the SFX Volume fraction
//! (`docs/ghidra/functions/psp-pulse-usa/race-intro.md`, "The music"). The fade multiplies the
//! SFX slider on [`Bus::Sfx`]; the composition root raises the two calls as
//! `oag_raceplay::intro_camera::IntroCue`s and one [`Audio::tick`] is one call.

use oag_title::group_volume::EffectsVolumeLaw;

use super::*;

/// The fade's state: its gain and the step it climbs by each tick. A step of zero holds the gain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct SfxFade {
    gain: f32,
    per_tick: f32,
    law: EffectsVolumeLaw,
}

impl Default for SfxFade {
    fn default() -> Self {
        Self {
            gain: 1.0,
            per_tick: 0.0,
            law: EffectsVolumeLaw::Linear,
        }
    }
}

impl SfxFade {
    /// Silence, held until [`Self::fade_in`].
    pub(super) fn silence(&mut self) {
        *self = Self {
            gain: 0.0,
            per_tick: 0.0,
            ..*self
        };
    }

    /// From silence, `per_tick` more gain on each [`Self::tick`].
    pub(super) fn fade_in(&mut self, per_tick: f32) {
        *self = Self {
            gain: 0.0,
            per_tick,
            ..*self
        };
    }

    /// Back to no fade at all: full gain, nothing climbing, and the menus' linear law (the race
    /// arms its own with [`Audio::set_effects_volume_law`] after this, on launching one).
    pub(super) fn release(&mut self) {
        *self = Self::default();
    }

    /// The bus gain for the SFX slider's fraction under the fade.
    pub(super) fn bus_gain(&self, slider: f32) -> f32 {
        self.law.gain(slider * self.gain)
    }

    /// One call of `Audio_UpdateGroupVolumes`: adds the step, up to one.
    pub(super) fn tick(&mut self) {
        if self.per_tick > 0.0 {
            self.gain = (self.gain + self.per_tick).min(1.0);
        }
    }

    pub(super) fn gain(&self) -> f32 {
        self.gain
    }
}

impl Audio {
    /// The curve from the SFX slider and the fade to the effects bus, for the title about to race
    /// (`oag_title::Title::effects_volume_law`). Applied at once.
    pub fn set_effects_volume_law(&mut self, law: EffectsVolumeLaw) {
        self.sfx_fade.law = law;
        self.apply_sfx_fade();
    }

    /// The flyby's first call: the effects go silent and stay so until [`Self::fade_in_sfx`].
    pub fn silence_sfx(&mut self) {
        debug!("audio: effects fade: silent");
        self.sfx_fade.silence();
        self.apply_sfx_fade();
    }

    /// The flyby's counter reached the fade's value: the effects return from silence, `per_tick`
    /// more gain on each [`Self::tick`].
    pub fn fade_in_sfx(&mut self, per_tick: f32) {
        debug!("audio: effects fade: rising from silence by {per_tick} a tick");
        self.sfx_fade.fade_in(per_tick);
        self.apply_sfx_fade();
    }

    /// The effects fade's gain now, `0..=1`.
    #[must_use]
    pub fn sfx_fade_gain(&self) -> f32 {
        self.sfx_fade.gain()
    }

    /// Steps the fade once and puts it on the bus.
    pub(super) fn tick_sfx_fade(&mut self) {
        let before = self.sfx_fade.gain();
        self.sfx_fade.tick();
        if self.sfx_fade.gain() != before {
            if self.sfx_fade.gain() >= 1.0 {
                debug!("audio: effects fade: full");
            }
            self.apply_sfx_fade();
        }
    }

    /// Releases the fade, on leaving a race: a flyby quit half-way must not leave the menus' own
    /// effects quiet.
    pub(super) fn release_sfx_fade(&mut self) {
        self.sfx_fade.release();
        self.apply_sfx_fade();
    }

    /// The SFX slider times the fade, on the bus. HD's authored mix owns the bus on its titles
    /// and never raises the fade.
    pub(super) fn apply_sfx_fade(&self) {
        if self.hd.live.is_some() {
            return;
        }
        let slider = self.hd.sliders.get()[1];
        self.output
            .with_mixer(|mixer| mixer.set_bus_gain(Bus::Sfx, self.sfx_fade.bus_gain(slider)));
    }
}
