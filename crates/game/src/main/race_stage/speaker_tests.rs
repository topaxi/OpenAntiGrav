//! The end-of-race voice as the session drives it, without a device: the
//! screen order [`Flow`] walks and the [`Speaker`] reading it each tick, the
//! two halves `EndRaceRuntime::voice_tick` joins.

use oag_sound::sfx::Cue;
use oag_tables::race_campaign::Medal;
use oag_ui_screens::endrace::{Headline, Results, Rewards};

use super::ResultsModel;
use super::endrace_flow::{Flow, Which};
use super::endrace_voice::{Speaker, voice_for};

fn single_race_third() -> ResultsModel {
    ResultsModel::Pulse(Results {
        headline: Headline::Position(3),
        laps: Vec::new(),
        total_ticks: 0,
    })
}

/// What `Session::tick_endrace` does each tick for the voice: tick the flow,
/// ask the speaker, and on a confirm that the screen takes advance it.
fn walk(
    speaker: &mut Speaker,
    flow: &mut Flow,
    results: &ResultsModel,
    campaign: bool,
    ticks: u32,
    confirm_at: &[u32],
) -> Vec<(u32, Which, Cue)> {
    let mut spoken = Vec::new();
    for tick in 0..ticks {
        flow.tick();
        if let Some(cue) = speaker.tick(flow.which(), results) {
            spoken.push((tick, flow.which(), cue));
        }
        if confirm_at.contains(&tick) && flow.takes_confirm() {
            flow.advance(campaign);
        }
    }
    spoken
}

#[test]
fn a_single_race_speaks_the_place_once_on_results_and_nothing_on_the_photo_or_menu() {
    let results = single_race_third();
    let mut speaker = Speaker::new(voice_for(oag_race::Mode::SingleRace, &results, Some(3), None));
    let mut flow = Flow::new(true);
    let spoken = walk(&mut speaker, &mut flow, &results, false, 600, &[200, 300]);
    assert_eq!(spoken, [(201, Which::Results, Cue::ResultsThird)]);
    assert_eq!(flow.which(), Which::Menu);
}

#[test]
fn a_campaign_cell_speaks_the_place_then_the_medal() {
    let results = single_race_third();
    let rewards = Rewards {
        medal: Some(Medal::Bronze),
        campaign: true,
        loyalty: None,
    };
    let mut speaker = Speaker::new(voice_for(
        oag_race::Mode::SingleRace,
        &results,
        Some(3),
        Some(&rewards),
    ));
    let mut flow = Flow::new(true);
    let spoken = walk(&mut speaker, &mut flow, &results, true, 800, &[200, 300, 400]);
    assert_eq!(
        spoken,
        [
            (201, Which::Results, Cue::ResultsThird),
            (301, Which::Rewards, Cue::MedalBronze),
        ]
    );
}

#[test]
fn a_title_with_no_voice_speaks_nothing_on_any_screen() {
    let results = single_race_third();
    let mut speaker = Speaker::new(None);
    let mut flow = Flow::new(false);
    assert!(walk(&mut speaker, &mut flow, &results, true, 500, &[10, 100, 200]).is_empty());
}
