//! Which audio backend runs and what it reads.

/// Which audio backend runs and what it reads.
#[derive(clap::Args, Debug)]
pub(crate) struct AudioArgs {
    /// Write everything the mixer produced to a WAV instead of to a device.
    ///
    /// The audio counterpart of `--screenshot`, and for the same reason: this
    /// project's runs are headless and CI has no sound card, so a file is the
    /// only end-to-end evidence available that a sound was made at all.
    ///
    /// **Forces the null backend.** With a device attached `cpal`'s callback is
    /// already draining the mixer from its own thread, and pulling the same
    /// samples here would race it - so a run that dumps is silent on the
    /// speakers by construction rather than by accident.
    ///
    /// Length is a function of the tick count alone: `--ticks 600` is ten
    /// seconds whatever the machine's frame rate. Without
    /// `--screenshot` the window writes the file when it closes and the buffer
    /// grows (about 384 KB a second) until then: for a scripted walk, not a
    /// play session.
    #[arg(long, value_name = "FILE")]
    pub(crate) dump_audio: Option<std::path::PathBuf>,

    /// Force the null audio backend: no device, no dump, nothing played.
    ///
    /// The same forced-null trick `--dump-audio` uses, without the WAV: a
    /// `--screenshot --until` run that has to sit through a movie leg needs the
    /// mixer to be tick-clocked rather than paced by a real device, and a
    /// device attached to the machine otherwise makes that a wall-clock race a
    /// headless run always loses - it finishes in far less real time than the
    /// movie takes to play, so `--until` spends its tick ceiling on a picture
    /// that has barely moved. See `oag_sound::Audio::movie_playhead` and
    /// ADR-0019. `--dump-audio` forces the same backend and works too, but also requires
    /// `--screenshot` and grows a sample buffer for the run; this is the plain way to ask for the tick-clocked movie leg alone. Conflicts with `--tap-audio`, which needs a real stream.
    #[arg(long, conflicts_with_all = ["dump_audio", "tap_audio"])]
    pub(crate) no_audio: bool,

    /// Play without the soundtrack: the front end's loop and the race songs are never read or
    /// decoded, so nothing of them is ever playing (no now-playing line, no spectrum). Effects,
    /// speech and movie sound go on, and so does HD's authored mix. A PS2 song is 35 MiB of PCM
    /// off the disc and an Omega one an ATRAC9 decode, so this also spares a slow machine that
    /// work when the music is not wanted.
    #[arg(long)]
    pub(crate) no_soundtrack: bool,

    /// Read no sound bank for a race: no effect, engine or voice cue is decoded, so none plays
    /// and the load report carries no line about them. The soundtrack is `--no-soundtrack`'s.
    /// For a run that looks at the picture; a decode that nobody hears is most of a race load's
    /// sound work (ATRAC9 on Omega, ADPCM on Pulse).
    #[arg(long)]
    pub(crate) no_effects: bool,
}
