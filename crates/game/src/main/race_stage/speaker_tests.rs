//! The end-of-race voice as the session drives it, without a device: the
//! screen order [`Flow`] walks and the [`Speaker`] reading it each tick, the
//! two halves `EndRaceRuntime::voice_tick` joins.

use oag_sound::sfx::Cue;
use oag_tables::race_campaign::Medal;
use oag_ui_screens::endrace::{FieldResults, Headline, Results};

use super::ResultsModel;
use super::endrace_flow::{Flow, Which};
use super::endrace_voice::{Speaker, voice_for};

fn pulse() -> Option<oag_title::ResultsLaw> {
    oag_pulse::race::SOUND_BANKS
        .frontend
        .and_then(|front| front.results)
        .map(|results| results.law)
}

fn hd() -> Option<oag_title::ResultsLaw> {
    oag_hd::race::SOUND_BANKS
        .frontend
        .and_then(|front| front.results)
        .map(|results| results.law)
}

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
    let mut speaker = Speaker::new(voice_for(
        pulse(),
        oag_race::Mode::SingleRace,
        &results,
        Some(3),
        false,
        None,
    ));
    let mut flow = Flow::new(true);
    let spoken = walk(&mut speaker, &mut flow, &results, false, 600, &[200, 300]);
    assert_eq!(spoken, [(201, Which::Results, Cue::ResultsThird)]);
    assert_eq!(flow.which(), Which::Menu);
}

#[test]
fn a_campaign_cell_speaks_the_place_then_the_medal() {
    let results = single_race_third();
    let mut speaker = Speaker::new(voice_for(
        pulse(),
        oag_race::Mode::SingleRace,
        &results,
        Some(3),
        true,
        Some(Medal::Bronze),
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

fn hd_results() -> ResultsModel {
    ResultsModel::Hd(FieldResults {
        headline: Headline::Position(2),
        rows: Vec::new(),
        loyalty: None,
    })
}

#[test]
fn hd_speaks_the_place_on_its_first_screen_and_the_medal_instead_in_a_cell() {
    let results = hd_results();
    let free = voice_for(hd(), oag_race::Mode::SingleRace, &results, Some(2), false, None);
    let mut flow = Flow::new(false);
    let mut speaker = Speaker::new(free);
    let spoken = walk(&mut speaker, &mut flow, &results, false, 400, &[100]);
    assert_eq!(spoken, [(0, Which::Results, Cue::ResultsSecond)]);

    let cell = voice_for(
        hd(),
        oag_race::Mode::SingleRace,
        &results,
        Some(2),
        true,
        Some(Medal::Silver),
    );
    let mut flow = Flow::new(false);
    let mut speaker = Speaker::new(cell);
    let spoken = walk(&mut speaker, &mut flow, &results, false, 400, &[100]);
    assert_eq!(spoken, [(0, Which::Results, Cue::MedalSilver)]);
}
