use super::*;

/// `frames` samples of a full-scale sine at `hz`.
fn tone(hz: f32, sample_rate: u32, frames: usize) -> Vec<f32> {
    #[expect(clippy::cast_precision_loss, reason = "test data, small counts")]
    let rate = sample_rate as f32;
    (0..frames)
        .map(|i| {
            #[expect(clippy::cast_precision_loss, reason = "test data, small counts")]
            let t = i as f32 / rate;
            (2.0 * std::f32::consts::PI * hz * t).sin()
        })
        .collect()
}

/// The amplitudes of one spectrum of a tone, band by band.
fn band_sums(hz: f32, sample_rate: u32) -> [f32; BANDS] {
    let fft = Fft::new();
    let samples: Box<[f32; FFT_SIZE]> = tone(hz, sample_rate, FFT_SIZE)
        .into_boxed_slice()
        .try_into()
        .expect("FFT_SIZE samples");
    let mut real = Box::new([0.0; FFT_SIZE]);
    let mut imaginary = Box::new([0.0; FFT_SIZE]);
    let mut magnitudes = Box::new([0.0; HALF]);
    fft.magnitudes(&samples, &mut real, &mut imaginary, &mut magnitudes);
    bins_at(sample_rate as f32).map(|(first, last)| magnitudes[first..=last].iter().sum())
}

/// The band a tone fills most.
fn peak_band(sums: &[f32; BANDS]) -> usize {
    sums.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(band, _)| band)
        .expect("BANDS is non-zero")
}

/// A pure tone lands in the band whose authored bins hold its frequency, the
/// property that makes this a spectrum and not a loudness meter. The level is
/// the wrong place to look: [`Range`] fills every band that hears a steady
/// tone at all.
#[test]
fn a_pure_tone_lands_in_the_band_holding_its_frequency() {
    for (hz, band) in [(5_000.0, 10), (7_000.0, 11), (9_000.0, 12), (500.0, 4)] {
        let sums = band_sums(hz, 48_000);
        assert_eq!(peak_band(&sums), band, "{hz} Hz: {sums:?}");
    }
}

/// The bins are frequencies: a device off the authored rate reads the same
/// tone into the same band.
#[test]
fn the_bins_follow_the_sample_rate() {
    assert_eq!(peak_band(&band_sums(5_000.0, 44_100)), 10);
    assert_eq!(bins_at(48_000.0)[15], (329, 511));
}

/// The bins tile the spectrum, nothing read twice and nothing missed.
#[test]
fn the_bins_tile_the_spectrum() {
    assert_eq!(BINS[0].0, 0);
    assert_eq!(usize::from(BINS[BANDS - 1].1), HALF - 1);
    for pair in BINS.windows(2) {
        assert_eq!(pair[0].1 + 1, pair[1].0);
    }
}

/// One spectrum every hop, however the chunks that feed it fall.
#[test]
fn a_spectrum_lands_every_hop_whatever_the_chunk_size() {
    let samples = tone(5_000.0, 48_000, HOP * 8);
    let mut whole = Analyzer::new();
    let at_once = whole.process(&samples, 48_000);
    let mut parts = Analyzer::new();
    let mut in_pieces = [0.0; BANDS];
    for piece in samples.chunks(37) {
        in_pieces = parts.process(piece, 48_000);
    }
    assert_eq!(at_once, in_pieces);
}

/// Silence produces silence - no invented level with nothing playing.
#[test]
fn silence_reads_as_all_zero() {
    let sample_rate = 44_100;
    let samples = vec![0.0f32; HOP * 2];
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
/// `oag_mesh::mesh_render::zone::Hold`.
#[test]
fn a_silent_chunk_reads_zero_because_the_hold_lives_in_the_visualiser() {
    let sample_rate = 44_100;
    let mut analyzer = Analyzer::new();
    let loud = tone(1_000.0, sample_rate, FFT_SIZE);
    let first = analyzer.process(&loud, sample_rate);
    let peak = peak_band(&first);
    assert!(first[peak] > 0.0, "the tone should light its band");

    let silence = vec![0.0f32; FFT_SIZE];
    let second = analyzer.process(&silence, sample_rate);
    assert_eq!(second[peak], 0.0);
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
