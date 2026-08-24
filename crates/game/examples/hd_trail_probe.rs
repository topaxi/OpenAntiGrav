//! Scratch probe: does the HD trail tube produce visible geometry in a real
//! race - vertex counts, alpha and normals against the camera.
//!
//! ```sh
//! cargo run --release -p oag-game --example hd_trail_probe
//! ```
use oag_gameplay::InputSnapshot;

fn main() -> anyhow::Result<()> {
    let options = oag_game::race::Options {
        source: "data/images/hdfury-ps3-eu-dec.iso".into(),
        ..Default::default()
    };
    let loaded = oag_game::race::load(&options)?;
    let mut race = oag_game::race::Race::start(loaded.setup);
    race.set_autopilot(true);
    for _ in 0..700 {
        race.tick(&InputSnapshot::default());
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
    for v in &verts {
        let n = oag_core::math::Vec3::from_array(v.normal);
        let view = camera - oag_core::math::Vec3::from_array(v.position);
        let d = n.normalize_or_zero().dot(view.normalize_or_zero());
        let facing = (d.clamp(0.0, 0.15) / 0.15).clamp(0.0, 1.0);
        if facing * v.colour[3] > 0.05 {
            visible += 1;
        }
    }
    println!("verts passing facing*alpha > 0.05 from the race camera: {visible}");
    Ok(())
}
