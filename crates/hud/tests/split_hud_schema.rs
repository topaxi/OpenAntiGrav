//! `schemas/v1/split-hud.tosd` and `SplitHud::parse` must give the same verdict on the
//! shape of a split-HUD file, so neither can drift from the other unseen.
//!
//! What the schema does not carry is the loader's own range checks (`views` in 1 to 8,
//! `scale` in (0, 8], an `offset` inside one view, an increasing `aspect`): a file past
//! one is refused by the loader alone, and every example below is inside them.

use oag_hud::split_hud::{Anchor, SplitHud};
use oag_race::Mode;

const HEADER: &str = "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/split-hud.tosd\", version = \"1.0.0\" }\n";

const GROUP: &str = "[[layout]]\n[[layout.group]]\nwidgets = [\"Lap*\"]\nanchor = \"top-left\"\n";

fn cases() -> Vec<(&'static str, String, bool)> {
    let with = |body: &str| format!("{HEADER}{body}");
    let group_with = |extra: &str| with(&format!("{GROUP}{extra}"));
    vec![
        ("header only", HEADER.to_owned(), true),
        ("empty file", String::new(), true),
        ("one group", with(GROUP), true),
        (
            "every key",
            with("[[layout]]\nname = \"x\"\nviews = [2, 3]\naspect = [1.0, 2.5]\nmodes = [\"zone\"]\n[[layout.group]]\nwidgets = [\"a\", \"b*\"]\nanchor = \"bottom-right\"\noffset = [-0.1, 0.05]\nscale = 1.5\n"),
            true,
        ),
        ("integers in a pair", group_with("offset = [0, 0]\nscale = 1\n"), true),
        ("aspect in integers", with("[[layout]]\naspect = [1, 3]\n"), true),
        ("local schema path", with(GROUP).replace("https://oag.topaxi.com/mods/schemas", "file:///x/schemas"), true),
        ("unknown top key", with("groups = 1\n"), false),
        ("unknown group key", group_with("colour = 1\n"), false),
        ("unknown layout key", with("[[layout]]\nzoom = 2\n"), false),
        ("anchor not a point", with(&GROUP.replace("top-left", "middle")), false),
        ("no anchor", with("[[layout]]\n[[layout.group]]\nwidgets = [\"a\"]\n"), false),
        ("no widgets", with("[[layout]]\n[[layout.group]]\nanchor = \"top\"\n"), false),
        ("empty widgets", with("[[layout]]\n[[layout.group]]\nwidgets = []\nanchor = \"top\"\n"), false),
        ("offset of three", group_with("offset = [0.0, 0.0, 0.0]\n"), false),
        ("scale a string", group_with("scale = \"big\"\n"), false),
        ("views as strings", with("[[layout]]\nviews = [\"2\"]\n"), false),
        ("another major", with(GROUP).replace("/v1/", "/v2/"), false),
        ("another kind", with(GROUP).replace("split-hud.tosd", "pilot.tosd"), false),
        ("no version", "toml-schema = { location = \"https://oag.topaxi.com/mods/schemas/v1/split-hud.tosd\" }\n".to_owned(), false),
    ]
}

#[test]
fn the_schema_and_the_loader_agree_on_every_example() {
    for (what, text, accepted) in cases() {
        let schema = oag_schema::validate::accepts("split-hud", 1, &text);
        let loader = SplitHud::parse(&text);
        assert_eq!(schema.is_ok(), accepted, "schema, {what}: {schema:?}");
        assert_eq!(loader.is_ok(), accepted, "loader, {what}: {loader:?}");
    }
}

#[test]
fn every_shipped_file_is_valid_and_loads() {
    for title in ["Pulse", "Pure", "HD", "2048"] {
        let text = std::fs::read_to_string(format!(
            "{}/../../assets/fallback/Data/XML/SplitHud_{title}.toml",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        oag_schema::validate::accepts("split-hud", 1, &text)
            .unwrap_or_else(|e| panic!("{title}: {e:?}"));
        let file = SplitHud::parse(&text).unwrap_or_else(|e| panic!("{title}: {e}"));
        assert!(!file.rules.is_empty(), "{title}");
    }
}

#[test]
fn a_refused_file_names_what_it_found() {
    for (what, text, expect) in [
        ("major", with_header("/v7/"), "v7"),
        ("kind", HEADER.replace("split-hud", "track"), "\"track\""),
    ] {
        let error = SplitHud::parse(&text).unwrap_err().to_string();
        assert!(error.contains(expect), "{what}: {error}");
    }
}

fn with_header(version: &str) -> String {
    HEADER.replace("/v1/", version)
}

fn shipped(title: &str) -> SplitHud {
    let text = std::fs::read_to_string(format!(
        "{}/../../assets/fallback/Data/XML/SplitHud_{title}.toml",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    SplitHud::parse(&text).unwrap_or_else(|e| panic!("{title}: {e}"))
}

fn answer(file: &SplitHud, mode: Mode, views: usize, aspect: f32) -> &str {
    let at = file
        .rule_for(mode, views, aspect)
        .unwrap_or_else(|| panic!("no rule for {views} views at {aspect}"));
    &file.rules[at].name
}

/// A two-view race cut left and right is two tall columns, and Pulse, Pure and 2048 draw
/// them with their own `tall` rule: the column HUD of this project, not the lower rows of
/// eight views' `portrait`, which shares the shape and must stay as it is. Nothing a
/// horizontal cut or another count produces reaches it.
#[test]
fn a_vertical_two_view_split_reads_the_tall_rule_and_no_other_view_does() {
    for title in ["Pulse", "Pure", "2048"] {
        let file = shipped(title);
        for mode in [
            Mode::SingleRace,
            Mode::Zone,
            Mode::Eliminator,
            Mode::TimeTrial,
        ] {
            // 16:9 columns (0.89), 21:9 (1.19) and 32:9 (1.78): a wider screen's columns.
            for aspect in [0.89, 1.19, 1.78] {
                assert_eq!(
                    answer(&file, mode, 2, aspect),
                    "tall",
                    "{title} {mode:?} {aspect}"
                );
            }
            // Rows: a 4:3 screen's are 2.67 wide, 16:9's 3.56.
            for aspect in [2.67, 3.56] {
                assert_eq!(
                    answer(&file, mode, 2, aspect),
                    "strip",
                    "{title} {mode:?} {aspect}"
                );
            }
            // The lower rows of seven and eight views, and every other count.
            for views in [7, 8] {
                assert_eq!(
                    answer(&file, mode, views, 0.8),
                    "portrait",
                    "{title} {views}"
                );
            }
            assert_eq!(answer(&file, mode, 4, 1.78), "landscape", "{title}");
            assert_eq!(answer(&file, mode, 3, 3.5), "strip", "{title}");
        }
    }
}

/// HD draws the disc's own `vert_` layouts for two views, so its file has no rule of the
/// kind: its other modes' tall views stay on `portrait` by shape, as they did.
#[test]
fn hd_has_no_tall_rule_for_its_authored_two_view_split_to_lose_to() {
    let file = shipped("HD");
    assert!(file.rules.iter().all(|rule| rule.name != "tall"));
    assert_eq!(answer(&file, Mode::TimeTrial, 2, 0.89), "portrait");
}

/// Pure authors its weapon grid as a model, which the loader pins by its centre point
/// alone, so an offset smaller than half the grid's height hangs the rest below the view
/// and the scissor cuts it. Every rule raises it by more than that: the grid is about 39
/// units tall on the 272-unit screen (read off a render, 67 pixels at scale 1.3 in a
/// 358-pixel row), so a view of width over height `a` shows it `39 * scale * min(a / 1.765,
/// one) / 272` of its height. `a` is the widest view the rule answers, except for `tall`,
/// which answers up to 2.4 and is checked only to a 21:9 screen's column (1.19): a 32:9
/// column (1.78) would still clip the grid, and that is listed as open in
/// `docs/rendering/split-screen.md`, not fixed here.
#[test]
fn pures_every_rule_raises_the_weapon_grid_clear_of_the_bottom_edge() {
    let file = shipped("Pure");
    for (name, widest) in [
        ("tall", 1.19_f32),
        ("strip", 99.0),
        ("landscape", 2.4),
        ("square", 1.4),
        ("portrait", 1.0),
    ] {
        let rule = file.rules.iter().find(|rule| rule.name == name).unwrap();
        let grid = rule
            .groups
            .iter()
            .find(|group| group.widgets.iter().any(|w| w == "weapon_icon_grid"))
            .unwrap();
        let half = 0.5 * 39.0 * grid.scale * (widest / 1.765).min(1.0) / 272.0;
        assert!(
            -grid.offset[1] >= half,
            "{name}: offset {} leaves a grid of half-height {half} below the view",
            grid.offset[1]
        );
    }
}

/// The kill column is the Eliminator's, and a column has the room for it.
#[test]
fn pulses_tall_rule_places_the_eliminators_kill_column() {
    let file = shipped("Pulse");
    let at = file.rule_for(Mode::Eliminator, 2, 0.89).unwrap();
    assert!(
        file.rules[at]
            .groups
            .iter()
            .any(|group| group.widgets.iter().any(|w| w == "PosTag*"))
    );
}

/// A row is too short for eight kill lines and the speed and shield bars both: the column
/// hangs from the top corner at a scale that ends it above the bars' top. Both fractions
/// are read off renders at a height-bound view: the column (title and eight rows) is
/// 0.608 of the view's height at scale 1.0, and the bars with their labels 0.2 at scale
/// 1.0.
#[test]
fn pulses_kill_column_stops_above_the_bars_in_a_row() {
    let file = shipped("Pulse");
    for name in ["strip", "landscape"] {
        let rule = file.rules.iter().find(|rule| rule.name == name).unwrap();
        let group_of = |widget: &str| {
            rule.groups
                .iter()
                .find(|group| group.widgets.iter().any(|w| w == widget))
                .unwrap()
        };
        let kills = group_of("KillsText");
        assert_eq!(kills.anchor, Anchor::TopRight, "{name}");
        let bars = group_of("SpeedBar*");
        let column_end = kills.offset[1] + 0.608 * kills.scale;
        let bars_start = 1.0 + bars.offset[1] - 0.2 * bars.scale;
        assert!(
            column_end < bars_start,
            "{name}: the column ends at {column_end}, the bars start at {bars_start}"
        );
    }
}
