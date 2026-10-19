//! Scratch: every circuit's per-slot bound colour texture, for a before/after diff.
//!
//! `cargo run --example hd_albedo_dump -- <image> > out.txt`
use oag_mesh::mesh;
fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let specs: Vec<String> = (0..4)
        .map(|n| format!("{image}:PS3_GAME/USRDIR/DATA0{n}.PSARC"))
        .collect();
    let mut archives: Vec<_> = Vec::new();
    for s in &specs {
        if let Ok(a) = oag_assets::psarc::Archive::open(s) {
            archives.push((s.clone(), a));
        }
    }
    let mut tracks: Vec<(usize, String)> = Vec::new();
    for (i, (_, a)) in archives.iter().enumerate() {
        for p in a.paths() {
            if p.starts_with("/data/environments/")
                && (p.ends_with("/track.vex") || p.ends_with("/track_reversed.vex"))
            {
                tracks.push((i, p.clone()));
            }
        }
    }
    tracks.sort();
    for (i, path) in tracks {
        let spec = archives[i].0.clone();
        let Ok(data) = mesh::read_blob(&spec, &path) else {
            continue;
        };
        let Some(geometry) = mesh::rcs::sibling_geometry(&spec, &path, &data) else {
            continue;
        };
        let Ok(rcs) = oag_rcs::rcsmodel::Model::parse(&geometry) else {
            continue;
        };
        let Ok((model, _)) = mesh::rcs::build_scene(&path, &data, &geometry, &mut |n| {
            mesh::read_blob(&spec, n).ok()
        }) else {
            continue;
        };
        for (slot, m) in rcs.materials.iter().enumerate() {
            let label = model
                .textures
                .get(slot)
                .and_then(|t| t.as_ref())
                .map_or("-".to_string(), |t| t.label.clone());
            println!(
                "{path} {slot} {} {label}",
                m.name.rsplit('/').next().unwrap()
            );
        }
    }
    Ok(())
}
