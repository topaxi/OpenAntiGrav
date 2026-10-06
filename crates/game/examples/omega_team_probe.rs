//! Lists Omega's raceable teams as the catalogue reads them.
fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/extracted/ps4".into());
    let mut archives = oag_omega::open(&path).expect("opening Omega");
    let xml = archives
        .read_name(oag_omega::TITLE.plugin_definition)
        .expect("teams definition");
    let xml = String::from_utf8_lossy(&xml).into_owned();
    for t in oag_raceplay::catalogue::teams(&xml) {
        println!("{:<22} loc={:<40} name={:?}", t.id, t.location, t.name);
    }
}
