//! What the disc says about `.points2`, over all nineteen shipped clouds.
//!
//! The claims `oag_rcs::points2`' module docs rest on, made executable: the
//! layout words are what the parser checks, the count matches the bytes, every
//! position is inside the header's box to within a half-float ulp, every
//! normal but forty-six counted degenerates is unit length, and the padding
//! is zero.
//!
//! `#[ignore]`d because it needs `data/images/hdfury-ps3-eu-dec.iso`; run with
//! `just test-data`.

mod rcsmodel_common;

use oag_rcs::points2::{self, PointCloud, RECORD_SIZE};
use rcsmodel_common::image;

/// How many clouds the EU disc ships under `/data/fe/fury/`, so a run that
/// found none fails rather than passing empty.
const SHIPPED: usize = 19;

/// The archive the front end's data is in.
const ARCHIVE: &str = "DATA00.PSARC";

/// One half ulp at the hull's scale: positions reach `|7.3|`, where a half's
/// spacing is `2^-8`, so a rounded position may sit that far outside an
/// `f32` box.
const HALF_ULP: f32 = 1.0 / 256.0;

fn every_cloud() -> Vec<(String, Vec<u8>, PointCloud)> {
    let image = image().expect("checked by the caller");
    let spec = format!("{}:PS3_GAME/USRDIR/{ARCHIVE}", image.display());
    let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
    let paths: Vec<String> = open
        .paths()
        .iter()
        .filter(|p| p.ends_with(".points2"))
        .cloned()
        .collect();
    let mut out = Vec::new();
    for path in paths {
        let blob = open.read_path(&path).expect("the .points2 reads");
        let cloud = points2::parse(&blob)
            .unwrap_or_else(|e| panic!("{path}: every shipped .points2 parses, but {e}"));
        out.push((path, blob, cloud));
    }
    out
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso; run with `just test-data`"]
fn every_shipped_cloud_parses_and_its_count_is_the_file_length() {
    if image().is_none() {
        return;
    }
    let clouds = every_cloud();
    assert_eq!(clouds.len(), SHIPPED, "the EU disc ships nineteen clouds");
    for (path, blob, cloud) in &clouds {
        assert_eq!(
            blob.len(),
            points2::HEADER_SIZE + cloud.points.len() * RECORD_SIZE,
            "{path}: the count accounts for every byte after the header"
        );
        assert!(
            (51_336..=55_000).contains(&cloud.points.len()),
            "{path}: {} points is outside the measured range",
            cloud.points.len()
        );
    }
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso; run with `just test-data`"]
fn every_position_is_inside_the_headers_box() {
    if image().is_none() {
        return;
    }
    for (path, _, cloud) in every_cloud() {
        for axis in 0..3 {
            let (lo, hi) = cloud
                .points
                .iter()
                .fold((f32::MAX, f32::MIN), |(lo, hi), p| {
                    (lo.min(p.position[axis]), hi.max(p.position[axis]))
                });
            assert!(
                lo >= cloud.min[axis] - HALF_ULP && hi <= cloud.max[axis] + HALF_ULP,
                "{path}: axis {axis} spans {lo}..{hi} against the header's {}..{}",
                cloud.min[axis],
                cloud.max[axis]
            );
            // And the box is tight, not merely containing: the header was
            // computed from these points.
            assert!(
                (lo - cloud.min[axis]).abs() <= HALF_ULP
                    && (hi - cloud.max[axis]).abs() <= HALF_ULP,
                "{path}: axis {axis}'s box is not tight"
            );
        }
    }
}

/// The degenerate normals the tool that sampled these left behind: encoded
/// as the byte triple nearest zero (`127 127 127`, or `203 127 127` for
/// `assegai_n1`'s 33), so their length is `0.007..0.6` rather than one.
/// Forty-six of just over a million points, on four files; every other
/// normal is unit to within `0.012`, which is the byte quantisation. Counted
/// rather than tolerated, so a parser regression that zeroed a stream would
/// fail here rather than hide under a tolerance.
const DEGENERATE_NORMALS: &[(&str, usize)] = &[
    ("/data/fe/fury/ag_systems_c1.points2", 6),
    ("/data/fe/fury/ag_systems_n1.points2", 6),
    ("/data/fe/fury/assegai_n1.points2", 33),
    ("/data/fe/fury/piranha_c1.points2", 1),
];

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso; run with `just test-data`"]
fn every_normal_but_the_counted_degenerates_is_unit_length_and_the_padding_is_zero() {
    if image().is_none() {
        return;
    }
    for (path, blob, cloud) in every_cloud() {
        let (mut worst, mut degenerate) = (0.0f32, 0usize);
        for point in &cloud.points {
            let [x, y, z] = point.normal;
            let length = (x * x + y * y + z * z).sqrt();
            if length < 0.9 {
                degenerate += 1;
            } else {
                worst = worst.max((length - 1.0).abs());
            }
        }
        let expected = DEGENERATE_NORMALS
            .iter()
            .find(|(name, _)| *name == path)
            .map_or(0, |(_, count)| *count);
        assert_eq!(degenerate, expected, "{path}: degenerate normals");
        assert!(worst < 0.02, "{path}: a normal is {worst} off unit length");
        let records = &blob[points2::HEADER_SIZE..];
        let padding = records
            .as_chunks::<RECORD_SIZE>()
            .0
            .iter()
            .flat_map(|record| &record[13..16])
            .filter(|&&byte| byte != 0)
            .count();
        assert_eq!(padding, 0, "{path}: the three trailing bytes are padding");
        assert!(
            cloud
                .points
                .iter()
                .all(|p| p.rand.iter().all(|&b| b <= 254)),
            "{path}: rand bytes reach 254 and no further"
        );
    }
}
