use super::*;

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
    let mut watch = LapWatch::new(None);
    assert_eq!(watch.observe(false, lap(2, 3600, 3600)), ["IG_HUD_PLAP"]);
    watch.observe(true, LapEdge::default());
    assert_eq!(watch.observe(false, lap(3, 3600, 3600)), ["IG_HUD_FIN_LAP"]);
}

#[test]
fn a_scrape_before_the_first_crossing_taints_lap_one() {
    let mut watch = LapWatch::new(None);
    watch.observe(true, LapEdge::default());
    let first = LapEdge {
        completed: false,
        ..lap(1, 0, 0)
    };
    assert!(watch.observe(false, first).is_empty());
    assert!(watch.observe(false, lap(2, 3600, 3600)).is_empty());
}

#[test]
fn the_final_lap_line_comes_last_and_the_finish_says_nothing() {
    let mut watch = LapWatch::new(Some(4000));
    assert_eq!(
        watch.observe(false, lap(3, 3600, 3600)),
        ["IG_HUD_PLAP", "IG_HUD_NEWLAP_REC", "IG_HUD_FIN_LAP"]
    );
    let mut done = lap(4, 3500, 3500);
    done.finished = true;
    assert!(watch.observe(false, done).is_empty());
}

#[test]
fn a_record_needs_a_stored_one_to_beat_and_the_best_lap_of_the_race() {
    let mut none = LapWatch::new(None);
    assert!(
        !none
            .observe(true, lap(2, 100, 100))
            .contains(&"IG_HUD_NEWLAP_REC")
    );
    let mut slower = LapWatch::new(Some(3000));
    assert!(
        !slower
            .observe(true, lap(2, 3600, 3600))
            .contains(&"IG_HUD_NEWLAP_REC")
    );
    let mut worse_than_race_best = LapWatch::new(Some(4000));
    assert!(
        !worse_than_race_best
            .observe(true, lap(3, 3600, 3000))
            .contains(&"IG_HUD_NEWLAP_REC")
    );
}
