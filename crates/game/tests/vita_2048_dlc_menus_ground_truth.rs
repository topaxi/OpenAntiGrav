//! Wipeout 2048's downloadable campaigns on the front end, against the real
//! packages: which tiles of the mode grid open a map, which cells launch and
//! onto which circuit, and that a source without the packages is untouched.
//!
//! **`#[ignore]`d and never run in CI.** It needs the decrypted EU and USA
//! packages under `data/extracted/vita/` (`PCSF00007`: base, patch, `dlc1`,
//! `dlc2`; `PCSA00015`: base, patch, `dlc1`). Run with `just test-data`. The
//! screen's own behaviour on a fixture is `oag-ui`'s
//! `frontend::tests::campaign_tiers`.
//!
//! The two campaigns are Wipeout HD's own `PI_Grid`s (`grid_00`..`grid_15`),
//! `grid0`..`grid7` tagged `Campaign="HD"` and the rest `"Fury"`; the package
//! that ships a campaign's circuits is the gate, so the base package alone
//! offers neither. `docs/formats/2048-dlc.md` has the census.

use std::path::{Path, PathBuf};

use oag_2048::campaign::tier;
use oag_2048::frontend::states as w2048;
use oag_game::boot;
use oag_game::campaign::launch::{self, CellPlan, Refusal};
use oag_gameplay::input::{Button, Input};
use oag_tables::race_campaign::{Cell, Grid};
use oag_ui::frontend::{self, Launch};

fn package(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita")
        .join(name);
    if path.join("base/PSP2/data.psarc").exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// The EU package with its two downloadable archives left out: real folders
/// holding links to the base archive and the patch's two, because the source
/// walk does not follow a linked folder.
struct WithoutPacks(PathBuf);

impl WithoutPacks {
    fn of(package: &Path) -> Self {
        let root = std::env::temp_dir().join("oag-2048-dlc-menus-no-packs");
        let _ = std::fs::remove_dir_all(&root);
        for archive in [
            "base/PSP2/data.psarc",
            "patch-v104/PSP2/data1.psarc",
            "patch-v104/PSP2/data2.psarc",
        ] {
            let link = root.join(archive);
            std::fs::create_dir_all(link.parent().expect("a folder")).expect("a scratch folder");
            std::os::unix::fs::symlink(
                package
                    .join(archive)
                    .canonicalize()
                    .expect("a package archive"),
                link,
            )
            .expect("a link");
        }
        Self(root)
    }
}

impl Drop for WithoutPacks {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn options(source: &Path) -> boot::Options {
    boot::Options {
        language: Some("English".to_string()),
        source: source.display().to_string(),
        dlc: Vec::new(),
        leg: frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-2048-dlc-menus-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    }
}

/// A press and its release, the way the capture's `--press` gives one.
fn press(frontend: &mut frontend::Frontend, input: &mut Input, button: Button) {
    input.begin_frame(button.bit());
    frontend.update(1.0 / 60.0, input, None);
    input.begin_frame(0);
    frontend.update(1.0 / 60.0, input, None);
}

/// The whole boot, driven onto the mode grid on `source`.
fn on_the_grid(source: &Path) -> (frontend::Frontend, Input) {
    let mut frontend = boot::load(&options(source))
        .expect("the whole boot")
        .frontend;
    let mut input = Input::new();
    for tick in 0..3600u32 {
        input.begin_frame(if tick.is_multiple_of(2) {
            Button::Cross.bit()
        } else {
            0
        });
        frontend.update(1.0 / 60.0, &mut input, None);
        if frontend.machine().is(w2048::GAME_MODE_CHOICE) {
            frontend.take_notes();
            return (frontend, input);
        }
    }
    panic!("never reached the mode grid");
}

/// Chooses the tile `steps` to the right of the single-player one and
/// confirms it.
fn choose(frontend: &mut frontend::Frontend, input: &mut Input, steps: usize) {
    for _ in 0..steps {
        press(frontend, input, Button::Right);
    }
    press(frontend, input, Button::Cross);
    press(frontend, input, Button::Cross);
}

fn grid_of(name: &str) -> usize {
    name.trim_start_matches("grid")
        .split('_')
        .next()
        .and_then(|index| index.parse().ok())
        .unwrap_or_else(|| panic!("{name} is not a grid cell"))
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn both_packages_open_their_own_campaign_from_a_tile_each() {
    let Some(eu) = package("PCSF00007") else {
        return;
    };
    for (steps, tile, grids, cells) in [(1, "FE_RC_HD", 0..8, 87), (2, "FE_RC_FURY", 8..16, 80)] {
        let (mut frontend, mut input) = on_the_grid(&eu);
        assert!(frontend.has_campaign_tier(tile));
        choose(&mut frontend, &mut input, steps);
        assert!(
            frontend.machine().is(w2048::NEW_FE_SHELL),
            "{tile} opens a map: {:#?}",
            frontend.take_notes()
        );
        assert_eq!(frontend.campaign_tile(), Some(tile));
        let events = frontend.campaign_events();
        assert_eq!(events.len(), cells, "{tile}: every cell of its grids");
        assert!(
            events
                .iter()
                .all(|event| grids.contains(&grid_of(&event.name))),
            "{tile}: only its own grids"
        );
        let first = frontend
            .selected_event()
            .expect("an event under the cursor")
            .name
            .clone();
        press(&mut frontend, &mut input, Button::Cross);
        press(&mut frontend, &mut input, Button::Cross);
        assert!(frontend.is_finished(), "{tile}: a cell launches");
        assert_eq!(frontend.launch(), Some(&Launch::Cell(first)));
    }
}

#[test]
#[ignore = "needs data/extracted/vita/PCSA00015"]
fn the_usa_package_ships_the_hd_pack_alone_so_only_its_tile_opens() {
    let Some(usa) = package("PCSA00015") else {
        return;
    };
    let (mut frontend, mut input) = on_the_grid(&usa);
    assert!(frontend.has_campaign_tier("FE_RC_HD"));
    assert!(!frontend.has_campaign_tier("FE_RC_FURY"));
    choose(&mut frontend, &mut input, 1);
    assert_eq!(frontend.campaign_tile(), Some("FE_RC_HD"));
    assert_eq!(frontend.campaign_events().len(), 87);

    let (mut frontend, mut input) = on_the_grid(&usa);
    choose(&mut frontend, &mut input, 2);
    assert!(frontend.machine().is(w2048::GAME_MODE_CHOICE));
    assert_eq!(frontend.campaign_tile(), None);
    assert!(
        frontend
            .take_notes()
            .iter()
            .any(|note| note.contains("FE_RC_FURY is another title's campaign")),
        "the Fury tile says where it leads"
    );
}

/// "Without the pack, nothing changes": the tiles are on the grid the patch
/// authors and refuse by note, as they did before any pack was read.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn without_either_package_the_tiles_keep_their_note_and_no_map_opens() {
    let Some(eu) = package("PCSF00007") else {
        return;
    };
    let bare = WithoutPacks::of(&eu);
    let (shell, ..) = boot::load_shell(&options(&bare.0)).expect("the base package boots");
    assert!(shell.tiers.is_empty());
    for (steps, tile) in [(1, "FE_RC_HD"), (2, "FE_RC_FURY")] {
        let (mut frontend, mut input) = on_the_grid(&bare.0);
        assert!(!frontend.has_campaign_tier(tile));
        choose(&mut frontend, &mut input, steps);
        assert!(frontend.machine().is(w2048::GAME_MODE_CHOICE));
        assert_eq!(frontend.campaign_tile(), None);
        assert!(
            frontend
                .take_notes()
                .iter()
                .any(|note| note.contains(&format!("{tile} is another title's campaign"))),
            "{tile} refuses by note"
        );
    }
}

/// The cells of both campaigns and the plan each launches with, resolved the
/// way the session does: against the shell's own catalogue, which holds a
/// circuit only where its geometry is mounted.
struct Campaigns {
    shell: boot::Shell,
    archives: oag_assets::Archives,
    title: &'static oag_title::Title,
    tiers: Vec<boot::tiers2048::TierGrids>,
}

impl Campaigns {
    fn open(source: &Path) -> Self {
        let (shell, mut archives, title) = boot::load_shell(&options(source)).expect("the shell");
        let tiers = boot::tiers2048::read(&mut archives, &mut Vec::new());
        Self {
            shell,
            archives,
            title,
            tiers,
        }
    }

    /// `(campaign, grid, cell)`, HD's first.
    fn cells(&self) -> Vec<(&'static str, &Grid, &Cell)> {
        self.tiers
            .iter()
            .flat_map(|tier| {
                tier.grids.iter().flat_map(move |grid| {
                    grid.cells
                        .iter()
                        .map(move |cell| (tier.add_on.campaign, grid, cell))
                })
            })
            .collect()
    }

    fn plan(&self, cell: &Cell) -> Result<CellPlan, Refusal> {
        let ladder = self
            .title
            .race
            .speed_classes
            .map_or(&[][..], |classes| classes.names);
        launch::plan_cell_for(cell, "venom", ladder, |mode, id| {
            let list = if mode == oag_race::Mode::Zone {
                &self.shell.zone_tracks
            } else {
                &self.shell.tracks
            };
            list.iter()
                .find(|track| track.id == id)
                .map(oag_raceplay::catalogue::Track::entry_name)
        })
    }
}

/// Every cell either plans onto a `track.vex` the mounted packages hold, or is
/// refused for a mode this engine cannot run: never a circuit id that
/// resolves to nothing, which is what launching from a map would otherwise
/// find out one cell at a time.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn every_cell_plans_onto_a_held_circuit_or_is_refused_for_its_mode() {
    let Some(eu) = package("PCSF00007") else {
        return;
    };
    let campaigns = Campaigns::open(&eu);
    let (mut planned, mut refused, mut stranded) = (0, Vec::new(), Vec::new());
    let cells = campaigns.cells();
    assert_eq!(cells.len(), 87 + 80);
    for (campaign, _, cell) in &cells {
        match campaigns.plan(cell) {
            Ok(plan) => {
                planned += 1;
                for entry in &plan.leg_entries {
                    if campaigns.archives.locate(entry).is_none() {
                        stranded.push(format!("{campaign} {}: {entry}", cell.name));
                    }
                }
            }
            Err(Refusal::Mode(mode)) => refused.push((*campaign, mode)),
            Err(other) => stranded.push(format!("{campaign} {}: {other}", cell.name)),
        }
    }
    assert!(stranded.is_empty(), "{stranded:#?}");
    assert_eq!(planned, 142);
    assert_eq!(refused.len(), 25);
    assert!(
        refused.iter().all(|(campaign, mode)| *campaign == "Fury"
            && (mode == "Detonator" || mode == "NitroBattle")),
        "only Fury authors the two modes this engine cannot run: {refused:?}"
    );
}

/// A cell on a reversed circuit races the pack's own reversed geometry, not
/// the forward track with a flag: `track_reversed.vex` in the same folder.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn a_cell_on_a_reversed_circuit_races_its_reversed_geometry() {
    let Some(eu) = package("PCSF00007") else {
        return;
    };
    let campaigns = Campaigns::open(&eu);
    let reversed: Vec<_> = campaigns
        .cells()
        .into_iter()
        .filter(|(_, _, cell)| cell.track.as_deref().is_some_and(|id| id.ends_with("_R")))
        .collect();
    assert!(
        !reversed.is_empty(),
        "the campaigns race circuits backwards"
    );
    for (_, _, cell) in reversed {
        let Ok(plan) = campaigns.plan(cell) else {
            continue;
        };
        let entry = &plan.leg_entries[0];
        assert!(
            entry.ends_with(r"\track_reversed.vex"),
            "{}: {entry}",
            cell.name
        );
        assert!(
            campaigns.archives.locate(entry).is_some(),
            "{}: {entry} is mounted",
            cell.name
        );
    }
}

/// The first node is the one HD's own campaign opens on: a race on Vineta K in
/// Venom, three laps, flown in the HD roster's `Feisar`.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn the_first_hd_cell_races_vineta_k_from_the_pack() {
    let Some(eu) = package("PCSF00007") else {
        return;
    };
    let campaigns = Campaigns::open(&eu);
    let (tier, _, cell) = boot::tiers2048::find(&campaigns.tiers, "grid0_3_1").expect("a cell");
    assert_eq!(tier.add_on.campaign, "HD");
    assert_eq!(tier.add_on.default_team, "Feisar");
    let plan = campaigns.plan(cell).expect("the first cell launches");
    assert_eq!(plan.mode, oag_race::Mode::SingleRace);
    assert_eq!(
        plan.leg_entries,
        [r"Data\art\published\DLC1\environments\Vineta_K\track.vex"]
    );
    assert_eq!(plan.class, "Venom");
    assert_eq!(plan.laps_override, Some(3));
}

/// What a grid is unlocked by is the grid's own authoring, the law HD's
/// campaign applies; a mode this build cannot race is closed whatever the
/// save holds, and `unlock_all` opens grids but not those.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn a_node_is_closed_by_an_unearned_grid_or_a_mode_this_build_cannot_race() {
    let Some(eu) = package("PCSF00007") else {
        return;
    };
    let campaigns = Campaigns::open(&eu);
    let records = oag_game::records::Store::default();
    let closed = |unlock_all: bool, name: &str| {
        boot::tiers2048::closed(&campaigns.tiers, &records, "2048", unlock_all, name)
    };
    let cells = campaigns.cells();
    let named = |wanted: &dyn Fn(&Grid, &Cell) -> bool| {
        cells
            .iter()
            .find(|(_, grid, cell)| wanted(grid, cell))
            .map(|(_, _, cell)| cell.name.clone())
    };

    let open = named(&|grid, cell| !grid.locked && campaigns.plan(cell).is_ok())
        .expect("a grid the campaign opens with");
    assert!(!closed(false, &open), "{open} is in an unlocked grid");

    let shut = named(&|grid, cell| grid.locked && campaigns.plan(cell).is_ok())
        .expect("a grid that waits on another");
    assert!(closed(false, &shut), "{shut} waits on points not earned");
    assert!(!closed(true, &shut), "--unlock-all opens it");

    let unraceable = named(&|_, cell| campaigns.plan(cell).is_err())
        .expect("a cell of a mode this build cannot race");
    assert!(closed(false, &unraceable));
    assert!(
        closed(true, &unraceable),
        "unlocking does not make it raceable"
    );
}

/// The label of each group the map's wordmarks are cut from - eight HD, then
/// eight Fury, in the file's order - sits where the nodes this places for its
/// grid start: its bottom edge within one canvas unit of the grid's first row,
/// and over the columns the grid spans. The projection's scale, bias and
/// offsets are read off the executable's immediates; this is the file's own
/// layout agreeing with them for all sixteen grids.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn each_groups_label_sits_over_the_first_row_of_its_grids_nodes() {
    let Some(eu) = package("PCSF00007") else {
        return;
    };
    let mut campaigns = Campaigns::open(&eu);
    let blob = campaigns
        .archives
        .read_name(r"Data\Plugins\Frontend\NEWGUI\Definition.xml")
        .expect("the front end's own definition");
    let xml = oag_tables::fexml::text(&blob).expect("expanded");
    let labels = group_labels(&xml);
    assert_eq!(labels.len(), 16, "eight HD and eight Fury wordmarks");

    let mut by_grid: Vec<(usize, Vec<(f32, f32)>)> = Vec::new();
    for (_, _, cell) in campaigns.cells() {
        let index = grid_of(&cell.name);
        let node = tier::node(index, cell.grid_coords().expect("a grid cell")).expect("a row");
        match by_grid.iter_mut().find(|(i, _)| *i == index) {
            Some((_, nodes)) => nodes.push(node),
            None => by_grid.push((index, vec![node])),
        }
    }
    by_grid.sort_by_key(|(index, _)| *index);
    assert_eq!(by_grid.len(), 16);
    for ((index, nodes), (name, x, y, width, height)) in by_grid.iter().zip(&labels) {
        let top = nodes.iter().map(|n| n.1).fold(f32::MAX, f32::min);
        let (left, right) = nodes
            .iter()
            .fold((f32::MAX, f32::MIN), |(l, r), n| (l.min(n.0), r.max(n.0)));
        assert!(
            (y + height - top).abs() <= 1.0,
            "grid{index} {name}: label bottom {} against first row {top}",
            y + height
        );
        assert!(
            *x <= right && x + width >= left,
            "grid{index} {name}: label {x}..{} against columns {left}..{right}",
            x + width
        );
    }
}

/// `(name, x, y, width, height)` of the group wordmarks: the `<CanvasLabel>`s
/// named `<Group>Label`, which no trophy (`Trophy-2048-2-3`) or ship unlock
/// shares a spelling with, in file order.
fn group_labels(xml: &str) -> Vec<(String, f32, f32, f32, f32)> {
    const OPEN: &str = "<CanvasLabel name=\"";
    let mut labels = Vec::new();
    let mut rest = xml;
    while let Some(at) = rest.find(OPEN) {
        rest = &rest[at + OPEN.len()..];
        let Some(end) = rest.find('"') else { break };
        let name = &rest[..end];
        let Some(values) = rest.find("<Values ") else {
            break;
        };
        let values = &rest[values..];
        let values = &values[..values.find('>').unwrap_or(values.len())];
        let field = |key: &str| -> Option<f32> {
            let start = values.find(&format!("{key}=\""))? + key.len() + 2;
            values[start..].split('"').next()?.parse().ok()
        };
        if let (Some(group), Some(x), Some(y), Some(width), Some(height)) = (
            name.strip_suffix("Label"),
            field("x"),
            field("y"),
            field("width"),
            field("height"),
        ) && !group.contains('-')
        {
            labels.push((group.to_string(), x, y, width, height));
        }
    }
    labels
}
