//! Measures, on the disc's own circuit, the conventions a foreign track must
//! be mapped into: where a spline point sits against the floor, which way
//! `lateral` points, which way a floor triangle's normal points, and how a
//! closed ring is encoded. Run with `--nocapture` to read the numbers.

use oag_core::math::Vec3;
use oag_physics::{Ray, Raycaster};
use oag_raceplay as race;

#[test]
#[ignore = "needs data/images"]
fn pulse_conventions() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        ..race::Options::default()
    })
    .expect("loading");
    let ai = &loaded.setup.ai;
    println!("paths {} junctions {}", ai.paths.len(), ai.junctions.len());
    for (i, p) in ai.paths.iter().enumerate() {
        println!(
            "path {i}: {} points entry {:?} exit {:?} max_spacing {:.2}",
            p.points.len(),
            p.entry,
            p.exit,
            p.max_spacing
        );
    }
    for (i, j) in ai.junctions.iter().enumerate() {
        println!("junction {i}: prev {:?} next {:?}", j.prev, j.next);
    }
    let world = &loaded.setup.collision;
    let (mut ct_sum, mut tl_sum, mut n) = (0.0f32, 0.0f32, 0);
    let mut heights = Vec::new();
    let mut normals = Vec::new();
    let mut rl = (0.0f32, 0.0f32, 0.0f32);
    let mut widths = (0.0f32, 0.0f32);
    for p in &ai.paths[0].points {
        let t = Vec3::from_array(p.tangent);
        let d = Vec3::from_array(p.down);
        let l = Vec3::from_array(p.lateral);
        ct_sum += t.cross(d).dot(l);
        tl_sum += t.dot(l);
        n += 1;
        let up = -d;
        let from = Vec3::from_array(p.pos) + up * 3.0;
        if let Some(h) = world.raycast(Ray::new(from, d, 8.0), None, false) {
            heights.push(h.distance);
            normals.push(h.normal.dot(up));
        }
        rl.0 = rl.0.min(p.racing_line);
        rl.1 = rl.1.max(p.racing_line);
        rl.2 += p.progress;
        widths.0 += p.half_width_left;
        widths.1 += p.half_width_right;
    }
    let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len().max(1) as f32;
    println!(
        "mean dot(cross(t,down),lateral) {:.3}, mean dot(t,lateral) {:.3}",
        ct_sum / n as f32,
        tl_sum / n as f32
    );
    println!(
        "ray from pos+3*up along down: {} hits of {n}, mean t {:.3} (3.0 = pos on the floor), mean dot(normal, up) {:.3}",
        heights.len(),
        mean(&heights),
        mean(&normals)
    );
    println!(
        "racing_line range {:.2}..{:.2}; mean half widths L {:.2} R {:.2}; first progress {:.3}",
        rl.0,
        rl.1,
        widths.0 / n as f32,
        widths.1 / n as f32,
        ai.paths[0].points[0].progress
    );
    assert!(
        (ct_sum / n as f32 + 1.0).abs() < 1e-3,
        "lateral = tangent x up"
    );
    assert!(
        tl_sum.abs() / (n as f32) < 1e-2,
        "lateral is across the path"
    );
    assert_eq!(heights.len(), n, "floor under every spline point");
    assert!((mean(&heights) - 3.0).abs() < 0.2, "pos is on the floor");
    assert!(mean(&normals) > 0.99, "floor normals point up");
    assert_eq!(
        ai.paths.len(),
        2,
        "this ring is two paths and two junctions"
    );
    let p = &ai.paths[0].points[0];
    println!("first point: {p:?}");
    println!("start: {:?}", loaded.setup.start_position);
}
