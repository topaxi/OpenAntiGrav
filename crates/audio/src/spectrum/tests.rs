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
    let mut analyzer = Analyzer::new();
    let levels = analyzer.process(&samples, sample_rate);

    let frequencies = band_frequencies();
    let (peak_band, _) = levels
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

/// Decay is gradual, not instant: two chunks of the same tone should not
/// collapse a previously-lit band back to zero, which is what would happen
/// with no memory between chunks at all.
#[test]
fn a_band_decays_rather_than_snapping_to_the_next_chunks_reading() {
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
    assert!(
        second[peak_band] > 0.0 && second[peak_band] < first[peak_band],
        "one silent chunk after a loud one should decay, not snap to zero or hold: \
         {} -> {}",
        first[peak_band],
        second[peak_band]
    );
}
