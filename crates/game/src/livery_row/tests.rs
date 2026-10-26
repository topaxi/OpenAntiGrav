//! The livery row at the three loyalty points that matter and in the two forced modes.

use super::*;
use oag_race::Mode;

const T: &oag_title::Title = oag_pulse::TITLE;

/// Pulse's definition shape: `Assegai` declares skins under `Normal` and a `Concept` hull;
/// `Plain` declares a `Concept` hull and no skin at all (a pack's team).
const XML: &str = r#"<Plugin>
<PI_Team name="Assegai"><Values type="Race" location="Data\Ships\Assegai"></Values>
<PI_TeamModel name="Normal"><Values location="ship"></Values>
<PI_ModelSkin name="Alternative"><Values location="Data\Ships\Assegai\ship_alt.dat"></Values>
<Unlock Team="Assegai" loyalty="4000" Exclusive="true"></Unlock>
<Unlock Team="any" loyalty="60000" Exclusive="true"></Unlock></PI_ModelSkin>
<PI_ModelSkin name="Eliminator"><Values location="Data\Ships\Assegai\ship_eliminator.dat"></Values>
<Unlock Team="Assegai" loyalty="8000" Exclusive="true"></Unlock></PI_ModelSkin>
</PI_TeamModel>
<PI_TeamModel name="Concept"><Values location="extra"></Values>
<Unlock Team="Assegai" loyalty="25000" Exclusive="true"></Unlock></PI_TeamModel>
</PI_Team>
<PI_Team name="Plain"><Values type="Race" location="Data\Ships\Plain"></Values>
<PI_TeamModel name="Normal"><Values location="ship"></Values></PI_TeamModel>
<PI_TeamModel name="Concept"><Values location="extra"></Values>
<Unlock Team="Plain" loyalty="25000" Exclusive="true"></Unlock></PI_TeamModel>
</PI_Team></Plugin>"#;

fn teams() -> Vec<Team> {
    catalogue::teams(XML)
}

fn row(team: &str, loyalty: u32, mode: Mode) -> Livery {
    let mut records = Store::default();
    records.record_loyalty(T, team, loyalty);
    let teams = teams();
    ship_livery(
        T,
        team,
        teams.iter().find(|candidate| candidate.id == team),
        &oag_ui::language::StringTable::default(),
        &records,
        mode,
    )
}

fn ids(livery: &Livery) -> Vec<&str> {
    livery.variants.iter().map(|(id, _)| id.as_str()).collect()
}

/// **The reported bug.** A fresh profile has every skin locked; the row used to read that as "no
/// skins" and switch to the variant table, offering a locked Concept hull.
#[test]
fn a_fresh_profile_offers_classic_alone_and_never_the_locked_concept_hull() {
    let livery = row("Assegai", 0, Mode::TimeTrial);
    assert!(livery.skinned);
    assert_eq!(ids(&livery), [""], "Classic only");
}

/// Alternative's own-team price is 4,000: the maintainer's repro, where Concept used to vanish.
#[test]
fn the_first_unlocked_skin_joins_classic_and_nothing_else_appears() {
    let livery = row("Assegai", 4000, Mode::TimeTrial);
    assert_eq!(ids(&livery), ["", "Alternative"]);
}

/// A team on the variant table offers its Concept hull only once the hull's own rows pass.
#[test]
fn a_team_with_no_skins_hides_concept_until_it_is_unlocked_then_offers_both() {
    for (loyalty, expected) in [
        (0, vec!["Ship"]),
        (4000, vec!["Ship"]),
        (25_000, vec!["Ship", "extra"]),
    ] {
        let livery = row("Plain", loyalty, Mode::TimeTrial);
        assert!(!livery.skinned);
        assert_eq!(ids(&livery), expected, "loyalty {loyalty}");
    }
}

#[test]
fn an_eliminator_row_holds_the_eliminator_skin_alone_with_nothing_unlocked() {
    let livery = row("Assegai", 0, Mode::Eliminator);
    assert_eq!(ids(&livery), ["Eliminator"]);
    assert_eq!(livery.skins.len(), 1);
    assert!(livery.skins[0].1.ends_with("ship_eliminator.dat"));
}

#[test]
fn a_zone_row_holds_the_zone_hull_alone() {
    for team in ["Assegai", "Plain"] {
        let livery = row(team, 100_000, Mode::Zone);
        assert_eq!(livery.variants.len(), 1, "{team}");
        assert!(livery.skins.is_empty(), "{team}");
    }
}

/// The RACE page's VARIANT row goes through the same gate.
#[test]
fn the_variant_row_is_cut_by_the_hull_gate() {
    let teams = teams();
    let mut records = Store::default();
    assert_eq!(gated_variant_choices(T, "Plain", &teams, &records).len(), 1);
    records.record_loyalty(T, "Plain", 25_000);
    assert_eq!(gated_variant_choices(T, "Plain", &teams, &records).len(), 2);
}
