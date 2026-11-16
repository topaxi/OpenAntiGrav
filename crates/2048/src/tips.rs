//! Wipeout 2048's game tips: the 37-row table the race load picks one from.
//!
//! `g_GameTipTable` (`0x8151d230`, `docs/ghidra/functions/vita-2048-eu-v104/race-load.md`, "What
//! `CampaignEventCard_EnterLoad` does: the game tip"): per row a texture stem under
//! `data/fe/newimages/tips/` (the executable says `.gtf`, the package ships `.gxt`), a title key
//! and a body key. The words come out of the disc's string table; only the keys are here.

use oag_title::loading::{GameTip, GameTips};

const fn tip(texture: &'static str, title: &'static str, body: Option<&'static str>) -> GameTip {
    GameTip {
        texture,
        title,
        body,
    }
}

/// The table and the pick's lists (`GameTip_Pick`, step 3: always rows `1, 11, 12, 13, 14, 15,
/// 16, 28, 30, 31, 33, 0`; the class rows race 5, zone 8, combat 3, speed lap 6, time trial 7;
/// 32 for a combat event; 29 offline). Rows 34 to 36 (the controls diagrams) are drawn by an
/// unread routine and no list reaches them here.
pub const TIPS: GameTips = GameTips {
    dir: r"Data\FE\NewImages\tips\",
    question_mark: "question_mark",
    header: "TUT_TIP_HEADER",
    rows: &TABLE,
    always: &[1, 11, 12, 13, 14, 15, 16, 28, 30, 31, 33, 0],
    class_rows: [5, 8, 3, 6, 7],
    combat_row: 32,
    offline_row: 29,
};

/// `g_GameTipTable`, rows 0 to 36.
const TABLE: [GameTip; 37] = [
    tip("Tut_Absorb_0", "Tut_Absorb_0", Some("Tut_Absorb_2")),
    tip(
        "Tut_Airbrakes_0",
        "OPT_CTRL_AIRBRAKES",
        Some("Tut_Airbrakes_2"),
    ),
    tip(
        "Tut_Event_A_Plus_Challenge_0",
        "Tut_Event_A_Plus_Challenge_0",
        Some("Tut_Event_A_Plus_Challenge_1"),
    ),
    tip(
        "Tut_Event_Combat_0",
        "Tut_Event_Combat_0",
        Some("Tut_Event_Combat_1"),
    ),
    tip(
        "Tut_Event_Proto_Ship_Challenge_0",
        "Tut_Event_Proto_Ship_Challenge_0",
        Some("Tut_Event_Proto_Ship_Challenge_1"),
    ),
    tip(
        "Tut_Event_Race_0",
        "Tut_Event_Race_0",
        Some("Tut_Event_Race_1"),
    ),
    tip(
        "Tut_Event_Speed_Lap_0",
        "Tut_Event_Speed_Lap_0",
        Some("Tut_Event_Speed_Lap_1"),
    ),
    tip(
        "Tut_Event_Speed_Lap_0",
        "Tut_Event_Time_Trial_0",
        Some("Tut_Event_Time_Trial_1"),
    ),
    tip(
        "Tut_Event_Zone_0",
        "Tut_Event_Zone_0",
        Some("Tut_Event_Zone_1"),
    ),
    tip(
        "Tut_Green_Pads_0",
        "Tut_Green_Pads_0",
        Some("Tut_Green_Pads_2"),
    ),
    tip(
        "Tut_Yellow_Pads_0",
        "Tut_Yellow_Pads_0",
        Some("Tut_Yellow_Pads_2"),
    ),
    tip(
        "Tut_Pilot_Assist_0",
        "Tut_Pilot_Assist_0",
        Some("Tut_Pilot_Assist_1"),
    ),
    tip(
        "Tut_Event_Proto_Ship_Challenge_0",
        "Tut_Prototype_Ships_0",
        Some("Tut_Prototype_Ships_1"),
    ),
    tip(
        "Tut_Ship_Types_0",
        "Tut_Ship_Types_0",
        Some("Tut_Ship_Types_1"),
    ),
    tip(
        "Tut_Skillcuts_0",
        "Tut_Skillcuts_0",
        Some("Tut_Skillcuts_1"),
    ),
    tip(
        "Tut_Speed_Classes_0",
        "Tut_Speed_Classes_0",
        Some("Tut_Speed_Classes_1"),
    ),
    tip(
        "Tut_Speed_Pads_0",
        "Tut_Speed_Pads_0",
        Some("Tut_Speed_Pads_2"),
    ),
    tip(
        "Tut_Weapon_Autopilot_0",
        "FE_AUTOPILOT",
        Some("Tut_Weapon_Autopilot_1"),
    ),
    tip(
        "Tut_Weapon_Cannon_0",
        "FE_CANNON",
        Some("Tut_Weapon_Cannon_1"),
    ),
    tip(
        "Tut_Weapon_Leech_Beam_0",
        "FE_LEECHBEAM",
        Some("Tut_Weapon_Leech_Beam_1"),
    ),
    tip(
        "Tut_Weapon_Mines_Bombs_0",
        "Tut_Weapon_Mines_Bombs_0",
        Some("Tut_Weapon_Mines_Bombs_1"),
    ),
    tip(
        "Tut_Weapon_Missile_0",
        "FE_MISSILE",
        Some("Tut_Weapon_Missile_1"),
    ),
    tip(
        "Tut_Weapon_Plasma_0",
        "Tut_Weapon_Plasma_0",
        Some("Tut_Weapon_Plasma_1"),
    ),
    tip("Tut_Weapon_Quake_0", "FE_QUAKE", Some("Tut_Weapon_Quake_1")),
    tip(
        "Tut_Weapon_Rockets_0",
        "Tut_Weapon_Rockets_0",
        Some("Tut_Weapon_Rockets_1"),
    ),
    tip(
        "Tut_Weapon_Shield_0",
        "Tut_Weapon_Shield_0",
        Some("Tut_Weapon_Shield_1"),
    ),
    tip("Tut_Weapon_Turbo_0", "FE_TURBO", Some("Tut_Weapon_Turbo_1")),
    tip("Tut_XP_SP_0", "Tut_XP_MP_1_1", Some("Tut_XP_MP_1_0")),
    tip("Tut_XP_Rank_0", "Tut_XP_Rank_0", Some("Tut_XP_Rank_1")),
    tip("Tut_XP_SP_0", "Tut_XP_SP_1", Some("Tut_XP_SP_0")),
    tip(
        "Tut_barrel_roll",
        "Tut_Barrel_Roll_0",
        Some("Tut_Barrel_Roll_2"),
    ),
    tip("Tut_side_shift", "Tut_Sideshift_0", Some("Tut_Sideshift_2")),
    tip("Tut_flip_0", "Tut_Combat_Spin_0", Some("Tut_Combat_Spin_1")),
    tip(
        "Tut_boost_start",
        "Tut_Boost_Start_0",
        Some("Tut_Boost_Start"),
    ),
    tip("Wipeout_cntrls", "Tut_Controls_Wipeout_2", None),
    tip("Racer_cntrls", "FE_CTRL_RACER", None),
    tip("Motion_cntrls", "Tut_Controls_Motion_2", None),
];

#[cfg(test)]
mod tests {
    use super::*;
    use oag_title::loading::TipClass;

    #[test]
    fn the_table_has_the_thirty_seven_rows_with_the_three_diagrams_last() {
        assert_eq!(TABLE.len(), 37);
        assert!(TABLE[..34].iter().all(|row| row.body.is_some()));
        assert!(TABLE[34..].iter().all(|row| row.body.is_none()));
        assert_eq!(TABLE[21].title, "FE_MISSILE");
        assert_eq!(
            TIPS.entry(TABLE[21].texture),
            r"Data\FE\NewImages\tips\Tut_Weapon_Missile_0.gtf"
        );
    }

    #[test]
    fn a_pick_names_only_rows_with_a_body_and_a_combat_event_adds_its_row() {
        for class in [
            TipClass::Race,
            TipClass::Zone,
            TipClass::Combat,
            TipClass::SpeedLap,
            TipClass::TimeTrial,
        ] {
            let rows = TIPS.candidates(class);
            assert!(
                rows.iter()
                    .all(|row| TABLE[usize::from(*row)].body.is_some())
            );
            assert_eq!(rows.contains(&32), class == TipClass::Combat);
        }
        assert!(TIPS.candidates(TipClass::Race).contains(&5));
        assert!(TIPS.candidates(TipClass::Zone).contains(&8));
    }
}
