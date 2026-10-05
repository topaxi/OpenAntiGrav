//! Scratch probe: does the HD trail tube produce visible geometry in a real
//! race - vertex counts, alpha and normals against the camera.
//!
//! ```sh
//! cargo run --release -p oag-game --example hd_trail_probe
//! ```
use oag_gameplay::PlayerInputs;

fn main() -> anyhow::Result<()> {
    let options = oag_raceplay::Options {
        source: "data/images/hdfury-ps3-eu-dec.iso".into(),
        ..Default::default()
    };
    let loaded = oag_raceplay::load(&options)?;
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.set_autopilot(true);
    for _ in 0..700 {
        race.tick(&PlayerInputs::none());
    }
    println!("hd_trail_active: {}", race.hd_trail_active());
    let exhaust = race.exhaust_of(0);
    println!(
        "speed_kmh {:.1} intensity {:.2} ramp {:.2}",
        exhaust.speed_kmh(),
        exhaust.intensity(),
        exhaust.speed_ramp()
    );
    let verts = race.hd_trail_vertices(0);
    println!("verts: {}", verts.len());
    if let Some(peak) = verts
        .iter()
        .max_by(|a, b| a.colour[3].total_cmp(&b.colour[3]))
    {
        println!(
            "peak alpha {:.3} colour {:?} uv {:?} normal {:?}",
            peak.colour[3], peak.colour, peak.texcoord, peak.normal
        );
    }
    let camera = race.camera_position();
    let mut visible = 0usize;
    let mut hist = [0usize; 10];
    let mut best = (0.0f32, 0usize);
    for (i, v) in verts.iter().enumerate() {
        let n = oag_core::math::Vec3::from_array(v.normal);
        let view = camera - oag_core::math::Vec3::from_array(v.position);
        let d = n.normalize_or_zero().dot(view.normalize_or_zero());
        let facing = (d.clamp(0.0, 0.15) / 0.15).clamp(0.0, 1.0);
        let a = facing * v.colour[3] * 0.75;
        if a > 0.05 {
            visible += 1;
        }
        hist[((a * 9.99) as usize).min(9)] += 1;
        if a > best.0 {
            best = (a, i);
        }
    }
    println!("verts passing facing*alpha*depth > 0.05: {visible}");
    println!("alpha histogram (0.1 bins): {hist:?}");
    println!(
        "best effective alpha {:.3} at vert {} pos {:?}",
        best.0, best.1, verts[best.1].position
    );
    let per_fin = verts.len() / 3;
    for fin in 0..3 {
        let mut facing_sum = 0.0f32;
        let mut alpha_sum = 0.0f32;
        let mut n_behind = 0usize;
        for v in &verts[fin * per_fin..(fin + 1) * per_fin] {
            let n = oag_core::math::Vec3::from_array(v.normal);
            let view = camera - oag_core::math::Vec3::from_array(v.position);
            let d = n.normalize_or_zero().dot(view.normalize_or_zero());
            if d < 0.0 {
                n_behind += 1;
            }
            facing_sum += (d.clamp(0.0, 0.15) / 0.15).clamp(0.0, 1.0);
            alpha_sum += v.colour[3];
        }
        println!(
            "fin {fin}: mean facing {:.3}, mean vertex alpha {:.3}, backfacing {}/{}",
            facing_sum / per_fin as f32,
            alpha_sum / per_fin as f32,
            n_behind,
            per_fin
        );
    }
    println!("camera {:?}", camera);
    println!(
        "head pos {:?} tail pos {:?}",
        verts[0].position,
        verts[per_fin - 2].position
    );
    Ok(())
}
