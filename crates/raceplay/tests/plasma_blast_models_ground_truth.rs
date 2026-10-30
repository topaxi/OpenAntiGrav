//! The Pulse Plasma blast's three `.vex` models off the disc, against the
//! original's GE dumps (`docs/ghidra/functions/psp-pulse-usa/plasma.md`,
//! "2026-10-30: the blast and the head read off GE dumps").
//!
//! What this pins is what a wire has to rely on and a unit test cannot see:
//! the authored node spin and UV tracks, played at the blast's **age** (rate
//! 1), land on the dumps' measured values, and `hemisphere1` alone is
//! single-sided with a winding that shows nothing from inside its shell, which
//! is what makes the white-out cut when the eye enters it.
//!
//! **`#[ignore]`d and never run in CI** (needs the PSP USA disc image).

use oag_mesh::mesh;
use oag_pulse as pulse;

fn model(name: &str) -> Option<mesh::Model> {
    let image = oag_testdata::image("pulse-psp-usa.chd")?;
    let mut archives = pulse::open(&image.display().to_string()).expect("opening the PSP archives");
    let entry = format!(r"Data\Weapons\pulse_plasma_{name}.vex");
    let blob = archives
        .read_name(&entry)
        .expect("the blast model is on the disc");
    Some(mesh::build_with_textures(&entry, &blob, None).expect("the blast model decodes"))
}

/// The angle a node's matrix turns by, degrees, from its first two columns'
/// `x` and the third's `z` entries (`about_y`) or the first column (`about_z`).
fn turn(node: &[f32; 16], about_y: bool) -> f32 {
    if about_y {
        (-node[2]).atan2(node[0]).to_degrees()
    } else {
        node[1].atan2(node[0]).to_degrees()
    }
}

/// At the age itself, `hemisphere2` turns 127 degrees by 0.7167 s (the dump's
/// 182 deg/s over the age less a tick) and scrolls its texture `0.357`; the
/// `age * 0.07` clock the blast played until 2026-10-30 gave 6 degrees and
/// `0.025`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hemisphere2_spins_and_scrolls_at_the_dumps_rate_when_played_at_the_age() {
    let Some(model) = model("hemisphere2") else {
        return;
    };
    let age = 0.7167_f32;
    let nodes = model.sample_anim_nodes(age);
    assert_eq!(nodes.len(), 1);
    let degrees = turn(&nodes[0], true).rem_euclid(360.0);
    assert!((degrees - 127.0).abs() < 3.0, "turned {degrees} degrees");
    let (_, offset) = model.anim_tracks[0].sample(age);
    assert!(
        (offset[0] - 0.357).abs() < 0.005 && (offset[1] - 0.357).abs() < 0.005,
        "{offset:?}"
    );
    let (_, slow) = model.anim_tracks[0].sample(age * 0.07);
    assert!(slow[0] < 0.03, "the old clock reads {slow:?}");
}

/// `hemisphere1`'s V offset runs `0.0202 - 1.2205 age` and clamps at `-1.1992`
/// from about 0.98 s: the track at the age is `-0.204` at 0.1833 and clamps.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hemisphere1_scrolls_to_the_dumps_clamp_when_played_at_the_age() {
    let Some(model) = model("hemisphere1") else {
        return;
    };
    let v = |age: f32| model.anim_tracks[0].sample(age).1[1];
    assert!((v(0.1833) + 0.2035).abs() < 0.005, "{}", v(0.1833));
    assert!((v(1.05) + 1.1992).abs() < 0.001, "{}", v(1.05));
    assert!((v(1.4) + 1.1992).abs() < 0.001, "it stays clamped");
}

/// The halo is two parts turning opposite ways, about 177 and -59 degrees a
/// second (the dumps' two submeshes), and scrolls `(0.352, -0.705)` at 0.7167.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_halos_two_parts_turn_opposite_ways_and_its_texture_scrolls() {
    let Some(model) = model("halo1") else { return };
    let nodes = model.sample_anim_nodes(0.7167);
    assert_eq!(nodes.len(), 2);
    let (first, second) = (turn(&nodes[0], false), turn(&nodes[1], false));
    assert!(
        (first - 127.0).abs() < 3.0 || (first.rem_euclid(360.0) - 127.0).abs() < 3.0,
        "{first}"
    );
    assert!((second + 42.4).abs() < 2.0, "{second}");
    let (_, offset) = model.anim_tracks[0].sample(0.7167);
    assert!(
        (offset[0] - 0.352).abs() < 0.01 && (offset[1] + 0.705).abs() < 0.01,
        "{offset:?}"
    );
}

/// Only `hemisphere1` is authored single-sided (the GE's `cullenable` 1 on its
/// prims and 0 on the other two).
#[test]
#[ignore = "needs a disc image in data/images/"]
fn only_hemisphere1_is_single_sided() {
    for (name, expected) in [
        ("halo1", false),
        ("hemisphere2", false),
        ("hemisphere1", true),
    ] {
        let Some(model) = model(name) else { return };
        let draws: Vec<_> = model
            .draws
            .iter()
            .chain(&model.alpha_tested_draws)
            .chain(&model.transparent_draws)
            .collect();
        assert!(!draws.is_empty());
        assert!(draws.iter().all(|d| d.culled == expected), "{name}");
    }
}

/// `hemisphere1`'s winding shows its outside: from a point inside its dome no
/// triangle faces the eye, from outside some do. Fails if the winding were
/// reversed, which would put the white-out after the eye is inside the shell
/// instead of before.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hemisphere1_shows_nothing_from_inside_its_shell_and_something_from_outside() {
    let Some(model) = model("hemisphere1") else {
        return;
    };
    let front_facing = |eye: [f32; 3]| {
        let mut count = 0;
        for draw in model
            .draws
            .iter()
            .chain(&model.alpha_tested_draws)
            .chain(&model.transparent_draws)
        {
            for triangle in
                model.indices[draw.range.start as usize..draw.range.end as usize].chunks_exact(3)
            {
                let p = |i: u32| model.vertices[i as usize].position;
                let (a, b, c) = (p(triangle[0]), p(triangle[1]), p(triangle[2]));
                let sub = |u: [f32; 3], v: [f32; 3]| [u[0] - v[0], u[1] - v[1], u[2] - v[2]];
                let (e1, e2, to_eye) = (sub(b, a), sub(c, a), sub(eye, a));
                let normal = [
                    e1[1] * e2[2] - e1[2] * e2[1],
                    e1[2] * e2[0] - e1[0] * e2[2],
                    e1[0] * e2[1] - e1[1] * e2[0],
                ];
                if normal[0] * to_eye[0] + normal[1] * to_eye[1] + normal[2] * to_eye[2] > 0.0 {
                    count += 1;
                }
            }
        }
        count
    };
    assert_eq!(
        front_facing([0.0, 2.0, 0.0]),
        0,
        "an eye inside the dome sees only back faces"
    );
    assert!(
        front_facing([0.0, 3.0, 60.0]) > 20,
        "an eye outside sees the near side"
    );
}
