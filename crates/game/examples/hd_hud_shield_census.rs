//! Scratch probe for `docs/formats/hd-hud.md#what-is-not-done`'s
//! `ShieldBarText` item: does HD author a `%`-unit companion widget the way it
//! authors `SpeedBarTextKMH` beside `SpeedBarText`, and what does
//! `ShieldBarText`'s resolved colour actually come out to on both titles?
//!
//! ```sh
//! cargo run -q -p oag-game --example hd_hud_shield_census -- data/images/hdfury-ps3-eu-dec.iso
//! cargo run -q -p oag-game --example hd_hud_shield_census -- data/images/pulse-psp-usa.chd
//! ```

use oag_game::hud;

fn dump_labels(name: &str, layout: &hud::Layout) {
    for label in &layout.labels {
        if label.name.to_ascii_lowercase().contains("percent")
            || label.string.as_deref().unwrap_or("").contains('%')
        {
            println!(
                "  {name}: PERCENT? name={:?} idstring={:?} string={:?}",
                label.name, label.idstring, label.string
            );
        }
        if label.name.to_ascii_lowercase().contains("shield") {
            println!(
                "  {name}: Text name={:?} idstring={:?} string={:?} color={:?} border={:?}",
                label.name, label.idstring, label.string, label.color, label.border
            );
        }
    }
    for sprite in &layout.sprites {
        if sprite.name.to_ascii_lowercase().contains("shield") {
            println!(
                "  {name}: Image name={:?} color={:?} src={:?}",
                sprite.name, sprite.color, sprite.src
            );
        }
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".to_string());

    if path.contains("hdfury") {
        let mut archives = oag_hd::open(&path)?;
        let mut read = |p: &str| archives.read_name(p).ok();
        for root in oag_hd::hud::ROOTS {
            if let Some(composed) = hud::compose(root, &mut read) {
                dump_labels(root, &composed.layout);
            }
        }
    } else {
        let mut archives = oag_pulse::open(&path).or_else(|_| oag_pure::open(&path))?;
        for (mode, root) in [
            ("arcade", oag_pulse::hud::layouts::ARCADE),
            ("elimination", oag_pulse::hud::layouts::ELIMINATION),
            ("timetrial", oag_pulse::hud::layouts::TIME_TRIAL),
            ("zone", oag_pulse::hud::layouts::ZONE),
        ] {
            let Ok(blob) = archives.read_name(root) else {
                continue;
            };
            let xml = oag_tables::fexml::text(&blob)?;
            let node = oag_tables::fexml::parse(&xml);
            let layout = hud::Layout::from_tree(&node);
            dump_labels(mode, &layout);
        }
    }
    Ok(())
}
