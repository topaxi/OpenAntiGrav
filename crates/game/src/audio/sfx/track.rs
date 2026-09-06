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
//! on the spot, so a circuit does not start its ambience when the player comes
//! near it - it starts all of it at load, and the out-of-range latch in
//! `SoundEmitter_ServiceRequests` (`0x089394dc`) is what keeps a distant one
//! silent. That distinction is not cosmetic: a design that started a cue on
//! approach would fade one in every time the player rounded a corner, and this
//! one does not.
//!
//! # The cone is deliberately absent
//!
//! `soundcone` `0x3e9` carries two authored angles and its own enable byte, and
//! **which of them reaches the emitter's `+0x40` half-angle is unread** -
//! `soundcone`'s own init has not been found. Per `CLAUDE.md`, an effect whose
//! trigger is unrecovered stays unwired: a cone played as a sphere would be a
//! stand-in for something the disc already specifies, audible and plausible and
//! wrong. [`TrackEmitters::cones`] counts them so the absence is a number in
//! the load report rather than a silence.

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
    /// 2. **The cue binds no waveform.** `14_Track`'s `~SETREG_01` and
    ///    `~SETREG_02` are in `fortcle` and each runs a single command that is
    ///    not one of the opcodes `oag_formats::sblk` reads. Six nodes ask for
    ///    them. That is this port's gap rather than the disc's, and it is not
    ///    on `track-sound-emitters.md`'s dangling list because that sweep
    ///    checked the name table and not the command run.
    /// 3. **No loaded bank carries that label**, which is what a title with no
    ///    shared track bank gets for every `gentrak` node.
    ///
    /// All three play nothing and say so, which is what `CLAUDE.md` asks of an
    /// asset that will not resolve.
    pub sound: Option<Loaded>,
}

/// A circuit's authored emitters, split by what this port can honestly play.
#[derive(Debug, Default, Clone)]
pub struct TrackEmitters {
    /// The omnidirectional `sound` `0x3e1` nodes, in authored order.
    pub omni: Vec<Authored>,
    /// How many `soundcone` `0x3e9` nodes the circuit authors.
    ///
    /// Counted and not played, for the reason this module's own doc comment
    /// gives. A count rather than a list because nothing downstream may act on
    /// one; what it is for is the load report.
    pub cones: usize,
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
        let cones = authored.iter().filter(|e| e.cone.is_some()).count();
        let omni: Vec<_> = authored
            .into_iter()
            .filter(|e| e.cone.is_none())
            .map(|emitter| Authored {
                emitter,
                sound: None,
            })
            .collect();
        report.push(format!(
            "track audio: {track} authors {} omnidirectional emitter(s) and {cones} cone(s); \
             the cones are not played, because which of a cone's two authored angles reaches \
             the emitter's half-angle is unread",
            omni.len(),
        ));
        Self {
            omni,
            cones,
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
        for node in &mut parsed.omni {
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

        let playing = parsed.omni.iter().filter(|n| n.sound.is_some()).count();
        parsed.report.push(format!(
            "track audio: {playing} of {} emitter(s) resolved to a cue",
            parsed.omni.len()
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

    /// How many emitters the listener is inside the radius of, this tick.
    ///
    /// The budget question, answered off the recovered law rather than
    /// estimated: `SoundEmitter_ServiceRequests` refuses to touch a request
    /// whose emitter is out of range, so this is exactly the set of authored
    /// cues that want a voice. See
    /// `docs/ghidra/functions/psp-pulse-usa/positional-audio.md`.
    #[must_use]
    pub fn in_range(&self, listener: &oag_audio::Listener, frame: f32) -> usize {
        self.placed(listener, frame).count()
    }

    /// Every in-range emitter, as its index and where it is heard from.
    ///
    /// `frame` is the emitter's age in curve ticks - the circuit's own tick
    /// count since load, because `VexSound_Update` (`0x08925c4c`) resamples the
    /// radius curve into the emitter every frame rather than reading the `f32`
    /// beside it. Pulse authors one key on all 1,298 nodes so the two agree
    /// today; `sample_radius` is still the honest call and the `f32` the
    /// shortcut.
    pub fn placed<'a>(
        &'a self,
        listener: &'a oag_audio::Listener,
        frame: f32,
    ) -> impl Iterator<Item = (usize, oag_audio::Placed)> + 'a {
        self.omni.iter().enumerate().filter_map(move |(at, node)| {
            let emitter = oag_audio::Emitter {
                position: node.emitter.position(),
                radius: node.emitter.sample_radius(frame),
            };
            // The volume `Sound_Play` is handed at every recovered call site,
            // and `VexSound_Init`'s is no exception - it passes `1.0f`.
            Some((at, emitter.place(listener, 1.0)?))
        })
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

#[cfg(test)]
mod tests;
