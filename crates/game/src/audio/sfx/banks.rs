//! The engine's own fixed cue set, decoded once at race load: [`Banks`] and
//! the one-per-cue [`Loaded`] audio it indexes.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.
//! [`load_named_cue`] is `pub(super)` rather than private because
//! [`super::announcer::Announcer`] shares it: a milestone's cue name is built
//! at load time from per-title data, not one of [`super::Cue`]'s fixed,
//! statically-named set, so it cannot go through [`load_cue`] itself.

use std::collections::BTreeMap;
use std::sync::Arc;

use log::info;
use oag_assets::source::Archives;
use oag_audio::Sound;
use oag_core::Rng;
use oag_formats::sblk;

use super::{BankName, Cue};

/// One cue's decoded audio: every waveform its command run binds.
#[derive(Debug, Clone)]
pub struct Loaded {
    /// The alternates, in command order, each with **its own** loop flag from
    /// the descriptor's `+0x0e`. Never empty.
    ///
    /// Per waveform rather than per cue, because the flag is per waveform and
    /// several banked cues mix the two: `hud.bnk`'s `~BLOWUP` has one looping
    /// waveform of two and `~AIRBRAKE_MONO` one of three. Every cue this port
    /// currently fires happens to be uniform, so collapsing them would be
    /// invisible today and would silently play a loop as a one-shot the first
    /// time one of those was wired.
    pub waveforms: Vec<(Arc<Sound>, bool)>,
}

/// The sound banks a race needs, decoded and indexed by cue.
#[derive(Debug, Default, Clone)]
pub struct Banks {
    // `pub(super)` (visible in `sfx` and its whole subtree) rather than
    // private: `sfx::tests` and `sfx::engine::tests` build a `Banks` by
    // struct literal to test `pick`/`pick_at` without decoding a real bank,
    // and both are siblings of this module rather than descendants of it.
    pub(super) sounds: BTreeMap<Cue, Loaded>,
    /// What loading did, for the race's own report.
    pub report: Vec<String>,
}

impl Banks {
    /// Reads and decodes every cue in [`Cue::ALL`] out of `archives`.
    ///
    /// **Never fails, and reports every cue either way.** Every title in the
    /// lineage does carry banks - a claim this project got wrong about Pure for
    /// months - but a cue can still be missing for reasons that are ordinary
    /// rather than broken: Wipeout HD has no `~ENGINE` at all, its ship audio
    /// being a per-event `c_*` set rather than a held loop. Each miss is a line
    /// in [`Self::report`] and silence, not an error and not a substitute.
    ///
    /// A race that refused to start because a bank was missing would make the
    /// audio work a precondition for every other kind of work in the tree.
    #[must_use]
    pub fn load(archives: &mut Archives, banks: &oag_title::SoundBanks, zone: bool) -> Self {
        let mut sounds = BTreeMap::new();
        let mut report = Vec::new();
        let mut blobs: BTreeMap<BankName, Vec<u8>> = BTreeMap::new();

        for cue in Cue::ALL {
            let entry = cue.bank().entry(banks, zone);
            let blob = match blobs.get(&cue.bank()) {
                Some(blob) => blob,
                None => match archives.read_name(entry) {
                    Ok(blob) => blobs.entry(cue.bank()).or_insert(blob),
                    Err(e) => {
                        report.push(format!("sfx: {entry} not read: {e}"));
                        continue;
                    }
                },
            };
            match load_cue(blob, cue) {
                Ok((loaded, skipped)) => {
                    let undecoded = if skipped == 0 {
                        String::new()
                    } else {
                        format!(", {skipped} skipped as not PS-ADPCM")
                    };
                    report.push(format!(
                        "sfx: {} -> {} waveform(s) from {entry}{undecoded}",
                        cue.name(),
                        loaded.waveforms.len()
                    ));
                    if let Some(line) = not_one_event(cue, &loaded) {
                        report.push(line);
                    }
                    sounds.insert(cue, loaded);
                }
                // Deliberately a report line and not a fallback. Nothing is
                // substituted for a cue that will not resolve; it stays silent
                // and says so.
                Err(e) => report.push(format!("sfx: {} not loaded: {e}", cue.name())),
            }
        }
        for line in &report {
            info!("{line}");
        }
        Self { sounds, report }
    }

    /// Whether anything at all decoded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sounds.is_empty()
    }

    /// One *named* waveform of a cue, by index, clamped to what it binds.
    ///
    /// For the one cue whose alternates are **not** interchangeable:
    /// [`Cue::LockOn`] binds two, and which of them plays is the original's
    /// seeking/locked parameter rather than a draw. Every other cue goes
    /// through [`Self::pick`] and should - see the module docs on why the
    /// selecting opcode being unread makes a random draw the honest default.
    ///
    /// Clamped rather than `None` on an out-of-range index: a bank that binds
    /// one waveform where this expects two should play the one it has, not go
    /// silent.
    #[must_use]
    pub fn pick_at(&self, cue: Cue, index: usize) -> Option<(Arc<Sound>, bool)> {
        let loaded = self.sounds.get(&cue)?;
        let index = index.min(loaded.waveforms.len().checked_sub(1)?);
        let (sound, looping) = &loaded.waveforms[index];
        Some((Arc::clone(sound), *looping))
    }

    /// One waveform for a cue, chosen by `rng` when the cue has alternates.
    ///
    /// `None` when the cue did not load. See the module docs for why the choice
    /// is made here rather than by the bank: the selecting opcode is unread.
    #[must_use]
    pub fn pick(&self, cue: Cue, rng: &mut Rng) -> Option<(Arc<Sound>, bool)> {
        let loaded = self.sounds.get(&cue)?;
        let index = match u32::try_from(loaded.waveforms.len()) {
            Ok(len) if len > 1 => rng.below(len) as usize,
            _ => 0,
        };
        let (sound, looping) = &loaded.waveforms[index];
        Some((Arc::clone(sound), *looping))
    }
}

/// Resolves one cue in one bank blob and decodes what it binds.
fn load_cue(blob: &[u8], cue: Cue) -> anyhow::Result<(Loaded, usize)> {
    let bank = sblk::Bank::parse(blob)?;
    load_named_cue(&bank, cue.name())
}

/// The length ratio past which a cue's waveforms cannot all be alternates of
/// one event.
///
/// `crates/game/tests/sfx_ground_truth.rs` already applies this test to Pulse's
/// `.COLLISIONS` and asserts the fifteen span under 0.2 s - "a cue whose
/// commands were meant to play *together* would be a stack of different
/// lengths". Two is generous against that: Pulse's fifteen span 0.204 s to
/// 0.350 s, a ratio of 1.7.
const ALTERNATE_LENGTH_RATIO: f32 = 2.0;

/// How many waveforms a cue needs before the ratio above means anything.
///
/// A cue that binds two is a pair, and a long one beside a short one is an
/// ordinary way to author a pair - Pulse's `~SHIELD` is 0.501 s and 1.087 s and
/// its `~BLOWUP` 0.091 s and 0.276 s, and neither is a severity tree. The thing
/// worth reporting is *many* waveforms spanning a wide range, which is what a
/// flattened tree looks like and what a set of takes does not.
const ALTERNATE_COUNT: usize = 4;

/// Says so when a cue's waveforms are too unalike to be alternates.
///
/// **[`Banks::pick`] chooses uniformly among whatever a cue resolves to**,
/// which is right when they are takes of one event and wrong when they are a
/// tree. Wipeout HD's `.COLLISIONS` is the case that made this necessary: it
/// binds nothing itself and plays `c_CShipShip` and `c_CShipWall`, each with
/// Small/Medium/Large children of its own - **112 waveforms spanning 0.410 s to
/// 3.266 s**, where the longest are 1.4 s impacts carrying half their energy
/// below 120 Hz and peaking at 0.809 of full scale in that band. Picked at
/// random for a light graze, one of those is a bass thump, and on HD it lands
/// in near-silence because that title's ship bank has no `~ENGINE` cue at all.
///
/// Choosing correctly needs the contact's own surface and severity, and which
/// of the two the original reads for which is not recovered - so this reports
/// rather than guesses, on the same terms `Banks::load` reports a cue it could
/// not decode.
fn not_one_event(cue: Cue, loaded: &Loaded) -> Option<String> {
    let mut shortest = f32::MAX;
    let mut longest: f32 = 0.0;
    for (sound, _) in &loaded.waveforms {
        shortest = shortest.min(sound.seconds());
        longest = longest.max(sound.seconds());
    }
    (loaded.waveforms.len() > ALTERNATE_COUNT
        && shortest > 0.0
        && longest / shortest > ALTERNATE_LENGTH_RATIO)
        .then(|| {
            format!(
                "sfx: {}'s {} waveform(s) span {shortest:.3}s to {longest:.3}s, \
             which is a tree and not alternates of one event - this engine picks \
             among them uniformly, so a light event can play a heavy one's sound",
                cue.name(),
                loaded.waveforms.len()
            )
        })
}

/// Resolves one **named** cue in an already-parsed bank and decodes what it
/// binds.
///
/// Split out of [`load_cue`] so [`super::announcer::Announcer`] can decode a
/// cue by a name built at load time (`"zone_5"`, `"zone_10"`, ...) rather than
/// through [`Cue`]'s closed, statically-named set - the milestone ladder is
/// per-title data, not an engine-wide cue, so it cannot be a `Cue` variant
/// without hard-coding one title's numbers into the engine.
pub(super) fn load_named_cue(bank: &sblk::Bank, name: &str) -> anyhow::Result<(Loaded, usize)> {
    let record = bank
        .cue_named(name)
        .ok_or_else(|| anyhow::anyhow!("{name:?} names no cue in {}", bank.name))?;
    // **The tree, not the cue's own run.** On the PSP, PS2 and Pure discs no
    // wired cue plays a child, so this is `cue_sounds` there and the two are
    // the same call. Wipeout HD's `.COLLISIONS` binds nothing itself and plays
    // `c_CShipShip` and `c_CShipWall`, each of which has S/M/L children of its
    // own - 112 waveforms in all. See `oag_formats::sblk::child`.
    let sounds = bank.cue_tree_sounds(&record);
    anyhow::ensure!(
        !sounds.is_empty(),
        "{name} binds no waveform: its {} command(s) are all opcodes this does not read",
        record.commands
    );

    let mut waveforms = Vec::with_capacity(sounds.len());
    let mut skipped = 0;
    for sound in &sounds {
        // **Not every waveform is PS-ADPCM, and the descriptor says which.**
        // On Wipeout HD about a third set `+0x0e`'s `0x80`, a codec this
        // project has not identified; decoding one as ADPCM would produce 28
        // samples a block of noise, which is exactly the plausible-looking
        // stand-in `CLAUDE.md` forbids. Dropped, and counted so the load
        // report says so. See `oag_formats::sblk::NOT_ADPCM_FLAG`.
        if !sound.is_adpcm() {
            skipped += 1;
            continue;
        }
        let data = bank
            .waveform(sound)
            .ok_or_else(|| anyhow::anyhow!("{name} reaches outside the waveform section"))?;
        waveforms.push((
            Arc::new(Sound::new(
                sblk::decode_adpcm(data),
                1,
                sblk::ASSUMED_SAMPLE_RATE,
            )?),
            sound.is_looping(),
        ));
    }
    anyhow::ensure!(
        !waveforms.is_empty(),
        "all {skipped} of {name}'s waveforms are in a codec this does not decode"
    );
    Ok((Loaded { waveforms }, skipped))
}
