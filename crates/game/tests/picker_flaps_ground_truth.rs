//! Pulse's Ship Select seats each craft's airbrake flaps where the race hull has them at rest.
//!
//! `#[ignore]`d: needs a Pulse disc image under `data/images/`. `just test-data`.
//!
//! A `ship_FE.vex` hangs `AirBrake_Left`/`AirBrake_Right` straight under its turntable
//! `Anim Transform`, with no locator above them, so the flap vertices are in hinge space and drew
//! at the model origin, in the middle of the hull. The race hull (`Ship.vex`, `Zone.vex`) carries
//! the locators. Every flap vertex of the preview model, loaded through the loader both the live
//! screen and the capture call (`preview::variant::model_or_default`), must land on a vertex of
//! the race model's flaps; every team the shell offers (twelve with the four DLC packs mounted), in a time trial and in Zone.

use oag_core::math::Vec3;
use oag_mesh::mesh::{self, Model};

const TOLERANCE: f32 = 5e-3;

fn flap_points(model: &Model) -> Vec<Vec3> {
    model
        .airbrakes
        .iter()
        .flatten()
        .flat_map(|flap| {
            model.vertices[flap.vertices.start as usize..flap.vertices.end as usize]
                .iter()
                .map(|v| Vec3::from_array(v.position))
        })
        .collect()
}

/// The furthest any preview flap vertex is from the race model's flap vertices, and how many it
/// judged.
fn worst_gap(preview: &Model, race: &Model) -> (f32, usize) {
    let race = flap_points(race);
    let own = flap_points(preview);
    let worst = own
        .iter()
        .map(|p| {
            race.iter()
                .map(|q| (*p - *q).length())
                .fold(f32::MAX, f32::min)
        })
        .fold(0.0, f32::max);
    (worst, own.len())
}

fn check(image: &str) {
    let Some(image) = oag_testdata::image(image) else {
        return;
    };
    let title = oag_pulse::TITLE;
    let packs = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/dlc");
    let dlc: Vec<std::path::PathBuf> = packs.exists().then_some(packs).into_iter().collect();
    let options = oag_game::boot::Options {
        source: image.to_string_lossy().into_owned(),
        language: None,
        dlc: dlc.clone(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-picker-flaps-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(1),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    let teams = oag_game::boot::load_shell(&options)
        .expect("the shell loads")
        .0
        .teams;
    let mut archives = oag_game::preview::open_archives(&image.to_string_lossy(), &dlc)
        .expect("the archives open");
    // Eight on the disc, four more when the Auricom, Harimau, Icaras and Mirage packs are mounted.
    assert!(teams.len() >= 8, "Pulse's roster reads: {}", teams.len());
    println!("{} team(s) previewed", teams.len());

    let mut judged = 0;
    let mut bad = Vec::new();
    for mode in [oag_race::Mode::TimeTrial, oag_race::Mode::Zone] {
        for team in &teams {
            let entry = oag_game::preview::ship_entry(title, &team.location, mode);
            let race_hull = oag_game::preview::race_hull_entry(title, &team.id, mode)
                .unwrap_or_else(|| panic!("{}: Pulse previews a front-end twin", team.id));
            let (preview, _) = oag_game::preview::variant::model_or_default(
                &mut archives,
                &entry,
                None,
                Some(&race_hull),
            )
            .unwrap_or_else(|e| panic!("{entry}: {e:#}"));
            let race_blob = archives.read_name(&race_hull).expect(&race_hull);
            let race = mesh::build(&race_hull, &race_blob).expect(&race_hull);
            assert!(
                race.airbrakes.iter().all(Option::is_some),
                "{race_hull}: the race hull's two flaps are found (PS2's Triakis names them \
                 `andy_ship:AirBrake_left`)"
            );
            assert!(
                preview.airbrakes.iter().all(Option::is_some),
                "{entry}: two flaps"
            );
            let (worst, points) = worst_gap(&preview, &race);
            assert!(points > 0, "{entry}: no flap vertices");
            if worst >= TOLERANCE {
                bad.push(format!(
                    "{} {mode:?}: a flap vertex of {entry} is {worst} from the race hull's {race_hull}",
                    team.id
                ));
            }
            judged += 1;
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
    assert_eq!(judged, teams.len() * 2);
}

#[test]
#[ignore = "needs a Pulse disc image in data/images/"]
fn every_psp_craft_previews_its_flaps_where_the_race_hull_has_them() {
    check("data/images/pulse-psp-eu.chd");
}

/// The PS2 pressing ships its own `ship_FE.vex` set: the same claim on its files.
#[test]
#[ignore = "needs a Pulse disc image in data/images/"]
fn every_ps2_craft_previews_its_flaps_where_the_race_hull_has_them() {
    check("data/images/pulse-ps2-eu.chd");
}

/// The wiring is what the test above stands on: left unseated, the same model is far off.
#[test]
#[ignore = "needs a Pulse disc image in data/images/"]
fn an_unseated_front_end_flap_is_far_from_the_race_hull() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&image.to_string_lossy(), oag_pulse::TITLE)
        .expect("the archives open");
    let (preview, _) = oag_game::preview::variant::model_or_default(
        &mut archives,
        r"Data\Ships\Qirex\ship_FE.vex",
        None,
        None,
    )
    .expect("the preview hull");
    let blob = archives.read_name(r"Data\Ships\Qirex\Ship.vex").unwrap();
    let race = mesh::build("Ship.vex", &blob).unwrap();
    let (worst, _) = worst_gap(&preview, &race);
    assert!(worst > 5.0, "Qirex's unseated flaps are only {worst} off");
}
