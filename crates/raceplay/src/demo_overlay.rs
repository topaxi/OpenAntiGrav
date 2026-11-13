//! The screen an attract demo shows over its race, read off the disc at race load.
//!
//! Pulse's is `Demo InGame` in `Data\Plugins\PI001\GUI\Demo_Definition.xml`: two cyan rules that
//! slide in, the `demo_logo.mip` mark and a pulsing `BOOT_PRESS_ANY2` ("Press any button"), over a
//! race with no HUD (`docs/ghidra/functions/psp-pulse-usa/attract-demo.md`, "The state graph is
//! data"). The picture is `oag_game::demo_overlay::Overlay`; this half is the [`Assets`] a load
//! reads for it and carries in [`crate::Loaded::demo_overlay`].
//!
//! Nothing stands in for what will not read: a file, a state or a texture that is missing leaves
//! the overlay out, and the load report says which.

use oag_hud::sprite::Sheet;
use oag_ui::frontend::{Draw, Frontend, MoviePlan, Sequence, Step};
use oag_ui::screen::Screens;

/// Everything the overlay draws from, read once at race load.
#[derive(Debug, Clone)]
pub struct Assets {
    screens: Screens,
    strings: oag_ui::language::StringTable,
    state: &'static str,
    /// The screen's images, in one sheet.
    pub sheet: Sheet,
    /// The face its text draws in.
    pub font: oag_ui::font::Atlas,
    /// The grid the file's coordinates are in: the source's, as the HUD's are (the PS2 pressing
    /// authors 640x448).
    pub space: oag_display::space::Space,
}

impl Assets {
    /// The front end that draws the screen: the same drawing the boot's own screens get, so a
    /// pulsing text, a fading image and a sliding rule are the player the disc's other screens use.
    #[must_use]
    pub fn frontend(&self) -> Frontend {
        let sequence = Sequence {
            steps: vec![Step {
                state: self.state,
                movie: MoviePlan::none((480, 272)),
            }],
            backdrop_parent: None,
        };
        let mut frontend = Frontend::booting(
            sequence,
            self.screens.clone(),
            self.strings.clone(),
            Vec::new(),
            self.sheet.entries().to_vec(),
        );
        frontend.set_space(self.space);
        frontend
    }

    /// The state the screen is in the file.
    #[must_use]
    pub fn state(&self) -> &'static str {
        self.state
    }

    /// What the screen draws `seconds` after it appeared, over the race: no backdrop of its own.
    #[must_use]
    pub fn draw_list(&self, seconds: f32) -> Vec<Draw> {
        self.frontend()
            .draw_screen_over(self.state, f64::from(seconds))
    }
}

/// The overlay for `screen`, or `None` with a line in `report` for anything missing.
#[must_use]
pub fn read(
    archives: &mut oag_assets::Archives,
    screen: &oag_title::demo::Screen,
    strings: oag_ui::language::StringTable,
    font: oag_ui::font::Atlas,
    space: oag_display::space::Space,
    report: &mut Vec<String>,
) -> Option<Assets> {
    let entry = screen.entry;
    let xml = match archives
        .read_name(entry)
        .map_err(|e| e.to_string())
        .and_then(|blob| oag_tables::fexml::text(&blob).map_err(|e| e.to_string()))
    {
        Ok(xml) => xml,
        Err(why) => {
            report.push(format!("demo screen: {entry} did not read ({why}); none"));
            return None;
        }
    };
    let screens = Screens::from_xml(&xml);
    let Some(authored) = screens
        .screens
        .iter()
        .find(|s| s.name == screen.state || s.path == screen.state)
    else {
        report.push(format!(
            "demo screen: {entry} has no {}; none",
            screen.state
        ));
        return None;
    };
    let mut blobs: Vec<(String, Vec<u8>)> = Vec::new();
    for image in &authored.images {
        if blobs.iter().any(|(src, _)| *src == image.src) {
            continue;
        }
        // Pure's file spells the logo without its extension.
        match archives.read_texture(&image.src) {
            Ok((_, blob)) => blobs.push((image.src.clone(), blob)),
            Err(why) => report.push(format!(
                "demo screen: {} did not read ({why}); that texture is left out",
                image.src
            )),
        }
    }
    let mut notes = Vec::new();
    let sheet = Sheet::default().extended(&blobs, &mut notes);
    report.extend(notes.into_iter().map(|n| format!("demo screen: {n}")));
    report.push(format!(
        "demo screen: {} {:?}, {} text(s), {} image(s), {} rule(s)",
        entry,
        screen.state,
        authored.texts.len(),
        blobs.len(),
        authored.fills.len()
    ));
    Some(Assets {
        screens,
        strings,
        state: screen.state,
        sheet,
        font,
        space,
    })
}
