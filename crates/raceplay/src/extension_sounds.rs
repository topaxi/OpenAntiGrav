//! A mod's own sounds: the `[sounds]` files of every mod that is on for this race, read
//! on the load thread with the race and decoded to PCM16 (`oag_audio::wav::decode`). They
//! reach the mixer through `oag_sound::sfx::ModSounds`, which a module's
//! `play-mod-sound` names by index.
//!
//! A file that fails to read or decode plays nothing; the loader report names it and the
//! race runs. Sounds are outputs: never hashed, never in the simulation
//! (`docs/formats/mod-sound.md`).

use crate::extensions::Source;
use oag_sound::sfx::ModSounds;

/// Reads and decodes the `[sounds]` of every mod that is on for `title`, in each mod's
/// name order; a slot is empty where the file did not load, and `report` names it.
#[must_use]
pub fn load(
    sources: &[Source],
    title: &str,
    foreign_track: Option<&std::path::Path>,
    report: &mut Vec<String>,
) -> ModSounds {
    #[cfg(not(feature = "extensions"))]
    {
        let _ = (sources, title, foreign_track, report);
        ModSounds::default()
    }
    #[cfg(feature = "extensions")]
    {
        read(sources, title, foreign_track, report)
    }
}

#[cfg(feature = "extensions")]
fn read(
    sources: &[Source],
    title: &str,
    foreign_track: Option<&std::path::Path>,
    report: &mut Vec<String>,
) -> ModSounds {
    use std::sync::Arc;
    let mut seen = std::collections::BTreeSet::new();
    let mut out = ModSounds::default();
    for source in crate::extensions::for_race(sources, title, foreign_track) {
        if source.sounds.is_empty() || !seen.insert(source.mod_id.clone()) {
            continue;
        }
        let mut loaded = Vec::new();
        for (name, path) in &source.sounds {
            let label = format!("mod {}: sound {name}", source.mod_id);
            let decoded = (source.files)(path).and_then(|bytes| {
                oag_audio::wav::decode(&bytes).map_err(|e| format!("{path}: {e}"))
            });
            match decoded {
                Ok(sound) => {
                    report.push(format!(
                        "{label}: {path} loaded ({} frames, {} Hz, {} ch)",
                        sound.frames(),
                        sound.sample_rate(),
                        sound.channels()
                    ));
                    loaded.push(Some(Arc::new(sound)));
                }
                Err(why) => {
                    let line = format!("{label} left out: {why}; plays nothing");
                    log::info!("{line}");
                    report.push(line);
                    loaded.push(None);
                }
            }
        }
        out.0.push((source.mod_id.clone(), loaded));
    }
    out
}
