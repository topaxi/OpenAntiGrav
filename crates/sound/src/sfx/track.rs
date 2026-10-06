//! A circuit's own authored sound emitters: the ambience a track carries with
//! its geometry, rather than a cue any craft fires.
//!
//! [`super`] plays the cues a race raises; this is the other half of what a
//! circuit sounds like: 1,298 `sound` and `soundcone` nodes authored across the
//! twelve Pulse circuits, each naming a bank, a cue and a radius.
//! `oag_vex::sound_emitters` decodes them and
//! `docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md` is the
//! evidence; this module is the wiring above it.
//!
//! # Every emitter is live from construction
//!
//! `VexSound_Init` (`0x089259a4`) allocates the emitter and calls `Sound_Play`
//! on the spot, so the original starts all of a circuit's ambience at load, and
//! the out-of-range latch in `SoundEmitter_ServiceRequests` (`0x089394dc`) keeps
//! a distant one silent.
//!
//! This port starts a cue on approach, its one knowing deviation, because it
//! also stops one on departure, where the original leaves a latched request
//! untouched (reasoning in [`Ambience::tick`]). A looping emitter therefore
//! restarts from sample zero on each pass, which matters when comparing a
//! capture against the original.
//!
//! # The cone is wired
//!
//! `soundcone` `0x3e9` carries two authored angles and its own enable byte.
//! `VexSoundCone_Init` (`0x08925ff4`, `track-sound-emitters.md`) is a four-line
//! wrapper around `VexSound_Init` that copies the node's `+0x00` to the
//! emitter's `+0x40` half-angle: `Cone::wide()`, never smaller than the other
//! angle on any of the 134 nodes. [`TrackEmitters::directional`] carries them,
//! placed by [`oag_audio::Emitter::cone`], as [`TrackEmitters::omni`] uses the
//! same law.
//!
//! A cone's radius is `SoundEmitter::radius`, not `sample_radius`:
//! `VexSound_Update`, the only function that resamples the radius curve, has two
//! static call sites, both gated on the payload's `+0x3c`, which is `0` on all
//! 1,298 nodes. The curve is never driven, so a cone's curve value key reading
//! `0` on all 134 is dead data. Reading it as the radius would silence every
//! cone; `+0x10`, what `Init` writes, plays the authored value.
//!
//! A cone inside its radius but outside its angle still holds a voice at zero
//! gain, a straight read of the law: `oag_audio::spatial::Emitter::place` only
//! refuses past the radius and the angle term is a multiplier that can reach
//! zero (the distinction `docs/.../positional-audio.md` draws for the radius).
//! So [`Ambience::tick`] holds a silent voice for a cone whose angle the
//! listener never enters, a real cost against the 32-voice pool, not yet
//! measured as the omnidirectional census was.

use std::collections::BTreeMap;

use oag_assets::source::Archives;
use oag_formats::sblk;
use oag_vex::sound_emitters::{self, SoundEmitter};

use super::{Loaded, banks::load_track_cue};

/// One authored emitter, with the audio its cue names where it resolves.
#[derive(Debug, Clone)]
pub struct Authored {
    /// The node, as `oag_vex::sound_emitters` decoded it.
    pub emitter: SoundEmitter,
    /// The cue's waveforms, or [`None`] where the reference does not resolve.
    /// The cue's waveforms, or [`None`] where the reference does not resolve.
    ///
    /// [`None`] is reported, with its reason, because the three causes are
    /// different findings:
    ///
    /// 1. **The reference dangles.** Five on the Pulse disc name a cue or bank
    ///    that does not exist (the disc's own bugs, `track-sound-emitters.md`);
    ///    whether the original drops one or falls back to a bank-index lookup is
    ///    unread (`Scream_FindSoundInBank`'s name comparison is not decompiled).
    /// 2. **The cue binds no waveform**, its command run being opcodes
    ///    `oag_formats::sblk` does not read: 38 nodes across eight circuits,
    ///    the report naming `0x1e` on every `~SetReg*` cue and `0x14` on
    ///    `moather~birds`, `dekonst~CRANE` and both of `talonsj`'s. This port's
    ///    gap, and not on the page's dangling list, which checked the name
    ///    table and not the command run.
    /// 3. **No loaded bank carries that label**, what a title with no shared
    ///    track bank gets for every `gentrak` node.
    ///
    /// All three play nothing and say so (`CLAUDE.md`).
    pub sound: Option<Loaded>,
}

/// A circuit's authored emitters, split by shape rather than by what plays.
#[derive(Debug, Default, Clone)]
pub struct TrackEmitters {
    /// The omnidirectional `sound` `0x3e1` nodes, in authored order.
    pub omni: Vec<Authored>,
    /// The directional `soundcone` `0x3e9` nodes, in authored order. Split from
    /// [`Self::omni`] because [`Ambience::tick`] places each under its own law.
    pub directional: Vec<Authored>,
    /// What parsing did, for the race's own report.
    pub report: Vec<String>,
}

impl TrackEmitters {
    /// Reads every authored emitter out of one circuit's `.vex`, unresolved.
    ///
    /// Never fails: a `.vex` with no nodes, a Zone circuit (none of the three
    /// classes) or a never-swept title gives an empty result and a report line,
    /// as [`super::Banks::load`] does. Every [`Authored::sound`] is [`None`];
    /// [`Self::load`] resolves them.
    #[must_use]
    pub fn parse(track: &str, blob: &[u8]) -> Self {
        let mut report = Vec::new();
        let Ok(nodes) = oag_vex::vex::nodes(blob) else {
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
    /// # Where a circuit's banks are
    ///
    /// An emitter spells its bank by that bank's seven-character **label**
    /// (`basilic`, `gentrak`), not a path or truncation
    /// (`docs/formats/psp-audio.md`), so the two banks are opened by name and
    /// matched on the label each reports:
    ///
    /// 1. The circuit's own bank is named by its `trackstartup.xml`
    ///    `<LoadSoundBank Filename="...">` and sits beside it:
    ///    `Data\Environments\01_Track\BASILICO_ENV.bnk`, 253,664 bytes, label
    ///    `basilic`.
    /// 2. The shared banks are `oag_title::SoundBanks::track`'s list, the entries
    ///    no cue names and the executable does.
    ///
    /// A cue is decoded once per distinct pair, not per node (`~ELEVATOR`
    /// authored four times is one `Loaded` shared four ways).
    #[must_use]
    pub fn load(
        archives: &mut Archives,
        banks: &oag_title::SoundBanks,
        track: &str,
        blob: &[u8],
    ) -> Self {
        let mut parsed = Self::parse(track, blob);

        // Read whole, parsed after: `sblk::Bank` borrows the blob.
        let mut blobs: Vec<(&str, String, Vec<u8>)> = Vec::new();
        // Whether some bank this circuit should have loaded was not: a miss is
        // only the disc's dangling reference when none was missing.
        let mut a_bank_is_missing = false;
        if banks.track.shared.is_empty() {
            parsed.report.push(
                "track audio: this title names no shared track bank, so only a circuit's own \
                 bank can resolve"
                    .to_string(),
            );
        }
        for entry in banks.track.shared {
            match archives.read_name(entry) {
                Ok(bytes) => blobs.push(("shared", (*entry).to_string(), bytes)),
                Err(e) => {
                    a_bank_is_missing = true;
                    parsed.report.push(format!(
                        "track audio: {entry} not read: {e}; every emitter naming its label plays nothing"
                    ));
                }
            }
        }
        let candidates = circuit_bank_entries(archives, banks.track.circuit, track);
        if candidates.is_empty() {
            a_bank_is_missing = true;
            parsed.report.push(format!(
                "track audio: no trackstartup.xml beside {track} names a sound bank, so only the \
                 shared bank(s) can resolve"
            ));
        } else {
            let mut last = None;
            let mut found = None;
            for entry in &candidates {
                match archives.read_name(entry) {
                    Ok(bytes) => {
                        found = Some((entry.clone(), bytes));
                        break;
                    }
                    Err(e) => last = Some(e),
                }
            }
            match (found, last) {
                (Some((entry, bytes)), _) => blobs.push(("circuit", entry, bytes)),
                (None, Some(e)) => {
                    a_bank_is_missing = true;
                    parsed.report.push(format!(
                        "track audio: {} not read: {e}; every emitter naming its label plays nothing",
                        candidates.join(" or ")
                    ));
                }
                (None, None) => {}
            }
        }

        // Keyed by the bank's own label, which a node spells and which is no
        // truncation of the path.
        let mut by_label: BTreeMap<String, sblk::Bank<'_>> = BTreeMap::new();
        for (why, entry, bytes) in &blobs {
            match sblk::Bank::parse(bytes) {
                Ok(bank) => {
                    // A hashed bank keeps no 16-byte names, so its count is the
                    // cue table's.
                    let cues = if bank.is_hashed() {
                        usize::from(bank.cue_count)
                    } else {
                        bank.sound_names().len()
                    };
                    parsed.report.push(format!(
                        "track audio: {why} {entry} is bank {:?}, {cues} cue(s)",
                        bank.name,
                    ));
                    by_label.insert(bank.name.clone(), bank);
                }
                Err(e) => {
                    a_bank_is_missing = true;
                    parsed.report.push(format!(
                        "track audio: {entry} is not a sound bank: {e}; every emitter naming its \
                         label plays nothing"
                    ));
                }
            }
        }

        // Decoded once per distinct pair, with the failure kind cached. A miss
        // is only a *dangling reference* when every bank this circuit tried to
        // load parsed: a Wwise bank that did not parse (Omega) makes every
        // label look unmatched, and that is an absence, not the disc's bug.
        let every_bank_read = !a_bank_is_missing;
        let mut cache: BTreeMap<(String, String), Result<Loaded, (Miss, String)>> = BTreeMap::new();
        let mut unplayed: BTreeMap<(String, String), (usize, (Miss, String))> = BTreeMap::new();
        // Both lists together: a cone names bank and cue as a plain `sound` does.
        for node in parsed.omni.iter_mut().chain(&mut parsed.directional) {
            let key = (node.emitter.bank.clone(), node.emitter.cue.clone());
            let loaded = cache
                .entry(key.clone())
                .or_insert_with(|| {
                    if key.0.is_empty() && key.1.is_empty() {
                        return Err((Miss::Unassigned, String::new()));
                    }
                    let Some(bank) = by_label.get(&node.emitter.bank) else {
                        let kind = if every_bank_read {
                            Miss::Dangling
                        } else {
                            Miss::Absent
                        };
                        return Err((
                            kind,
                            format!(
                                "no bank this circuit loads is labelled {:?}",
                                node.emitter.bank
                            ),
                        ));
                    };
                    load_track_cue(bank, &node.emitter.cue)
                        .map(|(loaded, _)| loaded)
                        .map_err(|e| {
                            let kind = if e.downcast_ref::<super::banks::ControlOnlyCue>().is_some()
                            {
                                Miss::Control
                            } else if e.downcast_ref::<super::banks::NoSuchCue>().is_some() {
                                Miss::Dangling
                            } else {
                                Miss::Absent
                            };
                            (kind, e.to_string())
                        })
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
        // Reported per reference, so a decode that broke a different reference
        // cannot hide behind fixing as many (`sound_emitter_ground_truth` pins a list).
        // Only an absence (a bank that would not read, a cue that decodes to
        // nothing) says "play nothing" on its own line: a dangling reference is
        // the disc's own authoring and gets one summary line instead.
        let (mut dangling_cues, mut dangling_nodes) = (0, 0);
        let mut unassigned = 0;
        for ((bank, cue), (nodes, (miss, why))) in &unplayed {
            match miss {
                Miss::Control => parsed.report.push(format!(
                    "track audio {bank}{cue}: {nodes} node(s) are control only: {why}"
                )),
                Miss::Absent => parsed.report.push(format!(
                    "track audio {bank}{cue}: {nodes} node(s) play nothing: {why}"
                )),
                Miss::Dangling => {
                    dangling_cues += 1;
                    dangling_nodes += nodes;
                    parsed.report.push(format!(
                        "track audio {bank}{cue}: {nodes} node(s) dangle: {why}"
                    ));
                }
                Miss::Unassigned => unassigned += nodes,
            }
        }
        if unassigned > 0 {
            parsed.report.push(format!(
                "track audio: {unassigned} node(s) author no bank and no cue name, an exporter \
                 default nothing was assigned to"
            ));
        }
        if dangling_cues > 0 {
            parsed.report.push(format!(
                "track audio: {dangling_cues} cue(s) the circuit names and its banks do not \
                 author ({dangling_nodes} node(s)) play nothing"
            ));
        }

        parsed
    }

    /// How many emitters the listener is inside the radius (and cone) of this
    /// tick: exactly the cues that want a voice, since
    /// `SoundEmitter_ServiceRequests` refuses an out-of-range request
    /// (`docs/ghidra/functions/psp-pulse-usa/positional-audio.md`).
    #[must_use]
    pub fn in_range(&self, listener: &oag_audio::Listener) -> usize {
        self.placed(listener).count()
    }

    /// Every emitter, [`Self::omni`] then [`Self::directional`]: the order
    /// [`Ambience`] indexes its voices by.
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
            // The volume every recovered `Sound_Play` call site passes, `VexSound_Init`'s included.
            Some((at, placed_emitter(&node.emitter).place(listener, 1.0)?))
        })
    }
}

/// Why one authored reference plays nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Miss {
    /// The cue runs only no-ops and register writes: silent by design.
    Control,
    /// The bank parsed and spells no such cue, or every bank parsed and none
    /// carries the label: the disc's own dangling reference.
    Dangling,
    /// No bank and no cue named at all: a node nothing was assigned to.
    Unassigned,
    /// A bank that would not read, or a cue that decodes to no waveform.
    Absent,
}

/// Builds the `oag_audio::Emitter` a node's own decode specifies.
///
/// The radius is [`SoundEmitter::radius`], not [`SoundEmitter::sample_radius`]
/// (see the module header): the curve resample is never driven for anything
/// Pulse ships, and using it would silence a `soundcone` (value key `0` on all
/// 134).
fn placed_emitter(node: &SoundEmitter) -> oag_audio::Emitter {
    oag_audio::Emitter {
        position: node.position(),
        radius: node.radius,
        cone: node.cone.map(|cone| oag_audio::Cone {
            // Row `1` of the emitter's world matrix, raw (not renormalised, see
            // `oag_audio::spatial`).
            axis: [node.to_world[4], node.to_world[5], node.to_world[6]],
            half_angle: cone.wide(),
        }),
    }
}

/// The archive entries the circuit's own sound bank may be, in the order they
/// are tried; empty when its manifest names none.
///
/// `trackstartup.xml` sits beside the `.vex` and its `<LoadSoundBank
/// Filename="...">` names a file. On Pulse, Pure and HD that file is in the
/// track's directory, not under `Data\Sound\` where the executable's
/// literal-named banks live. Pulse, measured:
/// `Data\Sound\BASILICO_ENV.bnk` hashes to nothing on `pulse-psp-usa`, while
/// `Data\Environments\01_Track\BASILICO_ENV.bnk` is a 253,664-byte `SBlk`
/// labelled `basilic`, which `01_Track`'s fifty non-`gentrak` emitters spell.
///
/// 2048 reads no bank beside the track at all: its loader formats
/// `data/audio/sound/%s` and, when that file does not exist, `data/audio/DLC1/%s`
/// ([`oag_title::CircuitBanks::Directories`]), so the copies shipped beside its
/// downloadable circuits are never loaded.
fn circuit_bank_entries(
    archives: &mut Archives,
    circuit: oag_title::CircuitBanks,
    track: &str,
) -> Vec<String> {
    let Some(at) = track.rfind(['/', '\\']) else {
        return Vec::new();
    };
    let Some(file) = circuit_manifest(archives, track).and_then(|m| m.sound_bank) else {
        return Vec::new();
    };
    match circuit {
        oag_title::CircuitBanks::BesideTrack => {
            let (beside, separator) = (&track[..at], &track[at..=at]);
            vec![format!("{beside}{separator}{file}")]
        }
        oag_title::CircuitBanks::Directories(directories) => directories
            .iter()
            .map(|directory| format!("{directory}\\{file}"))
            .collect(),
    }
}

/// The circuit's own `trackstartup.xml`, parsed, or `None` where it ships none:
/// one read for the sound bank above and, from `oag_raceplay`'s scenery effects, its weather.
pub fn circuit_manifest(
    archives: &mut Archives,
    track: &str,
) -> Option<oag_tables::trackstartup::TrackStartup> {
    let at = track.rfind(['/', '\\'])?;
    // The circuit's own separator is kept: Pulse spells paths with `\\`, HD with
    // `/`, and a mix reads like a bug even where the archive's hash tolerates it.
    let (directory, separator) = (&track[..at], &track[at..=at]);
    let manifest = archives
        .read_name(&format!("{directory}{separator}trackstartup.xml"))
        .ok()?;
    Some(oag_tables::trackstartup::TrackStartup::parse(
        &String::from_utf8_lossy(&manifest),
    ))
}

/// The held voices a circuit's ambience owns, one slot per authored emitter:
/// held because these cues loop and the original opens each once at load.
#[derive(Debug, Default)]
pub struct Ambience {
    /// Index-parallel to [`TrackEmitters::all`], `omni` then `directional`;
    /// [`None`] where the emitter is out of range and so has no voice at all.
    voices: Vec<Option<oag_audio::VoiceId>>,
    /// Index-parallel to [`Self::voices`]: each voice's last-frame distance for
    /// the doppler term (the emitter is fixed, so the change is the listener's).
    dopplers: Vec<oag_audio::Doppler>,
    /// The authored volume group the emitters play on (`user8`), when the
    /// title has an authored mix; the effects bus otherwise.
    bus: Option<oag_audio::Bus>,
}

impl Ambience {
    /// Plays every emitter on `bus`, an authored group.
    #[must_use]
    pub fn on_bus(mut self, bus: Option<oag_audio::Bus>) -> Self {
        self.bus = bus;
        self
    }

    /// How many of the circuit's emitters are sounding now. Asked of the mixer,
    /// not the slots: a slot holds a [`oag_audio::VoiceId`] while in range and a
    /// one-shot alternate will have ended meanwhile ([`Self::tick`]'s
    /// finished-cue arm).
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
    /// completely untouched (per `positional-audio.md`, "not even reaped"), so
    /// the original's voice keeps playing at the last in-range gain, near zero
    /// since the falloff reaches zero at the radius. What reclaims it is
    /// unread.
    ///
    /// This stops the voice instead, matching [`super`]'s reading of the latch's
    /// other half: `Sound_Play` refuses to start a cue on a latched emitter, so a
    /// distant rival's scrape is not started rather than started silent. Holding
    /// 97 silent loops against a 32-voice pool would starve the race's cues to
    /// reproduce something inaudible. The gap is the unread reclamation path,
    /// recorded here rather than dressed as a decision.
    pub fn tick(
        &mut self,
        mixer: &mut oag_audio::Mixer,
        emitters: &TrackEmitters,
        listener: &oag_audio::Listener,
        rng: &mut oag_core::Rng,
        doppler_enabled: bool,
        dt: f32,
    ) {
        let count = emitters.omni.len() + emitters.directional.len();
        self.voices.resize(count, None);
        self.dopplers.resize(count, oag_audio::Doppler::default());
        for ((node, held), doppler) in emitters.all().zip(&mut self.voices).zip(&mut self.dopplers)
        {
            let placed = placed_emitter(&node.emitter).place(listener, 1.0);
            match (placed, *held) {
                // In range with a voice open: the per-frame
                // `SoundInstance_UpdateSpatial`, doppler included (the distance
                // change is the listener's); `doppler_enabled` is the
                // camera-cut guard at `mgr+0x8d`, the caller's as the manager's
                // one answer covers every voice this tick.
                (Some(placed), Some(id)) if mixer.is_playing(id) => {
                    mixer.set_gain(id, placed.gain);
                    mixer.set_pan(id, Some(placed.pan));
                    mixer.set_pitch(id, doppler.ratio(placed.distance, dt, doppler_enabled));
                }
                // In range, opened once, finished: leave it alone. Not every
                // alternate loops (`platinu~BIRDS` binds sixteen waveforms, only
                // some loop; five cues on the disc are mixed so), so a cue can end
                // while its emitter is in range. The original runs `Sound_Play`
                // once at construction; restarting here would machine-gun a
                // one-shot at 60 Hz, which sounds like a broken sample and so
                // would survive listening.
                (Some(_), Some(_)) => {}
                // In range with nothing open: open one. A refused voice is counted
                // by `Mixer::starved`; a never-resolved cue holds `None` forever.
                (Some(placed), None) => {
                    // Seeds the doppler with this frame's distance, so the first
                    // held frame reads no change.
                    doppler.reset();
                    doppler.ratio(placed.distance, dt, false);
                    *held = node.sound.as_ref().and_then(|loaded| {
                        let (sound, looping) = pick(loaded, rng)?;
                        let bus = self.bus.unwrap_or(oag_audio::Bus::Sfx);
                        let play = if looping {
                            oag_audio::Play::looping(sound, bus)
                        } else {
                            oag_audio::Play::once(sound, bus)
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
                    doppler.reset();
                }
                (None, None) => {}
            }
        }
    }

    /// Releases every voice this holds, on leaving a race.
    pub fn stop(&mut self, mixer: &mut oag_audio::Mixer) {
        for id in self.voices.drain(..).flatten() {
            mixer.stop(id);
        }
    }
}

/// Which of a cue's waveforms an emitter opens: a uniform draw, as
/// [`super::Banks::pick`] reads off opcode `0x19` minus its no-repeat cache,
/// which matters for a one-shot fired repeatedly, not one opened once.
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
