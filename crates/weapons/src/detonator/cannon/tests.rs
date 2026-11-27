//! Invented layouts, a straight lap and a flat world: the law, not the disc.

use super::*;
use crate::detonator::{Entry, Placed};
use crate::test_craft::Ship;
use oag_core::math::Quat;

const DT: f32 = 1.0 / 60.0;

fn lap() -> Survey {
    Survey {
        entries: (0..60)
            .map(|i| Entry {
                usable: true,
                lanes: 7,
                blocked: 0,
                centre: Vec3::new(0.0, 0.0, i as f32 * 10.0),
                lateral: Vec3::X,
                up: Vec3::Y,
            })
            .collect(),
        ..Survey::default()
    }
}

fn config() -> Config {
    Config {
        rounds: 15,
        interval: 0.1,
        rebounds: 1,
        shots_to_destroy: 4,
        mine_radius: 3.0,
        bomb_radius: 6.0,
        lockout_manual: 0.75,
        lockout_empty: 0.75,
        lane_width: 3.5,
        scoring: Scoring {
            multiplier: 1.0,
            new_mine_score: 100,
            old_mine_score: 50,
            bomb_hit_score: 0,
            bomb_score: 2000,
            bomb_increment: 500,
            ..Scoring::default()
        },
        muzzles: [None, None],
    }
}

/// A craft at the origin flying down +Z, the way the lap runs.
fn craft() -> Ship {
    let mut ship = Ship::default();
    ship.physics.body.orientation = Quat::from_xyzw(0.0, 1.0, 0.0, 0.0);
    // A mine stands `MINE_LIFT` above its lane point: the craft flies at that height.
    ship.physics.body.position.y = crate::detonator::MINE_LIFT;
    ship
}

fn open_air(_: Vec3, _: Vec3) -> Sweep {
    Sweep::None
}

const FIRE: Input = Input {
    fire: true,
    reload: false,
};
const IDLE: Input = Input {
    fire: false,
    reload: false,
};

fn mine(entry: u16, live: bool) -> Placed {
    Placed {
        entry,
        lane: 0,
        live,
    }
}

fn field_with(mines: &[Placed], bombs: &[Placed]) -> Field {
    let mut field = Field::new();
    for (i, m) in mines.iter().enumerate() {
        field.mines[i] = *m;
    }
    field.live_mines = mines.len() as u16;
    for (i, b) in bombs.iter().enumerate() {
        field.bombs[i] = *b;
    }
    field.live_bombs = bombs.len() as u8;
    field
}

fn tick(field: &mut Field, input: Input, ship: &Ship, sweep: &impl Fn(Vec3, Vec3) -> Sweep) {
    step_cannon(field, &lap(), &config(), input, ship, DT, sweep);
}

fn tick_events(
    field: &mut Field,
    input: Input,
    ship: &Ship,
    sweep: &impl Fn(Vec3, Vec3) -> Sweep,
) -> Vec<Event> {
    step_cannon(field, &lap(), &config(), input, ship, DT, sweep)
}

/// One volley, then the rounds left to fly.
fn volley_and_fly(field: &mut Field, ship: &Ship, ticks: usize) {
    tick(field, FIRE, ship, &open_air);
    for _ in 0..ticks {
        tick(field, IDLE, ship, &open_air);
    }
}

#[test]
fn a_held_trigger_fires_a_volley_of_two_on_the_first_tick() {
    let (mut field, ship) = (Field::new(), craft());
    tick(&mut field, FIRE, &ship, &open_air);
    assert!(field.cannon.armed);
    assert_eq!(field.cannon.clip, 14);
    assert_eq!(field.cannon.rounds_in_flight(), 2);
    assert_eq!(
        field.cannon.hits[..2],
        [1, 1],
        "each round counts its own slot"
    );
}

#[test]
fn a_held_trigger_fires_a_volley_every_tenth_of_a_second() {
    let (mut field, ship) = (Field::new(), craft());
    for _ in 0..60 {
        tick(&mut field, FIRE, &ship, &open_air);
    }
    let fired = 15 - usize::from(field.cannon.clip);
    assert!((10..=11).contains(&fired), "{fired} volleys in a second");
    assert!(field.cannon.rounds_in_flight() <= 22);
}

#[test]
fn a_released_trigger_fires_nothing_and_restarts_the_countdown() {
    let (mut field, ship) = (Field::new(), craft());
    for _ in 0..3 {
        tick(&mut field, FIRE, &ship, &open_air);
    }
    tick(&mut field, IDLE, &ship, &open_air);
    assert_eq!(field.cannon.trigger, 0.0);
    let clip = field.cannon.clip;
    tick(&mut field, IDLE, &ship, &open_air);
    assert_eq!(field.cannon.clip, clip);
    tick(&mut field, FIRE, &ship, &open_air);
    assert_eq!(
        field.cannon.clip,
        clip - 1,
        "the first held tick fires again"
    );
}

#[test]
fn a_round_flies_at_five_hundred_and_dies_after_a_second() {
    let (mut field, ship) = (Field::new(), craft());
    volley_and_fly(&mut field, &ship, 29);
    let round = field
        .cannon
        .rounds
        .iter()
        .find(|r| r.live)
        .expect("in flight");
    assert!((round.vel.length() - 500.0).abs() < 1e-2);
    assert!(round.pos.z > 200.0, "{:?}", round.pos);
    for _ in 0..40 {
        tick(&mut field, IDLE, &ship, &open_air);
    }
    assert_eq!(field.cannon.rounds_in_flight(), 0);
}

#[test]
fn a_shot_mine_is_destroyed_scored_and_counted() {
    let (mut field, ship) = (field_with(&[mine(15, true)], &[]), craft());
    volley_and_fly(&mut field, &ship, 30);
    assert_eq!(field.live_mines, 0);
    assert_eq!(field.cannon.destroyed_mines, 1);
    assert_eq!(field.cannon.score, 100);
}

#[test]
fn a_dark_mine_pays_the_flat_score_whatever_the_multiplier() {
    let (mut field, ship) = (field_with(&[mine(15, false)], &[]), craft());
    field.cannon.m_mine = 7.0;
    volley_and_fly(&mut field, &ship, 30);
    assert_eq!(field.cannon.destroyed_mines, 1);
    assert_eq!(field.cannon.score, 50);
    assert_eq!(field.cannon.m_mine, 7.0, "a dark mine does not raise it");
}

#[test]
fn live_mines_in_one_clip_score_a_hundred_two_hundred_three_hundred() {
    let mines = [mine(15, true), mine(16, true), mine(17, true)];
    let (mut field, ship) = (field_with(&mines, &[]), craft());
    // The volley's two rounds fly together and each takes a mine.
    volley_and_fly(&mut field, &ship, 30);
    assert_eq!(field.cannon.score, 100 + 200);
    assert_eq!(field.cannon.m_mine, 3.0);
    assert_eq!(field.cannon.destroyed_mines, 2);
    // The next volley takes the third.
    volley_and_fly(&mut field, &ship, 30);
    assert_eq!(field.cannon.score, 100 + 200 + 300);
    assert_eq!(field.cannon.destroyed_mines, 3);
}

#[test]
fn the_multiplier_goes_back_to_one_when_the_reload_ends() {
    let (mut field, ship) = (field_with(&[mine(15, true)], &[]), craft());
    volley_and_fly(&mut field, &ship, 30);
    assert_eq!(field.cannon.m_mine, 2.0);
    let mut reload = IDLE;
    reload.reload = true;
    tick(&mut field, reload, &ship, &open_air);
    assert!(!field.cannon.ready, "locked out");
    assert_eq!(field.cannon.m_mine, 2.0, "not at the request");
    for _ in 0..60 {
        tick(&mut field, IDLE, &ship, &open_air);
    }
    assert!(field.cannon.ready);
    assert_eq!(field.cannon.m_mine, 1.0, "at the first ready tick");
    assert_eq!(field.cannon.hits, [0; POOL]);
}

#[test]
fn a_reload_locks_the_trigger_out_for_three_quarters_of_a_second() {
    let (mut field, ship) = (Field::new(), craft());
    tick(&mut field, FIRE, &ship, &open_air);
    let mut reload = FIRE;
    reload.reload = true;
    tick(&mut field, reload, &ship, &open_air);
    assert_eq!(field.cannon.clip, 15, "refilled");
    for _ in 0..40 {
        tick(&mut field, FIRE, &ship, &open_air);
    }
    assert_eq!(field.cannon.clip, 15, "0.67 s in, the trigger does nothing");
    for _ in 0..20 {
        tick(&mut field, FIRE, &ship, &open_air);
    }
    assert!(field.cannon.clip < 15, "and fires once the lockout is over");
}

#[test]
fn an_empty_clip_reloads_only_once_the_trigger_is_released() {
    let (mut field, ship) = (Field::new(), craft());
    for _ in 0..200 {
        tick(&mut field, FIRE, &ship, &open_air);
    }
    assert_eq!(field.cannon.clip, 0, "held, it runs dry and stays dry");
    tick(&mut field, IDLE, &ship, &open_air);
    assert_eq!(field.cannon.clip, 15);
    assert!(!field.cannon.ready);
}

#[test]
fn a_bomb_takes_four_hits_and_the_fourth_destroys_it_and_sets_off_the_mines_near_it() {
    // Entry 20 is z = 200; entry 22 is 20 away, entry 30 is 100 away.
    let mines = [mine(22, true), mine(30, true)];
    let (mut field, ship) = (field_with(&mines, &[mine(20, true)]), craft());
    volley_and_fly(&mut field, &ship, 35);
    assert_eq!(field.live_bombs, 1);
    assert_eq!(field.bomb_hits[0], 2, "two rounds, two hits");
    assert_eq!(
        field.cannon.score, 0,
        "a hit that does not destroy pays nothing"
    );
    volley_and_fly(&mut field, &ship, 35);
    assert_eq!(field.live_bombs, 0);
    assert_eq!(field.cannon.destroyed_bombs, 1);
    assert_eq!(
        field.cannon.score,
        2000 + 100,
        "the bomb, then the live mine it set off"
    );
    assert_eq!(field.live_mines, 1, "the far mine stays");
    assert_eq!(
        field.cannon.destroyed_mines, 0,
        "a chain mine is not counted"
    );
}

#[test]
fn each_bomb_destroyed_pays_five_hundred_more() {
    let (mut field, ship) = (field_with(&[], &[mine(20, true)]), craft());
    field.cannon.bombs_shot = 2;
    for _ in 0..2 {
        volley_and_fly(&mut field, &ship, 35);
    }
    assert_eq!(field.cannon.score, 2000 + 2 * 500);
    assert_eq!(field.cannon.bombs_shot, 3);
}

#[test]
fn a_chain_reaction_multiplier_starts_again_at_each_bomb() {
    let mines = [mine(22, true), mine(23, true)];
    let (mut field, ship) = (field_with(&mines, &[mine(20, true)]), craft());
    field.cannon.m_chain = 9.0;
    for _ in 0..2 {
        volley_and_fly(&mut field, &ship, 35);
    }
    assert_eq!(field.cannon.score, 2000 + 100 + 200);
}

#[test]
fn a_bomb_removal_keeps_the_hit_counts_with_their_bombs() {
    let mut field = field_with(&[], &[mine(20, true), mine(25, true)]);
    field.bomb_hits = [3, 1];
    field.remove_bomb(0);
    assert_eq!(field.live_bombs, 1);
    assert_eq!(field.bombs[0], mine(25, true));
    assert_eq!(field.bomb_hits, [1, 0]);
}

fn volleys_with_hits(cannon: &mut Cannon, hit: impl Fn(usize) -> bool) {
    for volley in 0..POOL / 2 {
        cannon.hits[2 * volley] = 1;
        cannon.hits[2 * volley + 1] = if hit(volley) { 2 } else { 1 };
    }
}

#[test]
fn five_volleys_with_a_hit_in_a_row_pay_a_thousand_once() {
    let mut cannon = Cannon::new();
    volleys_with_hits(&mut cannon, |v| v < 5);
    award_streaks(&mut cannon);
    assert_eq!(cannon.score, 1000);
    award_streaks(&mut cannon);
    assert_eq!(cannon.score, 1000, "a scan again pays nothing");
}

#[test]
fn a_miss_breaks_the_streak() {
    let mut cannon = Cannon::new();
    volleys_with_hits(&mut cannon, |v| v < 4 || (5..9).contains(&v));
    award_streaks(&mut cannon);
    assert_eq!(cannon.score, 0);
}

#[test]
fn ten_in_a_row_pay_three_thousand_and_fifteen_pay_five_thousand() {
    let mut cannon = Cannon::new();
    volleys_with_hits(&mut cannon, |v| v < 10);
    award_streaks(&mut cannon);
    assert_eq!(cannon.score, 1000 + 3000, "five on the way, then ten");
    volleys_with_hits(&mut cannon, |_| true);
    award_streaks(&mut cannon);
    assert_eq!(cannon.score, 1000 + 3000 + 5000);
    award_streaks(&mut cannon);
    assert_eq!(cannon.score, 9000, "each pays once");
}

#[test]
fn the_award_counts_run_over_the_race_and_a_second_five_counts_again() {
    let mut cannon = Cannon::new();
    volleys_with_hits(&mut cannon, |v| v < 5 || (6..11).contains(&v));
    award_streaks(&mut cannon);
    assert_eq!((cannon.fives, cannon.tens, cannon.fifteens), (2, 0, 0));
    volleys_with_hits(&mut cannon, |_| true);
    award_streaks(&mut cannon);
    assert_eq!(
        (cannon.fives, cannon.tens, cannon.fifteens),
        (2, 1, 1),
        "a ten and a fifteen count once each, the five is already paid"
    );
    award_streaks(&mut cannon);
    assert_eq!((cannon.fives, cannon.tens, cannon.fifteens), (2, 1, 1));
}

#[test]
fn a_reload_clears_the_clips_awards_and_keeps_the_race_counts() {
    let (mut field, ship) = (Field::new(), craft());
    tick(&mut field, IDLE, &ship, &open_air);
    field.cannon.fives = 2;
    field.cannon.tens = 1;
    field.cannon.awards.first_five = true;
    field.cannon.awards.ten = true;
    let mut reload = IDLE;
    reload.reload = true;
    tick(&mut field, reload, &ship, &open_air);
    for _ in 0..60 {
        tick(&mut field, IDLE, &ship, &open_air);
    }
    assert_eq!(field.cannon.awards, Awards::default());
    assert_eq!((field.cannon.fives, field.cannon.tens), (2, 1));
}

#[test]
fn a_second_five_pays_when_a_separate_streak_ends_and_a_longer_first_one_does_not() {
    let mut cannon = Cannon::new();
    volleys_with_hits(&mut cannon, |v| v < 5 || (6..11).contains(&v));
    award_streaks(&mut cannon);
    assert_eq!(
        cannon.score, 2000,
        "the second five ends at volley 10, six on"
    );
    let mut long = Cannon::new();
    volleys_with_hits(&mut long, |v| v < 9);
    award_streaks(&mut long);
    assert_eq!(
        long.score, 1000,
        "volleys 5 to 8 of one streak are not a second five"
    );
}

#[test]
fn the_percentage_is_the_destroyed_mines_over_the_whole_race() {
    let mut values = GamePlayValues {
        mine_count_stage: [
            80, 90, 95, 100, 105, 110, 120, 130, 140, 150, 170, 180, 190, 200,
        ],
        ..GamePlayValues::default()
    };
    assert_eq!(mines_in_the_race(&values), 1860.0);
    assert_eq!(percentage(186, &values), 10.0);
    assert_eq!(percentage(0, &values), 0.0);
    values.mine_count_stage = [0; 14];
    assert_eq!(percentage(5, &values), 0.0, "no division by nothing");
}

#[test]
fn a_round_rides_above_the_floor_it_finds() {
    let mut ship = craft();
    ship.physics.body.position = Vec3::new(0.0, 5.0, 0.0);
    let floor = |from: Vec3, to: Vec3| {
        if from.y > 0.0 && to.y <= 0.0 {
            Sweep::Floor {
                point: Vec3::new(from.x, 0.0, from.z),
                normal: Vec3::Y,
            }
        } else {
            Sweep::None
        }
    };
    let mut field = Field::new();
    tick(&mut field, FIRE, &ship, &floor);
    let round = field
        .cannon
        .rounds
        .iter()
        .find(|r| r.live)
        .expect("in flight");
    assert_eq!(round.pos.y, 3.5);
}

#[test]
fn a_round_turns_with_the_slope_it_rides() {
    // A floor tilted 30 degrees about X: its normal leans back, so the heading must follow it.
    let tilt = Vec3::new(0.0, 0.866_025_4, -0.5);
    let slope = move |from: Vec3, to: Vec3| {
        if to.y < from.y {
            Sweep::Floor {
                point: Vec3::new(from.x, from.y - 4.0, from.z),
                normal: tilt,
            }
        } else {
            Sweep::None
        }
    };
    let ship = craft();
    let mut field = Field::new();
    tick(&mut field, FIRE, &ship, &slope);
    tick(&mut field, IDLE, &ship, &slope);
    let round = field
        .cannon
        .rounds
        .iter()
        .find(|r| r.live)
        .expect("in flight");
    assert!(
        round.vel.dot(round.normal).abs() < 1e-3,
        "velocity stays in the surface plane: {:?}",
        round.vel
    );
    assert!(round.vel.y > 100.0, "climbing the slope: {:?}", round.vel);
    assert!((round.vel.length() - 500.0).abs() < 1e-2);
}

#[test]
fn a_round_is_still_drawn_along_its_incoming_heading_on_the_tick_it_bounces() {
    let ship = craft();
    let mut field = Field::new();
    tick(&mut field, FIRE, &ship, &wall_at_fifty);
    let mut bounced = None;
    for _ in 0..12 {
        tick(&mut field, IDLE, &ship, &wall_at_fifty);
        if let Some(r) = field
            .cannon
            .rounds
            .iter()
            .find(|r| r.live && r.rebounds == 0)
        {
            bounced = Some(*r);
            break;
        }
    }
    let round = bounced.expect("bounced");
    assert!(round.vel.z < 0.0, "reflected: {:?}", round.vel);
    assert!(
        round.heading.z > 0.9,
        "heading is the incoming one: {:?}",
        round.heading
    );
    tick(&mut field, IDLE, &ship, &|_, _| Sweep::None);
    let next = field
        .cannon
        .rounds
        .iter()
        .find(|r| r.live)
        .expect("flying back");
    assert!(
        next.heading.z < -0.9,
        "then the reflected one: {:?}",
        next.heading
    );
}

fn wall_at_fifty(from: Vec3, to: Vec3) -> Sweep {
    if (from.z - 50.0) * (to.z - 50.0) < 0.0 {
        Sweep::Wall {
            point: Vec3::new(from.x, from.y, 50.0),
            normal: Vec3::NEG_Z,
        }
    } else {
        Sweep::None
    }
}

#[test]
fn a_round_bounces_once_off_a_wall_and_the_next_wall_kills_it() {
    let ship = craft();
    let mut field = Field::new();
    tick(&mut field, FIRE, &ship, &wall_at_fifty);
    for _ in 0..8 {
        tick(&mut field, IDLE, &ship, &wall_at_fifty);
    }
    let round = field
        .cannon
        .rounds
        .iter()
        .find(|r| r.live)
        .expect("bounced, not dead");
    assert!(round.vel.z < 0.0, "now flying back: {:?}", round.vel);
    assert_eq!(round.rebounds, 0);
    assert!(round.pos.z < 50.0);
    let mut turned = field;
    // Sent at the wall again from the near side: no rebound left, so it dies.
    for round in &mut turned.cannon.rounds {
        if round.live {
            round.pos = Vec3::new(0.0, 0.0, 45.0);
            round.vel = Vec3::new(0.0, 0.0, 500.0);
        }
    }
    for _ in 0..2 {
        tick(&mut turned, IDLE, &ship, &wall_at_fifty);
    }
    assert_eq!(turned.cannon.rounds_in_flight(), 0);
}

#[test]
fn a_field_that_never_armed_a_cannon_stays_empty() {
    assert!(Field::new().is_empty());
    let mut field = Field::new();
    tick(&mut field, IDLE, &craft(), &open_air);
    assert!(!field.is_empty(), "arming is a change");
}

#[test]
fn a_volley_reports_itself_once_at_the_craft() {
    let (mut field, ship) = (Field::new(), craft());
    let first = tick_events(&mut field, FIRE, &ship, &open_air);
    assert_eq!(
        first,
        [Event {
            kind: EventKind::Volley,
            at: ship.physics.body.position,
            points: None
        }]
    );
    assert!(tick_events(&mut field, FIRE, &ship, &open_air).is_empty());
}

#[test]
fn a_shot_mine_reports_whether_it_was_live_and_where_it_was() {
    for live in [true, false] {
        let (mut field, ship) = (field_with(&[mine(15, live)], &[]), craft());
        tick(&mut field, FIRE, &ship, &open_air);
        let mut shot = Vec::new();
        for _ in 0..30 {
            shot.extend(tick_events(&mut field, IDLE, &ship, &open_air));
        }
        assert_eq!(
            shot,
            [Event {
                kind: EventKind::MineShot { live },
                at: Vec3::new(0.0, crate::detonator::MINE_LIFT, 150.0),
                points: Some(if live { 100 } else { 50 }),
            }]
        );
    }
}

#[test]
fn a_destroyed_bomb_and_the_mines_it_chains_report_themselves() {
    let (mut field, ship) = (
        field_with(&[mine(16, true), mine(40, false)], &[mine(15, true)]),
        craft(),
    );
    field.bomb_hits[0] = 3;
    tick(&mut field, FIRE, &ship, &open_air);
    let mut events = Vec::new();
    for _ in 0..30 {
        events.extend(tick_events(&mut field, IDLE, &ship, &open_air));
    }
    let kinds: Vec<EventKind> = events.iter().map(|e| e.kind).collect();
    assert!(kinds.contains(&EventKind::BombShot), "{kinds:?}");
    assert!(
        kinds.contains(&EventKind::MineChained { live: true }),
        "the mine beside the bomb goes with it: {kinds:?}"
    );
    assert!(
        !kinds
            .iter()
            .any(|k| matches!(k, EventKind::MineChained { live: false })),
        "the far mine stays: {kinds:?}"
    );
    let paid: Vec<i32> = events.iter().filter_map(|e| e.points).collect();
    assert_eq!(paid, [2000, 100], "the bomb, then the chained live mine");
}

#[test]
fn a_round_dying_on_a_wall_reports_the_wall() {
    let ship = craft();
    let mut field = Field::new();
    tick(&mut field, FIRE, &ship, &wall_at_fifty);
    let mut walls = Vec::new();
    for _ in 0..40 {
        walls.extend(
            tick_events(&mut field, IDLE, &ship, &wall_at_fifty)
                .into_iter()
                .filter(|e| matches!(e.kind, EventKind::RoundWall { .. })),
        );
    }
    assert_eq!(walls.len(), 0, "a round that bounces is not dead yet");
    for round in &mut field.cannon.rounds {
        if round.live {
            round.pos = Vec3::new(0.0, 0.0, 45.0);
            round.vel = Vec3::new(0.0, 0.0, 500.0);
            round.rebounds = 0;
        }
    }
    for _ in 0..2 {
        walls.extend(
            tick_events(&mut field, IDLE, &ship, &wall_at_fifty)
                .into_iter()
                .filter(|e| matches!(e.kind, EventKind::RoundWall { .. })),
        );
    }
    assert!(!walls.is_empty());
    assert_eq!(walls[0].at.z, 50.0);
    // The round flies `+Z` into a wall facing `-Z`: it would rebound straight back, and
    // the spark spawns on the road, `RIDE` below the hit.
    let EventKind::RoundWall {
        normal,
        spawn,
        rebound,
    } = walls[0].kind
    else {
        panic!("{:?}", walls[0].kind)
    };
    assert_eq!(normal, Vec3::NEG_Z);
    assert!(rebound.distance(Vec3::NEG_Z) < 1e-4, "rebound {rebound:?}");
    assert!(
        spawn.distance(walls[0].at - Vec3::Y * 3.5) < 1e-4,
        "spawn {spawn:?} from the hit {:?}",
        walls[0].at
    );
}

#[test]
fn a_volley_leaves_from_the_hulls_cannon_locators() {
    let ship = craft();
    let (left, right) = (Vec3::new(1.5, 0.25, 2.0), Vec3::new(-1.5, 0.25, 2.0));
    let config = Config {
        muzzles: [Some(left), Some(right)],
        ..config()
    };
    let mut field = Field::new();
    step_cannon(&mut field, &lap(), &config, FIRE, &ship, DT, &open_air);
    let from: Vec<Vec3> = field
        .cannon
        .rounds
        .iter()
        .filter(|r| r.live)
        .map(|r| r.prev)
        .collect();
    let body = &ship.physics.body;
    assert_eq!(from.len(), 2);
    assert_eq!(from[0], crate::projectile::plasma::hull_point(body, left));
    assert_eq!(from[1], crate::projectile::plasma::hull_point(body, right));
    assert!(
        (from[0] - body.position).dot(body.right()) < 0.0,
        "the left locator is on the craft's left"
    );
}
