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

use oag_formats::sound_emitters::{self, SoundEmitter};

/// A circuit's authored emitters, split by what this port can honestly play.
#[derive(Debug, Default, Clone)]
pub struct TrackEmitters {
    /// The omnidirectional `sound` `0x3e1` nodes, in authored order.
    pub omni: Vec<SoundEmitter>,
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
    /// Reads every authored emitter out of one circuit's `.vex`.
    ///
    /// **Never fails.** A `.vex` with no nodes, a Zone circuit (which authors
    /// none of the three classes at all) and a title that has never been swept
    /// all produce an empty result and a report line, on the same terms
    /// [`super::Banks::load`] reports a bank it could not read: silence with a
    /// reason beats a race that will not start.
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
        let omni: Vec<_> = authored.into_iter().filter(|e| e.cone.is_none()).collect();
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
        self.omni.iter().enumerate().filter_map(move |(at, e)| {
            let emitter = oag_audio::Emitter {
                position: e.position(),
                radius: e.sample_radius(frame),
            };
            // The volume `Sound_Play` is handed at every recovered call site,
            // and `VexSound_Init`'s is no exception - it passes `1.0f`.
            Some((at, emitter.place(listener, 1.0)?))
        })
    }
}

#[cfg(test)]
mod tests;
