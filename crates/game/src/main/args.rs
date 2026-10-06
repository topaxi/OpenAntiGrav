//! The small parsers and resolvers behind individual flags.
//!
//! Each turns one command-line string, or one settings-over-flag pair, into
//! the type the rest of the run wants. Together they are what [`crate::cli`]
//! declares and nothing interprets.

use anyhow::{Context, Result, ensure};
use log::warn;

use oag_game::input;
use oag_game::{loading, movie, prefetch, settings};
use oag_gameplay::ControlScheme;
use oag_input::bindings::Bindings;
use oag_input::pad::TriggerMode;

use crate::cli::Cli;

/// How good the opponents are: `[ai] difficulty`, or the default.
///
/// An unrecognised token is **reported and ignored** rather than fatal, exactly
/// as [`resolve_scheme`] handles its own: a profile written by a build with a
/// level this one does not have should still race.
pub(crate) fn resolve_difficulty(settings: &settings::Settings) -> oag_ai::Difficulty {
    let token = &settings.ai.difficulty;
    oag_ai::Difficulty::from_name(token).unwrap_or_else(|| {
        let fallback = oag_ai::Difficulty::default();
        warn!(
            "ignoring [ai] difficulty = {token:?}; using {}",
            fallback.name()
        );
        fallback
    })
}

/// The control scheme this run uses: `--scheme` over `[controls] scheme`.
///
/// An unrecognised token in the settings file is **reported and ignored**
/// rather than fatal, matching how `race.mode` and `race.class` behave: a
/// profile written by a build that had a scheme this one does not should still
/// boot. The flag cannot be unrecognised - clap rejects it at parse time
/// through `ControlScheme`'s `FromStr`, which is why its error message names
/// the valid values.
pub(crate) fn resolve_scheme(cli: &Cli, settings: &settings::Settings) -> ControlScheme {
    if let Some(scheme) = cli.scheme {
        return scheme;
    }
    let token = &settings.controls.scheme;
    ControlScheme::from_name(token).unwrap_or_else(|| {
        let fallback = ControlScheme::default();
        warn!("ignoring [controls] scheme = {token:?}; using {fallback}");
        fallback
    })
}

/// What the button prompts draw: `--prompt-style`, then `[controls]
/// prompt_style`, then `auto`. An unrecognised token is reported and ignored,
/// as [`resolve_scheme`]'s is.
pub(crate) fn resolve_prompt_style(
    cli: &Cli,
    settings: &settings::Settings,
) -> oag_input::prompt::PromptStyle {
    use oag_input::prompt::PromptStyle;
    let token = cli
        .menu_args
        .prompt_style
        .as_deref()
        .unwrap_or(&settings.controls.prompt_style);
    PromptStyle::from_name(token).unwrap_or_else(|| {
        warn!("ignoring prompt style {token:?}; using auto");
        PromptStyle::Auto
    })
}

/// What the analog triggers do: `[controls] triggers`, or the default.
///
/// An unrecognised token is **reported and ignored** rather than fatal, exactly
/// as [`resolve_scheme`] handles its own. There is no flag to override it: the
/// CONTROLS page can change it mid-race, which is the thing a `--triggers`
/// would exist to make convenient.
///
/// The sensitivity beside it needs no resolver - it is typed, so serde has
/// already refused anything outside its range by the time settings load.
pub(crate) fn resolve_triggers(settings: &settings::Settings) -> TriggerMode {
    let token = &settings.controls.triggers;
    TriggerMode::from_name(token).unwrap_or_else(|| {
        let fallback = TriggerMode::default();
        warn!("ignoring [controls] triggers = {token:?}; using {fallback}");
        fallback
    })
}

/// The live key-to-button table: `[controls] bindings`, falling back to the
/// default for any entry that does not parse.
///
/// Every fallback is **reported**, unlike [`settings::Controls::live_bindings`]:
/// this is the table a real keyboard resolves through, so a settings file a
/// newer build wrote - or a hand edit with a typo - should say what it lost
/// rather than silently drive the ship with a default the player never chose.
/// No flag overrides it: rebinding happens on the CONTROLS page, the same way
/// [`resolve_triggers`] has none.
pub(crate) fn resolve_bindings(settings: &settings::Settings) -> Bindings {
    let (bindings, ignored) = Bindings::from_pairs(&settings.controls.bindings);
    for name in ignored {
        warn!("ignoring [controls.bindings] {name:?}; using its default key");
    }
    bindings
}

/// Parses `--loading-step`'s value into the phase the capture draws.
///
/// **Same reason `--loading-screen` exists, one level down.** A cache hit is
/// over in milliseconds and a transcode holds one number for a second at a time,
/// so neither is a state a capture can be *timed* to catch; stating it is the
/// only way to look at the layout. No value at all is the prefetch phase, which
/// is what `--loading-screen` alone has always drawn.
pub(crate) fn parse_step(spec: Option<&str>) -> Result<loading::Phase> {
    let Some(spec) = spec else {
        return Ok(loading::Phase::Prefetch);
    };
    let bad = || {
        anyhow::anyhow!(
            "{spec:?} is not a load step; write it as race, cached, decoding, \
             or transcoding:DONE/TOTAL (TOTAL may be omitted)"
        )
    };
    // **A phase rather than a step**, and the only value here that is: the
    // other three describe what one movie load is doing, and this is the wait
    // between the menus and the grid. It is spelled here anyway because this
    // flag is the only way to look at that screen without a window - a race
    // load is seconds long, so there is no timing a capture to it.
    if spec == "race" {
        return Ok(loading::Phase::Race);
    }
    Ok(loading::Phase::Media(Some(match spec {
        "cached" => movie::Step::Cached,
        "decoding" => movie::Step::Decoding,
        _ => {
            let frames = spec.strip_prefix("transcoding:").ok_or_else(bad)?;
            let (done, total) = frames
                .split_once('/')
                .map_or((frames, None), |(d, t)| (d, Some(t)));
            let done: usize = done.trim().parse().map_err(|_| bad())?;
            let total = total.map(|t| t.trim().parse::<usize>()).transpose()?;
            ensure!(
                total.is_none_or(|total| done <= total),
                "frame {done} of {} is past the end",
                total.unwrap_or_default()
            );
            movie::Step::Transcoding { done, total }
        }
    })))
}

/// The saved front-end styling, as a name a title can match, or `None` for
/// "whatever this source leads with".
///
/// An empty setting is `None` rather than `Some("")`: the key is always in the
/// file (see [`settings::Display::front_end_style`]) and empty is what it holds
/// until somebody chooses, which must not be matched against a real style name.
#[must_use]
pub(crate) fn style_of(settings: &settings::Settings) -> Option<&str> {
    let name = settings.display.front_end_style.trim();
    (!name.is_empty()).then_some(name)
}

/// Turns a comma-separated list of abstract button names into a mask.
///
/// Unknown names are skipped rather than fatal: `none` is a real value in the
/// game's own XML and means no button.
pub(crate) fn button_mask(names: Option<&str>) -> u32 {
    names.map_or(0, |list| {
        list.split(',')
            .filter_map(|name| input::button_from_name(name.trim()))
            .fold(0u32, |mask, button| mask | button.bit())
    })
}

/// Parses `--loading-screen`'s `DONE/TOTAL`.
///
/// Only the two figures the bar is drawn from, because everything else on the
/// screen follows from them: `planning` is over by definition once there is a
/// total, and `finished` is `done == total`, which is also the state the fade
/// runs in. `cached` and `failed` are left at zero rather than invented - they
/// are counts of things that really happened, and a flag that made them up
/// would be drawing a run nobody had.
pub(crate) fn parse_progress(spec: &str) -> Result<prefetch::Progress> {
    let bad = || {
        anyhow::anyhow!("{spec:?} is not a conversion state; write it as DONE/TOTAL, e.g. 37/115")
    };
    let (done, total) = spec.split_once('/').ok_or_else(bad)?;
    let done: usize = done.trim().parse().map_err(|_| bad())?;
    let total: usize = total.trim().parse().map_err(|_| bad())?;
    ensure!(
        done <= total,
        "{done} converted of {total} is more than all of them"
    );
    Ok(prefetch::Progress {
        planning: false,
        total,
        done,
        cached: 0,
        failed: 0,
        // A real label, built the way `prefetch` builds one, so the line reads
        // as the thing it will read as in a window rather than as filler.
        //
        // **Pulse's name, whatever the source is**, and deliberately: this flag
        // previews the loading screen's *layout*, and the pipeline behind it is
        // Pulse-only by construction anyway (`prefetch.rs` hardcodes
        // `oag_pulse::TITLE` - finding S11). Named here so a reader of an HD
        // preview knows the text is a stand-in for width rather than a claim
        // about that disc. Finding G5 of the 2026-08-18 review.
        current: (done < total).then(|| format!("Data.wad {}", oag_pulse::names::INTRO_MOVIE)),
        finished: done == total,
        load_stage: 0,
    })
}

/// A capture's default size, front end and race alike.
///
/// Three times the PSP's screen, so the 5x7 glyphs stay legible and a screenshot
/// frames what the window would have shown at its own default. The *window's*
/// size is `graphics.window_size` and is a setting - see
/// [`display::Size::default`], which is this same shape.
pub(crate) const DEFAULT_SIZE: &str = "1440x816";

/// Parses `WIDTHxHEIGHT`.
pub(crate) fn parse_size(text: &str) -> Result<(u32, u32)> {
    let bad = || anyhow::anyhow!("{text:?} is not a size; write it as WIDTHxHEIGHT, e.g. 1440x816");
    let (width, height) = text.split_once(['x', 'X']).ok_or_else(bad)?;
    let width: u32 = width.trim().parse().map_err(|_| bad())?;
    let height: u32 = height.trim().parse().map_err(|_| bad())?;
    if width == 0 || height == 0 {
        return Err(bad());
    }
    Ok((width, height))
}

/// Parses `--force-shake`'s `TICK:SEVERITY`.
pub(crate) fn force_shake(text: Option<&str>) -> Result<Option<(u32, f32)>> {
    let Some(text) = text else { return Ok(None) };
    let bad = || {
        anyhow::anyhow!("{text:?} is not a forced shake; write it as TICK:SEVERITY, e.g. 120:1.0")
    };
    let (tick, severity) = text.split_once(':').ok_or_else(bad)?;
    let severity: f32 = severity.trim().parse().map_err(|_| bad())?;
    if !(0.0..=1.0).contains(&severity) {
        return Err(bad());
    }
    Ok(Some((tick.trim().parse().map_err(|_| bad())?, severity)))
}

/// Parses every `--force-shield TICK:PERCENT`.
pub(crate) fn force_shield(texts: &[String]) -> Result<Vec<(u32, f32)>> {
    texts
        .iter()
        .map(|text| {
            let bad = || {
                anyhow::anyhow!(
                    "{text:?} is not a forced shield; write it as TICK:PERCENT, e.g. 0:15"
                )
            };
            let (tick, percent) = text.split_once(':').ok_or_else(bad)?;
            let percent: f32 = percent.trim().parse().map_err(|_| bad())?;
            if !(0.0..=1000.0).contains(&percent) {
                return Err(bad());
            }
            Ok((tick.trim().parse().map_err(|_| bad())?, percent))
        })
        .collect()
}

/// Parses every `--force-medal TICK:TIER` into the tick and the language id of
/// the end-of-race phrase for that tier.
pub(crate) fn force_medal(texts: &[String]) -> Result<Vec<(u32, &'static str)>> {
    texts
        .iter()
        .map(|text| {
            let bad = || {
                anyhow::anyhow!(
                    "{text:?} is not a forced medal; write it as TICK:TIER, e.g. 10:gold"
                )
            };
            let (tick, tier) = text.split_once(':').ok_or_else(bad)?;
            let id = match tier.trim() {
                "gold" => "ER_GMA",
                "silver" => "ER_SMA",
                "bronze" => "ER_BMA",
                _ => return Err(bad()),
            };
            Ok((tick.trim().parse().map_err(|_| bad())?, id))
        })
        .collect()
}

/// Parses `--force-wreck`'s `TICK:SLOT`.
pub(crate) fn force_wreck(text: Option<&str>) -> Result<Option<(u32, usize)>> {
    let Some(text) = text else { return Ok(None) };
    let bad =
        || anyhow::anyhow!("{text:?} is not a forced wreck; write it as TICK:SLOT, e.g. 60:3");
    let (tick, slot) = text.split_once(':').ok_or_else(bad)?;
    Ok(Some((
        tick.trim().parse().map_err(|_| bad())?,
        slot.trim().parse().map_err(|_| bad())?,
    )))
}

/// Resolves `--give`'s spelling to a weapon, case-insensitively.
///
/// Rejects an unknown name rather than ignoring it: a silent no-op here looks
/// exactly like the flag working and the weapon never being drawn, which is the
/// failure mode the flag exists to remove. The error lists what the weapon table
/// actually holds, so a typo is one read away from fixed.
pub(crate) fn give_weapon(name: Option<&str>) -> Result<Option<oag_tables::weapons::Weapon>> {
    let Some(name) = name else { return Ok(None) };
    let found = oag_tables::weapons::Weapon::ALL
        .into_iter()
        .find(|weapon| weapon.as_type().eq_ignore_ascii_case(name));
    match found {
        Some(weapon) => Ok(Some(weapon)),
        None => {
            let known: Vec<&str> = oag_tables::weapons::Weapon::ALL
                .iter()
                .map(|weapon| weapon.as_type())
                .collect();
            anyhow::bail!(
                "--give {name:?} is not a weapon; the table holds {}",
                known.join(", ")
            )
        }
    }
}

/// Reads and parses `--input-script`'s file, for whichever leg is driving a
/// race from one - `--race --screenshot` and `--trace-out` both go through
/// this, so the two cannot drift the way they did before it existed: one read
/// `cli.input_script`, the other silently never did. See
/// `oag_trace::script::Script`.
pub(crate) fn input_script(
    path: Option<&std::path::Path>,
) -> Result<Option<oag_trace::script::Script>> {
    let Some(path) = path else { return Ok(None) };
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    oag_trace::script::Script::parse(&text)
        .map(Some)
        .map_err(|e| anyhow::anyhow!("parsing {}: {e}", path.display()))
}

/// Resolves `--autopilot-pilot`'s spelling to a pilot, against the same
/// roster [`oag_raceplay::Race::start`] deals the grid from.
///
/// **A missing or unreadable pilot directory falls back to the built-in
/// four**, the same way `Race::start` does - a player who has never
/// authored one should still be able to name `aggressive`. An unknown name is
/// rejected rather than ignored, for the reason [`give_weapon`] gives.
pub(crate) fn autopilot_pilot(name: Option<&str>) -> Result<Option<oag_ai::Pilot>> {
    let Some(name) = name else { return Ok(None) };
    let roster = oag_raceplay::pilots::load().unwrap_or_else(|e| {
        warn!("pilots: {e:#} - matching --autopilot-pilot against the built-in four");
        oag_raceplay::pilots::Roster::built_in()
    });
    match roster.find(name) {
        Some(entry) => Ok(Some(entry.pilot)),
        None => {
            let known: Vec<&str> = roster.entries().iter().map(|e| e.name.as_str()).collect();
            anyhow::bail!(
                "--autopilot-pilot {name:?} is not a pilot; the roster holds {}",
                known.join(", ")
            )
        }
    }
}

/// The campaign cell `--campaign-cell` names, found on whichever grid carries it
/// in `title`'s own archives, with the rung the race is judged at (`Medium`,
/// the one `Cell::evaluate_medal` always used). `None` when the flag is absent.
pub(crate) fn campaign_cell(
    options: &oag_raceplay::Options,
    title: &oag_title::Title,
    name: Option<&str>,
) -> Result<
    Option<(
        oag_tables::race_campaign::Cell,
        oag_tables::race_campaign::Difficulty,
    )>,
> {
    let Some(name) = name else {
        return Ok(None);
    };
    let Some(entry) = title.campaign.definition_entry else {
        anyhow::bail!(
            "--campaign-cell reads Pulse's and HD's grids; {} has none",
            title.name
        );
    };
    let mut opened = oag_source::title::open_source(&options.source, Vec::new(), Vec::new())?;
    let grids = oag_game::campaign::read_grids(&mut opened.archives, entry)?;
    let cell = grids
        .into_iter()
        .flat_map(|grid| grid.cells)
        .find(|cell| cell.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| anyhow::anyhow!("no campaign cell named {name:?}"))?;
    Ok(Some((cell, oag_tables::race_campaign::Difficulty::Medium)))
}
