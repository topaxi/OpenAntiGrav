//! The `graphics.texture_compression` choice: turning the ASTC re-encode on or off,
//! and whether the machine offers the row at all.
//!
//! The row is hardware's, not a platform's: it is shown on a device that cannot
//! sample BC and can sample ASTC (`oag_mesh::mesh_render::astc::takes_astc`), on
//! any operating system, and hidden on every other device, where the setting is
//! inert (`docs/rendering/texture-compression.md`, "The setting").

use oag_display::display::TextureCompression;

/// The menu key of the row.
pub const SETTING: &str = "graphics.texture_compression";

/// Puts `compression` into effect for the next race load: `astc` opens the cache
/// under the user's cache directory, `original` closes it.
pub fn apply(compression: TextureCompression) {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    oag_mesh::mesh_render::astc::configure(match compression {
        TextureCompression::Original => None,
        TextureCompression::Astc => Some(oag_source::cache::default_texture_cache_dir()),
    });
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    let _ = compression;
}

/// One loader-report line for the race just built: how many of its textures came up as ASTC
/// from the cache and how many went up as RGBA8 and were queued. Silent when neither happened
/// (the setting is `original`, or the device keeps the disc's blocks).
pub fn report_load() {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        let tally = oag_mesh::mesh_render::astc::take_tally();
        if tally.hits + tally.misses > 0 {
            log::debug!(
                "astc: {} texture(s) uploaded as ASTC 4x4 from the cache ({:.1} MiB), {} as RGBA8 \
                 and queued for encoding; {} encoded this run",
                tally.hits,
                tally.hit_bytes as f64 / (1024.0 * 1024.0),
                tally.misses,
                tally.encoded,
            );
        }
    }
}

/// Whether the row is kept, from what the device said: `offered` is
/// `oag_mesh::mesh_render::astc::offered`, `None` before any device was asked for.
/// Kept while unknown, so a menu built before the device (a boot's) is trimmed once
/// the device is up ([`trim_row`]).
#[must_use]
pub fn row_kept(offered: Option<bool>) -> bool {
    offered != Some(false)
}

/// What this process's device said, `Some(false)` on a build with no encoder.
#[must_use]
pub fn offered() -> Option<bool> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    return oag_mesh::mesh_render::astc::offered();
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    Some(false)
}

/// Drops the row from `definition` unless [`row_kept`] keeps it.
pub fn trim_row(definition: &mut oag_ui::menu::Definition, offered: Option<bool>) {
    if !row_kept(offered) {
        definition.drop_settings(&[SETTING]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_row_is_dropped_only_once_the_device_said_no() {
        assert!(row_kept(None), "kept while no device was asked for");
        assert!(row_kept(Some(true)), "kept on a device that takes ASTC");
        assert!(
            !row_kept(Some(false)),
            "dropped on a device with BC or without ASTC"
        );
    }

    fn has_row(definition: &oag_ui::menu::Definition) -> bool {
        definition
            .pages
            .iter()
            .flat_map(|page| page.entries.iter())
            .any(|entry| entry.setting() == Some(SETTING))
    }

    #[test]
    fn the_built_in_menu_has_the_row_until_the_device_says_no() {
        let built = || {
            oag_ui::menu::Definition::built_in(&oag_ui::language::StringTable::default())
                .expect("the built-in menu must parse")
        };
        assert!(has_row(&built()), "the graphics page lists the row");
        for (offered, kept) in [(None, true), (Some(true), true), (Some(false), false)] {
            let mut definition = built();
            trim_row(&mut definition, offered);
            assert_eq!(has_row(&definition), kept, "offered {offered:?}");
        }
    }
}
