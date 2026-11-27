//! The Ship Select preview of a race flying the mode's own craft wears what the race's hull build
//! puts on it: the one Zone (or Detonator) hull, in the picked team's livery
//! (`docs/ghidra/functions/ps3-hdfury-eu/zone-craft.md`, "As wired").
//!
//! Each case builds the hull twice for every team the definition gives a Zone livery: once as the
//! race does (`oag_livery::load`, given the rows the race load reads) and once as the picker's
//! preview does (`oag_game::preview::own_craft_entry` and `variant::own_craft_folders` over the
//! front end's own catalogue), and requires the same entry, the same livery folder and the same
//! texels. A preview that stopped applying the livery draws the hull's default skin and fails the
//! texel comparison; a race that stopped would fail it from the other side.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'test(/^zone_preview_ground_truth::/)'
//! ```

use oag_game::preview::variant::own_craft_folders;
use oag_mesh::mesh::{Model, Texels};
use oag_race::Mode;
use std::hash::{Hash, Hasher};

fn texel_fingerprint(model: &Model) -> Vec<(u32, u32, u64)> {
    model
        .textures
        .iter()
        .map(|slot| match slot {
            None => (0, 0, 0),
            Some(texture) => {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                match &texture.texels {
                    Texels::Rgba8(bytes) => bytes.hash(&mut hasher),
                    Texels::Chain(levels) => levels.hash(&mut hasher),
                    // The bytes hashed as bytes: their `Debug` text was most of the test's CPU.
                    Texels::Blocks { format, levels } => {
                        format!("{format:?}").hash(&mut hasher);
                        levels.hash(&mut hasher);
                    }
                    other => format!("{other:?}").hash(&mut hasher),
                }
                (texture.width, texture.height, hasher.finish())
            }
        })
        .collect()
}

/// The race's hull for `team`, built from the rows `liveries`.
fn race_hull(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    liveries: &[(String, String)],
    team: &str,
    mode: Mode,
) -> (Model, String) {
    let mut report = Vec::new();
    let loaded = oag_livery::load(
        archives,
        &[team.to_string()],
        &oag_livery::LoadContext {
            store: Default::default(),
            race: title.race,
            mode,
            flare: title.flare,
            hull_overlay: false,
            leach_overlay: false,
            hull_shine: false,
            hull_wreck: false,
            absorb_shell: false,
            own_shield: false,
            zone_liveries: liveries,
            forced_skins: &[],
            circuit_hull: None,
            player_livery: None,
        },
        None,
        None,
        &[],
        &mut report,
    )
    .unwrap_or_else(|e| panic!("{team}: the {mode:?} hull loads: {e:#}"));
    let hull = loaded.into_iter().next().expect("one craft").hull;
    (hull, report.join("\n"))
}

/// Every team with a Zone livery wears it in the preview exactly as in the race. Returns how many
/// teams that was. `has_selector` is whether the title's front end draws a ship preview: 2048's
/// authors no Ship Select that this build reads (`docs/formats/2048-frontend.md`), so there the
/// shared decision is pinned and the picker's entry rule is not asked. `only` limits the teams
/// (Omega's 8192 px textures take seconds a build); `default_skin_of` is the team whose livery is
/// byte-identical to the hull's default skin, so it is the one allowed to match it.
fn check(
    name: &str,
    title: &'static oag_title::Title,
    archives: &mut oag_assets::Archives,
    mode: Mode,
    (has_selector, only, default_skin_of): (bool, Option<&[&str]>, Option<&str>),
) -> usize {
    let blob = archives
        .read_name(title.plugin_definition)
        .expect("teams definition");
    let xml = oag_tables::fexml::text(&blob).expect("the definition reads");
    let catalogue = oag_raceplay::catalogue::teams(&xml);
    let liveries: Vec<(String, String)> = catalogue
        .iter()
        .flat_map(|team| team.craft_liveries(title.race, mode))
        .collect();
    let craft = oag_livery::entry::own_craft(title.race.ships(), mode).expect("an own craft");
    let key = craft.livery_key().expect("a livery key");
    let word = oag_livery::entry::craft_label(mode);
    let mut checked = 0;
    let mut default_texels = None;
    for team in &catalogue {
        let rows = team.craft_liveries(title.race, mode);
        let Some((_, authored)) = rows.first() else {
            continue;
        };
        let authored = authored.as_str();
        if !authored.starts_with("zoneship_")
            || only.is_some_and(|only| !only.iter().any(|id| id.eq_ignore_ascii_case(&team.id)))
        {
            continue;
        }
        let label = format!("{name}/{}", team.id);
        let ships = title.race.ships_for(&team.id);
        let craft = oag_livery::entry::own_craft(ships, mode).expect("an own craft");
        let entry = ships.hull_entry_name(&team.id, Some(craft), None);
        if has_selector {
            assert_eq!(
                oag_game::preview::own_craft_entry(title, &team.id, mode),
                Some(entry.clone()),
                "{label}: the preview draws the hull the race builds"
            );
        }
        let folders = own_craft_folders(title, team, mode)
            .unwrap_or_else(|| panic!("{label}: a {word} livery folder"));
        assert_eq!(folders.own, key, "{label}");
        assert_eq!(folders.worn, authored, "{label}: the definition's own name");
        assert!(
            folders.fall_back,
            "{label}: the default skin backs the livery"
        );
        assert_eq!(
            folders.folder().rewrite(&format!("art/{key}/Team.tex")),
            Some(format!("art/{authored}/team.tex").to_ascii_lowercase()),
            "{label}"
        );

        let (race, report) = race_hull(archives, title, &liveries, &team.id, mode);
        assert!(
            report.contains(&format!(
                "{}: {word} livery {authored} replaces {key} on ",
                team.id
            )) && !report.contains("does not hold"),
            "{label}: {report}"
        );
        let preview =
            oag_game::preview::psp2_scene::model_in(archives, &entry, Some(folders.folder()))
                .unwrap_or_else(|e| panic!("{label}: {entry}: {e:#}"));
        let default = default_texels.get_or_insert_with(|| {
            let model = oag_game::preview::psp2_scene::model_in(archives, &entry, None)
                .unwrap_or_else(|e| panic!("{name}: {entry}: {e:#}"));
            texel_fingerprint(&model)
        });
        let (preview_texels, race_texels) = (texel_fingerprint(&preview), texel_fingerprint(&race));
        assert!(!race_texels.is_empty(), "{label}: the hull has textures");
        assert_eq!(
            preview_texels, race_texels,
            "{label}: the preview wears the race's livery"
        );
        if default_skin_of.is_some_and(|id| id.eq_ignore_ascii_case(&team.id)) {
            assert_eq!(
                &preview_texels, default,
                "{label}: its livery is the default skin"
            );
        } else {
            assert_ne!(
                &preview_texels, default,
                "{label}: the livery is not the default skin"
            );
        }

        assert_eq!(
            own_craft_folders(title, team, Mode::TimeTrial),
            None,
            "{label}"
        );
        assert_eq!(
            oag_game::preview::own_craft_entry(title, &team.id, Mode::TimeTrial),
            None,
            "{label}"
        );
        checked += 1;
    }
    checked
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_zone_preview_wears_the_livery_the_race_does() {
    let Some(path) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut archives =
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("opening HD/Fury")
            .archives;
    for mode in [Mode::Zone, Mode::Detonator] {
        let checked = check("HD", oag_hd::TITLE, &mut archives, mode, (true, None, None));
        assert_eq!(
            checked, 12,
            "every team the definition gives a {mode:?} livery"
        );
    }
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn vita_2048_zone_preview_wears_the_livery_the_race_does() {
    let Some(path) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let mut archives = oag_2048::open(&path.display().to_string()).expect("opening 2048");
    for mode in [Mode::Zone, Mode::Detonator] {
        let checked = check(
            "2048",
            oag_2048::TITLE,
            &mut archives,
            mode,
            (false, None, None),
        );
        assert_eq!(checked, 12, "{mode:?}");
    }
}

#[test]
#[ignore = "needs the corrected Omega extraction"]
fn omega_zone_preview_wears_the_livery_the_race_does() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let mut archives = oag_omega::open(&path.display().to_string()).expect("opening Omega");
    // Auricom's `Team.gnf` is byte-identical to the default folder's `zoneship_team` on the Zone
    // hull; the Detonator hull's default matches no team's of these four.
    let only = ["Qirex", "Auricom", "AG_Systems", "Assegai"];
    for mode in [Mode::Zone, Mode::Detonator] {
        let checked = check(
            "Omega",
            oag_omega::TITLE,
            &mut archives,
            mode,
            (true, Some(&only), (mode == Mode::Zone).then_some("Auricom")),
        );
        assert_eq!(checked, only.len(), "{mode:?}");
    }
}
