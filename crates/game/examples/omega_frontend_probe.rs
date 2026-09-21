//! Scratch probe: does this project's own front-end parser
//! ([`oag_ui::screen::Screens`]) make sense of the PS4 Omega Collection's
//! `skin.xml`/`mainmenu_definition.xml`, read straight out of the patch
//! archive (`data09.psarc`)? Omega is not an `oag-title` yet, so this reads
//! the `.psarc` directly rather than going through `oag_assets::Archives`.
//!
//! See `docs/formats/omega-frontend.md` for the census this supports.
//!
//! ```sh
//! cargo run -p oag-game --example omega_frontend_probe -- \
//!     data/extracted/ps4/omega-eu-patch/uroot/data09.psarc
//! ```
use oag_ui::screen::Screens;

fn probe(archive: &mut oag_assets::psarc::Archive, entry: &str) {
    let Ok(bytes) = archive.read_path(entry) else {
        println!("{entry}: not found in this archive");
        return;
    };
    let xml = if oag_tables::fexml::is_fexml(&bytes) {
        match oag_tables::fexml::expand(&bytes) {
            Ok(text) => text,
            Err(e) => {
                println!(
                    "{entry}: {} bytes, fexml-encoded, failed to expand: {e}",
                    bytes.len()
                );
                return;
            }
        }
    } else {
        match String::from_utf8(bytes.clone()) {
            Ok(text) => text,
            Err(e) => {
                println!("{entry}: {} bytes, not UTF-8: {e}", bytes.len());
                return;
            }
        }
    };
    let screens = Screens::from_xml(&xml);
    println!(
        "{entry}: {} bytes -> {} screens, {} globals, {} LoadXML includes",
        bytes.len(),
        screens.screens.len(),
        screens.globals.len(),
        screens.load_xml.len()
    );
    for name in [
        "MenuXOffset",
        "MenuYOffset",
        "TitleXOffset",
        "TitleYOffset",
        "TextColor",
    ] {
        if let Some(value) = screens.globals.get(name) {
            println!("  {name} = {value}");
        }
    }
    for screen in screens.screens.iter().take(5) {
        println!("  screen: {}", screen.name);
    }
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: omega_frontend_probe <path/to/dataNN.psarc>");
    let mut archive =
        oag_assets::psarc::Archive::open(&path).unwrap_or_else(|e| panic!("open {path}: {e}"));
    for entry in [
        "data/plugins/frontend/gui/skin.xml",
        "data/plugins/frontend/gui/mainmenu_definition.xml",
        "data/plugins/frontend/gui/cellmode_definition.xml",
        "data/plugins/frontend/gui/campaign2048_definition.xml",
    ] {
        probe(&mut archive, entry);
    }
}
