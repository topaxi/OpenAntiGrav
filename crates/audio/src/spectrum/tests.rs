use super::*;

/// `frames` stereo frames of a full-scale sine at `hz`, interleaved.
fn tone(hz: f32, sample_rate: u32, frames: usize) -> Vec<f32> {
    #[expect(clippy::cast_precision_loss, reason = "test data, small counts")]
    let rate = sample_rate as f32;
    let mut out = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        #[expect(clippy::cast_precision_loss, reason = "test data, small counts")]
        let t = i as f32 / rate;
        let s = (2.0 * std::f32::consts::PI * hz * t).sin();
        out.push(s);
        out.push(s);
    }
    out
}

/// A pure tone lights up the band nearest its own frequency more than every
/// other band - the property that makes this usable as a spectrum at all,
/// not just a loudness meter.
#[test]
fn a_pure_tone_peaks_the_band_nearest_its_own_frequency() {
    let sample_rate = 44_100;
    let samples = tone(1_000.0, sample_rate, 512);
    let mono: Vec<f32> = samples
        .as_chunks::<2>()
        .0
        .iter()
        .map(|frame| (frame[0] + frame[1]) * 0.5)
        .collect();

    // **On the amplitude, not on the level.** `Range` normalises each band
    // against its own recent span, so a *steady* tone eventually fills every
    // band that hears any of it at all - that is the recovered curve working,
    // not a defect, and it makes the level the wrong place to look for
    // frequency selectivity. The selectivity is Goertzel's, and it is here.
    let frequencies = band_frequencies();
    let amplitudes: Vec<f32> = frequencies
        .iter()
        .map(|&hz| {
            band_amplitude(
                goertzel_magnitude(&mono, sample_rate as f32, hz),
                mono.len(),
            )
        })
        .collect();
    let (peak_band, _) = amplitudes
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .expect("BANDS is non-zero");
    let nearest = frequencies
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (*a - 1_000.0).abs().total_cmp(&(*b - 1_000.0).abs()))
        .map(|(i, _)| i)
        .expect("BANDS is non-zero");

    // Within one band of the true nearest: geometric spacing means adjacent
    // bands near 1 kHz are close enough that Goertzel's own filter width can
    // legitimately favour a neighbour by a hair.
    assert!(
        peak_band.abs_diff(nearest) <= 1,
        "a 1 kHz tone peaked band {peak_band} ({:.0} Hz), nearest is {nearest} ({:.0} Hz)",
        frequencies[peak_band],
        frequencies[nearest]
    );
}

/// Silence produces silence - no invented level with nothing playing.
#[test]
fn silence_reads_as_all_zero() {
    let sample_rate = 44_100;
    let samples = vec![0.0f32; 512 * 2];
    let mut analyzer = Analyzer::new();
    let levels = analyzer.process(&samples, sample_rate);
    assert!(
        levels.iter().all(|&l| l == 0.0),
        "silence should read as zero on every band, got {levels:?}"
    );
}

/// A `Spectrum` answers silence until the first chunk is published, and
/// answers the published levels afterwards - the contract
/// `crate::output::render::Ahead` and a reader on the other side both rely
/// on.
#[test]
fn spectrum_starts_silent_and_reflects_the_latest_publish() {
    let spectrum = Spectrum::new();
    assert_eq!(spectrum.levels(), [0.0; BANDS]);

    let mut published = [0.0; BANDS];
    published[5] = 0.8;
    spectrum.publish(published);
    assert_eq!(spectrum.levels(), published);
}

/// **The analyser reports an instant level and does not hold one.** A silent
/// chunk after a loud one reads zero on that band, because the level is
/// "where this chunk's amplitude sits between the band's own floor and peak"
/// and a silent chunk sits on the floor.
///
/// This is the recovered shape and it replaces the opposite assertion. The
/// decay this file used to test for was an invention in the wrong layer; the
/// original decays in the *visualiser*, per frame, and that is ported as
/// `oag_render::mesh_render::zone::Hold`.
#[test]
fn a_silent_chunk_reads_zero_because_the_hold_lives_in_the_visualiser() {
    let sample_rate = 44_100;
    let mut analyzer = Analyzer::new();
    let loud = tone(1_000.0, sample_rate, 512);
    let first = analyzer.process(&loud, sample_rate);
    let peak_band = first
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(i, _)| i)
        .expect("BANDS is non-zero");
    assert!(first[peak_band] > 0.0, "the tone should light its band");

    let silence = vec![0.0f32; 512 * 2];
    let second = analyzer.process(&silence, sample_rate);
    assert_eq!(second[peak_band], 0.0);
}

/// **The auto-ranging curve's own contract**: a band measures its amplitude
/// against its own recent range, so the *same* amplitude reads differently
/// once the band has heard something louder - which is exactly what a fixed
/// dB floor could not do.
#[test]
fn a_band_normalises_against_its_own_recent_range() {
    let mut range = Range::default();
    // The first thing a band ever hears defines its whole range, so it fills
    // the meter.
    assert_eq!(range.level(0.5, 0.01), 1.0);
    // Something quieter, with the peak still up, reads part way rather than
    // full - and the floor has not yet risen to swallow it.
    let quiet = range.level(0.25, 0.01);
    assert!(
        (0.0..1.0).contains(&quiet),
        "a quieter chunk should read below full, got {quiet}"
    );
    // A louder one takes the peak with it and fills the meter again.
    assert_eq!(range.level(2.0, 0.01), 1.0);
}

/// A band that has never heard anything has no span, and reports silence
/// rather than amplifying its own noise into a full bar - the
/// [`Range::MIN_SPAN`] guard, which is what stops fifteen empty bands
/// flickering at full height.
#[test]
fn an_empty_band_has_no_span_and_reads_zero() {
    let mut range = Range::default();
    for _ in 0..10 {
        assert_eq!(range.level(0.0, 0.01), 0.0);
    }
}
