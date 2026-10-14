use super::*;
use crate::{Context, Readout, draw_list};
use oag_race::Mode;

const HEADER: &str = "[toml-schema]\nlocation = \"https://oag.topaxi.com/mods/schemas/v1/split-hud.tosd\"\nversion = \"1.0.0\"\n";

const HEIGHTS: LineHeights = LineHeights {
    hud: 25.0,
    small: 10.0,
    default: 13.0,
};

fn file(body: &str) -> String {
    format!("{HEADER}{body}")
}

const FOUR: &str = r#"
[[layout]]
name = "four"
views = [3, 4]
aspect = [1.2, 2.2]

[[layout.group]]
widgets = ["Speed*"]
anchor = "bottom-right"
offset = [-0.02, -0.03]
scale = 0.5

[[layout.group]]
widgets = ["Place", "Speed2"]
anchor = "top-left"

[[layout]]
name = "any"
"#;

/// A speed bar bottom right and a place text top left, in a 480x272 grid.
const SAMPLE: &str = r#"
<Screen>
<Image name="SpeedBar" x="300" y="220" width="160" height="30" Src="a.mip" U="0" V="0" TxtrWidth="16" TxtrHeight="3"/>
<Text name="Place"><Values scale="1.0" font="HUD" align="left" vertalign="top" x="10" y="20"/></Text>
<Text name="Speed2"><Values scale="1.0" font="HUD" align="left" vertalign="top" x="400" y="20"/></Text>
</Screen>
"#;

#[test]
fn a_file_parses_and_the_first_matching_rule_answers() {
    let hud = SplitHud::parse(&file(FOUR)).unwrap();
    assert_eq!(hud.rules.len(), 2);
    assert_eq!(hud.rule_for(Mode::SingleRace, 4, 1.5), Some(0));
    assert_eq!(
        hud.rule_for(Mode::SingleRace, 4, 3.5),
        Some(1),
        "outside the range"
    );
    assert_eq!(
        hud.rule_for(Mode::SingleRace, 2, 1.5),
        Some(1),
        "another count"
    );
    assert_eq!(
        hud.rules_for(Mode::SingleRace, 4).collect::<Vec<_>>(),
        [0, 1]
    );
}

#[test]
fn a_rule_can_be_for_one_mode() {
    let hud = SplitHud::parse(&file(
        "[[layout]]\nmodes = [\"zone\"]\n[[layout.group]]\nwidgets = [\"a\"]\nanchor = \"top\"\n",
    ))
    .unwrap();
    assert_eq!(hud.rule_for(Mode::Zone, 2, 1.0), Some(0));
    assert_eq!(hud.rule_for(Mode::SingleRace, 2, 1.0), None);
}

#[test]
fn a_refused_file_names_what_is_wrong() {
    let refused = |text: String| SplitHud::parse(&text).unwrap_err().to_string();
    let major = refused(HEADER.replace("/v1/", "/v9/"));
    assert!(
        major.contains("v9") && major.contains("split-hud"),
        "{major}"
    );
    let kind = refused(HEADER.replace("split-hud", "pilot"));
    assert!(kind.contains("\"pilot\""), "{kind}");
    assert!(refused(file("[[layout]]\nviews = [9]\n")).contains("`views` has 9"));
    assert!(refused(file("[[layout]]\naspect = [2.0, 1.0]\n")).contains("`aspect`"));
    let group = |extra: &str| {
        file(&format!(
            "[[layout]]\nname = \"x\"\n[[layout.group]]\nwidgets = [\"a\"]\nanchor = \"top\"\n{extra}"
        ))
    };
    assert!(refused(group("scale = 0.0\n")).contains("layout x: `scale`"));
    assert!(refused(group("offset = [2.0, 0.0]\n")).contains("`offset`"));
    assert!(
        refused(group("colour = 1\n")).contains("colour"),
        "an unknown key"
    );
    assert!(
        refused(file(
            "[[layout]]\n[[layout.group]]\nwidgets = [\"a\"]\nanchor = \"middle\"\n"
        ))
        .contains("middle")
    );
}

#[test]
fn a_file_without_a_header_still_loads() {
    assert!(SplitHud::parse("[[layout]]\nviews = [2]\n").is_ok());
}

#[test]
fn a_group_keeps_the_widgets_it_claims_and_the_first_claim_wins() {
    let layout = Layout::from_xml(SAMPLE);
    let hud = SplitHud::parse(&file(FOUR)).unwrap();
    let pieces = arrange(&hud.rules[0], &layout, HEIGHTS);
    assert_eq!(pieces.len(), 2);
    assert_eq!(pieces[0].layout.sprites.len(), 1);
    assert_eq!(pieces[0].layout.labels.len(), 1, "Speed2 matches `Speed*`");
    assert_eq!(pieces[0].layout.labels[0].name, "Speed2");
    assert_eq!(
        pieces[1].layout.labels.len(),
        1,
        "Place; Speed2 is already taken"
    );
    assert_eq!(pieces[1].layout.labels[0].name, "Place");
}

#[test]
fn an_unclaimed_widget_is_not_in_any_piece() {
    let layout = Layout::from_xml(SAMPLE);
    let hud = SplitHud::parse(&file(
        "[[layout]]\n[[layout.group]]\nwidgets = [\"Place\"]\nanchor = \"top\"\n",
    ))
    .unwrap();
    let pieces = arrange(&hud.rules[0], &layout, HEIGHTS);
    let drawn: usize = pieces.iter().map(|p| p.layout.widget_count()).sum();
    assert_eq!(drawn, 1);
}

#[test]
fn an_anchor_lands_on_the_same_point_of_the_view() {
    let layout = Layout::from_xml(SAMPLE);
    let hud = SplitHud::parse(&file(
        "[[layout]]\n[[layout.group]]\nwidgets = [\"SpeedBar\"]\nanchor = \"bottom-right\"\noffset = [-0.1, -0.2]\nscale = 0.5\n",
    ))
    .unwrap();
    let pieces = arrange(&hud.rules[0], &layout, HEIGHTS);
    assert_eq!(
        pieces[0].source,
        [460.0, 250.0],
        "the bar's own bottom right"
    );
    // A 240x136 view at (100, 50) in a 480x272 screen: the fit is 0.5, so the
    // group is drawn at 0.25 of its authored size.
    let view = [100.0, 50.0, 240.0, 136.0];
    let placement = pieces[0].placement(view, (480.0, 272.0));
    assert_eq!(placement.scale, 0.25);
    assert_eq!(placement.clip, view);
    let landed = [
        placement.scale * 460.0 + placement.offset[0],
        placement.scale * 250.0 + placement.offset[1],
    ];
    assert_eq!(landed, [100.0 + 240.0 * 0.9, 50.0 + 136.0 * 0.8]);
}

#[test]
fn a_label_contributes_its_text_height_to_the_bounds() {
    let layout = Layout::from_xml(
        "<Screen><Text name=\"T\"><Values scale=\"2.0\" font=\"HUD\" align=\"left\" vertalign=\"bottom\" x=\"10\" y=\"100\"/></Text></Screen>",
    );
    let hud = SplitHud::parse(&file(
        "[[layout]]\n[[layout.group]]\nwidgets = [\"T\"]\nanchor = \"top-left\"\n",
    ))
    .unwrap();
    let pieces = arrange(&hud.rules[0], &layout, HEIGHTS);
    // Bottom-aligned at y=100, 25 * 2 tall: the top is at 50.
    assert_eq!(pieces[0].source, [10.0, 50.0]);
}

/// What the per-group draw rests on: a layout with no widgets draws nothing, whatever
/// the readout holds and whichever title's rules are in force, so a group's frame is its
/// own widgets and not a copy of everything the readout can summon. The reticle is the
/// one exception and is stripped by the caller, which draws it once per view.
#[test]
fn a_layout_with_no_widgets_draws_nothing_from_a_full_readout() {
    let strings = crate::tests::strings();
    let sheet = crate::sprite::Sheet::default();
    let arts: [(&str, &'static oag_title::HudArt); 4] = [
        ("Pulse", oag_pulse::hud::ART),
        ("Pure", oag_pure::hud::ART),
        ("HD", oag_hd::hud::ART),
        ("2048", oag_2048::hud::ART),
    ];
    for (title, art) in arts {
        for mode in [
            Mode::SingleRace,
            Mode::Eliminator,
            Mode::Head2Head,
            Mode::TimeTrial,
            Mode::SpeedLap,
            Mode::Zone,
            Mode::Tournament,
        ] {
            let layout = Layout::default();
            let cx = Context {
                sheet: &sheet,
                art,
                ..crate::tests::context(&layout, &strings)
            };
            let mut readout = Readout {
                speed_kmh: 400.0,
                shield: 50.0,
                shield_max: 100.0,
                lap: 2,
                laps: 3,
                place: 2,
                ships: 8,
                race_ticks: 5000,
                lap_ticks: 1200,
                best_lap_ticks: Some(1100),
                wrong_way: true,
                zone: 4,
                score: 10,
                zone_stage: 1,
                zone_next_in: Some(2),
                pilot_assist: true,
                shield_flashing: true,
                shield_flashing_whole: true,
                shield_absorbing: true,
                shield_blink_phase: 0.5,
                shield_blink_phase_whole: 0.5,
                thrust_chase_percent: 40.0,
                energy_bar_delay_fraction: 0.5,
                pickup: Some(oag_tables::weapons::Weapon::Rocket),
                head_to_head: Some(crate::HeadToHead {
                    gap: 12.0,
                    player_leads: true,
                    ..crate::HeadToHead::default()
                }),
                kill_tags: vec![crate::KillTag {
                    team: "Feisar".into(),
                    kills: 3,
                    player: true,
                }],
                kill_target: 5,
                mode,
                ..Readout::blank()
            };
            readout.assist = crate::assist::AssistReadout {
                enabled: true,
                main: true,
                left: true,
                right: true,
            };
            let frame = draw_list(&cx, &readout);
            assert!(frame.is_empty(), "{title} {mode:?}: {frame:?}");
        }
    }
}

fn assets(origin: Origin, count: usize) -> SplitHudAssets {
    SplitHudAssets::new(
        "x",
        origin,
        SplitHud::parse(&file(FOUR)).unwrap(),
        &Layout::from_xml(SAMPLE),
        HEIGHTS,
        Mode::SingleRace,
        count,
    )
}

fn authored() -> crate::SplitAssets {
    crate::SplitAssets {
        layouts: [Layout::default(), Layout::default()],
        view: (1920.0, 540.0),
        step: (0.0, 540.0),
    }
}

/// A player's view in the layouts' grid sits one step on from the one before: down for the
/// rows, right for the columns.
#[test]
fn a_players_region_is_one_step_on_from_the_last() {
    let rows = authored();
    assert_eq!(rows.region(0), [0.0, 0.0, 1920.0, 540.0]);
    assert_eq!(rows.region(1), [0.0, 540.0, 1920.0, 540.0]);
    let columns = crate::SplitAssets {
        view: (960.0, 1080.0),
        step: (960.0, 0.0),
        ..authored()
    };
    assert_eq!(columns.region(1), [960.0, 0.0, 960.0, 1080.0]);
}

fn is(source: &Source<'_>) -> &'static str {
    match source {
        Source::Authored(_) => "authored",
        Source::File(_, _, Origin::Override) => "override",
        Source::File(_, _, Origin::Fallback) => "fallback",
        Source::Scaled => "scaled",
    }
}

/// The order a mod author is promised: their file, then the disc's own layout,
/// then this project's file, then the whole-screen layout scaled to fit.
#[test]
fn the_chain_is_override_then_authored_then_our_file_then_scaled() {
    let authored = authored();
    let ours = assets(Origin::Fallback, 2);
    let theirs = assets(Origin::Override, 2);
    let pick = |authored, file, count| is(&source(authored, file, Mode::SingleRace, count, 1.5));
    assert_eq!(pick(Some(&authored), Some(&theirs), 2), "override");
    assert_eq!(pick(Some(&authored), Some(&ours), 2), "authored");
    assert_eq!(pick(None, Some(&ours), 2), "fallback");
    assert_eq!(pick(None, None, 2), "scaled");
    assert_eq!(pick(Some(&authored), None, 2), "authored");
}

/// A file answers only where one of its rules does; an override that covers four
/// views leaves two to the disc.
#[test]
fn a_file_answers_only_the_counts_and_shapes_it_covers() {
    let authored = authored();
    let theirs = |count| {
        SplitHudAssets::new(
            "x",
            Origin::Override,
            SplitHud::parse(&file(
                "[[layout]]\nviews = [4]\n[[layout.group]]\nwidgets = [\"Place\"]\nanchor = \"top\"\n",
            ))
            .unwrap(),
            &Layout::from_xml(SAMPLE),
            HEIGHTS,
            Mode::SingleRace,
            count,
        )
    };
    let pick = |authored, count: usize| {
        is(&source(
            authored,
            Some(&theirs(count)),
            Mode::SingleRace,
            count,
            1.5,
        ))
    };
    assert_eq!(pick(Some(&authored), 4), "override");
    assert_eq!(
        pick(Some(&authored), 2),
        "authored",
        "the override has no layout for two"
    );
    assert_eq!(pick(None, 2), "scaled");
}

/// Only the views a rule is for are arranged, so a race pays for its own count.
#[test]
fn only_the_rules_for_this_races_count_are_cut() {
    let two = assets(Origin::Fallback, 2);
    assert!(two.arranged[0].is_none(), "the 3-4 view rule");
    assert!(two.arranged[1].is_some(), "the rule for any count");
    let four = assets(Origin::Fallback, 4);
    assert!(four.arranged[0].is_some());
}

#[test]
fn a_quarter_turned_sprite_is_bounded_where_it_lies() {
    // 676 wide and 153 tall, turned a quarter: a column 153 wide and 676 tall about
    // the same centre (140, 580).
    let mut layout = Layout::from_xml(
        "<Screen><Image name=\"Bar\" x=\"-198\" y=\"503.5\" width=\"676\" height=\"153\" Src=\"a.mip\" U=\"0\" V=\"0\" TxtrWidth=\"1\" TxtrHeight=\"1\"/></Screen>",
    );
    layout.sprites[0].rotation = std::f32::consts::FRAC_PI_2;
    let hud = SplitHud::parse(&file(
        "[[layout]]\n[[layout.group]]\nwidgets = [\"Bar\"]\nanchor = \"top-left\"\n",
    ))
    .unwrap();
    let pieces = arrange(&hud.rules[0], &layout, HEIGHTS);
    assert_eq!(pieces[0].source, [140.0 - 76.5, 580.0 - 338.0]);
    // A small tilt does not move the box.
    layout.sprites[0].rotation = 0.385;
    let pieces = arrange(&hud.rules[0], &layout, HEIGHTS);
    assert_eq!(pieces[0].source, [-198.0, 503.5]);
}
