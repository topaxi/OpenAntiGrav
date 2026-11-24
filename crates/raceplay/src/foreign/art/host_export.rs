//! A host circuit's textures written out as PNG files, for a Blender add-on that
//! shows a track's host references with their real pictures
//! (`docs/tools/blender.md`, "Host textures").
//!
//! **Local use only**: the output is content from the user's own disc. It goes
//! under `data/`, is never committed and never packed into a `.blend` or a mod;
//! the add-on links it as an external file.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use oag_assets::Archives;

use super::host_table;

/// Where a host texture's PNG lives under the export folder:
/// `<title slug>/<circuit entry>/<texture name>.png`, lowercase, `\` a
/// directory separator. A texture name is unique within its circuit, and the
/// engine matches names without case, so lowercasing loses nothing a lookup
/// keeps; the add-on spells the same rule (`oag_track/images.py`).
///
/// `None` for names that would leave the folder or name no file: a `.` or `..`
/// part, a part with a `:` (a Windows drive or stream) or a NUL, or an entry or a
/// name with no part at all. A circuit's own asset paths are data of the disc, not trusted.
#[must_use]
pub fn texture_file(slug: &str, entry: &str, name: &str) -> Option<PathBuf> {
    let split = |path: &str| -> Vec<String> {
        path.split(['\\', '/'])
            .filter(|part| !part.is_empty())
            .map(str::to_ascii_lowercase)
            .collect()
    };
    let (entry, name) = (split(entry), split(name));
    if entry.is_empty() || name.is_empty() {
        return None;
    }
    let parts = entry.into_iter().chain(name);
    let parts: Vec<String> = parts.collect();
    if parts
        .iter()
        .any(|part| part == "." || part == ".." || part.contains([':', '\0']))
    {
        return None;
    }
    let mut path = PathBuf::from(slug.to_ascii_lowercase());
    path.extend(parts);
    path.set_extension("png");
    Some(path)
}

/// What an export wrote and what it could not.
#[derive(Debug, Default)]
pub struct Exported {
    pub written: Vec<PathBuf>,
    /// One line per texture that was left out, and why.
    pub skipped: Vec<String>,
}

/// Writes the textures of the circuit `entry` under `out`: the ones `names`
/// spells, or every `Texture` node when `names` is `None`. A name that matches
/// no decodable node is a line in [`Exported::skipped`], never a stand-in.
///
/// # Errors
///
/// The circuit not reading on this disc, or a file not writing.
pub fn export_entry(
    archives: &mut Archives,
    slug: &str,
    entry: &str,
    names: Option<&BTreeSet<String>>,
    out: &Path,
) -> anyhow::Result<Exported> {
    let table = host_table(archives, entry)?;
    let mut done = Exported::default();
    let mut seen = BTreeSet::new();
    let wanted: Vec<(usize, String)> = match names {
        None => table
            .names
            .iter()
            .enumerate()
            .map(|(i, n)| {
                (
                    i,
                    n.clone()
                        .unwrap_or_else(|| format!("unnamed texture node {i}")),
                )
            })
            .collect(),
        Some(names) => names
            .iter()
            .filter_map(|name| match table.find(name) {
                Some((first, _)) => Some((first, name.clone())),
                None => {
                    done.skipped
                        .push(format!("{entry}: no texture node {name}"));
                    None
                }
            })
            .collect(),
    };
    for (at, name) in wanted {
        let Some(relative) = texture_file(slug, entry, &name) else {
            done.skipped.push(format!(
                "{entry}: {name} would leave the export folder; not written"
            ));
            continue;
        };
        let file = out.join(relative);
        if !seen.insert(file.clone()) {
            done.skipped.push(format!(
                "{entry}: {name} names a second node; the first is written"
            ));
            continue;
        }
        let Some(texture) = table.slots.get(at).cloned().flatten() else {
            done.skipped
                .push(format!("{entry}: {name} is not decodable"));
            continue;
        };
        let Some(rgba) = texture.to_rgba() else {
            done.skipped
                .push(format!("{entry}: {name} has no RGBA form"));
            continue;
        };
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        write_png(&file, texture.width, texture.height, &rgba)?;
        done.written.push(file);
    }
    Ok(done)
}

/// `rgba` (top row first) as a compressed PNG at `file`. A stored-block PNG
/// (`oag_texture::png`) would be the decoded size: a PS4 circuit's pictures
/// come to gigabytes.
fn write_png(file: &Path, width: u32, height: u32, rgba: &[u8]) -> anyhow::Result<()> {
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(std::fs::File::create(file)?),
        width,
        height,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fastest);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(rgba)?;
    writer.finish()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::texture_file;

    #[test]
    fn a_texture_lands_under_its_title_and_circuit_in_lowercase() {
        assert_eq!(
            texture_file(
                "wipeout-pulse",
                "Data\\Environments\\03_Track\\track.vex",
                "Data\\Environments\\GenericTrackTextures\\_Wall6_1.tga",
            )
            .as_deref()
            .and_then(|p| p.to_str()),
            Some(
                "wipeout-pulse/data/environments/03_track/track.vex/\
                 data/environments/generictracktextures/_wall6_1.png"
            ),
        );
    }

    /// The same rows as `HOST_TITLE_PATHS` in `scripts/blender/roundtrip.py`.
    #[test]
    fn each_titles_texture_names_land_where_the_add_on_looks() {
        for (slug, entry, name, path) in [
            (
                "wipeout-hd",
                "Data\\Environments\\03_Track\\track.vex",
                "/data/environments/03_track/hd_textures/dds/ds_speedup_cs.gtf",
                "wipeout-hd/data/environments/03_track/track.vex/\
                 data/environments/03_track/hd_textures/dds/ds_speedup_cs.png",
            ),
            (
                "wipeout-2048",
                "Data\\art\\published\\environments\\altima\\track.vex",
                "data/art/published/environments/altima/textures/jreilly_crane_da.gxt",
                "wipeout-2048/data/art/published/environments/altima/track.vex/\
                 data/art/published/environments/altima/textures/jreilly_crane_da.png",
            ),
            (
                "wipeout-omega-collection",
                "Data\\environments\\tech_de_ra\\track.vex",
                "data/environments/tech_de_ra/texturesps4/hull.gnf",
                "wipeout-omega-collection/data/environments/tech_de_ra/track.vex/\
                 data/environments/tech_de_ra/texturesps4/hull.png",
            ),
            (
                "wipeout-pure",
                "Data\\Environments\\01_Vineta_K\\track.vex",
                "Data\\Environments\\01_Vineta_K\\Textures\\ag03.tga",
                "wipeout-pure/data/environments/01_vineta_k/track.vex/\
                 data/environments/01_vineta_k/textures/ag03.png",
            ),
        ] {
            assert_eq!(
                texture_file(slug, entry, name)
                    .as_deref()
                    .and_then(|p| p.to_str()),
                Some(path),
                "{slug}"
            );
        }
    }

    #[test]
    fn a_name_with_no_directory_and_no_extension_gets_a_png_extension() {
        assert_eq!(
            texture_file("wipeout-hd", "a.vex", "Sky")
                .as_deref()
                .and_then(|p| p.to_str()),
            Some("wipeout-hd/a.vex/sky.png")
        );
    }

    #[test]
    fn a_name_that_leaves_the_folder_or_names_no_file_has_no_path() {
        let e = "Data\\Environments\\03_Track\\track.vex";
        for name in [
            "..\\..\\x.tga",
            "Data\\..\\..\\tmp\\out.tga",
            "a/../../b.tga",
            "c:\\windows\\x.tga",
            "stream:x.tga",
            "a\0b.tga",
            ".",
            "\\",
            "",
        ] {
            assert_eq!(texture_file("wipeout-pulse", e, name), None, "{name:?}");
        }
        assert_eq!(texture_file("wipeout-pulse", "..", "x.tga"), None);
        assert_eq!(texture_file("wipeout-pulse", "/", "/"), None);
        assert!(texture_file("wipeout-pulse", e, "a.b\\c.tga").is_some());
    }
}
