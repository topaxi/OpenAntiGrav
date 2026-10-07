//! Omega opened straight from `omega-ps4-eu.pkg` (its patch found beside it)
//! must be the same Omega as the extracted folders: the same archive for every
//! name, and the same bytes out of it. A whole race frame was compared outside
//! this suite (`docs/formats/ps4-package.md`).

use std::path::PathBuf;

const NAMES: [&str; 6] = [
    r"Data\Plugins\Frontend\Gui\Skin.xml",
    r"Data\Plugins\Grids\grid_00.xml",
    r"Data\Plugins\tracks\Definition.xml",
    r"Data\Plugins\teams\Definition.xml",
    r"Data\environments\tech_de_ra\track.vex",
    r"Data\environments\tech_de_ra\track.EnvSettings",
];

fn sources() -> Option<(String, String)> {
    let folder = oag_testdata::exact("data/extracted/ps4")?;
    let pkg: PathBuf = oag_testdata::exact("omega-ps4-eu.pkg")?;
    Some((pkg.display().to_string(), folder.display().to_string()))
}

/// The archive that serves a name is the same one, by file name, in both.
fn tail(spec: &str) -> &str {
    spec.rsplit(['/', '\\']).next().unwrap_or(spec)
}

#[test]
#[ignore = "needs data/images/omega-ps4-eu*.pkg and data/extracted/ps4"]
fn the_pkg_pair_serves_what_the_extracted_folders_serve() {
    let Some((pkg, folder)) = sources() else {
        return;
    };
    let mut from_pkg = oag_omega::open(&pkg).expect("opens from the .pkg");
    let mut from_folder = oag_omega::open(&folder).expect("opens from the folders");
    for name in NAMES {
        let (a, b) = (
            from_pkg
                .locations(name)
                .iter()
                .map(|s| tail(s).to_string())
                .collect::<Vec<_>>(),
            from_folder
                .locations(name)
                .iter()
                .map(|s| tail(s).to_string())
                .collect::<Vec<_>>(),
        );
        assert_eq!(a, b, "{name}: archives holding it, in search order");
        assert!(!a.is_empty(), "{name} is in neither");
        let (x, y) = (from_pkg.read_name(name), from_folder.read_name(name));
        assert_eq!(x.is_ok(), y.is_ok(), "{name}");
        if let (Ok(x), Ok(y)) = (x, y) {
            assert!(x == y, "{name}: bytes differ");
        }
    }
}
