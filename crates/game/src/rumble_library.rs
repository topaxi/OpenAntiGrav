//! A race's pattern library: every pattern the event map names, from whichever
//! layer holds it, and a report of where each came from.
//!
//! Precedence per name: a player's override (`<name>.xml` or `<name>.toml`),
//! then the disc's `<name>.xml`, then this project's own `<name>.toml` in the
//! fallback layer (ADR-0061). A title whose disc ships none (Pulse, Pure) plays
//! the fallback's; a name nothing holds is left out and reported, and a title
//! with no pattern at all plays nothing.

use oag_assets::Archives;
use oag_assets::layers::Kind as Layer;
use oag_input::rumble::map::ROWS;
use oag_tables::rumble::Library;

use crate::rumble::disc_pattern;

/// Where the fallback and override layers keep a TOML pattern.
fn toml_entry(name: &str) -> String {
    format!("Data\\XML\\Rumble\\{name}.toml")
}

/// Where a library's pattern came from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Names read off the disc.
    pub disc: usize,
    /// Names this project's own fallback answered.
    pub fallback: usize,
    /// Names a player's override answered.
    pub overridden: usize,
    /// Names no layer holds, so their rows play nothing.
    pub missing: Vec<String>,
}

impl Report {
    /// The loader-report line.
    #[must_use]
    pub fn line(&self) -> String {
        if self.disc + self.fallback + self.overridden == 0 {
            return "rumble: no pattern found for this title, so the pad stays still".to_owned();
        }
        let mut line = format!(
            "rumble: {} patterns from the disc, {} from this project's own fallback, {} from overrides",
            self.disc, self.fallback, self.overridden
        );
        if !self.missing.is_empty() {
            line.push_str(&format!("; none for {}", self.missing.join(", ")));
        }
        line
    }
}

/// The library for the names [`ROWS`] fires, read through `archives`.
pub fn load(archives: &mut Archives) -> (Library, Report) {
    let mut library = Library::new();
    let mut report = Report::default();
    let mut names: Vec<&str> = ROWS.iter().map(|r| r.pattern).collect();
    names.sort_unstable();
    names.dedup();
    for name in names {
        let entry = toml_entry(name);
        let from_layer = |archives: &mut Archives| {
            let bytes = archives.read_name(&entry).ok()?;
            oag_mod::rumble::parse(std::str::from_utf8(&bytes).ok()?).ok()
        };
        let overridden = archives.layer_of(&entry) == Some(Layer::Override);
        if overridden && let Some(pattern) = from_layer(archives) {
            library.insert(name, pattern);
            report.overridden += 1;
        } else if let Ok(pattern) = disc_pattern(archives, name) {
            library.insert(name, pattern);
            report.disc += 1;
        } else if let Some(pattern) = from_layer(archives) {
            library.insert(name, pattern);
            report.fallback += 1;
        } else {
            report.missing.push(name.to_owned());
        }
    }
    (library, report)
}
