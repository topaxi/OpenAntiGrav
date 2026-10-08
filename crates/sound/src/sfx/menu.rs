//! [`MenuSfx`]: the front end's navigation sounds, played dry from the title's
//! front-end bank.
//!
//! The race's cues ride a tick of the simulation ([`super::Audio::race_tick`]);
//! these ride a player's press, so they start the moment the menu code asks and
//! hold no state beyond the decoded bank and the generator that picks among a
//! cue's alternates. Every trigger is recovered from Pulse's `BOOT.BIN`
//! (`docs/ghidra/functions/psp-pulse-usa/menu-sounds.md`); the cue names and
//! the bank are each title's own data ([`oag_title::SoundBanks::frontend`]).

use oag_core::Rng;

use super::{Audio, Banks, Cue, VoicePlace, start_voices};

/// A different stream from [`super::SFX_SEED`]: a menu choosing an alternate
/// must not move which collision a later race picks.
const MENU_SEED: u64 = 0x0aa9_5f10_0000_6d65;

/// The decoded front-end bank and its generator.
#[derive(Debug, Clone)]
pub struct MenuSfx {
    banks: Banks,
    rng: Rng,
}

impl MenuSfx {
    /// Reads [`Cue::FRONT_END`] out of the title's front-end bank, in the
    /// Fury style when `fury` (HD's `accept_fury` and `reject_fury`). Never fails;
    /// a title without one, or a cue that will not decode, is a line in
    /// [`Self::report`] and silence.
    #[must_use]
    pub fn load(
        archives: &mut oag_assets::Archives,
        banks: &oag_title::SoundBanks,
        tick: oag_title::SequenceTick,
        fury: bool,
    ) -> Self {
        Self {
            banks: Banks::load_front_end(archives, banks, tick, fury),
            rng: Rng::new(MENU_SEED),
        }
    }

    /// What loading did, one line per cue.
    #[must_use]
    pub fn report(&self) -> &[String] {
        &self.banks.report
    }

    /// Whether `cue` decoded and would sound.
    #[must_use]
    pub fn has(&self, cue: Cue) -> bool {
        self.banks.voices_available(cue)
    }

    /// Starts `cue` on `audio`'s mixer, at full volume and centred.
    ///
    /// Returns whether anything started. Nothing is substituted for a cue that
    /// did not load.
    pub fn play(&mut self, audio: &Audio, cue: Cue) -> bool {
        let Some(voices) = self.banks.voices(cue, &mut self.rng) else {
            return false;
        };
        audio.output.with_mixer(|mixer| {
            !start_voices(mixer, &voices, cue.bus(), VoicePlace::DRY).is_empty()
        })
    }
}
