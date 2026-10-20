//! Scratch probe: the spline points' light-scale bytes within a radius of a position.
//!
//! ```sh
//! cargo run -q -p oag-vex --example spline_scale_near -- data/images/pulse-psp-usa.chd 'Data\Environments\05_Track\track_reversed.vex' 20.22 -17.4 -67.57 60
//! ```
use oag_vex::{track, vex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut archives = oag_pulse::open(&a[0])?;
    let blob = archives.read_name(&a[1])?;
    let nodes = vex::nodes(&blob)?;
    let at: Vec<f32> = a[2..5].iter().map(|s| s.parse().unwrap()).collect();
    let radius: f32 = a[5].parse()?;
    let node = track::find_node(&blob, &nodes).ok_or("no track node")?;
    let track = track::parse(&blob[node.payload()])?;
    for (pi, path) in track.paths.iter().enumerate() {
        for (i, p) in path.points.iter().enumerate() {
            let d = ((p.pos[0] - at[0]).powi(2)
                + (p.pos[1] - at[1]).powi(2)
                + (p.pos[2] - at[2]).powi(2))
            .sqrt();
            if d < radius {
                println!(
                    "path {pi} point {i} d {d:.1} pos {:?} scale {:?}",
                    p.pos.map(|x| (x * 10.0).round() / 10.0),
                    p.light_scale
                );
            }
        }
    }
    Ok(())
}
