//! Every spike shape an HD or Fury `engineflare.vex` authors is split out and
//! placed, and only the ones the original's name lookup binds flicker.
//!
//! The census over the 42 hull directories HD ships found one deviation from
//! `ef_Spikes<N>Shape`: Feisar Fury's `_c1` and `_n1` spell the three main
//! spikes `ef_SpikesShape<N>`. The executable binds only the first spelling, so
//! the Fury shapes are submitted at their node's own matrix (the hull's scale
//! alone, read live: three draws at 1.000 times 0.75) while a lookup keyed on
//! the first spelling left them inside `EF_Main`, scaled with the body at about
//! 1.55. The expected count here is read off each file's own node tree, so a
//! third spelling fails by name.

const TEAMS: &[&str] = &[
    "ag_systems",
    "assegai",
    "auricom",
    "egx",
    "feisar",
    "goteki",
    "harimau",
    "icaras",
    "mirage",
    "piranha",
    "qirex",
    "triakis",
    "detonator",
    "zone",
];

/// Whether a flare shape name is a spike: any `ef_Spikes...` shape, in either
/// spelling, and neither a locator nor a group.
fn is_spike_shape(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.starts_with("ef_spikes") && n.contains("shape")
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_hd_and_fury_flare_draws_every_spike_it_authors() {
    let Some(path) = oag_testdata::exact("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_hd::TITLE)
        .expect("the archives open");
    let mut checked = 0;
    let mut fury = 0;
    for team in TEAMS {
        for dir in [team.to_string(), format!("{team}_c1"), format!("{team}_n1")] {
            let Ok(blob) = archives.read_name(&format!("Data/Ships/{dir}/engineflare.vex")) else {
                continue;
            };
            let authored = oag_vex::vex::nodes(&blob)
                .expect("the flare parses")
                .iter()
                .filter(|n| n.name.as_deref().is_some_and(is_spike_shape))
                .count();
            let mut report = Vec::new();
            let liveries = oag_livery::load(
                &mut archives,
                std::slice::from_ref(&dir),
                &oag_livery::LoadContext {
                    race: oag_hd::TITLE.race,
                    mode: oag_race::Mode::SingleRace,
                    flare: oag_hd::TITLE.flare,
                    hull_overlay: false,
                    hull_shine: false,
                    hull_wreck: false,
                    absorb_shell: false,
                    zone_liveries: &[],
                    forced_skins: &[],
                },
                None,
                None,
                &[],
                &mut report,
            )
            .expect("the livery loads");
            let boost_authored = oag_vex::vex::nodes(&blob)
                .expect("the flare parses")
                .iter()
                .filter(|n| {
                    n.name.as_deref().is_some_and(|n| {
                        let n = n.to_ascii_lowercase();
                        n.starts_with("ef_boost") && n.ends_with("shape") || n == "ef_diamondsshape"
                    })
                })
                .count();
            let parts = liveries.first().map_or(&[][..], |l| &l.flare_spikes[..]);
            let boost_drawn = parts
                .iter()
                .filter(|s| s.scale == oag_livery::Scale::BoostLength)
                .count();
            assert_eq!(
                boost_drawn,
                boost_authored,
                "{dir}: boost joint shapes authored and drawn\n{}",
                report.join("\n")
            );
            let drawn = parts.len() - boost_drawn;
            assert!(authored > 0, "{dir}: authors no spike shape at all");
            assert_eq!(
                drawn,
                authored,
                "{dir}: {authored} spike shapes authored, {drawn} drawn\n{}",
                report.join("\n")
            );
            let unbound = liveries.first().map_or(0, |l| {
                l.flare_spikes
                    .iter()
                    .filter(|s| s.scale == oag_livery::Scale::Hull)
                    .count()
            });
            assert_eq!(
                unbound,
                if dir.starts_with("feisar_") { 3 } else { 0 },
                "{dir}: spikes the executable's lookup does not bind"
            );
            if dir == "feisar_c1" {
                // Read live on RPCS3 (Metropia, 270 km/h): the three boost draws'
                // model matrices, taken into the hull's space, sit here.
                let live = [
                    (0.804, 0.0732, -4.6293),
                    (-0.8041, 0.0732, -4.6293),
                    (0.0, 0.1688, -5.8044),
                ];
                for (x, y, z) in live {
                    assert!(
                        parts
                            .iter()
                            .any(|p| p.scale == oag_livery::Scale::BoostLength
                                && (p.origin - oag_core::math::Vec3::new(x, y, z)).length()
                                    < 2.0e-3),
                        "no boost part at the live-read joint ({x}, {y}, {z})"
                    );
                }
            }
            checked += 1;
            fury += usize::from(dir.ends_with("_c1") || dir.ends_with("_n1"));
        }
    }
    assert!(checked >= 38, "only {checked} hulls were read");
    assert!(fury >= 24, "only {fury} Fury hulls were read");
}

/// Every spelling of a spike shape name across every copy of the named flare
/// file in `dirs`, as sorted `(name, copies)` rows.
fn spellings(
    archives: &mut oag_assets::Archives,
    path: &dyn Fn(&str) -> String,
    dirs: &[String],
) -> (usize, std::collections::BTreeMap<String, usize>) {
    let mut files = 0;
    let mut names = std::collections::BTreeMap::new();
    for dir in dirs {
        for (_, blob) in archives.read_every_name(&path(dir)) {
            let Ok(nodes) = oag_vex::vex::nodes(&blob) else {
                continue;
            };
            files += 1;
            for name in nodes.iter().filter_map(|n| n.name.as_deref()) {
                if is_spike_shape(name) {
                    *names.entry(name.to_string()).or_default() += 1;
                }
            }
        }
    }
    (files, names)
}

fn dirs() -> Vec<String> {
    TEAMS
        .iter()
        .flat_map(|t| [t.to_string(), format!("{t}_c1"), format!("{t}_n1")])
        .collect()
}

#[test]
#[ignore = "needs the Omega PS4 packages extracted under data/extracted/ps4"]
fn omega_flares_spell_their_spikes_as_hd_does() {
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&source.display().to_string()).expect("open omega");
    let (files, names) = spellings(
        &mut archives,
        &|d| format!("Data/art/published/hdships/{d}/engineflare.vex"),
        &dirs(),
    );
    println!("omega: {files} files, spellings {names:?}");
    assert!(files > 0, "no Omega flare read");
    println!("OMEGA_SPELLINGS {names:?}");
}

#[test]
#[ignore = "needs the decrypted 2048 package in data/extracted/vita/"]
fn the_2048_flares_spell_their_spikes_as_hd_does() {
    let Some(source) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let mut archives = oag_2048::open(&source.display().to_string()).expect("mounting 2048");
    let (files, names) = spellings(
        &mut archives,
        &|d| format!("Data/art/published/Ships/{d}/engineflare.vex"),
        &dirs(),
    );
    println!("2048: {files} files, spellings {names:?}");
    println!("2048_SPELLINGS {names:?}");
}
