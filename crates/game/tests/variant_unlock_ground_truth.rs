//! Pulse's craft-variant unlock, against the disc that authors it.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test variant_unlock_ground_truth --run-ignored all
//! ```
//!
//! Every price is read off the disc's own `<Unlock>` rows; none is written down here. The
//! laws under test are `Definition_IsUnlocked`'s: `Exclusive` rows are alternatives, and a
//! `Team="any"` row is met by the best single team, not by the teams' totals added up.

use oag_game::catalogue::{self, LoyaltyRow, Team};
use oag_game::records::Store;
use oag_game::unlock::loyalty_unlocked;

const TITLE: &str = "Wipeout Pulse";

fn teams() -> Option<Vec<Team>> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let mut opened =
        oag_game::title::open_source(&image.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    let blob = opened
        .archives
        .read_name(r"Data\Plugins\PI001\Definition.xml")
        .expect("the definition");
    let xml = oag_tables::fexml::text(&blob).expect("the definition expands");
    Some(catalogue::teams(&xml))
}

fn price<'a>(rows: &'a [LoyaltyRow], any: bool) -> &'a LoyaltyRow {
    rows.iter()
        .find(|row| row.team.eq_ignore_ascii_case("any") == any)
        .expect("each variant authors an own-team and an any row")
}

fn skin_rows<'a>(team: &'a Team, name: &str) -> &'a [LoyaltyRow] {
    &team.skin(name).expect("the skin").unlock
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_fresh_profile_offers_no_skin_and_no_concept_hull() {
    let Some(teams) = teams() else { return };
    assert!(!teams.is_empty());
    let fresh = Store::default();
    for team in &teams {
        assert!(!team.skins.is_empty(), "{} authors skins", team.id);
        for skin in &team.skins {
            assert!(!skin.unlock.is_empty());
            assert!(
                !loyalty_unlocked(&skin.unlock, &fresh, TITLE),
                "{} {}",
                team.id,
                skin.name
            );
        }
        for (stem, rows) in &team.hull_unlocks {
            if !rows.is_empty() {
                assert!(!loyalty_unlocked(rows, &fresh, TITLE), "{} {stem}", team.id);
            }
        }
    }
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_team_unlocks_its_own_skin_at_its_own_price_and_no_other_teams() {
    let Some(teams) = teams() else { return };
    let (a, b) = (&teams[0], &teams[1]);
    let rows = skin_rows(a, "Alternative");
    let own = price(rows, false);
    let mut store = Store::default();
    store.record_loyalty(TITLE, &a.id, own.loyalty - 1);
    assert!(!loyalty_unlocked(rows, &store, TITLE));
    store.record_loyalty(TITLE, &a.id, 1);
    assert!(loyalty_unlocked(rows, &store, TITLE));
    assert!(
        !loyalty_unlocked(skin_rows(b, "Alternative"), &store, TITLE),
        "{} earned it, {} did not",
        a.id,
        b.id
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn any_is_the_best_single_team_and_not_the_sum() {
    let Some(teams) = teams() else { return };
    let (a, b) = (&teams[0], &teams[1]);
    let rows = skin_rows(b, "Alternative");
    let any = price(rows, true).loyalty;

    let mut split = Store::default();
    split.record_loyalty(TITLE, &a.id, any / 2 + 1);
    split.record_loyalty(TITLE, &teams[2].id, any / 2 + 1);
    assert!(
        !loyalty_unlocked(rows, &split, TITLE),
        "two teams halfway there are not one team at the price"
    );

    let mut one = Store::default();
    one.record_loyalty(TITLE, &a.id, any);
    assert!(
        loyalty_unlocked(rows, &one, TITLE),
        "{} at the any price opens {}'s skin",
        a.id,
        b.id
    );
}
