use super::*;

const AWARDS: &str = r#"<XPAwards>
<ClassXPMultipliers venom="1.0" flash="1.0" rapier="1.0" phantom="1.0" superphantom="1.0" />
<SkillXPMultipliers easy="1.0" medium="1.0" hard="1.0" />
<Award index="0" zone="1" elim="2" zombie="3" race="4" speed="5" name="Event complete- PASS" />
<Award index="1" zone="10" elim="20" zombie="30" race="40" speed="50" name="Event complete- ELITE" />
<Award index="24" zone="6" elim="7" zombie="8" race="9" speed="10" name="Finished 1st!" />
<Award index="26" zone="1" elim="1" zombie="0" race="2" speed="1" name="Finished 3rd!" group="speed"/>
</XPAwards>"#;

const RANKS: &str = r#"<XPRanks><MaximumRank value="3"/>
<Rank index="1" value="100"/><Rank index="2" value="250"/><Rank index="3" value="500"/></XPRanks>"#;

fn ranks() -> Ranks {
    Ranks::parse(RANKS).expect("ranks")
}

#[test]
fn rank_is_the_count_of_thresholds_reached() {
    let r = ranks();
    assert_eq!(r.rank(0), 0);
    assert_eq!(r.rank(99), 0);
    assert_eq!(r.rank(100), 1);
    assert_eq!(r.rank(249), 1);
    assert_eq!(r.rank(250), 2);
    assert_eq!(r.rank(500), 3);
    assert_eq!(r.rank(u32::MAX), 3, "capped at the maximum rank");
}

#[test]
fn threshold_is_zero_authored_then_the_total() {
    let r = ranks();
    assert_eq!(r.threshold_xp(0, 7), 0);
    assert_eq!(r.threshold_xp(1, 7), 100);
    assert_eq!(r.threshold_xp(3, 7), 500);
    assert_eq!(r.threshold_xp(4, 7), 7);
}

#[test]
fn the_reveal_query_agrees_with_the_rank_at_every_boundary() {
    let r = ranks();
    for total in [0, 1, 99, 100, 101, 249, 250, 499, 500, 501, 10_000] {
        for required in 0..=3_i32 {
            assert_eq!(
                r.meets(total, Some(required)),
                r.rank(total) >= required as u32,
                "total {total}, rank required {required}"
            );
        }
    }
    assert!(r.meets(0, None));
}

#[test]
fn an_award_pays_its_value_in_the_mode_column_and_nothing_where_unauthored() {
    let a = Awards::parse(AWARDS).expect("awards");
    assert_eq!(a.xp(award::EVENT_PASS, Column::Race), 4);
    assert_eq!(a.xp(award::EVENT_ELITE, Column::Race), 40);
    assert_eq!(a.xp(award::FINISHED_1ST, Column::Zone), 6);
    assert_eq!(a.xp(award::FINISHED_1ST + 2, Column::Zombie), 0);
    assert_eq!(a.xp(award::FINISHED_1ST + 1, Column::Race), 0, "no 2nd row");
}
