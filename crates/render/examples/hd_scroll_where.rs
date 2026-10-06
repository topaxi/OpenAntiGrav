//! Where a circuit's vertex-scrolled surfaces are: per `AnimTrack::Scroll`
//! track, its rate and the bounding box and centre of the vertices it drives,
//! so a camera can be put in front of one.
//!
//! ```sh
//! cargo run -p oag-render --example hd_scroll_where -- <image:archive> /data/environments/amphiseum/track.vex
//! ```

use oag_mesh::mesh::{self, AnimTrack};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/amphiseum/track.vex".into());
    let data = mesh::read_blob(&spec, &name)?;
    let Some((model, _)) = mesh::rcs::scene_from(&spec, &name, &data)? else {
        anyhow::bail!("no PS3 sibling geometry");
    };
    for (i, track) in model.anim_tracks.iter().enumerate() {
        let AnimTrack::Scroll(rate) = track else {
            continue;
        };
        let (mut lo, mut hi, mut n) = ([f32::MAX; 3], [f32::MIN; 3], 0usize);
        for v in model.vertices.iter().filter(|v| v.anim as usize == i + 1) {
            n += 1;
            for k in 0..3 {
                lo[k] = lo[k].min(v.position[k]);
                hi[k] = hi[k].max(v.position[k]);
            }
        }
        let mut shown = std::collections::BTreeSet::new();
        for v in model.vertices.iter().filter(|v| v.anim as usize == i + 1) {
            let cell = [
                (v.position[0] / 40.0) as i32,
                (v.position[1] / 40.0) as i32,
                (v.position[2] / 40.0) as i32,
            ];
            if shown.len() < 4 && shown.insert(cell) {
                println!("  sample at {:?} normal {:?}", v.position, v.normal);
            }
        }
        println!(
            "track {} rate {rate:?}: {n} vertices, box {lo:?}..{hi:?}",
            i + 1
        );
    }
    Ok(())
}
