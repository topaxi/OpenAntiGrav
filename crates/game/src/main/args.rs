//! The small parsers and resolvers behind individual flags.
//!
//! Each turns one command-line string, or one settings-over-flag pair, into
//! the type the rest of the run wants. Together they are what [`crate::cli`]
//! declares and nothing interprets.

use anyhow::{Result, ensure};
use log::warn;

use oag_game::input;
use oag_game::{loading, movie, prefetch, settings};
use oag_gameplay::ControlScheme;

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
            "{spec:?} is not a load step; write it as cached, decoding, \
             or transcoding:DONE/TOTAL (TOTAL may be omitted)"
        )
    };
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

/// Resolves `--give`'s spelling to a weapon, case-insensitively.
///
/// Rejects an unknown name rather than ignoring it: a silent no-op here looks
/// exactly like the flag working and the weapon never being drawn, which is the
/// failure mode the flag exists to remove. The error lists what the weapon table
/// actually holds, so a typo is one read away from fixed.
pub(crate) fn give_weapon(name: Option<&str>) -> Result<Option<oag_formats::weapons::Weapon>> {
    let Some(name) = name else { return Ok(None) };
    let found = oag_formats::weapons::Weapon::ALL
        .into_iter()
        .find(|weapon| weapon.as_type().eq_ignore_ascii_case(name));
    match found {
        Some(weapon) => Ok(Some(weapon)),
        None => {
            let known: Vec<&str> = oag_formats::weapons::Weapon::ALL
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
