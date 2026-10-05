//! Scratch probe: every printable string in a `.vex` and its sibling
//! `.rcsmodel`, so a file that names other files can be told from one that
//! does not.

use oag_mesh::mesh;

fn strings(label: &str, data: &[u8]) {
    let mut out: Vec<String> = Vec::new();
    let mut run = Vec::new();
    for &b in data {
        if b.is_ascii_graphic() || b == b' ' {
            run.push(b);
        } else {
            if run.len() >= 6 {
                out.push(String::from_utf8_lossy(&run).into_owned());
            }
            run.clear();
        }
    }
    if run.len() >= 6 {
        out.push(String::from_utf8_lossy(&run).into_owned());
    }
    println!("{label}: {} string(s) of 6+ characters", out.len());
    let mut seen = std::collections::BTreeSet::new();
    for s in out {
        let lower = s.to_ascii_lowercase();
        if [
            ".vex",
            ".rcsmodel",
            ".rcsmaterial",
            ".gtf",
            ".ma",
            ".dae",
            ".xml",
            ".psarc",
        ]
        .iter()
        .any(|e| lower.contains(e))
            && seen.insert(lower.clone())
        {
            println!("  {s}");
        }
    }
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let spec = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso:PS3_GAME/USRDIR/DATA00.PSARC".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/talons_junction/track.vex".into());
    let data = mesh::read_blob(&spec, &name)?;
    strings(&name, &data);
    if let Some(geometry) = mesh::rcs::sibling_geometry(&spec, &name, &data) {
        strings("sibling .rcsmodel", &geometry);
    }
    Ok(())
}
