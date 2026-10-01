//! Every declared section of a `track.vex` with the box it authors, as JSON, for
//! `scripts/pvs-cull-check.py --sections`.
//!
//! ```sh
//! cargo run -q -p oag-render --example pvs_section_boxes -- IMAGE ENTRY
//! ```

use oag_vex::pvs::TrackPvs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: pvs_section_boxes IMAGE ENTRY";
    let image = args.next().ok_or(usage)?;
    let entry = args.next().ok_or(usage)?;
    let mut archives = oag_pulse::open(&image)?;
    let blob = archives.read_name(&entry)?;
    let pvs = TrackPvs::parse(&blob)?;
    let mut out = Vec::new();
    for id in pvs.ids() {
        match pvs.bounds_of(id) {
            Some(b) => out.push(format!(
                "{{\"id\":{id},\"min\":{:?},\"max\":{:?}}}",
                b.min, b.max
            )),
            None => out.push(format!("{{\"id\":{id}}}")),
        }
    }
    println!("[{}]", out.join(",\n"));
    Ok(())
}
