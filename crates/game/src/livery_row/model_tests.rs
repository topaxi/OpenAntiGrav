//! Wipeout HD/Fury's model row: Feisar's seven `PI_TeamModel`s in `DATA00`'s shape, gated by
//! their own `<Unlock>` rows. The medal thresholds are cut to the fixture's two-cell grids; the
//! disc's own 87 and 80 are pinned by the ground truth (`selection_screens_ground_truth`).

use super::*;
use crate::records::Medal;
use crate::unlock::ModelGate;
use oag_tables::race_campaign;

const T: &oag_title::Title = oag_hd::TITLE;
const CAMPAIGN: &str = "fixture";

const XML: &str = r#"<Screen>
<PI_Team name="Feisar"><Values type="Race" location="Data\Ships\Feisar"/>
<PI_TeamModel name="chrome_c1"><Values texturelocation="livery4" LiveryNumber="6" modellocation="Data\Ships\Feisar_c1"/><FE speed="8" thrust="8.5" handling="10" shield="8"/><Unlock locked="true" team="Feisar" MedalCountFURY="2"/></PI_TeamModel>
<PI_TeamModel name="nitro"><Values texturelocation="livery1" LiveryNumber="5" modellocation="Data\Ships\Feisar_n1"/><FE speed="8" thrust="8.5" handling="10" shield="8"/><Unlock grid="grid8"/></PI_TeamModel>
<PI_TeamModel name="concept1"><Values texturelocation="livery1" LiveryNumber="4" modellocation="Data\Ships\Feisar_c1"/><FE speed="8" thrust="8.5" handling="10" shield="8"/></PI_TeamModel>
<PI_TeamModel name="normal"><Values texturelocation="livery1" LiveryNumber="0" modellocation="Data\Ships\Feisar"/><FE speed="7" thrust="8" handling="10" shield="8"/></PI_TeamModel>
<PI_TeamModel name="SKIN1"><Values texturelocation="livery2" LiveryNumber="1" modellocation="Data\Ships\Feisar"/><FE speed="7" thrust="8" handling="10" shield="8"/><Unlock locked="true" team="Feisar" loyalty="4000"/></PI_TeamModel>
<PI_TeamModel name="SKIN2"><Values texturelocation="livery3" LiveryNumber="2" modellocation="Data\Ships\Feisar"/><FE speed="7" thrust="8" handling="10" shield="8"/><Unlock locked="true" team="Feisar" loyalty="8000"/></PI_TeamModel>
<PI_TeamModel name="chrome"><Values texturelocation="livery4" LiveryNumber="3" modellocation="Data\Ships\Feisar"/><FE speed="7" thrust="8" handling="10" shield="8"/><Unlock locked="true" team="Feisar" MedalCountHD="2"/></PI_TeamModel>
<PI_TeamModel name="zone"><Values texturelocation="zoneship_faisar" modellocation="Data\Ships\Zone"/></PI_TeamModel>
</PI_Team></Screen>"#;

fn grid(name: &str, campaign: &str, required: u32) -> race_campaign::Grid {
    let cell = |n: u32| {
        format!(
            r#"<PI_Cell name="{name}_{n}"><Values track="t" mode="Race" class="Venom" Weapons="on" damage="on" AICount="7" skill="1.75" laps="3" ship="None" ShipChoice="Yes"></Values>
<Gold Target="1"></Gold><Silver Target="2"></Silver><Bronze Target="3"></Bronze></PI_Cell>"#
        )
    };
    race_campaign::parse(&format!(
        r#"<PI_Grid name="{name}"><Values RequiredPoints="{required}" Locked="false" Campaign="{campaign}"></Values>{}{}</PI_Grid>"#,
        cell(1),
        cell(2)
    ))
    .expect("the fixture grid parses")
}

fn gate() -> ModelGate {
    ModelGate::on_grids(
        vec![grid("grid0", "HD", 3), grid("grid8", "Fury", 3)],
        CAMPAIGN,
    )
}

fn offered(records: &Store) -> Vec<String> {
    gated_variant_choices(T, "Feisar", &catalogue::teams(XML), records, &gate())
        .into_iter()
        .map(|choice| choice.value)
        .collect()
}

fn golds(records: &mut Store, grid: &str) {
    for n in 1..=2 {
        records.record_campaign(CAMPAIGN, &format!("{grid}_{n}"), Some(Medal::Gold), None);
    }
}

/// A fresh profile offers the two models with no `<Unlock>`: `concept1` (`_c1`) and `normal`
/// (`""`), in the hex column's file order.
#[test]
fn a_fresh_profile_offers_concept1_and_normal() {
    assert_eq!(offered(&Store::default()), ["_c1", ""]);
}

/// Feisar's own loyalty opens `SKIN1` at 4000 and `SKIN2` at 8000, each in its livery folder;
/// another team's total opens nothing.
#[test]
fn feisar_loyalty_opens_skin1_then_skin2() {
    let mut records = Store::default();
    records.record_loyalty(T, "Qirex", 9000);
    assert_eq!(
        offered(&records),
        ["_c1", ""],
        "Qirex's loyalty is not Feisar's"
    );
    let mut records = Store::default();
    records.record_loyalty(T, "Feisar", 4000);
    assert_eq!(offered(&records), ["_c1", "", "@livery2"]);
    records.record_loyalty(T, "Feisar", 4000);
    assert_eq!(offered(&records), ["_c1", "", "@livery2", "@livery3"]);
}

/// Gold medals in the grids whose `Campaign` is `HD` open `chrome`; in `Fury`'s, `chrome_c1`
/// (on Feisar's Fury hull) and, since `grid8`'s own points are met, `nitro`.
#[test]
fn hd_medals_open_chrome_and_fury_medals_chrome_c1() {
    let mut records = Store::default();
    golds(&mut records, "grid0");
    assert_eq!(offered(&records), ["_c1", "", "@livery4"]);
    let mut records = Store::default();
    golds(&mut records, "grid8");
    assert_eq!(offered(&records), ["_c1@livery4", "_n1", "_c1", ""]);
}

/// One silver in `grid8` is two points of its three: `nitro` waits for the grid's own points.
#[test]
fn nitro_waits_for_grid8s_own_points() {
    let mut records = Store::default();
    records.record_campaign(CAMPAIGN, "grid8_1", Some(Medal::Silver), None);
    assert_eq!(offered(&records), ["_c1", ""]);
    records.record_campaign(CAMPAIGN, "grid8_2", Some(Medal::Bronze), None);
    assert_eq!(offered(&records), ["_n1", "_c1", ""]);
}

/// A title that gates no model offers the variant table whole, as before.
#[test]
fn no_gate_offers_the_variant_table() {
    let values: Vec<String> = gated_variant_choices(
        T,
        "Feisar",
        &catalogue::teams(XML),
        &Store::default(),
        &ModelGate::none(),
    )
    .into_iter()
    .map(|choice| choice.value)
    .collect();
    assert_eq!(values, ["", "_c1", "_n1"]);
}

/// Each offered id names the model the hex column draws open: `SKIN1`'s row is `@livery2`'s.
#[test]
fn every_offered_id_opens_its_own_hex_row() {
    let mut records = Store::default();
    records.record_loyalty(T, "Feisar", 4000);
    let teams = catalogue::teams(XML);
    let ids = offered(&records);
    let cells = teams[0].hex_cells(
        oag_title::VariantJoin::Suffix,
        ids.iter().map(String::as_str),
    );
    let open: Vec<(usize, Option<usize>)> = cells
        .iter()
        .enumerate()
        .filter(|(_, cell)| cell.open)
        .map(|(row, cell)| (row, cell.variant))
        .collect();
    assert_eq!(open, [(2, Some(0)), (3, Some(1)), (4, Some(2))]);
}

/// `--unlock-all` opens every model, the zone hull aside (no directory of the variant table).
#[test]
fn unlock_all_opens_all_seven() {
    crate::unlock::set_unlock_all(true);
    let ids = offered(&Store::default());
    crate::unlock::set_unlock_all(false);
    assert_eq!(
        ids,
        [
            "_c1@livery4",
            "_n1",
            "_c1",
            "",
            "@livery2",
            "@livery3",
            "@livery4"
        ]
    );
}
