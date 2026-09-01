//! Scratch probe: what track labels does `remix::catalogue` build for
//! Wipeout HD - the reproducer for the fix that wires `crate::boot::load_circuit_names`
//! into it, since a direct `strings.get(&track.id)` resolves nothing on HD.

fn main() -> anyhow::Result<()> {
    let catalogue = oag_game::remix::catalogue("data/images/hdfury-ps3-eu-dec.iso")?;
    println!("{} track(s) offered:", catalogue.tracks.len());
    for (track, label) in &catalogue.tracks {
        println!("  id {:?} -> label {:?}", track.id, label);
    }
    Ok(())
}
