//! The engine's fixed cue set, decoded once at race load: [`Banks`] and the
//! one-per-cue [`Loaded`] audio it indexes. Split out of [`super`] under the
//! 1,000-line rule. [`load_named_cue`] is `pub(super)` because
//! [`super::announcer::Announcer`] shares it: a milestone's cue name is built
//! at load time from per-title data, so it cannot go through [`load_cue`].

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use log::warn;
use oag_assets::source::Archives;
use oag_audio::Sound;
use oag_core::Rng;
use oag_formats::sblk;

use super::layers::{self, CueVoice, Program, Timeline};
use super::{BankName, Cue};

/// One cue's decoded audio: every waveform its command run binds.
#[derive(Debug, Clone)]
pub struct Loaded {
    /// The alternates, in command order, each with its own loop flag from the
    /// descriptor's `+0x0e`. Never empty.
    ///
    /// Per waveform, not per cue: several banked cues mix the two (`hud.bnk`'s
    /// `~BLOWUP` has one looping waveform of two, `~AIRBRAKE_MONO` one of three).
    /// Every cue fired today is uniform, so collapsing them would play a loop as
    /// a one-shot the first time one of those was wired.
    pub waveforms: Vec<(Arc<Sound>, bool)>,
}

/// The sound banks a race needs, decoded and indexed by cue.
#[derive(Debug, Default, Clone)]
pub struct Banks {
    // `pub(super)`: `sfx::tests` and `sfx::engine::tests` build a `Banks` by
    // struct literal to test `pick`/`pick_at` without decoding a bank, and are
    // siblings, not descendants, of this module.
    pub(super) sounds: BTreeMap<Cue, Loaded>,
    /// What loading did, for the race's own report.
    pub report: Vec<String>,
    /// The previous pick for each multi-alternate cue, mirroring `operand[3]`,
    /// the byte `0x19`'s handler mutates in the cue's command data on every
    /// play. `RefCell`, not `&mut self` on [`Self::pick`]: the original's cache
    /// belongs to the bank, not the calling voice, and every call site holds a
    /// shared `&Banks`. See
    /// [`sound.md`](../../../../../docs/ghidra/functions/ps3-hdfury-eu/sound.md#0x19---alternate-selection-decoded).
    ///
    /// One slot per [`Cue`], shared by every voice that plays it, including
    /// [`Cue::Engine`]'s eight live craft. That `operand[3]` sits in command
    /// data is read directly; that eight open `~ENGINE` voices contend on that
    /// one byte is not (`sound.md`'s `+0xa8` gate covers one voice's re-entry
    /// within a play, not two voices). Read as sharing from the storage
    /// location, not from a traced multi-voice case.
    pub(super) last_pick: RefCell<BTreeMap<Cue, usize>>,
    /// Cues that play as the timeline they author, one entry per alternate
    /// combination; absent where the flat [`Self::pick`] plays. See [`super::layers`].
    pub(super) timelines: BTreeMap<Cue, Vec<Timeline>>,
    /// Cues that repeat while held and play as the list runs, in place of the
    /// flat pick or the one-shot timeline. See [`Cue::repeats`].
    pub(super) programs: BTreeMap<Cue, Program>,
    /// HD's per-team engine crossfade tables, keyed by the disc's team name;
    /// empty on a title with a `~ENGINE` cue. See [`super::xfade`].
    pub(super) xfade: BTreeMap<String, Arc<super::xfade::Team>>,
    /// HD's authored group volumes, read with the engine tables and gated the
    /// same way. See [`crate::hd_mix`].
    pub(super) mix: Option<Arc<crate::hd_mix::Maps>>,
}

impl Banks {
    /// Reads and decodes every cue in [`Cue::ALL`] out of `archives`.
    ///
    /// Never fails, and reports every cue either way: a cue can be missing for
    /// ordinary reasons (HD has no `~ENGINE`, its ship audio being a per-event
    /// `c_*` set), each a line in [`Self::report`] and silence, not an error or
    /// a substitute. A race refusing to start over a missing bank would make
    /// audio a precondition for every other kind of work. (Every title in the
    /// lineage carries banks, a claim this project got wrong about Pure for
    /// months.)
    #[must_use]
    pub fn load(
        archives: &mut Archives,
        banks: &oag_title::SoundBanks,
        zone: bool,
        tick: oag_title::SequenceTick,
    ) -> Self {
        let mut sounds = BTreeMap::new();
        let mut timelines = BTreeMap::new();
        let mut programs = BTreeMap::new();
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
            load_one(
                blob,
                entry,
                cue,
                tick,
                &mut sounds,
                Tables {
                    timelines: &mut timelines,
                    programs: &mut programs,
                },
                &mut report,
            );
        }

        Self {
            sounds,
            report,
            timelines,
            programs,
            ..Default::default()
        }
    }

    /// Loads the start-of-race voice, [`Cue::COUNTDOWN`], from `entry` (the speech
    /// bank the mode opened) beside what [`Self::load`] decoded.
    ///
    /// A separate pass because the bank is the mode's: `speech.bnk`,
    /// `speech_elim.bnk` (Eliminator), `speech_zone.bnk` (Zone), see
    /// [`oag_title::CountdownVoice`]. Never fails, on [`Self::load`]'s terms.
    pub fn load_countdown(
        &mut self,
        archives: &mut Archives,
        entry: &str,
        tick: oag_title::SequenceTick,
    ) {
        let blob = match archives.read_name(entry) {
            Ok(blob) => blob,
            Err(e) => {
                let line = format!("sfx: {entry} not read: {e}");
                warn!("{line}");
                self.report.push(line);
                return;
            }
        };
        let mut report = Vec::new();
        for cue in Cue::COUNTDOWN {
            load_one(
                &blob,
                entry,
                cue,
                tick,
                &mut self.sounds,
                Tables {
                    timelines: &mut self.timelines,
                    programs: &mut self.programs,
                },
                &mut report,
            );
        }

        self.report.extend(report);
    }

    /// Loads the crossfaded engine tables a grid needs, when this title's ship
    /// bank has no `~ENGINE` cue. Gated on the missing cue alone, so Pulse and
    /// Pure never read an `.xfx`. Never fails; see [`super::xfade::load`].
    pub fn load_xfade(
        &mut self,
        archives: &mut Archives,
        source: super::xfade::Source<'_>,
        slot_teams: &[String],
        report: &mut Vec<String>,
    ) {
        if self.sounds.contains_key(&Cue::Engine) {
            return;
        }
        self.xfade = super::xfade::load(archives, source, slot_teams, report);
        // Only where tables loaded: the mix is HD's, and a title that reads no
        // `.xfx` keeps its own buses.
        if !self.xfade.is_empty() {
            self.mix = crate::hd_mix::Maps::load(archives).map(Arc::new);
        }
    }

    /// The crossfade table for the team a slot flies, when one loaded.
    #[must_use]
    pub fn xfade_team(&self, slot_team: &str) -> Option<Arc<super::xfade::Team>> {
        self.xfade
            .get(&super::xfade::table_name(slot_team))
            .cloned()
    }

    /// The authored volume group `n` as a bus, when the title has an authored
    /// mix (HD); `None` leaves a cue on the bus it always had.
    #[must_use]
    pub fn group_bus(&self, n: u8) -> Option<oag_audio::Bus> {
        self.mix.as_ref().map(|_| oag_audio::Bus::Group(n))
    }

    /// Whether anything at all decoded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sounds.is_empty()
    }

    /// One *named* waveform of a cue, by index, clamped to what it binds.
    ///
    /// For the one cue whose alternates are not interchangeable: [`Cue::LockOn`]
    /// binds two and the original's seeking/locked parameter chooses, not a draw.
    /// Every other cue uses [`Self::pick`]. Clamped, not `None`, out of range: a
    /// bank binding one waveform where two are expected plays the one it has.
    #[must_use]
    pub fn pick_at(&self, cue: Cue, index: usize) -> Option<(Arc<Sound>, bool)> {
        let loaded = self.sounds.get(&cue)?;
        let index = index.min(loaded.waveforms.len().checked_sub(1)?);
        let (sound, looping) = &loaded.waveforms[index];
        Some((Arc::clone(sound), *looping))
    }

    /// The list of a cue that repeats while held, when it loaded as one.
    #[must_use]
    pub fn program(&self, cue: Cue) -> Option<&Program> {
        self.programs.get(&cue)
    }

    /// Everything one play of a cue starts, drawn by `rng`.
    ///
    /// A cue with a [timeline](super::layers) gives its voices for one alternate
    /// combination (never-repeat rule as [`Self::pick`]) and one shared bend draw
    /// per `0x1b`; any other cue gives the waveform [`Self::pick`] would. `None`
    /// when the cue did not load. Not for [`Cue::Engine`], whose layers one voice drives.
    #[must_use]
    pub fn voices(&self, cue: Cue, rng: &mut Rng) -> Option<Vec<CueVoice>> {
        let Some(variants) = self.timelines.get(&cue) else {
            let (sound, looping) = self.pick(cue, rng)?;
            return Some(vec![CueVoice::plain(sound, looping)]);
        };
        let index = self.draw(cue, variants.len(), rng);
        Some(variants[index].voices(rng))
    }

    /// The uniform, never-repeating draw among `len` alternates of `cue`.
    fn draw(&self, cue: Cue, len: usize, rng: &mut Rng) -> usize {
        match u32::try_from(len) {
            Ok(len) if len > 1 => {
                let draw = rng.below(len) as usize;
                let mut last_pick = self.last_pick.borrow_mut();
                let index = match last_pick.get(&cue) {
                    Some(&previous) if previous == draw => (draw + 1) % len as usize,
                    _ => draw,
                };
                last_pick.insert(cue, index);
                index
            }
            _ => 0,
        }
    }

    /// One waveform for a cue, chosen by `rng` when it has alternates; `None`
    /// when the cue did not load. Opcode `0x19` ([`Self::last_pick`]): a uniform
    /// draw never repeating the previous pick for the cue, re-rolled by
    /// advancing one alternate and wrapping, as the original does (a
    /// reject-and-retry would bias a small `count` differently).
    #[must_use]
    pub fn pick(&self, cue: Cue, rng: &mut Rng) -> Option<(Arc<Sound>, bool)> {
        let loaded = self.sounds.get(&cue)?;
        let index = self.draw(cue, loaded.waveforms.len(), rng);
        let (sound, looping) = &loaded.waveforms[index];
        Some((Arc::clone(sound), *looping))
    }
}

/// Decodes one cue out of an already-read bank blob into the maps [`Banks`]
/// keeps, one report line either way ([`load_one`] fills the per-cue tables).
struct Tables<'a> {
    timelines: &'a mut BTreeMap<Cue, Vec<Timeline>>,
    programs: &'a mut BTreeMap<Cue, Program>,
}

/// The body of [`Banks::load`]'s loop, lifted so [`Banks::load_countdown`] can
/// load from the *mode's* speech bank on the same terms.
fn load_one(
    blob: &[u8],
    entry: &str,
    cue: Cue,
    tick: oag_title::SequenceTick,
    sounds: &mut BTreeMap<Cue, Loaded>,
    tables: Tables,
    report: &mut Vec<String>,
) {
    let Tables {
        timelines,
        programs,
    } = tables;
    match load_cue(blob, cue, tick) {
        Ok((loaded, skipped, timeline, program)) => {
            let undecoded = if skipped == 0 {
                String::new()
            } else {
                format!(", {skipped} skipped: decoded to no samples")
            };
            // One line per cue: the timeline note rides on it.
            let mut line = format!(
                "sfx: {} -> {} waveform(s) from {entry}{undecoded}",
                cue.name(),
                loaded.waveforms.len()
            );
            let mut extra = None;
            match timeline {
                Ok(Some(t)) => {
                    line.push_str(&format!(
                        "; plays its timeline: {} variant(s) of {} voice(s)",
                        t.len(),
                        t.iter().map(|v| v.layers.len()).max().unwrap_or(0)
                    ));
                    timelines.insert(cue, t);
                }
                Ok(None) => extra = not_one_event(cue, &loaded),
                Err(e) => line.push_str(&format!("; timeline not built, flat pick kept: {e}")),
            }
            match program {
                Ok(Some(p)) => {
                    line.push_str(&format!(
                        "; plays its list as a repeating program ({} key-on(s))",
                        p.layers.len()
                    ));
                    programs.insert(cue, p);
                }
                Ok(None) => {}
                Err(e) => line.push_str(&format!("; program not built: {e}")),
            }
            report.push(line);
            report.extend(extra);
            sounds.insert(cue, loaded);
        }
        // A report line, not a fallback: nothing is substituted for a cue that
        // will not resolve.
        Err(e) => report.push(format!("sfx: {} not loaded: {e}", cue.name())),
    }
}

/// What building a cue's timeline came to: none worth having, or a failure that
/// leaves the flat pick in place.
type TimelineResult = anyhow::Result<Option<Vec<Timeline>>>;

/// Resolves one cue in one bank blob and decodes what it binds.
fn load_cue(
    blob: &[u8],
    cue: Cue,
    tick: oag_title::SequenceTick,
) -> anyhow::Result<(
    Loaded,
    usize,
    TimelineResult,
    anyhow::Result<Option<Program>>,
)> {
    let bank = sblk::Bank::parse(blob)?;
    let (loaded, skipped) = load_named_cue(&bank, cue.name())?;
    // The engine is driven by one voice's per-tick pitch and volume, and how
    // that law spreads over layers is unread: it keeps its flat set.
    let timeline = if cue == Cue::Engine {
        Ok(None)
    } else {
        layers::timelines(&bank, cue.name(), tick)
    };
    let program = if cue.repeats() {
        layers::program(&bank, cue.name(), tick)
    } else {
        Ok(None)
    };
    Ok((loaded, skipped, timeline, program))
}

/// The length ratio past which a cue's waveforms cannot all be alternates of one
/// event.
///
/// `crates/game/tests/sfx_ground_truth.rs` applies this to Pulse's `.COLLISIONS`
/// and asserts the fifteen span under 0.2 s ("a cue whose commands were meant to
/// play *together* would be a stack of different lengths"). Two is generous:
/// Pulse's fifteen span 0.204 s to 0.350 s, a ratio of 1.7.
const ALTERNATE_LENGTH_RATIO: f32 = 2.0;

/// How many waveforms a cue needs before the ratio above means anything.
/// A cue binding two is a pair, and a long one beside a short one is ordinary
/// (Pulse's `~SHIELD` is 0.501 s and 1.087 s, `~BLOWUP` 0.091 s and 0.276 s;
/// neither is a severity tree). What is worth reporting is *many* waveforms over
/// a wide range, a flattened tree rather than a set of takes.
const ALTERNATE_COUNT: usize = 4;

/// Says so when a cue's waveforms are too unalike to be alternates.
///
/// [`Banks::pick`] chooses uniformly among whatever a cue resolves to: right for
/// takes of one event, wrong for a tree. HD's `.COLLISIONS` binds nothing itself
/// and plays `c_CShipShip` and `c_CShipWall`, each with Small/Medium/Large
/// children: 112 waveforms spanning 0.410 s to 3.266 s, the longest 1.4 s
/// impacts with half their energy below 120 Hz (peak 0.809 of full scale in that
/// band). Picked at random for a light graze, one is a bass thump, and on HD it
/// lands in near-silence because the ship bank has no `~ENGINE`.
///
/// Choosing correctly needs the contact's surface and severity, and which the
/// original reads for which is not recovered, so this reports rather than
/// guesses, as `Banks::load` does for an undecodable cue.
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
/// binds. Split from [`load_cue`] so [`super::announcer::Announcer`] can decode
/// a name built at load time (`"zone_5"`, `"zone_10"`): the milestone ladder is
/// per-title data, so it cannot be a `Cue` variant.
pub(super) fn load_named_cue(bank: &sblk::Bank, name: &str) -> anyhow::Result<(Loaded, usize)> {
    let record = bank
        .cue_named(name)
        .ok_or_else(|| anyhow::anyhow!("{name:?} names no cue in {}", bank.name))?;
    load_cue_record(bank, &record, name)
}

/// [`load_named_cue`] for a circuit's authored emitter, which on a hashed bank
/// (2048's `SBlk` version 5) spells the cue and finds it by [`name_hash`].
///
/// Only the track emitters and the crossfade engine take the hashed path; the
/// race's own cues keep [`load_named_cue`], whose 2048 triggers are unchecked
/// (`sblk::Bank::cue_named`).
///
/// [`name_hash`]: oag_formats::sblk::cue::name_hash
pub(super) fn load_track_cue(bank: &sblk::Bank, name: &str) -> anyhow::Result<(Loaded, usize)> {
    let record = bank
        .cue_named_or_hashed(name)
        .ok_or_else(|| anyhow::anyhow!("{name:?} names no cue in {}", bank.name))?;
    load_cue_record(bank, &record, name)
}

/// [`load_named_cue`] for a cue reached by index: a crossfade layer whose name
/// is empty addresses its bank by the cue number alone.
pub(super) fn load_indexed_cue(
    bank: &sblk::Bank,
    index: u16,
) -> anyhow::Result<(Loaded, sblk::Cue)> {
    let name = format!("cue {index}");
    let record = bank.cue(index).ok_or_else(|| {
        anyhow::anyhow!(
            "{name} is past the {} cue(s) of {}",
            bank.cue_count,
            bank.name
        )
    })?;
    let (loaded, _) = load_cue_record(bank, &record, &name)?;
    Ok((loaded, record))
}

/// A cue that binds no waveform because every command it runs is a no-op or a
/// register write: it authors no sound, and nothing is missing.
#[derive(Debug)]
pub struct ControlOnlyCue {
    name: String,
    opcodes: Vec<u8>,
}

impl std::fmt::Display for ControlOnlyCue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} is a control cue: its commands run only {:02x?}, which do nothing or write \
             a register, so it authors no sound",
            self.name, self.opcodes
        )
    }
}

impl std::error::Error for ControlOnlyCue {}

/// Decodes what an already-found cue binds; `name` only labels errors.
pub(super) fn load_cue_record(
    bank: &sblk::Bank,
    record: &sblk::Cue,
    name: &str,
) -> anyhow::Result<(Loaded, usize)> {
    // The tree, not the cue's own run. On the PSP, PS2 and Pure discs no wired
    // cue plays a child, so this is `cue_sounds` there. HD's `.COLLISIONS` plays
    // `c_CShipShip` and `c_CShipWall`, each with S/M/L children (112 waveforms
    // in all). See `oag_formats::sblk::child`.
    let sounds = bank.cue_tree_sounds(record);
    // A cue whose every command was read and none starts a waveform is a
    // control cue, silent by design, not a decode gap: `0x14` is a bare no-op
    // and `0x1e` writes a register byte
    // (`docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md`).
    if sounds.is_empty() {
        let timeline = bank.cue_timeline(record);
        if timeline.is_complete() && timeline.grains.is_empty() && !timeline.passed.is_empty() {
            return Err(ControlOnlyCue {
                name: name.to_string(),
                opcodes: cue_opcodes(bank, record),
            }
            .into());
        }
    }
    // The opcodes go in the message: which opcode blocked a cue is the
    // question for anything that is not a control cue.
    anyhow::ensure!(
        !sounds.is_empty(),
        "{name} binds no waveform: its {} command(s) run only {:02x?}, opcodes this does not read",
        record.commands,
        cue_opcodes(bank, record)
    );

    let mut waveforms = Vec::with_capacity(sounds.len());
    let mut skipped = 0;
    for sound in &sounds {
        let pcm = decode_waveform(bank, sound, name)?;
        if pcm.is_empty() {
            skipped += 1;
            continue;
        }
        // `record`'s own cue, not each waveform's: right wherever
        // `cue_tree_sounds` is `cue_sounds`. HD's `.COLLISIONS` is the one case
        // where a waveform's binding cue differs from `record`
        // (`c_CShipShip` and its children); using `record`'s byte there is a
        // stated approximation (`oag_audio::spatial::pan_volume_gain`).
        let pan_volume_gain = oag_audio::spatial::pan_volume_gain(record.volume, sound.volume);
        waveforms.push((
            Arc::new(
                // Each waveform at the rate its descriptor keys it on with:
                // `Sound::pitch` is the `sceSasSetPitch` word for an unmodulated
                // play (`oag_formats::sblk::pitch`).
                Sound::new(pcm, 1, sound.sample_rate())?.with_pan_volume_gain(pan_volume_gain),
            ),
            sound.is_looping(),
        ));
    }
    anyhow::ensure!(
        !waveforms.is_empty(),
        "all {skipped} of {name}'s waveforms decoded to nothing"
    );
    Ok((Loaded { waveforms }, skipped))
}

/// Decodes one waveform a bank binds, in the codec its descriptor names.
///
/// `name` is only for the error: the cue the waveform was reached from.
pub(super) fn decode_waveform(
    bank: &sblk::Bank,
    sound: &sblk::Sound,
    name: &str,
) -> anyhow::Result<Vec<i16>> {
    let data = bank
        .waveform(sound)
        .ok_or_else(|| anyhow::anyhow!("{name} reaches outside the waveform section"))?;
    // Not every waveform is PS-ADPCM: on HD about a third set `+0x0e`'s `0x80`,
    // SCREAM's second voice type, 16-bit PCM behind a 16-byte header. See
    // `oag_formats::sblk::{NOT_ADPCM_FLAG, decode_pcm16}`.
    Ok(if sound.is_adpcm() {
        // The span's run-out block is not played: the encoder appends one past
        // the flagged end block and the hardware stops at the flag, so decoding the
        // whole span would replay it each loop (`oag_formats::sblk::adpcm_played`).
        sblk::decode_adpcm(sblk::adpcm_played(data))
    } else {
        sblk::decode_pcm16(data)
    })
}

/// The opcode byte of each command in a cue's own run, in command order: the high
/// byte of the command's first word (byte 3 in memory, as
/// `Scream_StepCommandList` reads it). Duplicated from
/// `oag_formats::sblk::Bank::sounds`, which does not expose it.
fn cue_opcodes(bank: &sblk::Bank, cue: &sblk::Cue) -> Vec<u8> {
    cue.range()
        .filter_map(|at| {
            let word = bank.commands.get(at * sblk::COMMAND_LEN..)?;
            Some((bank.order.u32(word, 0) >> 24) as u8)
        })
        .collect()
}
