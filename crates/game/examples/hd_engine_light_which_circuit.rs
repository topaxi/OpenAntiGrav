//! Scratch probe: which HD circuit's collision soup do the live-captured
//! SPU light records (`data/traces/hd-spu-light-companion/s*_slot.bin`) sit
//! closest to? The companion capture was logged as Amphiseum; the positions
//! reach outside Amphiseum's collision bounds in this project's frame, so
//! every circuit is scored - the one whose surfaces sit a ride height away
//! from every record is the one the capture ran on.
//!
//! ```sh
//! cargo run --release -p oag-game --example hd_engine_light_which_circuit
//! ```

use oag_core::math::Vec3;

fn point_triangle_distance(p: Vec3, [a, b, c]: [Vec3; 3]) -> f32 {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return ap.length();
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return bp.length();
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return (p - (a + ab * v)).length();
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return cp.length();
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return (p - (a + ac * w)).length();
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (p - (b + (c - b) * w)).length();
    }
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    (p - (a + ab * v + ac * w)).length()
}

fn nearest_surface(world: &oag_physics::collide::CollisionWorld, p: Vec3) -> f32 {
    let mut best = f32::MAX;
    for soup in world.colliders() {
        for t in 0..soup.triangle_count() {
            if let Some(tri) = soup.triangle(t) {
                best = best.min(point_triangle_distance(p, tri));
            }
        }
    }
    best
}

fn main() -> anyhow::Result<()> {
    let mut records = Vec::new();
    for tag in ["s0", "s1", "s2", "s3", "s4"] {
        let bytes = std::fs::read(format!("data/traces/hd-spu-light-companion/{tag}_slot.bin"))?;
        for record in bytes.as_chunks::<32>().0 {
            let r: Vec<f32> = record
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_be_bytes(*b))
                .collect();
            if r[7] != 0.0 {
                records.push((Vec3::new(r[0], r[1], r[2]), r[7]));
            }
        }
    }
    for environment in oag_hd::names::ENVIRONMENTS {
        let options = oag_raceplay::Options {
            source: "data/images/hdfury-ps3-eu-dec.iso".into(),
            track: Some(format!(r"Data\Environments\{environment}\track.vex")),
            team: Some("feisar_c1".into()),
            ..Default::default()
        };
        let loaded = match oag_raceplay::load(&options) {
            Ok(loaded) => loaded,
            Err(error) => {
                println!("{environment}: {error:#}");
                continue;
            }
        };
        let distances: Vec<f32> = records
            .iter()
            .map(|(p, _)| nearest_surface(&loaded.setup.collision, *p))
            .collect();
        let n = distances.len() as f32;
        let mean = distances.iter().sum::<f32>() / n;
        let min = distances.iter().cloned().fold(f32::MAX, f32::min);
        let max = distances.iter().cloned().fold(f32::MIN, f32::max);
        let within_5 = distances.iter().filter(|d| **d < 5.0).count();
        println!(
            "{environment:22} min {min:8.2} mean {mean:8.2} max {max:8.2}  within 5 units: {within_5}/{}",
            distances.len()
        );
    }
    Ok(())
}
