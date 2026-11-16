use super::*;

fn file(rows: &[Levels]) -> Vec<u8> {
    let mut bytes = b"AFEX".to_vec();
    bytes.extend(1u32.to_le_bytes());
    bytes.extend((BANDS as u32).to_le_bytes());
    bytes.extend((rows.len() as u32).to_le_bytes());
    for row in rows {
        for level in row {
            bytes.extend(level.to_le_bytes());
        }
    }
    bytes
}

fn row(band1: f32) -> Levels {
    let mut levels = [0.0; BANDS];
    levels[1] = band1;
    levels
}

#[test]
fn a_file_reads_back_its_rows() {
    let spectrum = Spectrum::parse(&file(&[row(0.25), row(0.75)])).unwrap();
    assert_eq!(spectrum.frames(), 2);
    assert_eq!(spectrum.rows[1][1], 0.75);
}

#[test]
fn sampling_blends_the_row_and_the_next() {
    let spectrum = Spectrum::parse(&file(&[row(0.0), row(1.0), row(0.0)])).unwrap();
    assert_eq!(spectrum.sample_at(0.0)[1], 0.0);
    // Half a row in: halfway between rows 0 and 1.
    assert!((spectrum.sample_at(0.5 / 30.0)[1] - 0.5).abs() < 1e-6);
    assert!((spectrum.sample_at(1.0 / 30.0)[1] - 1.0).abs() < 1e-6);
}

#[test]
fn time_outside_the_track_is_silent() {
    let spectrum = Spectrum::parse(&file(&[row(1.0), row(1.0)])).unwrap();
    assert_eq!(spectrum.sample_at(-0.1), [0.0; BANDS]);
    // The last row has no next row to blend with.
    assert_eq!(spectrum.sample_at(1.0 / 30.0), [0.0; BANDS]);
    assert_eq!(spectrum.sample_at(5.0), [0.0; BANDS]);
}

#[test]
fn a_bad_file_is_refused() {
    let good = file(&[row(1.0)]);
    let mut magic = good.clone();
    magic[0] = b'X';
    assert!(Spectrum::parse(&magic).is_err());
    assert!(Spectrum::parse(&good[..good.len() - 1]).is_err());
    assert!(Spectrum::parse(&good[..8]).is_err());
    let mut version = good.clone();
    version[4] = 2;
    assert!(Spectrum::parse(&version).is_err());
    let mut bands = good;
    bands[8] = 8;
    assert!(Spectrum::parse(&bands).is_err());
}
