//! A circuit's own authored sound emitters: the ambience a track carries with
//! its geometry, rather than a cue any craft fires.
//!
//! [`super`] plays the cues a *race* raises - a collision, a pad, an engine.
//! This is the other half of what a circuit sounds like: 1,298 `sound` and
//! `soundcone` nodes authored across the twelve Pulse circuits, each naming a
//! bank, a cue and a radius, each placed by the transform chain.
//! `oag_formats::sound_emitters` decodes them and
//! `docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md` is the
//! evidence; this module is only the wiring above it.
//!
//! # Every emitter is live from construction
//!
//! `VexSound_Init` (`0x089259a4`) allocates the emitter and calls `Sound_Play`
//! on the spot, so the original does not start a circuit's ambience when the
//! player comes near it: it starts all of it at load, and the out-of-range
//! latch in `SoundEmitter_ServiceRequests` (`0x089394dc`) is what keeps a
//! distant one silent.
//!
//! **This does start a cue on approach, and that is the port's one knowing
//! deviation** - because it also *stops* one on departure, where the original
//! leaves a latched request untouched. The two go together and neither is
//! free-standing: [`Ambience::tick`] has the reasoning and what is unread
//! behind it. The audible consequence is that a looping emitter restarts from
//! sample zero on each pass rather than being sampled mid-loop, which is worth
//! knowing before anyone compares a capture against the original.
//!
//! # The cone is wired
//!
//! `soundcone` `0x3e9` carries two authored angles and its own enable byte.
//! **Which of them reaches the emitter's `+0x40` half-angle is read**:
//! `VexSoundCone_Init` (`0x08925ff4`,
//! `docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md`'s own
//! section) is a four-line wrapper around `VexSound_Init` that copies the
//! node's own `+0x00` there - `Cone::wide()`, never smaller than the other
//! authored angle on any of the 134 nodes. [`TrackEmitters::directional`]
//! carries them, placed by [`oag_audio::Emitter::cone`], the same law
//! [`TrackEmitters::omni`] already used.
//!
//! **A cone's radius is `SoundEmitter::radius`, not `sample_radius`.**
//! `VexSound_Update` - the only function that ever resamples the radius
//! curve - has exactly two static call sites in the executable, both gated on
//! the payload's own `+0x3c`, which is `0` on all 1,298 authored nodes. So the
//! curve is never actually driven for anything Pulse ships, which is what
//! explains a cone's own curve value key reading `0` on all 134 of them: it is
//! dead data, not a second bug. Reading it as the radius would silence every
//! cone; reading `+0x10` - what `Init` actually writes into the emitter and
//! what stays there - plays the authored value.
//!
//! **A cone inside its radius but outside its angle still holds a voice, at
//! zero gain.** That is a straight read of the law, not a choice made here:
//! `oag_audio::spatial::Emitter::place` only refuses past the radius, and the
//! angle term is a multiplier that can reach zero without the emitter ever
//! becoming "out of range" - the same distinction
//! `docs/.../positional-audio.md` draws for the radius-only case. So
//! [`Ambience::tick`] opens and holds a silent voice for a cone the listener
//! never enters the angle of, which is real cost against the 32-voice pool
//! and not yet measured against it the way the omnidirectional census was.

use std::collections::BTreeMap;

use log::info;
use oag_assets::source::Archives;
use oag_formats::sblk;
use oag_formats::sound_emitters::{self, SoundEmitter};

use super::{Loaded, load_named_cue};

/// One authored emitter, with the audio its cue names where it resolves.
#[derive(Debug, Clone)]
pub struct Authored {
    /// The node, as `oag_formats::sound_emitters` decoded it.
    pub emitter: SoundEmitter,
    /// The cue's waveforms, or [`None`] where the reference does not resolve.
    ///
    /// **[`None`] is a real state and is reported rather than dropped**, with
    /// the reason, because there are three of them and they are different
    /// findings:
    ///
    /// 1. **The reference dangles.** Five of them on the Pulse disc name a cue
    ///    or a bank that does not exist - the disc's own bugs, decoded in
    ///    `track-sound-emitters.md` - and whether the original silently drops
    ///    one or falls back to a bank-index lookup is unread:
    ///    `Scream_FindSoundInBank`'s name comparison has not been decompiled.
    /// 2. **The cue binds no waveform**, because its whole command run is
    ///    opcodes `oag_formats::sblk` does not read. 38 nodes across eight
    ///    circuits, and the report names the opcodes: `0x1e` on every
    ///    `~SetReg*` cue, `0x14` on `moather~birds`, `dekonst~CRANE` and both
    ///    of `talonsj`'s. This port's gap rather than the disc's, and not on
    ///    `track-sound-emitters.md`'s dangling list, because that sweep checked
    ///    the name table and not the command run.
    /// 3. **No loaded bank carries that label**, which is what a title with no
    ///    shared track bank gets for every `gentrak` node.
    ///
    /// All three play nothing and say so, which is what `CLAUDE.md` asks of an
    /// asset that will not resolve.
    pub sound: Option<Loaded>,
}

/// A circuit's authored emitters, split by shape rather than by what plays.
#[derive(Debug, Default, Clone)]
pub struct TrackEmitters {
    /// The omnidirectional `sound` `0x3e1` nodes, in authored order.
    pub omni: Vec<Authored>,
    /// The directional `soundcone` `0x3e9` nodes, in authored order.
    ///
    /// Split from [`Self::omni`] because [`Ambience::tick`] needs to know
    /// which law to place each one under, not because either is treated as
    /// more real than the other - both are played.
    pub directional: Vec<Authored>,
    /// What parsing did, for the race's own report.
    pub report: Vec<String>,
}

impl TrackEmitters {
    /// Reads every authored emitter out of one circuit's `.vex`, unresolved.
    ///
    /// **Never fails.** A `.vex` with no nodes, a Zone circuit (which authors
    /// none of the three classes at all) and a title that has never been swept
    /// all produce an empty result and a report line, on the same terms
    /// [`super::Banks::load`] reports a bank it could not read: silence with a
    /// reason beats a race that will not start.
    ///
    /// Every [`Authored::sound`] is [`None`]; [`Self::load`] is the one that
    /// resolves them against the circuit's banks.
    #[must_use]
    pub fn parse(track: &str, blob: &[u8]) -> Self {
        let mut report = Vec::new();
        let Ok(nodes) = oag_formats::vex::nodes(blob) else {
            report.push(format!("track audio: {track} has no readable node table"));
            return Self {
                report,
                ..Self::default()
            };
        };
        let authored = sound_emitters::emitters(blob, &nodes);
        let (directional, omni): (Vec<_>, Vec<_>) =
            authored.into_iter().partition(|e| e.cone.is_some());
        let wrap = |emitter| Authored {
            emitter,
            sound: None,
        };
        let omni: Vec<_> = omni.into_iter().map(wrap).collect();
        let directional: Vec<_> = directional.into_iter().map(wrap).collect();
        report.push(format!(
            "track audio: {track} authors {} omnidirectional emitter(s) and {} cone(s)",
            omni.len(),
            directional.len(),
        ));
        Self {
            omni,
            directional,
            report,
        }
    }

    /// [`Self::parse`], with every cue looked up in the bank its node names.
    ///
    /// # Where a circuit's banks are, and how that is derived
    ///
    /// An emitter spells its bank by that bank's own seven-character **label**,
    /// not by a path - `basilic`, `gentrak` - and a label is not a truncation of
    /// anything (`docs/formats/psp-audio.md`). So the two banks are opened by
    /// name and then matched on the label each one reports about itself, which
    /// is what makes this generalise past the circuit it was written against:
    ///
    /// 1. The **circuit's own** bank is named by the circuit's own
    ///    `trackstartup.xml`, in its `<LoadSoundBank Filename="...">`, and sits
    ///    beside that manifest in the circuit's directory -
    ///    `Data\Environments\01_Track\BASILICO_ENV.bnk`, 253,664 bytes, label
    ///    `basilic`. Nothing needs to know the twelve circuit names.
    /// 2. The **shared** bank is `oag_title::SoundBanks::track_general`, the one
    ///    entry no cue names and the executable does - see that field.
    ///
    /// A cue is decoded **once per distinct pair**, not once per node: an
    /// `~ELEVATOR` authored four times over is one `Loaded` shared four ways.
    #[must_use]
    pub fn load(
        archives: &mut Archives,
        banks: &oag_title::SoundBanks,
        track: &str,
        blob: &[u8],
    ) -> Self {
        let mut parsed = Self::parse(track, blob);

        // Read whole first and parse after, because `sblk::Bank` borrows the
        // blob it was read out of and both have to outlive the cue lookups.
        let mut blobs: Vec<(&str, String, Vec<u8>)> = Vec::new();
        match banks.track_general {
            Some(entry) => match archives.read_name(entry) {
                Ok(bytes) => blobs.push(("shared", entry.to_string(), bytes)),
                Err(e) => parsed
                    .report
                    .push(format!("track audio: {entry} not read: {e}")),
            },
            None => parsed.report.push(
                "track audio: this title names no shared track bank, so only a circuit's own \
                 bank can resolve"
                    .to_string(),
            ),
        }
        match circuit_bank_entry(archives, track) {
            Some(entry) => match archives.read_name(&entry) {
                Ok(bytes) => blobs.push(("circuit", entry, bytes)),
                Err(e) => parsed
                    .report
                    .push(format!("track audio: {entry} not read: {e}")),
            },
            None => parsed.report.push(format!(
                "track audio: no trackstartup.xml beside {track} names a sound bank, so only the \
                 shared bank can resolve"
            )),
        }

        // Keyed by the bank's own label, which is what a node spells - a label
        // is not a truncation of the path, so nothing can be inferred from one.
        let mut by_label: BTreeMap<String, sblk::Bank<'_>> = BTreeMap::new();
        for (why, entry, bytes) in &blobs {
            match sblk::Bank::parse(bytes) {
                Ok(bank) => {
                    parsed.report.push(format!(
                        "track audio: {why} {entry} is bank {:?}, {} cue(s)",
                        bank.name,
                        bank.sound_names().len()
                    ));
                    by_label.insert(bank.name.clone(), bank);
                }
                Err(e) => parsed
                    .report
                    .push(format!("track audio: {entry} is not a sound bank: {e}")),
            }
        }

        // Decoded once per distinct pair, and the *reason* a pair failed is
        // cached with it: "no bank here spells that label", "that bank has no
        // such cue" and "the cue binds no waveform" are three different
        // findings and only the middle one is what
        // `track-sound-emitters.md` calls a dangling reference.
        let mut cache: BTreeMap<(String, String), Result<Loaded, String>> = BTreeMap::new();
        let mut unplayed: BTreeMap<(String, String), (usize, String)> = BTreeMap::new();
        // Both lists, together: a cone names a bank and a cue exactly the way
        // a plain `sound` does, and the resolution and its failure modes are
        // the same code either way.
        for node in parsed.omni.iter_mut().chain(&mut parsed.directional) {
            let key = (node.emitter.bank.clone(), node.emitter.cue.clone());
            let loaded = cache
                .entry(key.clone())
                .or_insert_with(|| {
                    let Some(bank) = by_label.get(&node.emitter.bank) else {
                        return Err(format!(
                            "no bank this circuit loads is labelled {:?}",
                            node.emitter.bank
                        ));
                    };
                    load_named_cue(bank, &node.emitter.cue)
                        .map(|(loaded, _)| loaded)
                        .map_err(|e| e.to_string())
                })
                .clone();
            match loaded {
                Ok(loaded) => node.sound = Some(loaded),
                Err(why) => {
                    let entry = unplayed.entry(key).or_insert((0, why));
                    entry.0 += 1;
                }
            }
        }

        let total = parsed.omni.len() + parsed.directional.len();
        let playing = parsed
            .omni
            .iter()
            .chain(&parsed.directional)
            .filter(|n| n.sound.is_some())
            .count();
        parsed.report.push(format!(
            "track audio: {playing} of {total} emitter(s) resolved to a cue"
        ));
        // Reported per reference rather than as a total, so a decode that broke
        // a *different* reference could not hide behind breaking as many as it
        // fixed - the same reason `sound_emitter_ground_truth` pins a list.
        for ((bank, cue), (nodes, why)) in &unplayed {
            parsed.report.push(format!(
                "track audio: {nodes} node(s) name {bank}{cue} and play nothing: {why}"
            ));
        }
        for line in &parsed.report {
            info!("{line}");
        }
        parsed
    }

    /// How many emitters the listener is inside the radius (and cone, where
    /// there is one) of, this tick.
    ///
    /// The budget question, answered off the recovered law rather than
    /// estimated: `SoundEmitter_ServiceRequests` refuses to touch a request
    /// whose emitter is out of range, so this is exactly the set of authored
    /// cues that want a voice. See
    /// `docs/ghidra/functions/psp-pulse-usa/positional-audio.md`.
    #[must_use]
    pub fn in_range(&self, listener: &oag_audio::Listener) -> usize {
        self.placed(listener).count()
    }

    /// Every emitter, [`Self::omni`] then [`Self::directional`], in that
    /// concatenated order - the order [`Ambience`] indexes its voices by.
    fn all(&self) -> impl Iterator<Item = &Authored> {
        self.omni.iter().chain(&self.directional)
    }

    /// Every in-range emitter, as its index into [`Self::all`] and where it is
    /// heard from.
    pub fn placed<'a>(
        &'a self,
        listener: &'a oag_audio::Listener,
    ) -> impl Iterator<Item = (usize, oag_audio::Placed)> + 'a {
        self.all().enumerate().filter_map(move |(at, node)| {
            // The volume `Sound_Play` is handed at every recovered call site,
            // and `VexSound_Init`'s is no exception - it passes `1.0f`.
            Some((at, placed_emitter(&node.emitter).place(listener, 1.0)?))
        })
    }
}

/// Builds the `oag_audio::Emitter` a node's own decode already specifies.
///
/// **The radius is [`SoundEmitter::radius`], not
/// [`SoundEmitter::sample_radius`]** - see this module's own header for why:
/// `VexSound_Update`'s curve resample has two static call sites, both gated on
/// a payload byte that is `0` on every one of the 1,298 authored nodes, so the
/// curve is never actually driven for anything Pulse ships and `+0x10` is the
/// one radius that plays. Using the curve here would be silently wrong for a
/// `soundcone` in particular: its own value key is `0` on all 134 of them.
fn placed_emitter(node: &SoundEmitter) -> oag_audio::Emitter {
    oag_audio::Emitter {
        position: node.position(),
        radius: node.radius,
        cone: node.cone.map(|cone| oag_audio::Cone {
            // Row `1` of the emitter's own world matrix, raw - see
            // `oag_audio::spatial`'s own header for why this is not
            // renormalised.
            axis: [node.to_world[4], node.to_world[5], node.to_world[6]],
            half_angle: cone.wide(),
        }),
    }
}

/// The archive entry holding the circuit's own sound bank, if it names one.
///
/// `trackstartup.xml` sits beside the circuit's `.vex` and its
/// `<LoadSoundBank Filename="...">` names a file in that same directory -
/// **not** under `Data\Sound\` where every bank the executable names by a
/// literal string lives. Measured: `Data\Sound\BASILICO_ENV.bnk` hashes to
/// nothing on `pulse-psp-usa`, and `Data\Environments\01_Track\BASILICO_ENV.bnk`
/// is a 253,664-byte `SBlk` whose own label is `basilic` - which is exactly what
/// `01_Track`'s fifty non-`gentrak` emitters spell.
fn circuit_bank_entry(archives: &mut Archives, track: &str) -> Option<String> {
    let at = track.rfind(['/', '\\'])?;
    // The circuit's own separator, kept rather than normalised: Pulse spells a
    // path with `\\` and Wipeout HD with `/`, and a name that mixes them reads
    // like a bug even where the archive's hash tolerates it.
    let (directory, separator) = (&track[..at], &track[at..=at]);
    let manifest = archives
        .read_name(&format!("{directory}{separator}trackstartup.xml"))
        .ok()?;
    let file = oag_formats::trackstartup::TrackStartup::parse(&String::from_utf8_lossy(&manifest))
        .sound_bank?;
    Some(format!("{directory}{separator}{file}"))
}

/// The held voices a circuit's ambience owns, one slot per authored emitter.
///
/// Held rather than fired, because these cues loop and the original opens each
/// of them once, at load. See this module's own header.
#[derive(Debug, Default)]
pub struct Ambience {
    /// Index-parallel to [`TrackEmitters::all`], `omni` then `directional`;
    /// [`None`] where the emitter is out of range and so has no voice at all.
    voices: Vec<Option<oag_audio::VoiceId>>,
}

impl Ambience {
    /// How many of the circuit's emitters are sounding right now.
    ///
    /// Asked of the mixer rather than counted off the slots, because a slot
    /// holds a [`oag_audio::VoiceId`] for as long as its emitter is in range
    /// and a one-shot alternate will have ended inside that window - see
    /// [`Self::tick`]'s finished-cue arm.
    #[must_use]
    pub fn playing(&self, mixer: &oag_audio::Mixer) -> usize {
        self.voices
            .iter()
            .flatten()
            .filter(|id| mixer.is_playing(**id))
            .count()
    }

    /// Opens, moves and closes each emitter's voice for this tick.
    ///
    /// # The one place this knowingly differs from the original
    ///
    /// `SoundEmitter_ServiceRequests` (`0x089394dc`) leaves a latched request
    /// **completely untouched** - not updated and, per `positional-audio.md`,
    /// "not even reaped". So the original's voice keeps playing at whatever
    /// gain the last in-range frame gave it, which is near zero because the
    /// linear falloff reaches zero exactly at the radius. What reclaims that
    /// voice is a path this project has not read.
    ///
    /// **This stops the voice instead**, which is the same reading [`super`]
    /// already applies to the other half of the latch: `Sound_Play` refuses to
    /// *start* a cue on a latched emitter, so a rival's scrape on the far side
    /// of the circuit is not started rather than started silent. Holding 97
    /// silent loops open against a 32-voice pool would starve the race's own
    /// cues to reproduce something inaudible either way. The gap is the unread
    /// reclamation path, and it is written down here rather than made to look
    /// like a decision.
    pub fn tick(
        &mut self,
        mixer: &mut oag_audio::Mixer,
        emitters: &TrackEmitters,
        listener: &oag_audio::Listener,
        rng: &mut oag_core::Rng,
    ) {
        self.voices
            .resize(emitters.omni.len() + emitters.directional.len(), None);
        for (node, held) in emitters.all().zip(&mut self.voices) {
            let placed = placed_emitter(&node.emitter).place(listener, 1.0);
            match (placed, *held) {
                // In range with a voice open: this is the per-frame
                // `SoundInstance_UpdateSpatial`, minus the doppler term. An
                // authored emitter does not move, so the only distance change
                // is the listener's own, which the original gates on the
                // camera-cut guard at `mgr+0x8d` rather than reading here.
                (Some(placed), Some(id)) if mixer.is_playing(id) => {
                    mixer.set_gain(id, placed.gain);
                    mixer.set_pan(id, Some(placed.pan));
                }
                // **In range, opened once, and finished: leave it alone.**
                // Not every alternate loops - `platinu~BIRDS` binds sixteen
                // waveforms and only some of them do, and five cues across the
                // disc are mixed like that - so a cue can end while its
                // emitter is still in range. The original runs `Sound_Play`
                // once, at construction, and nothing re-runs it while the
                // latch is clear; restarting here would machine-gun a one-shot
                // at 60 Hz, which sounds like a broken sample rather than like
                // an absence and so would survive listening to it.
                (Some(_), Some(_)) => {}
                // In range with nothing open: open one. A voice the pool
                // refuses is already counted by `Mixer::starved`, and an
                // emitter whose cue never resolved holds `None` forever.
                (Some(placed), None) => {
                    *held = node.sound.as_ref().and_then(|loaded| {
                        let (sound, looping) = pick(loaded, rng)?;
                        let play = if looping {
                            oag_audio::Play::looping(sound, oag_audio::Bus::Sfx)
                        } else {
                            oag_audio::Play::once(sound, oag_audio::Bus::Sfx)
                        };
                        mixer.play(oag_audio::Play {
                            gain: placed.gain,
                            pan: Some(placed.pan),
                            ..play
                        })
                    });
                }
                (None, Some(id)) => {
                    mixer.stop(id);
                    *held = None;
                }
                (None, None) => {}
            }
        }
    }

    /// Releases every voice this holds, which is what leaving a race does.
    pub fn stop(&mut self, mixer: &mut oag_audio::Mixer) {
        for id in self.voices.drain(..).flatten() {
            mixer.stop(id);
        }
    }
}

/// Which of a cue's waveforms an emitter opens.
///
/// A uniform draw, the same rule [`super::Banks::pick`] reads off opcode
/// `0x19`, minus its no-repeat cache: that cache matters for a one-shot fired
/// over and over, where this is opened once and then loops for the whole race.
fn pick(
    loaded: &Loaded,
    rng: &mut oag_core::Rng,
) -> Option<(std::sync::Arc<oag_audio::Sound>, bool)> {
    let at = match u32::try_from(loaded.waveforms.len()) {
        Ok(len) if len > 1 => rng.below(len) as usize,
        _ => 0,
    };
    loaded.waveforms.get(at).cloned()
}

#[cfg(test)]
mod tests;
