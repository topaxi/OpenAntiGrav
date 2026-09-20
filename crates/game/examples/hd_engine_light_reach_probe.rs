//! Scratch probe: how far is each SPU vertex light from the nearest track
//! surface - the original's captured records against ours?
//!
//! `EngineFlare_SubmitSpuLight`'s `D` is `Radius +- 0.1`, 0.6-2.1 units on
//! the whole disc, and a craft rides at `ride_height * 0.75` = 4.1 units
//! (Feisar). If the live-captured light positions in
//! `data/traces/hd-spu-light-companion/meta.json` sit about that far from
//! Talon's Junction's collision soup (the circuit the capture turns out to be on - see `hd_engine_light_which_circuit`), the original's engine light never reaches the
//! floor at ride height either, and the hull is its only receiver at rest.
//! If they sit within `D`, our placement is wrong somewhere.
//!
//! ```sh
//! cargo run --release -p oag-game --example hd_engine_light_reach_probe
//! ```

use oag_core::math::Vec3;
use oag_gameplay::PlayerInputs;

fn point_triangle_distance(p: Vec3, [a, b, c]: [Vec3; 3]) -> f32 {
    // Ericson, Real-Time Collision Detection, closest point on triangle.
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
    let options = oag_game::race::Options {
        source: "data/images/hdfury-ps3-eu-dec.iso".into(),
        track: Some(r"Data\Environments\talons_junction\track.vex".into()),
        team: Some("feisar_c1".into()),
        opponents: true,
        ..Default::default()
    };
    let loaded = oag_game::race::load(&options)?;
    let triangles: usize = loaded
        .setup
        .collision
        .colliders()
        .iter()
        .map(|s| s.triangle_count())
        .sum();
    println!("talons_junction collision: {triangles} triangles");
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for soup in loaded.setup.collision.colliders() {
        for t in 0..soup.triangle_count() {
            if let Some(tri) = soup.triangle(t) {
                for v in tri {
                    lo = lo.min(v);
                    hi = hi.max(v);
                }
            }
        }
    }
    println!("collision bounds {lo:?} .. {hi:?}");

    println!(
        "\noriginal (live capture), distance from each record to the nearest collision triangle:"
    );
    let mut original = Vec::new();
    for tag in ["s0", "s1", "s2", "s3", "s4"] {
        // 8 big-endian f32 per record: (x, y, z, w, r, g, b, D).
        let bytes = std::fs::read(format!("data/traces/hd-spu-light-companion/{tag}_slot.bin"))?;
        for record in bytes.as_chunks::<32>().0 {
            let r: Vec<f32> = record
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_be_bytes(*b))
                .collect();
            if r[7] == 0.0 {
                continue;
            }
            let p = Vec3::new(r[0], r[1], r[2]);
            let d = nearest_surface(&loaded.setup.collision, p);
            original.push(d);
            println!(
                "  {tag} ({:8.2}, {:8.2}, {:8.2}) colour ({:.0}, {:.0}, {:.0}) D {:.2}  nearest surface {:.2}  {}",
                r[0],
                r[1],
                r[2],
                r[4],
                r[5],
                r[6],
                r[7],
                d,
                if d < r[7] { "IN RANGE" } else { "" }
            );
        }
    }

    let mut race = oag_game::race::Race::start(loaded.setup);
    // The same input a `--hold cross` screenshot run drives, so a tick
    // printed here is the `--ticks` a capture wants.
    let mut held = oag_game::race::HeldButtons::new(oag_gameplay::input::Button::Cross.bit());
    let mut ours = Vec::new();
    let mut first_boost: Option<u32> = None;
    println!("\nours, every 100 ticks (player holding cross):");
    for tick in 1..=1200 {
        held.advance(None, 0, oag_gameplay::input::Button::Cross.bit(), tick);
        race.tick(&PlayerInputs::single(held.snapshot()));
        if first_boost.is_none()
            && let Some(player) = race.hd_engine_lights().first()
            && player.colour[0] > 200.0
        {
            first_boost = Some(tick);
            println!("  first tick with the player's light boosted past 5x: {tick}");
        }
        if tick % 100 == 0 {
            for light in race.hd_engine_lights() {
                let p = Vec3::from_slice(&light.position[..3]);
                let d = nearest_surface(race.collision(), p);
                ours.push(d);
                println!(
                    "  tick {tick} ({:8.2}, {:8.2}, {:8.2}) D {:.2}  nearest surface {:.2}  {}",
                    p.x,
                    p.y,
                    p.z,
                    light.colour[3],
                    d,
                    if d < light.colour[3] { "IN RANGE" } else { "" }
                );
            }
        }
    }
    let stats = |v: &[f32]| {
        let n = v.len() as f32;
        let mean = v.iter().sum::<f32>() / n;
        let min = v.iter().cloned().fold(f32::MAX, f32::min);
        let max = v.iter().cloned().fold(f32::MIN, f32::max);
        (min, mean, max)
    };
    println!("\noriginal: min/mean/max {:?}", stats(&original));
    println!("ours:     min/mean/max {:?}", stats(&ours));
    Ok(())
}
