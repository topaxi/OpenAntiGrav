//! Scratch probe: the live race report showed altima's `track.rcsmodel` as
//! carrying **no material table**, even though `vita_rcsmodel_material_probe.rs`
//! found `.rcsmaterial`/`.gxt` text all over its section B. Why?
//!
//! # The count and table pointer were right; the cap on them was not
//!
//! `oag_rcs::rcsmodel::psp2::material::read` rejects a file-level
//! material count over `MAX_COUNT`, calibrated at 64 from a census that only
//! sampled small props and a 16-submesh craft. Reading altima's own `+0x44`/
//! `+0x48` raw (this probe) gives count `527`, table `0x6f2c0` - and every
//! one of the 527 offsets that table holds lands on a header whose name
//! resolves to real `.rcsmaterial` text (checked here directly against the
//! first ten and the last). **64 was rejecting a genuine track outright**,
//! not catching a bad read - `MAX_COUNT` is `8192` now.
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_rcsmodel_material_count_bound
//! ```

use oag_rcs::rcsmodel::psp2;

const TRACK: &str = "data/extracted/vita/PCSF00007/base/PSP2/data.psarc";
const TRACK_PATH: &str = "Data/art/published/environments/altima/track.rcsmodel";

fn cstr_at(cpu: &[u8], at: usize) -> Option<String> {
    let end = (at..cpu.len()).find(|&k| cpu[k] == 0)?;
    (end != at).then(|| String::from_utf8_lossy(&cpu[at..end]).into_owned())
}

fn main() -> anyhow::Result<()> {
    let mut archive = oag_assets::psarc::Archive::open(TRACK)?;
    let blob = archive.read_path(TRACK_PATH)?;

    println!(
        "psp2::parse's own materials field: {} (with MAX_COUNT fixed)",
        psp2::parse(&blob)?.materials.len()
    );

    let model = psp2::parse(&blob)?;
    let cpu = model.sections[0];
    let bytes = &blob[cpu.at..cpu.at + cpu.len];
    let count = u32::from_le_bytes(bytes[0x44..0x48].try_into().unwrap()) as usize;
    let table = u32::from_le_bytes(bytes[0x48..0x4c].try_into().unwrap()) as usize;
    println!("raw file header: count {count} ({count:#x}), table @{table:#x}");

    let entry = |i: usize| {
        u32::from_le_bytes(bytes[table + i * 4..table + i * 4 + 4].try_into().unwrap()) as usize
    };
    let mut resolved = 0usize;
    for i in 0..count {
        let header_at = entry(i);
        let name_ptr = u32::from_le_bytes(bytes[header_at + 4..header_at + 8].try_into().unwrap());
        if cstr_at(bytes, name_ptr as usize)
            .is_some_and(|s| s.to_ascii_lowercase().ends_with(".rcsmaterial"))
        {
            resolved += 1;
        }
    }
    println!("{resolved}/{count} table entries resolve to real .rcsmaterial text");
    println!(
        "first entry @{:#x}, last entry @{:#x}",
        entry(0),
        entry(count - 1)
    );

    Ok(())
}
