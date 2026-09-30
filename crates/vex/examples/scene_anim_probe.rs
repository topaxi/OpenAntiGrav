//! Scratch: list a scene's animated transforms and their loop periods.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage")?;
    let data = std::fs::read(&path)?;
    let nodes = oag_vex::vex::nodes(&data).map_err(|e| format!("{e}"))?;
    let anims = oag_vex::vex::anim_transforms(&data, &nodes);
    for (n, a) in nodes.iter().zip(&anims) {
        if let Some(a) = a {
            let last = |c: &oag_vex::vex::AnimChannel| c.times.last().copied().unwrap_or(0);
            println!(
                "{:<32} loop {:>8.3}s tkeys {:>4} (last {:>5}) rkeys {:>4} (last {:>5}) skeys {:>3} spk {:.5} flags {} step {}",
                n.name.as_deref().unwrap_or("?"), a.loop_seconds,
                a.translation.times.len(), last(&a.translation),
                a.rotation.times.len(), last(&a.rotation),
                a.scale.times.len(), a.seconds_per_key, a.flags, a.step
            );
        }
    }
    Ok(())
}
