//! Playing a rumble pattern on a real pad outside a race: the `--rumble-pattern`
//! probe, and the RUMBLE setting's mapping to a player's law.
//!
//! The probe is a feel test and a wiring proof for a test:
//! it opens the pad subsystem with no window, plays one pattern through
//! [`oag_input::rumble::Player`] and the `gilrs` backend, logs every level it
//! sent, and exits with the motors silent.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use oag_assets::Archives;
use oag_input::Pad;
use oag_input::rumble::{Mode, MotorOutput, Motors, Player, SmallMotor, StepContext};
use oag_tables::rumble::{Kind, Library, Pattern};
use oag_title::rumble::Rumble;

/// Spellings of the pattern directory across the titles' archives
/// (`docs/formats/rumble-xml.md`, "Census"); each archive looks entries up by
/// the digest of one exact path.
const DIRECTORIES: [&str; 5] = [
    "Data/XML/Rumble",
    "Data/xml/rumble",
    "Data/xml/Rumble",
    "data/xml/rumble",
    "data/xml/Rumble",
];

/// Reads the disc's pattern called `name`, trying each spelling of its path.
///
/// # Errors
///
/// No spelling resolves, or the file is refused by the reader.
pub fn disc_pattern(archives: &mut Archives, name: &str) -> Result<Pattern> {
    for directory in DIRECTORIES {
        let path = format!("{directory}/{name}.xml");
        for candidate in [path.replace('/', "\\"), path] {
            if let Ok(blob) = archives.read_name(&candidate) {
                return oag_tables::rumble::parse(&blob)
                    .map_err(|e| anyhow::anyhow!("{candidate}: refused: {e}"));
            }
        }
    }
    bail!("no rumble pattern called {name:?} on this disc")
}

/// The small-motor law the `RUMBLE` token plays on a title, or `None` for
/// nothing (`none`, or `original` on a title whose original has no patterns).
#[must_use]
pub fn law(token: &str, original: Rumble) -> Option<SmallMotor> {
    Mode::from_name(token)
        .unwrap_or_default()
        .law(original_law(original))
}

/// What the title's original does with the small motor, in the player's words.
#[must_use]
pub fn original_law(original: Rumble) -> Option<SmallMotor> {
    original.small_motor().map(|law| match law {
        oag_title::rumble::SmallMotor::OnOff => SmallMotor::OnOff,
        oag_title::rumble::SmallMotor::Sum => SmallMotor::Sum,
    })
}

/// What `--rumble-pattern` was asked to do.
#[derive(Debug, Clone, Copy)]
pub struct Probe<'a> {
    /// A disc pattern's name, or the path of a `.toml` pattern.
    pub pattern: &'a str,
    /// The source (disc image or folder) for a disc pattern.
    pub source: Option<&'a str>,
    /// The `[controls] rumble` token.
    pub token: &'a str,
    /// Seconds a looping pattern is held before it is finished.
    pub hold: f32,
}

/// Plays the pattern on every force-feedback pad, printing each level sent.
///
/// # Errors
///
/// No pattern, no force-feedback pad, or `RUMBLE` set to play nothing.
pub fn probe(probe: &Probe<'_>) -> Result<()> {
    let (library, original) = library(probe)?;
    let law = law(probe.token, original).with_context(|| {
        format!(
            "RUMBLE is {:?}, which plays nothing on this title; use ps3 or ps4",
            probe.token
        )
    })?;
    if let Some(p) = library.at(0) {
        println!(
            "pattern: {:?}, {} frames, loop {}..{}, large peak {} ({} non-zero), small on {}",
            p.kind,
            p.frames(),
            p.loop_start,
            p.loop_end,
            p.large.iter().max().copied().unwrap_or(0),
            p.large.iter().filter(|&&b| b != 0).count(),
            p.small.iter().filter(|&&b| b != 0).count(),
        );
    }
    let mut pad = Pad::new();
    pad.poll_players();
    println!("pads: {:?}", pad.names());
    let motors = pad.motor_pads();
    println!("force feedback: {motors:?}");
    if motors.is_empty() {
        bail!("no connected pad reports force feedback (see the log for gilrs' own message)");
    }
    let mut player = Player::new(library, law);
    let kind = player
        .library()
        .at(0)
        .map(|p| p.kind)
        .context("empty library")?;
    let held = (kind == Kind::Looping).then(|| player.hold(0)).flatten();
    if held.is_none() {
        player.one_shot(0, None);
    }
    let started = Instant::now();
    let ctx = StepContext {
        race: true,
        ..StepContext::default()
    };
    let mut last = Motors::OFF;
    let mut changes = 0u32;
    let mut finished = held.is_none();
    while player.active_voices() > 0 && started.elapsed() < Duration::from_secs(12) {
        let tick = Instant::now();
        pad.poll_players();
        if let Some(handle) = held
            && !finished
            && started.elapsed().as_secs_f32() >= probe.hold
        {
            player.finish(handle);
            finished = true;
        }
        let now = player.drive(ctx, 0, &mut pad);
        if now != last {
            changes += 1;
            println!(
                "{:7.3}s large {:3} small {:3}",
                started.elapsed().as_secs_f32(),
                now.large,
                now.small
            );
            last = now;
        }
        if let Some(rest) = Duration::from_micros(16_667).checked_sub(tick.elapsed()) {
            std::thread::sleep(rest);
        }
    }
    pad.silence();
    pad.poll_players();
    println!(
        "{changes} level changes in {:.2}s, motors silenced",
        started.elapsed().as_secs_f32()
    );
    Ok(())
}

fn library(probe: &Probe<'_>) -> Result<(Library, Rumble)> {
    let mut library = Library::new();
    if probe.pattern.ends_with(".toml") {
        let text = std::fs::read_to_string(probe.pattern)
            .with_context(|| format!("reading {}", probe.pattern))?;
        let pattern =
            oag_mod::rumble::parse(&text).map_err(|e| anyhow::anyhow!("{}: {e}", probe.pattern))?;
        library.insert("probe", pattern);
        return Ok((library, Rumble::Absent));
    }
    let source = oag_source::source::resolve(probe.source, None)?;
    let opened = oag_source::title::open_source(&source, Vec::new(), Vec::new())
        .with_context(|| format!("opening {source}"))?;
    let mut archives = opened.archives;
    library.insert(probe.pattern, disc_pattern(&mut archives, probe.pattern)?);
    Ok((library, opened.title.rumble))
}
