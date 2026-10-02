//! Where a screen filter comes from: the presets this crate ships, and the
//! ones a player drops into their own `shaders/` directory.
//!
//! A preset is a `.wgsl` file - see [`oag_render::post::screen`] for what is
//! in one - and its **id is its file stem**, which is what
//! `[render_profiles.<title>] screen_filter` holds and what the menu row
//! stores. The built-ins are compiled into the binary from
//! `assets/shaders/screen/`, so a fresh install has them without any file on
//! disk; a player's own live beside `settings.toml`, in
//! `<config dir>/oag/shaders/`, and **a player's file with a built-in's stem
//! replaces it**. That is how a built-in is tuned: copy it out, edit it, and
//! the menu row goes on naming the same thing. There is no separate tuning
//! UI, and the `min`/`max`/`step` a header declares are for the one that may
//! come.
//!
//! # A file that changes is picked up while the game runs
//!
//! [`Catalogue::poll`] re-reads any user file whose modification time moved
//! and bumps its [`Preset::revision`], which is what tells the pass built
//! from the older revision to rebuild. The frame loop calls it once a second,
//! so authoring a filter is edit, save, look - with a compile error landing
//! in the log rather than in a crash, per the render module's own contract.
//! A file that stops parsing keeps the last good revision on screen and says
//! so; a file that is deleted falls back to the built-in it shadowed, or to
//! nothing.
//!
//! # Why the built-ins are only these
//!
//! Every shipped preset is this project's own text, written from the
//! technique rather than transliterated from a published shader - the same
//! route `oag_render::post::fxaa` took, and for the same reason: the popular
//! CRT and handheld shader collections are largely GPL, which this MIT and
//! Apache-2.0 tree cannot vendor. The user directory is the answer for
//! anything under a licence the repository cannot carry.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use log::{debug, warn};
use oag_render::post::screen::Preset;
use oag_ui::menu;

use crate::settings::SCREEN_FILTER_OFF;

/// The presets compiled into the binary, in the order the menu offers them.
///
/// PSP first, then the CRTs, because Pulse on the PSP is this project's
/// reference title and the first row after `off` is the one a player sees.
pub const BUILT_IN: &[(&str, &str)] = &[
    (
        "psp-3000",
        include_str!("../../../assets/shaders/screen/psp-3000.wgsl"),
    ),
    (
        "psp-lcd",
        include_str!("../../../assets/shaders/screen/psp-lcd.wgsl"),
    ),
    (
        "crt-interlaced",
        include_str!("../../../assets/shaders/screen/crt-interlaced.wgsl"),
    ),
    (
        "crt-shadow-mask",
        include_str!("../../../assets/shaders/screen/crt-shadow-mask.wgsl"),
    ),
    (
        "crt-aperture",
        include_str!("../../../assets/shaders/screen/crt-aperture.wgsl"),
    ),
];

/// One user file: what it parsed to, and when it was last read.
#[derive(Debug)]
struct UserFile {
    path: PathBuf,
    modified: Option<SystemTime>,
    /// `None` while the file has never parsed. A file that parsed once and
    /// then broke keeps its last good preset here - see the module doc.
    preset: Option<Preset>,
}

/// Every preset this run can offer.
#[derive(Debug)]
pub struct Catalogue {
    built_in: Vec<Preset>,
    user: Vec<UserFile>,
    directory: Option<PathBuf>,
    /// The next revision to hand a re-read file. Monotonic across the whole
    /// catalogue rather than per file, so a preset that is deleted and
    /// recreated does not reuse a number a pass has already been built from.
    next_revision: u64,
}

impl Catalogue {
    /// The built-ins alone, from the binary: what a capture or a test wants.
    ///
    /// # Panics
    ///
    /// On a built-in that does not parse, which is a build-time mistake the
    /// `every_built_in_preset_parses_and_validates` test catches first.
    #[must_use]
    pub fn built_in() -> Self {
        let built_in = BUILT_IN
            .iter()
            .map(|(id, source)| {
                Preset::parse(id, source)
                    .unwrap_or_else(|why| panic!("built-in screen filter {id}: {why:#}"))
            })
            .collect();
        Self {
            built_in,
            user: Vec::new(),
            directory: None,
            next_revision: 1,
        }
    }

    /// The built-ins plus whatever is in `directory`, read once now.
    ///
    /// A directory that does not exist is not an error - most installs never
    /// make one - and neither is a file in it that does not parse, which is
    /// reported and skipped so the rest of the list still loads.
    #[must_use]
    pub fn load(directory: Option<PathBuf>) -> Self {
        let mut catalogue = Self::built_in();
        catalogue.directory = directory;
        catalogue.poll();
        let user = catalogue.user.iter().filter(|f| f.preset.is_some()).count();
        match &catalogue.directory {
            Some(dir) => debug!(
                "screen filters: {} built in, {user} from {}",
                catalogue.built_in.len(),
                dir.display()
            ),
            None => debug!("screen filters: {} built in", catalogue.built_in.len()),
        }
        catalogue
    }

    /// Where a player's own presets live: `<config dir>/oag/shaders/`, beside
    /// `settings.toml`. `None` on a platform with no config directory.
    #[must_use]
    pub fn directory() -> Option<PathBuf> {
        crate::settings::path()
            .and_then(|settings| settings.parent().map(|dir| dir.join("shaders")))
    }

    /// The preset `id` names, a user file winning over a built-in of the same
    /// stem. `None` for `off`, and for an id nothing on this machine matches.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Preset> {
        if id == SCREEN_FILTER_OFF {
            return None;
        }
        self.user
            .iter()
            .filter_map(|file| file.preset.as_ref())
            .find(|preset| preset.id == id)
            .or_else(|| self.built_in.iter().find(|preset| preset.id == id))
    }

    /// The menu row's list: `off`, then every preset by id, labelled with the
    /// name its header gave it.
    ///
    /// Built-ins in their shipped order, a user file that shadows one in that
    /// built-in's place, and the rest of the user's files after, by id.
    #[must_use]
    pub fn choices(&self) -> Vec<menu::Choice> {
        let mut out = vec![menu::Choice::plain(SCREEN_FILTER_OFF)];
        for built_in in &self.built_in {
            let preset = self.get(&built_in.id).unwrap_or(built_in);
            out.push(menu::Choice::labelled(&preset.id, &preset.name));
        }
        let mut extra: Vec<&Preset> = self
            .user
            .iter()
            .filter_map(|file| file.preset.as_ref())
            .filter(|preset| !self.built_in.iter().any(|b| b.id == preset.id))
            .collect();
        extra.sort_by(|a, b| a.id.cmp(&b.id));
        out.extend(
            extra
                .into_iter()
                .map(|preset| menu::Choice::labelled(&preset.id, &preset.name)),
        );
        out
    }

    /// Re-reads the user directory: new files are parsed, changed files are
    /// re-parsed and given a fresh revision, deleted files are dropped.
    ///
    /// Returns whether anything changed. Cheap when nothing did - one
    /// `read_dir` and a `stat` per file - which is what lets the frame loop
    /// call it every second.
    pub fn poll(&mut self) -> bool {
        let Some(directory) = self.directory.clone() else {
            return false;
        };
        let mut changed = false;
        let mut seen: Vec<PathBuf> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&directory) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_none_or(|ext| ext != "wgsl") {
                    continue;
                }
                seen.push(path.clone());
                let modified = entry.metadata().and_then(|m| m.modified()).ok();
                match self.user.iter_mut().find(|file| file.path == path) {
                    Some(file) if file.modified == modified => {}
                    Some(file) => {
                        file.modified = modified;
                        changed |= read_into(file, self.next_revision);
                        self.next_revision += 1;
                    }
                    None => {
                        let mut file = UserFile {
                            path,
                            modified,
                            preset: None,
                        };
                        changed |= read_into(&mut file, self.next_revision);
                        self.next_revision += 1;
                        self.user.push(file);
                    }
                }
            }
        }
        let before = self.user.len();
        self.user.retain(|file| seen.contains(&file.path));
        changed |= self.user.len() != before;
        changed
    }
}

/// Parses `file` from disk into its `preset`, stamped with `revision`.
///
/// Returns whether the preset changed. A file that will not parse is reported
/// once per change and leaves the last good preset in place.
fn read_into(file: &mut UserFile, revision: u64) -> bool {
    let id = stem(&file.path);
    let source = match std::fs::read_to_string(&file.path) {
        Ok(source) => source,
        Err(why) => {
            warn!(
                "screen filter {id}: {} could not be read: {why}",
                file.path.display()
            );
            return false;
        }
    };
    match Preset::parse(&id, &source) {
        Ok(mut preset) => match preset.validate() {
            Ok(()) => {
                preset.revision = revision;
                debug!(
                    "screen filter {id}: loaded from {} (revision {revision})",
                    file.path.display()
                );
                file.preset = Some(preset);
                true
            }
            Err(why) => {
                warn!(
                    "screen filter {id}: {} does not compile; keeping what it was:\n{why:#}",
                    file.path.display()
                );
                false
            }
        },
        Err(why) => {
            warn!(
                "screen filter {id}: {} was skipped: {why:#}",
                file.path.display()
            );
            false
        }
    }
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
