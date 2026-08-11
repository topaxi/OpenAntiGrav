//! Pilots authored outside the tree, in `<config dir>/oag/pilots/<name>.toml`.
//!
//! # Why this is here and not in `oag-ai`
//!
//! `oag-ai` depends on `oag-core` and `oag-physics` and on nothing else, which
//! is what lets a driver be tested against a synthetic circle with no disc
//! image, no asset crate and no GPU. `include_str!` plus serde would put a TOML
//! parser inside a gameplay crate to save one indirection. So the four built-in
//! pilots stay Rust constants in `oag_ai::Pilot`, and this - the composition
//! root, which already owns `toml`, `serde` and `dirs` for
//! [`crate::settings`] - is the only thing that reads a file.
//!
//! # A file is ranges, not values
//!
//! Each axis is a two-element array, `[low, high]`, and a craft draws inside
//! it. One number is not offered as a shorthand: a pilot whose every axis is
//! fixed is one driver wearing eight hulls, which is the thing the whole
//! personality system exists to prevent, and making that the *easy* thing to
//! write would be an odd choice. `[0.4, 0.4]` still says it, for anyone who
//! means it.
//!
//! Anything a file leaves out comes from `balanced`, so the shortest useful
//! pilot is two lines.
//!
//! # Where this deliberately differs from `settings.toml`
//!
//! - **Unknown keys are an error.** `settings.rs` tolerates them and migrates;
//!   here a typo'd axis that silently does nothing is precisely the frustration
//!   this feature exists to remove, and there is no migration history to
//!   protect.
//! - **Nothing is rewritten on load.** `settings.rs` rewrites its file every run
//!   so every key is discoverable; these are hand-authored files with the
//!   author's own comments in them, and clobbering those would be hostile.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use oag_ai::{Lean, Pilot, Span};
use serde::Deserialize;

/// Where user pilots live: `<config dir>/oag/pilots/`.
///
/// `None` on a platform `dirs` cannot place a config directory on, exactly as
/// [`crate::settings::path`] handles the same case.
#[must_use]
pub fn directory() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("oag").join("pilots"))
}

/// One pilot as a file spells it, before any of it has been checked.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct PilotFile {
    line_bias: Option<[f32; 2]>,
    lean: Option<String>,
    wander: Option<[f32; 2]>,
    wander_period: Option<[f32; 2]>,
    look: Option<[f32; 2]>,
    commitment: Option<[f32; 2]>,
    patience: Option<[f32; 2]>,
    trail: Option<[f32; 2]>,
    width: Option<[f32; 2]>,
    inside: Option<[f32; 2]>,
    courtesy: Option<[f32; 2]>,
    defence: Option<[f32; 2]>,
    caution: Option<[f32; 2]>,
    ram: Option<[f32; 2]>,
    provocation_ticks: Option<[f32; 2]>,
    trigger: Option<[f32; 2]>,
}

/// The hard limits every axis is checked against, whatever a file asks for.
///
/// **`commitment`'s upper bound is the most important number in this module.**
/// It is the only thing standing between a hand-written file and an opponent
/// that corners faster than the physics allows, which is not a faster driver -
/// it is a driver in the wall, on someone else's machine, in a race they did
/// not author. See [`Pilot::MAX_COMMITMENT`].
fn limit(axis: &str) -> (f32, f32) {
    match axis {
        "commitment" => (0.5, Pilot::MAX_COMMITMENT),
        // A bias is a fraction of the corridor, and the corridor's own edge is
        // the backstop; more than all of it is meaningless rather than
        // dangerous.
        "line_bias" | "wander" | "inside" | "courtesy" | "defence" | "caution" | "ram"
        | "trigger" => (-1.0, 1.0),
        // Ticks. Ten seconds of grudge is plenty and a negative one is nonsense.
        "provocation_ticks" => (0.0, 600.0),
        // Ticks per drift step. Below a handful the wander is a twitch.
        "wander_period" => (30.0, 3_600.0),
        // Multipliers on a tuning this project measured. Wide, but not a way to
        // turn the controller into a different one.
        _ => (0.1, 3.0),
    }
}

/// Checks one axis and turns it into a [`Span`].
fn span(axis: &str, given: Option<[f32; 2]>, fallback: Span) -> Result<Span> {
    let Some([low, high]) = given else {
        return Ok(fallback);
    };
    if !low.is_finite() || !high.is_finite() {
        bail!("`{axis}` is not a number");
    }
    // **Checked, because a reversed span still draws** - and lands outside the
    // range it appears to state, with nothing complaining.
    if low > high {
        bail!("`{axis}` runs backwards: [{low}, {high}]");
    }
    let (min, max) = limit(axis);
    if low < min || high > max {
        bail!("`{axis}` is [{low}, {high}], outside the allowed [{min}, {max}]");
    }
    Ok(Span::new(low, high))
}

/// One entry the grid may draw from.
#[derive(Debug, Clone)]
pub struct Entry {
    /// What to call it, from the file stem.
    pub name: String,
    /// The pilot itself.
    pub pilot: Pilot,
    /// A fingerprint of every number in [`Entry::pilot`].
    ///
    /// See `oag_ai::Driver::pilot` for why this is hashed into the world.
    pub digest: u32,
    /// Whether this came out of a file rather than out of the tree.
    ///
    /// Only used to say so on the console: a player who has authored a pilot
    /// wants to see that it was picked up, and a player who has not wants to
    /// see that nothing unexpected was.
    pub from_file: bool,
}

/// A fingerprint of a pilot's **resolved numbers**, never of its name.
///
/// A name-only digest is the silent divergence this exists to prevent: two
/// files both called `winston` saying different things would agree, which is
/// exactly the case a fingerprint is for.
#[must_use]
pub fn digest(pilot: &Pilot) -> u32 {
    let mut hasher = oag_core::hash::StateHasher::new();
    for span in pilot.spans() {
        hasher.write_f32(span.low);
        hasher.write_f32(span.high);
    }
    hasher.write_u8(match pilot.lean {
        Lean::Either => 0,
        Lean::Left => 1,
        Lean::Right => 2,
    });
    let full = hasher.finish();
    // Folded rather than truncated, so both halves reach the field.
    (full as u32) ^ ((full >> 32) as u32)
}

/// Parses one pilot.
///
/// **No filesystem.** Split out so the whole of the validation is unit-testable
/// without going near a real config directory - which is also what keeps the
/// suite from reading the developer's own pilots and passing on one machine
/// only.
pub fn parse(name: &str, text: &str) -> Result<Entry> {
    let file: PilotFile = toml::from_str(text).with_context(|| format!("parsing pilot {name}"))?;
    let base = Pilot::BALANCED;

    let lean = match file.lean.as_deref() {
        None => base.lean,
        Some("either") => Lean::Either,
        Some("left") => Lean::Left,
        Some("right") => Lean::Right,
        Some(other) => bail!("pilot {name}: `lean` is {other:?}, not `either`, `left` or `right`"),
    };

    // One context for the whole build, so every axis error names the pilot it
    // came from - a message saying only "`commitment` is out of range" is not
    // one a player with six pilot files can act on.
    let pilot = (|| -> Result<Pilot> {
        Ok(Pilot {
            line_bias: span("line_bias", file.line_bias, base.line_bias)?,
            lean,
            wander: span("wander", file.wander, base.wander)?,
            wander_period: span("wander_period", file.wander_period, base.wander_period)?,
            look: span("look", file.look, base.look)?,
            commitment: span("commitment", file.commitment, base.commitment)?,
            patience: span("patience", file.patience, base.patience)?,
            trail: span("trail", file.trail, base.trail)?,
            width: span("width", file.width, base.width)?,
            inside: span("inside", file.inside, base.inside)?,
            courtesy: span("courtesy", file.courtesy, base.courtesy)?,
            defence: span("defence", file.defence, base.defence)?,
            caution: span("caution", file.caution, base.caution)?,
            ram: span("ram", file.ram, base.ram)?,
            provocation_ticks: span(
                "provocation_ticks",
                file.provocation_ticks,
                base.provocation_ticks,
            )?,
            trigger: span("trigger", file.trigger, base.trigger)?,
        })
    })()
    .and_then(|pilot| pilot.validated().map_err(|why| anyhow::anyhow!("{why}")))
    .with_context(|| format!("pilot {name}"))?;

    Ok(Entry {
        name: name.to_owned(),
        digest: digest(&pilot),
        pilot,
        from_file: true,
    })
}

/// Every pilot a race may field: the four built-ins, then whatever the
/// directory holds.
#[derive(Debug, Clone)]
pub struct Roster {
    entries: Vec<Entry>,
}

impl Roster {
    /// Just the four that ship.
    #[must_use]
    pub fn built_in() -> Self {
        Self {
            entries: Pilot::BUILT_IN
                .iter()
                .map(|(name, pilot)| Entry {
                    name: (*name).to_owned(),
                    pilot: *pilot,
                    digest: digest(pilot),
                    from_file: false,
                })
                .collect(),
        }
    }

    /// Everything in it, built-ins first.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// How many there are, for `oag_ai::pilot_for_slot`.
    #[must_use]
    pub fn len(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Whether it is empty. Never is - the built-ins are always there.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// One line naming every pilot this race may field.
    ///
    /// A file-loaded pilot is marked, and every entry carries its digest -
    /// which is the only thing that says *which* pilot file differs when two
    /// machines disagree on a world hash, since thirty-two bits cannot be
    /// inverted. See `oag_ai::Driver::pilot`.
    #[must_use]
    pub fn summary(&self) -> String {
        self.entries
            .iter()
            .map(|entry| {
                let mark = if entry.from_file { "*" } else { "" };
                format!("{}{mark} ({:08x})", entry.name, entry.digest)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Reads a directory of pilots.
///
/// **Sorted by file stem, byte-wise.** `read_dir` yields in the filesystem's own
/// order, which differs between machines and even between runs, and that order
/// would reach simulation state through `oag_ai::pilot_for_slot` - see
/// `docs/architecture/determinism.md` on iteration order.
///
/// A missing directory is not an error: it is the ordinary case of a player who
/// has never authored one.
pub fn load_from(dir: &Path) -> Result<Roster> {
    let mut roster = Roster::built_in();
    let listing = match std::fs::read_dir(dir) {
        Ok(listing) => listing,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(roster),
        Err(e) => return Err(e).with_context(|| format!("reading {}", dir.display())),
    };

    let mut files: Vec<PathBuf> = Vec::new();
    for entry in listing {
        let path = entry
            .with_context(|| format!("reading {}", dir.display()))?
            .path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            files.push(path);
        }
    }
    files.sort();

    for path in files {
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_owned();
        if name.is_empty() {
            continue;
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let entry = parse(&name, &text).with_context(|| format!("in {}", path.display()))?;
        // A file may replace a built-in by taking its name, which is how a
        // player retunes `aggressive` without editing the tree.
        match roster
            .entries
            .iter_mut()
            .find(|held| held.name == entry.name)
        {
            Some(held) => *held = entry,
            None => roster.entries.push(entry),
        }
    }
    Ok(roster)
}

/// The real thing: [`load_from`] the [`directory`], or the built-ins alone.
pub fn load() -> Result<Roster> {
    match directory() {
        Some(dir) => load_from(&dir),
        None => Ok(Roster::built_in()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pilot_file_round_trips_into_the_ranges_it_declares() {
        let entry = parse("winston", "commitment = [0.95, 1.02]\nram = [0.1, 0.4]\n")
            .expect("a well-formed pilot");
        assert_eq!(entry.name, "winston");
        assert_eq!(entry.pilot.commitment, Span::new(0.95, 1.02));
        assert_eq!(entry.pilot.ram, Span::new(0.1, 0.4));
        // Everything unstated comes from balanced.
        assert_eq!(entry.pilot.look, Pilot::BALANCED.look);
    }

    #[test]
    fn an_empty_pilot_file_is_the_balanced_pilot() {
        let entry = parse("plain", "").expect("an empty pilot");
        assert_eq!(entry.pilot, Pilot::BALANCED);
    }

    /// The number that stops a file making an opponent faster than the physics.
    #[test]
    fn a_pilot_with_a_commitment_no_hull_can_hold_is_rejected_by_name() {
        let error = parse("cheat", "commitment = [1.0, 5.0]\n").expect_err("must be rejected");
        let message = format!("{error:#}");
        assert!(message.contains("commitment"), "{message}");
        assert!(message.contains("cheat"), "{message}");
    }

    #[test]
    fn a_pilot_whose_range_runs_backwards_is_rejected() {
        let error = parse("backwards", "look = [1.2, 0.8]\n").expect_err("must be rejected");
        assert!(format!("{error:#}").contains("backwards"), "{error:#}");
    }

    /// A typo'd axis that silently did nothing is the frustration this feature
    /// exists to remove.
    #[test]
    fn an_unknown_key_in_a_pilot_file_is_an_error_and_names_itself() {
        let error = parse("typo", "comittment = [1.0, 1.0]\n").expect_err("must be rejected");
        assert!(format!("{error:#}").contains("comittment"), "{error:#}");
    }

    #[test]
    fn a_malformed_pilot_file_names_the_pilot_it_came_from() {
        let error = parse("broken", "look = [").expect_err("must be rejected");
        assert!(format!("{error:#}").contains("broken"), "{error:#}");
    }

    #[test]
    fn a_lean_that_is_not_a_side_is_rejected() {
        let error = parse("sideways", r#"lean = "sideways""#).expect_err("must be rejected");
        assert!(format!("{error:#}").contains("lean"), "{error:#}");
        assert_eq!(
            parse("port", "lean = \"left\"")
                .expect("left is a side")
                .pilot
                .lean,
            Lean::Left
        );
    }

    #[test]
    fn two_pilots_that_differ_in_one_number_digest_differently() {
        let a = parse("a", "look = [0.9, 1.1]\n").expect("valid");
        let b = parse("b", "look = [0.9, 1.2]\n").expect("valid");
        assert_ne!(a.digest, b.digest);
        // And the name is not part of it: two files saying the same thing agree.
        let c = parse("c", "look = [0.9, 1.1]\n").expect("valid");
        assert_eq!(a.digest, c.digest);
    }

    /// `Driver::default` carries `0`, and a plain line-follower has to stay
    /// distinguishable from a craft flying a real pilot.
    #[test]
    fn no_built_in_pilot_digests_to_zero() {
        for entry in Roster::built_in().entries() {
            assert_ne!(entry.digest, 0, "{} digests to zero", entry.name);
        }
    }

    /// A directory of pilots, cleaned up afterwards.
    ///
    /// **Never [`directory`].** Every test in this module goes through
    /// [`parse`] or [`load_from`] with a path it made itself, so the suite
    /// cannot read the developer's own pilots and pass on one machine only.
    struct Scratch(PathBuf);

    impl Scratch {
        fn with(files: &[(&str, &str)]) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "oag-pilots-{}-{:p}",
                std::process::id(),
                files as *const _
            ));
            std::fs::create_dir_all(&dir).expect("a scratch directory");
            for (name, body) in files {
                std::fs::write(dir.join(name), body).expect("a scratch pilot");
            }
            Self(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// `read_dir` yields in the filesystem's own order, which differs between
    /// machines - and that order would reach simulation state through
    /// `oag_ai::pilot_for_slot`.
    #[test]
    fn the_roster_is_sorted_by_name_and_not_by_the_filesystem() {
        let scratch = Scratch::with(&[
            ("zara.toml", "look = [0.9, 1.0]\n"),
            ("adrian.toml", "look = [0.9, 1.0]\n"),
            ("winston.toml", "look = [0.9, 1.0]\n"),
        ]);
        let roster = load_from(&scratch.0).expect("a readable directory");
        let names: Vec<&str> = roster
            .entries()
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        // Built-ins first, in their own declared order, then the files sorted.
        assert_eq!(
            names,
            [
                "balanced",
                "aggressive",
                "passive",
                "shy",
                "adrian",
                "winston",
                "zara"
            ]
        );
    }

    /// How a player retunes `aggressive` without editing the tree.
    #[test]
    fn a_file_may_replace_a_built_in_by_taking_its_name() {
        let scratch = Scratch::with(&[("aggressive.toml", "commitment = [0.9, 0.91]\n")]);
        let roster = load_from(&scratch.0).expect("a readable directory");
        assert_eq!(roster.len(), 4, "a replacement must not also be appended");
        let replaced = roster
            .entries()
            .iter()
            .find(|entry| entry.name == "aggressive")
            .expect("still there");
        assert_eq!(replaced.pilot.commitment, Span::new(0.9, 0.91));
        assert_ne!(replaced.digest, digest(&Pilot::AGGRESSIVE));
    }

    #[test]
    fn a_file_that_is_not_toml_is_ignored_rather_than_parsed() {
        let scratch = Scratch::with(&[("notes.txt", "this is not a pilot")]);
        assert_eq!(load_from(&scratch.0).expect("readable").len(), 4);
    }

    #[test]
    fn a_broken_pilot_file_names_the_path_it_came_from() {
        let scratch = Scratch::with(&[("bent.toml", "look = [1.5, 0.5]\n")]);
        let error = load_from(&scratch.0).expect_err("must be rejected");
        let message = format!("{error:#}");
        assert!(message.contains("bent.toml"), "{message}");
    }

    #[test]
    fn an_absent_pilot_directory_yields_exactly_the_four_built_ins() {
        let roster =
            load_from(Path::new("/nonexistent/oag/pilots")).expect("absent is not an error");
        assert_eq!(roster.len(), 4);
        assert_eq!(roster.entries()[0].name, "balanced");
    }
}
