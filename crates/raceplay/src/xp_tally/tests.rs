use super::*;

const XML: &str = r#"<XPAwards>
<ClassXPMultipliers venom="1.0" flash="1.0" rapier="1.0" phantom="1.0" superphantom="1.0"/>
<SkillXPMultipliers easy="1.0" medium="1.0" hard="1.0"/>
<Award index="0" name="pass" zone="100" elim="100" zombie="100" race="100" speed="100"/>
<Award index="6" name="kill" zone="50" elim="50" zombie="50" race="50" speed="50"/>
<Award index="5" name="perfect lap" zone="5" elim="5" zombie="5" race="5" speed="5"/>
<Award index="12" name="hit rocket" zone="12" elim="12" zombie="12" race="12" speed="12"/>
<Award index="19" name="hit leach" zone="19" elim="19" zombie="19" race="19" speed="19"/>
<Award index="21" name="taken 1st" zone="21" elim="21" zombie="21" race="21" speed="21"/>
<Award index="22" name="taken 2nd" zone="22" elim="22" zombie="22" race="22" speed="22"/>
<Award index="23" name="taken 3rd" zone="23" elim="23" zombie="23" race="23" speed="23"/>
<Award index="32" name="shield rocket" zone="32" elim="32" zombie="32" race="32" speed="32"/>
<Award index="39" name="shield leach" zone="39" elim="39" zombie="39" race="39" speed="39"/>
<Award index="44" name="full health" zone="44" elim="44" zombie="44" race="44" speed="44"/>
<Award index="9" name="zone" zone="10" elim="10" zombie="10" race="10" speed="10"/>
<Award index="10" name="pzone" zone="30" elim="30" zombie="30" race="30" speed="30"/>
<Award index="24" name="1st" zone="50" elim="50" zombie="50" race="50" speed="50"/>
<Award index="41" name="h30" zone="60" elim="60" zombie="0" race="60" speed="60"/>
<Award index="42" name="h20" zone="30" elim="30" zombie="0" race="30" speed="30"/>
<Award index="43" name="h10" zone="10" elim="10" zombie="0" race="10" speed="10"/>
<Award index="48" name="hz10" zone="10" elim="0" zombie="0" race="0" speed="0"/>
<Award index="57" name="hz100" zone="50" elim="0" zombie="0" race="0" speed="0"/>
</XPAwards>"#;

fn awards() -> Awards {
    Awards::parse(XML).expect("the fixture parses")
}

fn tally(column: Column) -> XpTally {
    XpTally::new(awards(), column)
}

#[test]
fn a_kill_pays_its_authored_value_once_per_kill() {
    let mut t = tally(Column::Race);
    t.observe_kills(1);
    t.observe_kills(1);
    assert_eq!(t.times_paid(6), 1);
    assert_eq!(t.total(), awards().xp(6, Column::Race));
    t.observe_kills(3);
    assert_eq!(t.times_paid(6), 3);
}

#[test]
fn a_zone_pays_perfect_or_plain_and_hit_zone_on_every_tenth() {
    let mut t = tally(Column::Zone);
    t.zone_cleared(1, true);
    assert_eq!(t.total(), awards().xp(10, Column::Zone));
    t.zone_cleared(2, false);
    assert_eq!(t.times_paid(9), 1);
    let before = t.total();
    t.zone_cleared(10, false);
    assert_eq!(t.times_paid(48), 1);
    assert!(t.total() > before + awards().xp(9, Column::Zone) - 1);
    t.zone_cleared(100, true);
    assert_eq!(t.times_paid(57), 1);
    t.zone_cleared(110, true);
    assert_eq!(t.times_paid(58), 0);
}

#[test]
fn an_award_the_column_does_not_pay_is_dropped_and_leaves_the_chain_alone() {
    let mut t = tally(Column::Race);
    t.award(6);
    t.award(48);
    assert_eq!(t.times_paid(48), 0);
    assert_eq!(t.total(), awards().xp(6, Column::Race));
}

#[test]
fn six_awards_inside_the_window_pay_the_chain_bonus_once() {
    let mut t = tally(Column::Race);
    let kill = awards().xp(6, Column::Race);
    for _ in 0..5 {
        t.award(6);
    }
    assert_eq!(t.total(), 5 * kill, "five awards raise the chain to four");
    t.award(6);
    assert_eq!(
        t.total(),
        6 * kill + CHAIN_LENGTH * 2,
        "the sixth reaches five and pays chain * 2"
    );
    t.award(6);
    assert_eq!(
        t.total(),
        7 * kill + 10,
        "the counter reset, so no second bonus"
    );
}

#[test]
fn a_gap_over_the_window_resets_the_chain() {
    let mut t = tally(Column::Race);
    let kill = awards().xp(6, Column::Race);
    for _ in 0..3 {
        t.award(6);
    }
    for _ in 0..CHAIN_WINDOW_TICKS {
        t.advance_window();
    }
    for _ in 0..3 {
        t.award(6);
    }
    assert_eq!(t.total(), 6 * kill, "the second run starts a new chain");
    let mut inside = tally(Column::Race);
    for _ in 0..3 {
        inside.award(6);
    }
    for _ in 0..CHAIN_WINDOW_TICKS - 1 {
        inside.advance_window();
    }
    for _ in 0..3 {
        inside.award(6);
    }
    assert_eq!(
        inside.total(),
        6 * kill + 10,
        "one tick short of the window keeps it"
    );
}

#[test]
fn holding_first_pays_at_ten_twenty_and_thirty_seconds() {
    let mut t = tally(Column::Race);
    for _ in 0..=30 * 60 + 5 {
        t.observe_place(1, 8);
        t.advance_window();
    }
    assert_eq!(t.times_paid(43), 1);
    assert_eq!(t.times_paid(42), 1);
    assert_eq!(t.times_paid(41), 1);
}

#[test]
fn losing_the_place_restarts_the_hold() {
    let mut t = tally(Column::Race);
    for _ in 0..9 * 60 {
        t.observe_place(1, 8);
    }
    t.observe_place(2, 8);
    for _ in 0..9 * 60 {
        t.observe_place(1, 8);
    }
    assert_eq!(t.times_paid(43), 0);
}

#[test]
fn the_hold_pays_only_first_place_against_rivals_and_not_in_elimination() {
    let mut second = tally(Column::Race);
    let mut alone = tally(Column::Race);
    let mut elim = tally(Column::Elim);
    for _ in 0..11 * 60 {
        second.observe_place(2, 8);
        alone.observe_place(1, 1);
        elim.observe_place(1, 8);
    }
    assert_eq!(second.total() + alone.total() + elim.total(), 0);
}

#[test]
fn finishing_pays_completion_and_place_and_take_empties_the_tally() {
    let mut t = tally(Column::Race);
    t.finish(
        Completion {
            pass: true,
            elite: false,
        },
        Some(1),
    );
    assert_eq!(
        t.total(),
        awards().xp(0, Column::Race) + awards().xp(24, Column::Race)
    );
    let taken = t.take();
    assert_eq!(
        taken,
        awards().xp(0, Column::Race) + awards().xp(24, Column::Race)
    );
    assert_eq!(t.total(), 0);
}

#[test]
fn the_completion_awards_pay_on_improvement_only() {
    let pass = Some(false);
    let elite = Some(true);
    let pays = |before, now| {
        let c = Completion::on_improvement(before, now);
        (c.pass, c.elite)
    };
    assert_eq!(pays(None, pass), (true, false));
    assert_eq!(pays(None, elite), (true, true), "an elite first pass pays both");
    assert_eq!(pays(pass, elite), (false, true));
    assert_eq!(pays(None, None), (false, false));
    assert_eq!(pays(pass, pass), (false, false));
    assert_eq!(pays(elite, elite), (false, false));
    assert_eq!(pays(elite, pass), (false, false), "a worse result pays nothing");
}

#[test]
fn the_completion_xp_is_the_authored_value_without_the_chain() {
    let mut t = tally(Column::Race);
    t.finish(
        Completion {
            pass: true,
            elite: false,
        },
        Some(1),
    );
    assert_eq!(t.completion_xp(), awards().xp(0, Column::Race));
    t.take();
    assert_eq!(t.completion_xp(), awards().xp(0, Column::Race), "kept past take");
}

#[test]
fn a_lap_is_perfect_until_health_drops_and_full_only_at_the_maximum() {
    let mut t = tally(Column::Race);
    t.observe_health(100.0);
    t.lap_completed(100.0, 100.0);
    assert_eq!((t.times_paid(5), t.times_paid(44)), (1, 1));
    t.observe_health(80.0);
    t.observe_health(100.0);
    t.lap_completed(100.0, 100.0);
    assert_eq!(
        (t.times_paid(5), t.times_paid(44)),
        (1, 2),
        "a hit and a refill is a damaged lap that still ends at full health"
    );
    t.lap_completed(100.0, 100.0);
    assert_eq!(t.times_paid(5), 2, "the damage is forgotten at the line");
    t.observe_health(70.0);
    t.lap_completed(70.0, 100.0);
    assert_eq!((t.times_paid(5), t.times_paid(44)), (2, 3), "a damaged lap below the maximum pays neither");
}

#[test]
fn each_of_the_first_three_places_is_taken_once_with_enough_craft() {
    let mut t = tally(Column::Race);
    t.observe_taken(5, 8);
    t.observe_taken(3, 8);
    t.observe_taken(3, 8);
    t.observe_taken(4, 8);
    t.observe_taken(1, 8);
    t.observe_taken(2, 8);
    t.observe_taken(1, 8);
    assert_eq!(
        [21, 22, 23].map(|i| t.times_paid(i)),
        [1, 0, 1],
        "2nd was passed through after 1st"
    );
    let mut alone = tally(Column::Race);
    alone.observe_taken(1, 1);
    alone.observe_taken(2, 2);
    assert_eq!(alone.total(), 0, "a place needs more craft than its number");
}

#[test]
fn a_weapon_pays_its_own_row_in_the_originals_type_order() {
    let mut t = tally(Column::Race);
    t.weapon_hit(Weapon::Rocket);
    t.weapon_hit(Weapon::LeachBeam);
    t.shielded_against(Weapon::Rocket);
    t.shielded_against(Weapon::LeachBeam);
    t.weapon_hit(Weapon::Turbo);
    t.shielded_against(Weapon::Repulser);
    assert_eq!([12, 19, 32, 39].map(|i| t.times_paid(i)), [1; 4]);
    assert_eq!(t.total(), 12 + 19 + 32 + 39, "a turbo and a repulser pay nothing");
}
