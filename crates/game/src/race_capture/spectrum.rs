//! The spectrum a capture feeds the Zone visualiser, split out of `race_capture.rs`
//! under the size ratchet.

use super::CaptureOptions;

/// One level per band, the width of [`oag_audio::BANDS`].
pub type Bands = [f32; oag_audio::BANDS];

/// Where a capture's band levels come from.
///
/// A capture on the null audio backend never publishes a spectrum, so an ordinary
/// capture shows no music-driven glow, and a real device's spectrum is live and
/// different on every run, the wrong input for a comparison. [`Self::Ramp`] and
/// [`Self::Fixed`] are the two repeatable inputs.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum ZoneFeed {
    /// The audio output's own spectrum.
    #[default]
    Live,
    /// A fixed ramp from 0 to 1 across the bands (`--zone-spectrum-test`).
    Ramp,
    /// Levels read off the original at the frame being compared (`--zone-bands`):
    /// `held` is what the floor's visualiser draws, `raw` what the mines'
    /// envelope reads.
    Fixed { held: Bands, raw: Bands },
}

impl ZoneFeed {
    /// The feed the flags name; `raw` defaults to `held`.
    #[must_use]
    pub fn from_flags(ramp: bool, held: Option<Bands>, raw: Option<Bands>) -> Self {
        match (held, ramp) {
            (Some(held), _) => Self::Fixed {
                held,
                raw: raw.unwrap_or(held),
            },
            (None, true) => Self::Ramp,
            (None, false) => Self::Live,
        }
    }

    /// The levels the visualiser is drawn from, `live` when the feed is [`Self::Live`].
    #[must_use]
    pub fn held(&self, live: impl FnOnce() -> Bands) -> Bands {
        match self {
            Self::Live => live(),
            Self::Ramp => ramp(),
            Self::Fixed { held, .. } => *held,
        }
    }

    /// The levels the Detonator mine's envelope reads, `live` when the feed is [`Self::Live`].
    #[must_use]
    pub fn raw(&self, live: impl FnOnce() -> Bands) -> Bands {
        match self {
            Self::Live => live(),
            Self::Ramp => ramp(),
            Self::Fixed { raw, .. } => *raw,
        }
    }
}

impl ZoneFeed {
    /// The whole-spectrum level the Detonator mine's visualiser reads, `live` when the feed is
    /// [`Self::Live`]. A comparison carries no level ([`Self::Fixed`] holds bands only; a pair pins
    /// the mines' volumes instead) and the ramp has none, so both read silence.
    #[must_use]
    pub fn level(&self, live: impl FnOnce() -> f32) -> f32 {
        match self {
            Self::Live => live(),
            Self::Ramp | Self::Fixed { .. } => 0.0,
        }
    }
}

fn ramp() -> Bands {
    std::array::from_fn(|i| {
        #[expect(clippy::cast_precision_loss, reason = "BANDS is 16, so exact")]
        let t = i as f32 / (oag_audio::BANDS - 1) as f32;
        t
    })
}

/// Parses `--zone-bands`: sixteen comma-separated levels, each finite and not negative.
///
/// # Errors
/// A message naming the count or the value that is wrong.
pub fn parse_bands(text: &str) -> Result<Bands, String> {
    let levels = text
        .split(',')
        .map(|part| {
            let level: f32 = part
                .trim()
                .parse()
                .map_err(|_| format!("`{part}` is not a number"))?;
            if level.is_finite() && level >= 0.0 {
                Ok(level)
            } else {
                Err(format!("`{part}` is not a finite, non-negative level"))
            }
        })
        .collect::<Result<Vec<f32>, String>>()?;
    let count = levels.len();
    Bands::try_from(levels).map_err(|_| {
        format!(
            "{count} levels given, {} wanted (one per band)",
            oag_audio::BANDS
        )
    })
}

/// The spectrum to feed `Scene::render` with: the options' [`ZoneFeed`], else
/// [`Output::spectrum`]'s live one.
///
/// [`Output::spectrum`]: oag_audio::Output::spectrum
pub(super) fn zone_spectrum(options: &CaptureOptions, audio: &oag_sound::Audio) -> Bands {
    options
        .zone_feed
        .held(|| audio.output().spectrum().levels())
}

/// [`zone_spectrum`], also handed to `scene` as the frame's raw band levels
/// ([`oag_raceplay::Scene::set_band_levels`], read by the Detonator mine's envelope).
pub(super) fn feed(
    options: &CaptureOptions,
    audio: &oag_sound::Audio,
    scene: &oag_raceplay::Scene,
) -> Bands {
    scene.set_band_levels(options.zone_feed.raw(|| audio.output().spectrum().levels()));
    scene.set_music_level(
        options
            .zone_feed
            .level(|| audio.output().spectrum().total()),
    );
    zone_spectrum(options, audio)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bands(f: impl Fn(usize) -> f32) -> Bands {
        std::array::from_fn(f)
    }

    #[test]
    fn a_fixed_feed_hands_back_its_own_levels_and_never_reads_the_live_spectrum() {
        let held = bands(|i| 0.01 * i as f32);
        let raw = bands(|i| 0.5 - 0.01 * i as f32);
        let feed = ZoneFeed::from_flags(false, Some(held), Some(raw));
        let never = || panic!("a fixed feed must not read the audio output");
        assert_eq!(feed.held(never), held);
        assert_eq!(feed.raw(never), raw);
    }

    #[test]
    fn the_raw_levels_default_to_the_held_ones() {
        let held = bands(|i| 0.02 * i as f32);
        let feed = ZoneFeed::from_flags(false, Some(held), None);
        assert_eq!(feed.raw(|| [9.0; 16]), held);
    }

    #[test]
    fn the_flags_pick_fixed_over_ramp_over_live() {
        let held = bands(|_| 0.25);
        assert_eq!(ZoneFeed::from_flags(false, None, None), ZoneFeed::Live);
        assert_eq!(ZoneFeed::from_flags(true, None, None), ZoneFeed::Ramp);
        assert!(matches!(
            ZoneFeed::from_flags(true, Some(held), None),
            ZoneFeed::Fixed { .. }
        ));
        let live = bands(|_| 0.75);
        assert_eq!(ZoneFeed::Live.held(|| live), live);
        let ramped = ZoneFeed::Ramp.held(|| live);
        assert!(ramped[0].abs() < f32::EPSILON && (ramped[15] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn sixteen_levels_parse_and_anything_else_is_refused() {
        let text = (0..16)
            .map(|i| format!("0.{i:02}"))
            .collect::<Vec<_>>()
            .join(",");
        let parsed = parse_bands(&text).expect("sixteen levels");
        assert!((parsed[3] - 0.03).abs() < 1e-6);
        let short = (0..15).map(|_| "0.5").collect::<Vec<_>>().join(",");
        assert!(parse_bands(&short).unwrap_err().contains("15 levels"));
        let long = (0..17).map(|_| "0.5").collect::<Vec<_>>().join(",");
        assert!(parse_bands(&long).unwrap_err().contains("17 levels"));
        for bad in ["NaN", "-0.1", "inf", "x"] {
            let text = format!("{bad}{}", ",0.5".repeat(15));
            assert!(parse_bands(&text).is_err(), "{bad}");
        }
    }
}
