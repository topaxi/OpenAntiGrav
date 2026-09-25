//! Scratch probe (not wired into any doc): dump any named `<Screen>` block
//! out of any `NEWGUI/*.xml` document, unparsed. Same crude nesting-depth
//! walk as `dump_newfeshell.rs`, generalised to any XML entry and any
//! screen name - used to rule out an authored event-card screen for
//! `campaign_map.rs`'s own "the bottom panel is gone" doc section by
//! checking `Launch 2048` (`InGame_Definition.xml`) and
//! `StartEventConfirm`/`FriendStartEventConfirm` (`Community_Definition.xml`)
//! alongside `newFEshell` itself.
//!
//! `cargo run -p oag-tools --example dump_screen_scratch -- <data.psarc> <xml entry path> <screen name>`

fn main() {
    let mut args = std::env::args().skip(1);
    let usage = "usage: dump_screen_scratch <data.psarc> <xml entry path> <screen name>";
    let path = args.next().expect(usage);
    let xml_path = args.next().expect(usage);
    let name = args.next().expect(usage);
    let mut archive =
        oag_assets::psarc::Archive::open(&path).unwrap_or_else(|e| panic!("open {path}: {e}"));
    let bytes = archive
        .read_path(&xml_path)
        .unwrap_or_else(|e| panic!("read {xml_path}: {e}"));
    let xml = String::from_utf8(bytes).expect("not UTF-8");
    let needle = format!("name=\"{name}\"");
    let start = xml
        .find(&needle)
        .unwrap_or_else(|| panic!("{name} not found in {xml_path}"));
    let tag_start = xml[..start].rfind("<Screen").expect("enclosing <Screen");
    let mut depth = 0usize;
    let mut end = tag_start;
    let bytes_xml = xml.as_bytes();
    let mut i = tag_start;
    while i < bytes_xml.len() {
        if xml[i..].starts_with("<Screen") {
            depth += 1;
            i += 7;
        } else if xml[i..].starts_with("</Screen>") {
            depth -= 1;
            i += 9;
            if depth == 0 {
                end = i;
                break;
            }
        } else {
            i += 1;
        }
    }
    println!("{}", &xml[tag_start..end]);
}
