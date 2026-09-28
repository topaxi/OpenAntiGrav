//! Scratch probe: `M_RANKREQUIRED` across every event, to check whether the
//! five "P Ship Challenge"/ten "S Phantom Challenge" side events (measured as
//! open on a fresh save by the unlock graph alone) also carry a rank gate the
//! engine does not enforce.
//!
//! `cargo run -p oag-tools --example rank_required_scan -- <base data.psarc>`

fn main() {
    let psarc_path = std::env::args()
        .nth(1)
        .expect("usage: rank_required_scan <base data.psarc>");
    let mut archive = oag_assets::psarc::Archive::open(&psarc_path).unwrap();
    let sp_bytes = archive
        .read_path(r"Data\xml\SP.xml")
        .or_else(|_| archive.read_path("data/xml/SP.xml"))
        .unwrap();
    let sp_xml = String::from_utf8(sp_bytes).unwrap();
    let doc = oag_tables::mjolnir::parse(&sp_xml);

    let event_typedefs = [-1915183557i64, -1353052320, 1311982788, 1018671239];
    let mut with_rank = 0usize;
    let mut total = 0usize;
    for inst in &doc.instances {
        if !event_typedefs.contains(&inst.typedef_id) {
            continue;
        }
        total += 1;
        if let Some(rank) = inst.field("M_RANKREQUIRED").and_then(|f| f.int()) {
            with_rank += 1;
            println!("{}: M_RANKREQUIRED={}", inst.name, rank);
        }
    }
    println!("\n{with_rank}/{total} events author M_RANKREQUIRED");
}
