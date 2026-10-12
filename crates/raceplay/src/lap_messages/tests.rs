use super::*;

fn pulse(stored: Option<u32>) -> LapWatch {
    LapWatch::new(oag_pulse::hud::ART.lap_hud.expect("read"), stored)
}

fn lap(lap: u32, ticks: u32, best: u32) -> LapEdge {
    LapEdge {
        completed: true,
        finished: false,
        lap,
        laps_target: Some(3),
        lap_ticks: Some(ticks),
        best_lap_ticks: Some(best),
    }
}

#[test]
fn a_clean_lap_is_perfect_and_a_scrape_spoils_it() {
    let mut watch = pulse(None);
    assert_eq!(
        watch.observe(false, false, lap(2, 3600, 3600)),
        ["IG_HUD_PLAP"]
    );
    watch.observe(true, false, LapEdge::default());
    assert_eq!(
        watch.observe(false, false, lap(3, 3600, 3600)),
        ["IG_HUD_FIN_LAP"]
    );
}

#[test]
fn a_scrape_before_the_first_crossing_taints_lap_one() {
    let mut watch = pulse(None);
    watch.observe(true, false, LapEdge::default());
    let first = LapEdge {
        completed: false,
        ..lap(1, 0, 0)
    };
    assert!(watch.observe(false, false, first).is_empty());
    assert!(watch.observe(false, false, lap(2, 3600, 3600)).is_empty());
}

#[test]
fn the_final_lap_line_comes_last_and_the_finish_says_nothing() {
    let mut watch = pulse(Some(4000));
    assert_eq!(
        watch.observe(false, false, lap(3, 3600, 3600)),
        ["IG_HUD_PLAP", "IG_HUD_NEWLAP_REC", "IG_HUD_FIN_LAP"]
    );
    let mut done = lap(4, 3500, 3500);
    done.finished = true;
    assert!(watch.observe(false, false, done).is_empty());
}

#[test]
fn a_record_needs_a_stored_one_to_beat_and_the_best_lap_of_the_race() {
    let mut none = pulse(None);
    assert!(
        !none
            .observe(true, false, lap(2, 100, 100))
            .contains(&"IG_HUD_NEWLAP_REC")
    );
    let mut slower = pulse(Some(3000));
    assert!(
        !slower
            .observe(true, false, lap(2, 3600, 3600))
            .contains(&"IG_HUD_NEWLAP_REC")
    );
    let mut worse_than_race_best = pulse(Some(4000));
    assert!(
        !worse_than_race_best
            .observe(true, false, lap(3, 3600, 3000))
            .contains(&"IG_HUD_NEWLAP_REC")
    );
}

/// HD raises no new-lap-record line in Time Trial (its `0x91d20` skips it in
/// mode 5 while the profile byte is clear); Pulse and Pure do.
#[test]
fn only_hd_skips_the_record_line_in_time_trial() {
    let beaten = lap(2, 3000, 3000);
    for (art, expected) in [
        (oag_hd::hud::ART, false),
        (oag_pulse::hud::ART, true),
        (oag_pure::hud::ART, true),
    ] {
        let hud = art.lap_hud.expect("read");
        let mut watch = LapWatch::new(hud, Some(4000));
        let in_time_trial = watch.observe(true, true, beaten);
        assert_eq!(in_time_trial.contains(&hud.new_lap_record), expected);
        let mut watch = LapWatch::new(hud, Some(4000));
        let elsewhere = watch.observe(true, false, beaten);
        assert!(elsewhere.contains(&hud.new_lap_record), "any other mode");
    }
}

/// Each title raises its own phrase ids, not Pulse's.
#[test]
fn pure_raises_its_own_keys() {
    let hud = oag_pure::hud::ART.lap_hud.expect("read");
    let mut watch = LapWatch::new(hud, Some(4000));
    assert_eq!(
        watch.observe(false, false, lap(3, 3600, 3600)),
        ["HUD_Perfect Lap!", "HUD_New Lap Record", "HUD_Final Lap!"]
    );
}
