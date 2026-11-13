//! Each HD team's raised shield settles to the `ShieldColour` its own
//! `shipshield.rcsmodel` authors, and a fired shell lerps there.
//!
//! The expected table is the census on
//! `docs/ghidra/functions/ps3-hdfury-eu/shield.md` ("The steady-state colour is
//! read"), read off the disc by walking each shield model's material for
//! `0xaaf53119`. It lives here and nowhere in the engine.

use oag_render::shield::{HD_PALETTE, Palette, ShipShield};

const TABLE: &[(&str, [f32; 4])] = &[
    ("feisar", [0.0, 0.0314, 0.9961, 1.0]),
    ("assegai", [0.2110, 0.6567, 1.0, 1.0]),
    ("piranha", [1.0, 0.5333, 0.0, 1.0]),
    ("qirex", [0.7294, 0.1843, 0.5490, 1.0]),
    ("goteki", [0.7490, 0.2039, 0.7725, 1.0]),
    ("egx", [0.2196, 1.0, 0.2196, 1.0]),
    ("ag_systems", [0.2652, 1.0, 1.0, 1.0]),
    ("triakis", [0.4078, 0.3804, 0.1137, 1.0]),
    // Two copies on disc: DATA02's is (0.4078, 0.3804, 0.1137, 1). Our archive
    // order mounts DATA00 first, so DATA00's older copy (alpha 0) is what loads;
    // which one the original reads is not settled (shield.md, "Carried to
    // Omega and 2048" and the Zone note).
    ("zone", [1.0, 0.3022, 0.0, 0.0]),
    ("auricom", [1.0, 1.0, 1.0, 1.0]),
    ("harimau", [0.0, 1.0, 1.0, 1.0]),
    ("icaras", [0.0, 0.0314, 0.9961, 1.0]),
    ("mirage", [1.0, 0.1142, 1.0, 1.0]),
];

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_hd_team_shield_settles_to_its_authored_shield_colour() {
    let Some(path) = oag_testdata::exact("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&path.to_string_lossy(), oag_hd::TITLE)
        .expect("the archives open");
    for (team, expected) in TABLE {
        let mut report = Vec::new();
        let liveries = oag_livery::load(
            &mut archives,
            std::slice::from_ref(&(*team).to_string()),
            &oag_livery::LoadContext {
                store: Default::default(),
                race: oag_hd::TITLE.race,
                mode: oag_race::Mode::SingleRace,
                flare: oag_hd::TITLE.flare,
                hull_overlay: false,
                leach_overlay: false,
                hull_shine: false,
                hull_wreck: false,
                absorb_shell: false,
                own_shield: false,
                zone_liveries: &[],
                forced_skins: &[],
            },
            None,
            None,
            &[],
            &mut report,
        )
        .expect("the livery loads");
        let colour = liveries[0]
            .shield_colour
            .unwrap_or_else(|| panic!("{team}: no ShieldColour\n{}", report.join("\n")));
        for (channel, (got, want)) in colour.iter().zip(expected).enumerate() {
            assert!(
                (got - want).abs() < 1.0 / 255.0,
                "{team}: channel {channel} is {got}, the census has {want}"
            );
        }
        let mut shell = ShipShield::with_palette(Palette {
            target: colour,
            ..HD_PALETTE
        });
        shell.activate();
        for _ in 0..240 {
            shell.advance(1.0 / 60.0);
        }
        // Alpha is modulated by the shell's flicker; the colour channels settle.
        for (got, want) in shell.colour().iter().zip(colour).take(3) {
            assert!(
                (got - want).abs() < 1e-3,
                "{team}: settled {got}, not {want}"
            );
        }
    }
}
