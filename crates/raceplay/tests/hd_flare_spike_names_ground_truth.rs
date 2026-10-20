//! Every spike shape an HD or Fury `engineflare.vex` authors is drawn.
//!
//! The census over the 42 hull directories HD ships found one deviation from
//! `ef_Spikes<N>Shape`: Feisar Fury's `_c1` and `_n1` spell the three main
//! spikes `ef_SpikesShape<N>`. A lookup keyed on the first spelling drew no
//! spike at all for those two craft, silently. The expected count here is read
//! off each file's own node tree, so a third spelling fails by name.

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
                &[dir.clone()],
                &oag_livery::LoadContext {
                    race: oag_hd::TITLE.race,
                    mode: oag_race::Mode::SingleRace,
                    flare: oag_hd::TITLE.flare,
                    hull_overlay: false,
                    hull_shine: false,
                    hull_wreck: false,
                    absorb_shell: false,
                    zone_liveries: &[],
                },
                None,
                None,
                &[],
                &mut report,
            )
            .expect("the livery loads");
            let drawn = liveries.first().map_or(0, |l| l.flare_spikes.len());
            assert!(authored > 0, "{dir}: authors no spike shape at all");
            assert_eq!(
                drawn,
                authored,
                "{dir}: {authored} spike shapes authored, {drawn} drawn\n{}",
                report.join("\n")
            );
            checked += 1;
            fury += usize::from(dir.ends_with("_c1") || dir.ends_with("_n1"));
        }
    }
    assert!(checked >= 38, "only {checked} hulls were read");
    assert!(fury >= 24, "only {fury} Fury hulls were read");
}
