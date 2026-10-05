//! Where an effect lives in an archive.

/// Where the original looks an effect up: `Data\Psys\<name>.POB`, built at
/// `FUN_089156a0` and hashing to the blob's own WAD entry on all 35 PSP
/// systems.
///
/// Backslashes, the way the executable writes them.
#[must_use]
pub fn effect_path(name: &str) -> String {
    effect_path_in(r"Data\Psys", name)
}

/// [`effect_path`] under a title's own effect directory - `Data\Psys` on every
/// title but Wipeout 2048, which keeps them in `Data\Particles2048`.
#[must_use]
pub fn effect_path_in(dir: &str, name: &str) -> String {
    format!(r"{dir}\{name}.POB")
}
