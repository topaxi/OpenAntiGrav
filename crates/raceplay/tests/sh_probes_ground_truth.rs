//! Omega's hull ambient reaches the draw: an Omega circuit loads its `track.shprobes` and the two
//! weights beside it, a craft's blend is the one the original's live block showed, and a Zone
//! circuit (which ships no probe file) keeps the constant ambient.
//!
//! **`#[ignore]`d and never run in CI.** It needs the corrected Omega extraction
//! (`data/extracted/ps4`, or `$OAG_OMEGA_SOURCE`). The census over every shipped file rides
//! `omega_lightmap_ground_truth`'s pass; the HD and 2048 controls (no probes bound) are in
//! `hd_env_probes_ground_truth`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^sh_probes_ground_truth::/)'
//! ```

use oag_raceplay as race;

fn source() -> Option<String> {
    let path = match std::env::var_os("OAG_OMEGA_SOURCE") {
        Some(path) => oag_testdata::repo_root().join(path),
        None => oag_testdata::exact("data/extracted/ps4")?,
    };
    Some(path.display().to_string())
}

fn load(track: &str) -> Option<race::Loaded> {
    let source = source()?;
    Some(
        race::load(&race::Options {
            source,
            track: Some(track.to_string()),
            team: Some("FEISAR_C1".to_string()),
            opponents: true,
            ..race::Options::default()
        })
        .unwrap_or_else(|e| panic!("loading {track}: {e:#}")),
    )
}

/// Modesto Heights loads its 9,216 records and `K = Ks = 0.25`; the load says so.
#[test]
#[ignore = "needs the corrected Omega extraction"]
fn an_omega_circuit_loads_its_probes_and_weights() {
    let Some(loaded) = load(r"Data\Environments\modesto_heights\track.vex") else {
        return;
    };
    let sh = loaded.probes.sh.as_ref().expect("Modesto ships shprobes");
    assert_eq!(sh.probes.len(), 9_216);
    assert_eq!((sh.k, sh.ks), (0.25, 0.25));
    assert!(
        loaded
            .report
            .iter()
            .any(|l| l.contains("9216 spherical-harmonic probes, weights K 0.25 / Ks 0.25")),
        "{:#?}",
        loaded.report
    );
    assert!(loaded.probes.hd.is_none(), "no HD probes beside it");
}

/// The Zone circuits ship no `.shprobes` (and no `(shadow)` weight): their hulls keep the
/// constant ambient and the load binds nothing.
#[test]
#[ignore = "needs the corrected Omega extraction"]
fn a_zone_circuit_binds_no_probes() {
    let Some(loaded) = load(r"Data\Environments\zone_1\track.vex") else {
        return;
    };
    assert!(loaded.probes.sh.is_none());
}

/// **The blend is the original's** (`hull-tone.md`, "Read 1, live"): on Modesto the live block
/// of the player equalled `record 1388 + 0.076 * (record 1391 - record 1388)` to five decimals.
/// A craft on that lane at that fraction along it locates the same two records and weight, and
/// its row is the formula within `1e-4`.
///
/// **Ours' own grid slot 8 does not land there**: the player spawns at
/// `(-290.6, 194.8, -291.8)`, 3 units further along than the original's `(~-283.9, ~-287.6)`
/// (its blend sits at weight 0.85 between records 1394 and 1397), but on the same lane (records
/// `= 2 mod 3`), which is what the nearest-lane rule needs.
#[test]
#[ignore = "needs the corrected Omega extraction"]
fn a_craft_blends_the_two_records_of_its_lane_along_the_track() {
    let Some(loaded) = load(r"Data\Environments\modesto_heights\track.vex") else {
        return;
    };
    let sh = loaded.probes.sh.as_ref().expect("Modesto ships shprobes");
    let probes = &sh.probes;
    let (a, b) = (probes.position(1388), probes.position(1391));
    let at = [
        a[0] + 0.076 * (b[0] - a[0]),
        a[1] + 0.076 * (b[1] - a[1]),
        a[2] + 0.076 * (b[2] - a[2]),
    ];
    let taps = probes.locate(at);
    assert_eq!((taps.from, taps.to), (1388, 1391));
    assert!((taps.weight - 0.076).abs() < 1e-4, "{taps:?}");
    let blend = probes.blend(taps);
    let (c, d) = (probes.coefficients(1388), probes.coefficients(1391));
    for i in 0..27 {
        let expected = c[i] + 0.076 * (d[i] - c[i]);
        assert!((blend[i] - expected).abs() < 1e-4, "coefficient {i}");
    }
    // The row the hull reads: the blend, then `(K, Ks, 1, 0)`.
    let row = sh.row(at);
    for k in 0..9 {
        assert_eq!(row[4 * k..4 * k + 3], blend[3 * k..3 * k + 3], "texel {k}");
    }
    assert_eq!(row[36..], [0.25, 0.25, 1.0, 0.0]);

    // Ours' player, started and settled on the grid, takes the same lane.
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..100 {
        race.tick(&oag_gameplay::PlayerInputs::single(Default::default()));
    }
    let taps = probes.locate(race.ship_probe_position(0));
    assert_eq!(taps.from % 3, 2, "{taps:?}");
    assert_eq!(taps.to % 3, 2, "{taps:?}");
}
