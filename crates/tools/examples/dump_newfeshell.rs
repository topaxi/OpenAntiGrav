//! Scratch probe (not wired into any doc): dump the raw `newFEshell` screen
//! block out of `NEWGUI/Definition.xml`, unparsed, to see every attribute
//! and image reference the disc's own XML carries for it.
//!
//! `cargo run -p oag-tools --example dump_newfeshell -- <base data.psarc>`

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: dump_newfeshell <base data.psarc>");
    let mut archive =
        oag_assets::psarc::Archive::open(&path).unwrap_or_else(|e| panic!("open {path}: {e}"));
    let bytes = archive
        .read_path("data/plugins/frontend/NEWGUI/Definition.xml")
        .expect("read Definition.xml");
    let xml = String::from_utf8(bytes).expect("not UTF-8");
    // Crude: find the <Screen name="newFEshell" ...> block by nesting depth
    // of <Screen> tags, since this is a scratch probe and not a real parser.
    let start = xml
        .find("name=\"newFEshell\"")
        .expect("newFEshell not found");
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
