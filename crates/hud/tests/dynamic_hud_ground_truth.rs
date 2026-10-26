//! Pulse's `Dynamic HUD` option against the discs it was read off.
//!
//! `#[ignore]`d: needs real images under `data/images/`. `just test-data`.
//!
//! `oag_pulse::TITLE`'s `HudArt::dynamic_hud` names the entries of the list its Game Options
//! page authors; this reads the list off both PSP discs and holds the constant to it, so the
//! row this build offers can not drift from what the disc says.

use oag_tables::fexml::{self, Node};

fn find<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    if node.name.eq_ignore_ascii_case("List") && node.attr("name") == Some(name) {
        return Some(node);
    }
    node.children.iter().find_map(|child| find(child, name))
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and pulse-psp-eu.chd"]
fn pulses_dynamic_hud_list_is_what_the_constant_says() {
    let list = oag_pulse::TITLE
        .hud_art
        .dynamic_hud
        .expect("Pulse offers the option")
        .value;
    for image in [
        "data/images/pulse-psp-usa.chd",
        "data/images/pulse-psp-eu.chd",
    ] {
        let Some(path) = oag_testdata::image(image) else {
            continue;
        };
        let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_pulse::TITLE)
            .expect("the archives open");
        let raw = archives
            .read_name(r"Data\Plugins\PI001\GUI\Additional_Definition.xml")
            .expect("the options definition");
        let xml = fexml::text(&raw).expect("it is text");
        let root = fexml::parse(&xml);
        let dynamic = find(&root, "Dynamic HUD").unwrap_or_else(|| panic!("{image}: no list"));
        let entries: Vec<&str> = dynamic
            .children
            .iter()
            .filter(|child| child.name.eq_ignore_ascii_case("Entry"))
            .filter_map(|entry| entry.attr("idstring"))
            .collect();
        assert_eq!(entries, list.entries, "{image}");
        assert_eq!(dynamic.attr("Default"), Some(list.entries[0]), "{image}");
        assert!(entries.contains(&list.off), "{image}");
    }
}
