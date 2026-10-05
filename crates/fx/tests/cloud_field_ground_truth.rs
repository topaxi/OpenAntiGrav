//! `05_Track`'s cloud fields, built from the seeds one boot of the original
//! kept, against what that boot held in RAM.
//!
//! Live, de Konstruct Black (`05_Track` forward), PPSSPP, 2026-10-04: three
//! `cloudGroup` instances with seeds 2079, 6378 and 9169, holding 46, 46 and
//! 277 authored records and keeping 14, 14 and 74. The constants below are
//! that dump's own values, copied here because scratch dumps are not kept.
//! `docs/ghidra/functions/psp-pulse-usa/clouds.md` has the reading and the
//! check of all 369 records; this test pins enough of it that dropping any step
//! of the build fails: the per-cube count, the scatter, the half-size, the
//! cull, the colour ramp's height range (every record's, culled ones too), the
//! nested group's second field and the variant draw.

use oag_fx::cloud::field::{self, Baked};
use oag_vex::{cloud, vex};

/// One group as the original held it: seed, authored and kept counts, the
/// first and last authored record (`[x, y, z, half]`), and the first two kept
/// sprites (`phase, rate, atlas cell, packed colour`).
struct Live {
    seed: u32,
    authored: usize,
    kept: usize,
    first: [f32; 4],
    last: [f32; 4],
    baked: [(f32, f32, u8, u32); 2],
}

const LIVE: [Live; 3] = [
    Live {
        seed: 2079,
        authored: 46,
        kept: 14,
        first: [-201.637_51, -1.573_757_2, 356.135_28, 37.683_376],
        last: [-236.317_47, -23.365_805, 332.574_77, 36.570_343],
        baked: [
            (0.987_878_7, -0.001_295_708_1, 6, 0xffab_c0d6),
            (4.506_113_5, -0.000_438_280_05, 7, 0xffb1_c6dc),
        ],
    },
    Live {
        seed: 6378,
        authored: 46,
        kept: 14,
        first: [-227.644_5, -18.520_485, 372.312_7, 37.695_675],
        last: [-194.759_02, -46.704_628, 381.793_34, 42.360_82],
        baked: [
            (2.201_602, -0.001_727_268_8, 6, 0xffcd_cdcd),
            (3.679_371_4, -0.001_301_023, 6, 0xffba_baba),
        ],
    },
    Live {
        seed: 9169,
        authored: 277,
        kept: 74,
        first: [-1_070.159, -82.941_086, -31.362_57, 37.703_663],
        last: [-1_034.808_2, 17.743_519, 208.586_2, 39.074_25],
        baked: [
            (0.897_423_9, 0.000_133_123_95, 7, 0xff77_91ac),
            (4.059_150_7, -0.001_083_378_3, 7, 0xff81_9bb7),
        ],
    },
];

fn close(a: f32, b: f32, tolerance: f32) -> bool {
    (a - b).abs() <= tolerance
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn de_konstruct_s_cloud_fields_match_the_original_s_ram_for_its_seeds() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("archives");
    let blob = archives
        .read_name(r"Data\Environments\05_Track\track.vex")
        .expect("track.vex");
    let nodes = vex::nodes(&blob).expect("nodes");
    let groups = cloud::cloud_groups(&blob, &nodes);
    assert_eq!(groups.len(), 3, "three cloudGroups build a field");

    for (group, live) in groups.iter().zip(&LIVE) {
        let mut rng = oag_fx::ranrot::Ranrot::reseeded(live.seed);
        let mut records = field::scatter(&group.cubes, &mut rng);
        assert_eq!(records.len(), live.authored, "seed {}: authored", live.seed);
        for (record, want) in [
            (records[0], live.first),
            (records[records.len() - 1], live.last),
        ] {
            let got = [
                record.position[0],
                record.position[1],
                record.position[2],
                record.half_size,
            ];
            assert!(
                got.iter().zip(want).all(|(g, w)| close(*g, w, 0.01)),
                "seed {}: record {got:?}, the original's {want:?}",
                live.seed
            );
        }
        field::cull(&mut records, group.attributes.overlap);
        assert_eq!(
            records.iter().filter(|r| r.cube.is_some()).count(),
            live.kept,
            "seed {}: kept after the Overlap cull",
            live.seed
        );

        let baked: Vec<Baked> = field::build(group, live.seed);
        assert_eq!(baked.len(), live.kept);
        for (sprite, (phase, rate, cell, colour)) in baked.iter().zip(live.baked) {
            assert!(close(sprite.phase, phase, 1e-5), "seed {}", live.seed);
            assert!(close(sprite.rate, rate, 1e-9), "seed {}", live.seed);
            assert_eq!(sprite.cell, cell, "seed {}: atlas cell", live.seed);
            assert_eq!(
                sprite.colour, colour,
                "seed {}: baked colour {:08x}, the original's {colour:08x}",
                live.seed, sprite.colour
            );
        }
    }
}

/// `05_Track`'s reversed layout (de Konstruct White) authors the same three
/// groups over the same cubes: live on a boot of that layout, 46, 46 and 277
/// records, the same counts as forward.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_reversed_layout_builds_the_same_three_fields() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("archives");
    let blob = archives
        .read_name(r"Data\Environments\05_Track\track_reversed.vex")
        .expect("track_reversed.vex");
    let nodes = vex::nodes(&blob).expect("nodes");
    let counts: Vec<usize> = cloud::cloud_groups(&blob, &nodes)
        .iter()
        .map(|group| group.cubes.iter().map(field::record_count).sum())
        .collect();
    assert_eq!(counts, [46, 46, 277]);
}
