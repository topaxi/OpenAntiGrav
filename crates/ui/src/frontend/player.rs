//! The movie playhead: a frame counter, a pause flag, and the pacing rules.
//!
//! Split out of `crate::movie` rather than defined there: this is the part the
//! intro state machine and [`super::Frontend`] talk to, and it is deliberately
//! free of any decoding or rendering - the original's player never reads the
//! pad either, skipping is the *state's* job. `crate::movie` keeps the demux,
//! the transcode and the decode ring on the other side of that same line.

/// The rate the PSP presents `.PMF` frames at, as a rational.
///
/// PS2 movies are not this rate: a raw MPEG-2 program stream carries its own,
/// read with `ffprobe` in `oag_game::movie::open_mpeg2_ps` rather than assumed.
/// `oag_game::movie::Movie` carries whichever rate is correct for the file it
/// came from, and [`Player`] paces against that rather than a single constant.
pub const FRAME_RATE: (u64, u64) = (30_000, 1001);

/// The display aspect every PS2 movie is drawn at: the frame it fills.
///
/// **Not the `4:3` the `.PSS` cuts declare, which this used to be.** The film's
/// closing card is the `wipEout PULSE` logo, the same artwork as
/// `pulse_logo.mip`: its ink measures 4.20 wide-to-tall where the PSP front end
/// draws that texture and 4.23 with the film scaled to 16:9, against 3.14 at
/// the declared 4:3. The PS2 draws its `Movie` widget with no size of its own
/// over a `640x448` black `Image`, so the film fills the frame - and spelling
/// this `(480, 272)` says whose frame: [`oag_display::space::Space::PS2`]'s. An
/// `IPUF` declares no aspect and takes this as it takes its rate. See
/// `docs/ps2/aspect-ratio.md`.
pub const PS2_DISPLAY_ASPECT: (u32, u32) = (480, 272);

/// Plays a movie: a frame counter, a pause flag, and the pacing rules.
///
/// This is the part the intro state machine talks to, and it is deliberately
/// free of any decoding or rendering. The original's player never reads the pad
/// either; skipping is the *state's* job.
#[derive(Debug)]
pub struct Player {
    frames: usize,
    /// The rate this movie presents frames at. A `.PMF` is always
    /// [`FRAME_RATE`]; a PS2 `.PSS` carries its own - see
    /// `crate::movie::Movie::frame_rate`.
    frame_rate: (u64, u64),
    /// Seconds accumulated toward the next frame.
    accumulator: f64,
    frame: usize,
    /// How many frames have gone by, counting every loop - see
    /// [`Player::position`].
    position: u64,
    paused: bool,
    finished: bool,
    repeat: bool,
    /// Seconds of sound that went by while the picture was held, which the
    /// audio clock has to be read net of.
    ///
    /// Only the `--reel` leg ever holds a movie - see
    /// [`super::PAUSE_FRAMES`] - and it holds for two seconds at a
    /// time. A sound card cannot be held with it, so without this the picture
    /// would jump sixty frames the moment it resumed. Zero on the disc's own
    /// leg, which never pauses.
    held_seconds: f64,
    /// Where the audio clock was when the current hold started, if one is on.
    held_from: Option<f64>,
}

impl Player {
    /// A player positioned at frame zero.
    #[must_use]
    pub fn new(frames: usize, repeat: bool, frame_rate: (u64, u64)) -> Self {
        Self {
            frames,
            frame_rate,
            accumulator: 0.0,
            frame: 0,
            position: 0,
            paused: false,
            finished: frames == 0,
            repeat,
            held_seconds: 0.0,
            held_from: None,
        }
    }

    /// Advances by `dt` seconds.
    pub fn update(&mut self, dt: f64) {
        if self.paused || self.finished {
            return;
        }

        let (num, den) = self.frame_rate;
        let per_frame = den as f64 / num as f64;
        self.accumulator += dt;

        while self.accumulator >= per_frame {
            self.accumulator -= per_frame;
            self.frame += 1;
            self.position += 1;

            if self.frame >= self.frames {
                if self.repeat {
                    self.frame = 0;
                } else {
                    self.frame = self.frames.saturating_sub(1);
                    // Clamped with the frame it clamps to, so a finished player
                    // does not keep counting positions no frame exists for.
                    self.position = self.frame as u64;
                    self.finished = true;
                    return;
                }
            }
        }
    }

    /// Advances to wherever the movie's own audio has got to, in seconds.
    ///
    /// **This is the audio clock ADR-0019 requires**, and the alternative to
    /// [`Player::update`] rather than an addition to it: a movie with a track
    /// is paced by one or the other on any given tick, never both, because two
    /// clocks on one playhead is the two-playhead bug this file has already
    /// had once.
    ///
    /// # Why audio leads and video follows
    ///
    /// A sound card consumes samples at its own rate and cannot be asked to
    /// wait. Stretching the picture to fit is invisible - a frame held or
    /// dropped at 30 Hz is 33 ms - where stretching the sound is a click. So
    /// the playhead is read off the mixer and the frame number is derived from
    /// it, which makes drift structurally impossible rather than merely small:
    /// there is no second accumulator to disagree with.
    ///
    /// # It only ever goes forwards
    ///
    /// Clamped against the position already reached, because a
    /// `crate::movie::Feed` hands each frame over exactly once and compares
    /// positions to do it - see `crate::movie::Ring::take_upto`. A playhead
    /// that went backwards would ask for a frame the ring had already dropped
    /// and get nothing, freezing the picture. Nothing should make it go
    /// backwards; the clamp is what stops a resampler's rounding from
    /// mattering if it did.
    /// # A held picture is discounted rather than skipped over
    ///
    /// The `--reel` leg holds the picture for two seconds at three points and
    /// the sound runs on underneath, so the seconds spent held are subtracted
    /// rather than treated as playback. This only works because the caller
    /// keeps calling while the hold is on - see
    /// [`super::Frontend::update_intro`] - which is what lets the
    /// hold's start and end both be observed.
    pub fn follow(&mut self, seconds: f64) {
        if self.paused {
            // Where the sound was when the hold began, so its length can be
            // measured when it ends. `get_or_insert` because every tick of the
            // hold arrives here and only the first one is the start of it.
            self.held_from.get_or_insert(seconds);
            return;
        }
        if let Some(from) = self.held_from.take() {
            self.held_seconds += (seconds - from).max(0.0);
        }
        if self.finished || self.frames == 0 {
            return;
        }

        let (num, den) = self.frame_rate;
        // The movie's own rate, not the audio's: a frame index is what the feed
        // is addressed by, and `seconds` is only how far along we are.
        let elapsed = ((seconds - self.held_seconds).max(0.0) * num as f64 / den as f64) as u64;
        let position = elapsed.max(self.position);

        if self.repeat {
            self.position = position;
            // `frames` came from a `usize`, so the remainder fits one.
            self.frame = (position % self.frames as u64) as usize;
        } else if position >= self.frames as u64 {
            // Clamped with the frame it clamps to, exactly as `update` does.
            self.frame = self.frames - 1;
            self.position = self.frame as u64;
            self.finished = true;
        } else {
            self.position = position;
            self.frame = position as usize;
        }
    }

    /// The frame that should be on screen, counting from zero.
    #[must_use]
    pub fn frame(&self) -> usize {
        self.frame
    }

    /// How many frames have gone by since playback started, counting loops.
    ///
    /// [`Player::frame`] wraps and this does not, which is the whole reason it
    /// exists: a `crate::movie::Feed` decodes forward forever and needs a
    /// number that only ever goes up to compare against. Frame 12 of the third
    /// time round a 270-frame loop is position 552, and there is no confusing
    /// it with frame 12 of the first.
    ///
    /// The invariant, and it is tested: `position % frames == frame` for a movie
    /// that repeats, and `position == frame` for one that does not.
    #[must_use]
    pub fn position(&self) -> u64 {
        self.position
    }

    /// The frame number as the original counts it, from one.
    ///
    /// The intro state compares against 144, 231 and 260, and those are counts
    /// of frames produced rather than a zero-based index.
    #[must_use]
    pub fn frames_produced(&self) -> usize {
        self.frame + 1
    }

    /// Whether playback has run out of frames.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Whether playback is paused.
    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Holds the current frame.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resumes from the current frame.
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Total frames available.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.frames
    }
}
