//! Scratch probe for the `handover/frontend/hd-furys-hud-draws-and-three-of-pulses.md`
//! tint items: dumps the **raw, unresolved** XML for `DamageBar`/`DamageBarBg`/
//! `LapBar*`/`PosBar*` across every fragment file each of the eighteen composed
//! layouts reads, to check whether either references a symbolic
//! `FEConst->`/`FEGlobals->` name that the HUD's own constant sweep never
//! declares (which would silently resolve to white today) versus a literal
//! colour already baked into the widget.
//!
//! ```sh
//! cargo run -q -p oag-game --example hd_hud_tint_census -- data/images/hdfury-ps3-eu-dec.iso
//! ```

use std::collections::BTreeSet;

const WATCH: &[&str] = &[
    "DamageBar",
    "DamageBarBg",
    "LapBar0",
    "LapBar1",
    "LapBar2",
    "LapBar3",
    "LapBar4",
    "LapBar5",
    "LapBar6",
    "PosBar0",
    "PosBar1",
    "PosBar2",
    "PosBar3",
    "PosBar4",
    "PosBar5",
    "PosBar6",
    "PosBar7",
];

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".to_string());
    let mut archives = oag_hd::open(&path)?;
    let mut read = |p: &str| archives.read_name(p).ok();

    let mut all_files: BTreeSet<String> = BTreeSet::new();
    for root in oag_hd::hud::ROOTS {
        let Some(composed) = oag_game::hud::compose(root, &mut read) else {
            continue;
        };
        all_files.extend(composed.files);
    }

    for file in &all_files {
        let Some(blob) = read(file) else { continue };
        let Ok(xml) = oag_formats::fexml::text(&blob) else {
            continue;
        };
        for name in WATCH {
            let needle = format!("name=\"{name}\"");
            for (idx, _) in xml.match_indices(&needle) {
                // Print the enclosing element: back up to the last '<' and
                // forward to the next '>'.
                let start = xml[..idx].rfind('<').unwrap_or(idx);
                let end = xml[idx..]
                    .find("</Image>")
                    .map(|e| idx + e + "</Image>".len())
                    .or_else(|| xml[idx..].find("/>").map(|e| idx + e + 2))
                    .unwrap_or(xml.len());
                println!("  {file}: {}", xml[start..end].replace('\n', " "));
            }
        }
    }
    Ok(())
}
