use super::*;
use oag_ai::Difficulty;

fn event(level: usize) -> EventAi {
    EventAi {
        skill_scale: 0.5,
        pad_tracking: 0.25,
        weapon_avoidance: 0.75,
        level,
    }
}

#[test]
fn the_default_is_the_original_driver() {
    assert_eq!(AiChoice::default(), AiChoice::Original);
    assert_eq!(
        decide(AiChoice::default(), true, Source::Free(Difficulty::Elite)),
        Decision::Original {
            skill: None,
            level: 1,
            event: None
        }
    );
}

#[test]
fn ours_overrides_every_source() {
    for source in [
        Source::Campaign {
            skill: Some(1.0),
            level: 2,
        },
        Source::Event2048(Some(event(1))),
        Source::Free(Difficulty::Ace),
    ] {
        assert_eq!(
            decide(AiChoice::Ours, true, source),
            Decision::Ours(Reason::Flag)
        );
    }
}

#[test]
fn no_opponents_keeps_ours() {
    assert_eq!(
        decide(AiChoice::Original, false, Source::Free(Difficulty::Elite)),
        Decision::Ours(Reason::NoOpponents)
    );
}

#[test]
fn a_campaign_cell_keeps_its_own_skill_and_rung() {
    assert_eq!(
        decide(
            AiChoice::Original,
            true,
            Source::Campaign {
                skill: Some(0.4),
                level: 2
            }
        ),
        Decision::Original {
            skill: Some(0.4),
            level: 2,
            event: None
        }
    );
}

#[test]
fn a_2048_event_carries_its_two_skills_and_elimination_keeps_ours() {
    assert_eq!(
        decide(AiChoice::Original, true, Source::Event2048(Some(event(2)))),
        Decision::Original {
            skill: Some(0.5),
            level: 2,
            event: Some((0.25, 0.75))
        }
    );
    assert_eq!(
        decide(AiChoice::Original, true, Source::Event2048(None)),
        Decision::Ours(Reason::EventWithoutAi)
    );
}

#[test]
fn the_free_level_runs_easy_to_hard_and_the_default_is_medium() {
    assert_eq!(free_level(Difficulty::Novice), 0);
    assert_eq!(free_level(Difficulty::default()), 1);
    assert_eq!(free_level(Difficulty::Ace), 2);
}
