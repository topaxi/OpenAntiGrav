//! Whether a race tells its HUD that an `SP.xml` event is being raced
//! ([`oag_hud::Readout::campaign_event`]): what `load_event` sets on [`Setup`] and a plain
//! load leaves off. The disc-backed walk over both loads is
//! `crates/raceplay/tests/vita_2048_hud_event_ground_truth.rs`.

use super::*;

/// The flag rides `Setup` into `Race::start` and out through `Race::readout`. Dropping any of
/// the three hops (the `Setup` field, the `RaceView` copy, the readout's fill) fails one of
/// the two assertions.
#[test]
fn the_setups_campaign_event_reaches_the_readout() {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    assert!(!Race::start(setup.clone()).readout().campaign_event);

    setup.campaign_event = true;
    assert!(Race::start(setup).readout().campaign_event);
}
