//! The server extractor: records what the races a server can offer read from a title's
//! source, and writes only that into a folder `oag-server` opens in place of the source.
//!
//! The mechanism is `oag_assets::record` (every archive byte range a load reads),
//! `oag_assets::sparse` (an archive kept as just those ranges) and
//! `oag_source::extract` (the manifest and the files). This module is the part that knows
//! what a server offers: [`offer`] reads a title's circuits, teams and speed classes from its
//! own definitions, [`plan`] turns them into the races worth loading, and [`extract`] loads
//! them under the recorder.
//!
//! # What "every race the server can offer" is
//!
//! The lobby fixes the difficulty (`elite`), control scheme and weapons (off) of every race
//! (`online::rooms`), so the only axes are a room's settings and each seat's team: the
//! circuit, the speed class, the mode, how many people race and with what. The plan walks
//! each axis once with the others at their defaults, then crosses classes with modes on two
//! circuits, and [`extract`] closes by loading the whole plan again from the extraction and
//! requiring the same tick-0 state hash. A read the plan missed would fail there by name
//! (`NotExtracted`), not as a wrong race.
//!
//! **Not offered, so not extracted:** downloadable content (the server mounts none: its
//! race options carry no pack directory), and the asset layers' player override.

use std::path::Path;

use anyhow::{Context, Result, anyhow};
use oag_raceplay::catalogue;
use oag_source::extract::{MANIFEST, Manifest};

use crate::wire::RaceParams;

/// What a title's source offers a room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    /// `Title::name`.
    pub title: &'static str,
    /// The source's platform.
    pub platform: String,
    /// The source's serial, if it has one.
    pub serial: Option<String>,
    /// Every raceable circuit, as the entry name a room's `track` takes.
    pub tracks: Vec<String>,
    /// The circuits a Zone race is offered on.
    pub zone_tracks: Vec<String>,
    /// Every team a person can pick.
    pub teams: Vec<String>,
    /// Every team the title's plugin definition declares: the pool the AI fills the grid from,
    /// which is wider than [`Self::teams`] on a title that declares craft it cannot all race
    /// (2048's HD-era roster), and drawn from by the race's seed.
    pub pool: Vec<String>,
    /// The speed classes the ordinary RACE page offers.
    pub classes: Vec<String>,
}

/// Reads what `source` offers. Under the recorder, this is also the first thing recorded:
/// the definition files.
///
/// # Errors
///
/// A source that will not open as a title.
pub fn offer(source: &str) -> Result<Offer> {
    let mut opened = oag_source::title::open_source(source, Vec::new(), Vec::new())
        .with_context(|| format!("{source} is not a disc or folder of a title this build knows"))?;
    let title = opened.title;
    let definition = title.plugin_definition;
    let documents = catalogue::documents(
        &mut opened.archives,
        &[
            definition,
            title.track_plugin_definition.unwrap_or(definition),
        ],
    );
    let archives = &opened.archives;
    let raceable = |track: &catalogue::Track| archives.locate(&track.entry_name()).is_some();
    let race_tracks = catalogue::all_tracks(&documents);
    let zone_tracks: Vec<String> = title
        .race
        .zone
        .menu_tracks(
            &race_tracks,
            |track| track.available_in_zone,
            |names| catalogue::all_tracks_named(&documents, names),
        )
        .iter()
        .filter(|track| raceable(track))
        .map(catalogue::Track::entry_name)
        .collect();
    let tracks = race_tracks
        .iter()
        .filter(|track| raceable(track))
        .map(catalogue::Track::entry_name)
        .collect();
    let pool = documents
        .first()
        .map(|document| {
            catalogue::teams(document)
                .into_iter()
                .map(|t| t.id)
                .collect()
        })
        .unwrap_or_default();
    let teams = catalogue::raceable_teams(archives, title.race, catalogue::all_teams(&documents))
        .into_iter()
        .map(|team| team.id)
        .collect();
    // A title that has not listed its ladder (`speed_classes: None`) is offered the ordinary
    // four, the rungs every title's `handlingstats.xml` carries.
    let classes = title.race.speed_classes.map_or_else(
        || {
            oag_title::SpeedClasses::PULSE_LADDER
                .iter()
                .map(|name| (*name).to_string())
                .collect()
        },
        |classes| {
            classes
                .names
                .iter()
                .filter(|name| !classes.remix_only.contains(name))
                .map(|name| (*name).to_string())
                .collect()
        },
    );
    Ok(Offer {
        title: title.name,
        platform: opened.archives.layout.platform.to_string(),
        serial: opened.archives.layout.serial.clone(),
        tracks,
        zone_tracks,
        teams,
        pool,
        classes,
    })
}

/// A race to load, named for the report.
#[derive(Debug, Clone)]
pub struct Planned {
    /// Which axis it walks.
    pub label: String,
    /// The race.
    pub params: RaceParams,
}

/// The class a plan holds fixed while it walks the other axes: the second rung, the lobby's
/// ordinary pick (`VENOM`), else the first.
fn base_class(offer: &Offer) -> String {
    offer
        .classes
        .iter()
        .find(|class| class.eq_ignore_ascii_case("VENOM"))
        .or_else(|| offer.classes.first())
        .cloned()
        .unwrap_or_default()
}

/// Seeds (ascending) whose grids, against a player on `player`, put every other team of `pool`
/// in a slot at least once: the grid is `oag_race::roster::teams_for_slots`, the loader's own.
fn covering_seeds(pool: &[String], player: &str) -> Vec<u64> {
    let mut uncovered: Vec<&String> = pool.iter().filter(|team| *team != player).collect();
    let mut seeds = Vec::new();
    for seed in 1..=4096u64 {
        if uncovered.is_empty() {
            break;
        }
        let grid = oag_race::roster::teams_for_slots(player, pool, oag_gameplay::MAX_SHIPS, seed);
        let before = uncovered.len();
        uncovered.retain(|team| !grid.contains(team));
        if uncovered.len() < before {
            seeds.push(seed);
        }
    }
    seeds
}

/// The races worth loading to cover what a room can ask for; see the module docs.
#[must_use]
pub fn plan(offer: &Offer) -> Vec<Planned> {
    let base = RaceParams {
        title: offer.title.to_string(),
        class: base_class(offer),
        mode: oag_race::Mode::SingleRace.name().to_string(),
        difficulty: "elite".into(),
        scheme: "veteran".into(),
        seed: 1,
        people: 1,
        opponents: true,
        weapons: false,
        ..RaceParams::default()
    };
    let with = |label: String, edit: &dyn Fn(&mut RaceParams)| {
        let mut params = base.clone();
        edit(&mut params);
        Planned { label, params }
    };
    let mut out = Vec::new();
    for track in &offer.tracks {
        out.push(with(format!("circuit {track}"), &|p| {
            p.track.clone_from(track)
        }));
    }
    for track in &offer.zone_tracks {
        out.push(with(format!("zone {track}"), &|p| {
            p.track.clone_from(track);
            p.mode = oag_race::Mode::Zone.name().to_string();
        }));
    }
    for (at, team) in offer.teams.iter().enumerate() {
        out.push(with(format!("team {team}"), &|p| p.team.clone_from(team)));
        let seats: Vec<String> = (0..offer.teams.len().min(8))
            .map(|seat| offer.teams[(at + seat) % offer.teams.len()].clone())
            .collect();
        out.push(with(format!("grid from {team}"), &|p| {
            p.team = seats[0].clone();
            p.people = u8::try_from(seats.len()).unwrap_or(8);
            p.teams.clone_from(&seats);
        }));
    }
    let crossed = [
        offer.tracks.first(),
        offer.tracks.get(offer.tracks.len() / 2),
    ];
    for track in crossed.into_iter().flatten() {
        for class in &offer.classes {
            for mode in oag_race::Mode::ALL {
                out.push(with(format!("{class} {} on {track}", mode.name()), &|p| {
                    p.track.clone_from(track);
                    p.class.clone_from(class);
                    p.mode = mode.name().to_string();
                }));
            }
        }
    }
    // The AI's teams are drawn from the whole pool by the race's seed, and every team's craft,
    // handling and engine sound are read when it flies a slot. Per player team, pick seeds until
    // every pool team has flown a grid slot against them.
    for team in &offer.teams {
        for seed in covering_seeds(&offer.pool, team) {
            out.push(with(format!("opponents of {team}, seed {seed}"), &|p| {
                p.team.clone_from(team);
                p.seed = seed;
            }));
        }
    }
    out.push(with("no opponents".into(), &|p| p.opponents = false));
    out
}

/// How one planned race loaded.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// The plan entry's label.
    pub label: String,
    /// The tick-0 state hash, or why the race did not load.
    pub result: std::result::Result<u64, String>,
}

/// Loads every planned race from `source`, calling `each` after every one.
pub fn run(source: &str, plan: &[Planned], mut each: impl FnMut(&Outcome)) -> Vec<Outcome> {
    plan.iter()
        .map(|planned| {
            let result = crate::race::load(&planned.params, source)
                .map(|(race, _)| crate::race::fingerprint(&race))
                .map_err(|e| format!("{e:#}"));
            let outcome = Outcome {
                label: planned.label.clone(),
                result,
            };
            each(&outcome);
            outcome
        })
        .collect()
}

/// SHA-256 of a file, streamed, as lowercase hex.
///
/// # Errors
///
/// The file will not read.
pub fn sha256_file(path: &Path) -> Result<String> {
    use std::io::Read;
    let mut file =
        std::fs::File::open(path).with_context(|| format!("reading {}", path.display()))?;
    let mut context = ring::digest::Context::new(&ring::digest::SHA256);
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        context.update(&buf[..n]);
    }
    Ok(hex(context.finish().as_ref()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// What an extraction did.
#[derive(Debug)]
pub struct Report {
    /// The manifest as written.
    pub manifest: Manifest,
    /// Planned races that loaded from the source.
    pub loaded: usize,
    /// Planned races the source itself refused (not an extraction gap), with why.
    pub refused: Vec<Outcome>,
    /// Races that loaded from the source and not, or differently, from the extraction.
    pub gaps: Vec<String>,
}

/// Extracts what the races `source` offers read, into `out`, and checks the result by loading
/// the same races from it.
///
/// # Errors
///
/// A source or output that cannot be read or written. A gap is reported in [`Report::gaps`],
/// not as an error, so the caller can say what is missing.
pub fn extract(
    source: &str,
    out: &Path,
    build: &str,
    mut progress: impl FnMut(&str),
) -> Result<Report> {
    if out.join(MANIFEST).exists() {
        return Err(anyhow!(
            "{} already holds an extraction; remove it first",
            out.display()
        ));
    }
    let root = Path::new(source);
    oag_assets::record::start(root);
    let sweep = offer(source).map(|offer| {
        progress(&format!(
            "{}: {} circuits, {} zone circuits, {} teams, classes {:?}",
            offer.title,
            offer.tracks.len(),
            offer.zone_tracks.len(),
            offer.teams.len(),
            offer.classes
        ));
        let plan = plan(&offer);
        progress(&format!("loading {} races", plan.len()));
        let outcomes = run(source, &plan, |o| {
            progress(&format!(
                "  {} {}",
                if o.result.is_ok() { "ok " } else { "ERR" },
                o.label
            ));
        });
        (offer, outcomes)
    });
    let recording =
        oag_assets::record::stop().ok_or_else(|| anyhow!("the recorder stopped early"))?;
    let (offered, outcomes) = sweep?;
    let swept = vec![
        format!(
            "{} circuits, {} zone circuits, {} teams, classes {}",
            offered.tracks.len(),
            offered.zone_tracks.len(),
            offered.teams.len(),
            offered.classes.join(" ")
        ),
        format!(
            "{} races loaded of {} planned",
            outcomes.iter().filter(|o| o.result.is_ok()).count(),
            outcomes.len()
        ),
    ];
    let mut rows = oag_source::extract::write_files(root, &recording, out)?;
    for row in &mut rows {
        row.sha256 = sha256_file(&out.join(&row.path))?;
    }
    let manifest = Manifest {
        title: offered.title.to_string(),
        platform: offered.platform.clone(),
        serial: offered.serial.clone().unwrap_or_default(),
        source: root
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
        source_sha256: if root.is_dir() {
            "folder".into()
        } else {
            sha256_file(root)?
        },
        extractor: build.to_string(),
        swept,
        files: rows,
    };
    std::fs::write(out.join(MANIFEST), manifest.to_text())?;

    let extraction = out.to_string_lossy().into_owned();
    let planned = plan(&offered);
    progress("checking the extraction");
    let again = run(&extraction, &planned, |_| {});
    let mut gaps: Vec<String> = oag_assets::record::take_misses()
        .into_iter()
        .map(|miss| format!("a read the extraction could not answer: {miss}"))
        .collect();
    for (before, after) in outcomes.iter().zip(&again) {
        if let Ok(hash) = before.result
            && after.result != Ok(hash)
        {
            gaps.push(format!("{}: {:?}", after.label, after.result));
        }
    }
    let refused = outcomes
        .iter()
        .filter(|o| o.result.is_err())
        .cloned()
        .collect();
    Ok(Report {
        manifest,
        loaded: outcomes.iter().filter(|o| o.result.is_ok()).count(),
        refused,
        gaps,
    })
}
