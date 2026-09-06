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
//!
//! # Nothing is rewritten on *save*, either - the in-game editor included
//!
//! `settings.rs` saves with `toml::to_string_pretty`
//! ([`crate::settings::save`]), which serialises a struct and emits canonical
//! TOML: every comment, every blank line, every hand-chosen key order, gone.
//! That is fine for a settings file this build owns end to end and rewrites on
//! every launch anyway - it is not fine for a pilot file, which is meant to be
//! read and edited by a person, and which the in-game editor
//! ([`set_axis`]) now also writes to.
//!
//! **`set_axis` goes through `toml_edit`, never `toml::to_string_pretty`, and
//! must not start doing the latter.** It reads the document, mutates only the
//! array the edited axis names, and writes the rest of the file back byte for
//! byte - a hand-authored comment above `commitment` survives a player nudging
//! that same row in the menu. This is the maintainer's pick of the three ways
//! out that were on the table - the other two were an editor confined to
//! files it created itself, and a save-as that kept the original untouched
//! and multiplied files; neither was taken.
//!
//! [`rename_pilot`] keeps the same promise for free and more strongly: it is
//! a **move**, so the file arrives at its new name byte for byte because
//! nothing ever read it. [`delete_pilot`] is the one operation here that is
//! not about text at all - and the one whose *meaning* is not what its name
//! suggests, because a file named after a built-in was replacing it and
//! removing the file puts the built-in back. See [`is_built_in_name`].

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

/// The longest a pilot name may be, in characters.
///
/// **Chosen, not measured.** Nothing on any disc authors a pilot name - the
/// whole concept is this project's - so there is no original to read a limit
/// off. Twenty-four is what fits the `PILOT` row's own value column at the
/// measured 480x272 row scale without the marquee having to scroll it, which
/// is a presentation argument rather than a filesystem one; every filesystem
/// this build runs on allows far more.
pub const MAX_NAME: usize = 24;

/// Whether `name` is one of the four pilots the binary itself ships.
///
/// **The question the delete and rename wording turns on.** A file called
/// `aggressive.toml` does not *add* a pilot, it **replaces** the built-in of
/// that name for as long as it exists - so deleting it restores the built-in
/// rather than removing a pilot, and renaming some other file *onto* that
/// name silently takes the built-in's place. Both are surprising unless the
/// menu says so, which is what this is for. See [`Pilot::BUILT_IN`].
#[must_use]
pub fn is_built_in_name(name: &str) -> bool {
    Pilot::BUILT_IN
        .iter()
        .any(|(built_in, _)| *built_in == name)
}

/// Checks a name a player typed, **before** it is ever joined onto a path.
///
/// [`write_pilot`], [`rename_pilot`] and [`delete_pilot`] all build a path
/// with `dir.join(format!("{name}.toml"))`, and every one of them is public.
/// So the invariant lives here rather than in whichever on-screen keyboard
/// happens to be offering the characters: a charset the keyboard cannot type
/// is one caller's guard, not a guarantee about the argument.
///
/// The character set is deliberately narrow - **lowercase** ASCII letters,
/// digits, `-` and `_`. That rules out a path separator, a `..`, a leading
/// dot (`.toml` is a hidden file with an empty stem, which [`load_from`]
/// skips and a player would never see again) and a name whose spelling only
/// differs by case, which is two distinct pilots on Linux and one file
/// clobbering another on macOS or Windows. Excluding uppercase is **chosen,
/// not measured**, on the same footing as [`MAX_NAME`].
///
/// # Errors
///
/// With a sentence naming what is wrong, meant to be shown to the player
/// rather than only logged - a rename that silently does nothing is the
/// failure this whole error path exists to avoid.
pub fn check_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("a pilot needs a name");
    }
    if name.chars().count() > MAX_NAME {
        bail!("a pilot name is at most {MAX_NAME} characters");
    }
    if let Some(bad) = name
        .chars()
        .find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-' || *c == '_'))
    {
        bail!("{bad:?} cannot be used in a pilot name: only a-z, 0-9, - and _");
    }
    Ok(())
}

/// Renames one pilot's file, **keeping every byte of it**.
///
/// A move, not a read-parse-write: the file arrives at its new name with its
/// comments, its blank lines, its key order and its exact number spellings
/// untouched, because nothing ever looked at them. That is the strongest form
/// of the promise [`set_axis`] makes more carefully for an edit, and it is
/// free here.
///
/// # Errors
///
/// If `to` is not a name [`check_name`] allows; if `from` has no file (an
/// untouched built-in lives in the binary and there is nothing on disk to
/// move - the caller should refuse before reaching this); or if `to` is
/// already taken. The last one is the reason this is a check-then-move rather
/// than a bare `std::fs::rename`, which **silently overwrites** the
/// destination on Unix and would lose a file the player still wanted.
pub fn rename_pilot(dir: &Path, from: &str, to: &str) -> Result<()> {
    check_name(to)?;
    if from == to {
        return Ok(());
    }
    let source = dir.join(format!("{from}.toml"));
    if !source.is_file() {
        bail!("{from} has no file of its own to rename");
    }
    let destination = dir.join(format!("{to}.toml"));
    if destination.exists() {
        bail!("there is already a pilot called {to}");
    }
    std::fs::rename(&source, &destination)
        .with_context(|| format!("renaming {} to {}", source.display(), destination.display()))
}

/// Deletes one pilot's file.
///
/// **This does not always delete a pilot.** For a file whose name is one
/// [`is_built_in_name`] answers yes to, the file was *replacing* the built-in
/// of that name, so removing it puts the built-in back - the pilot goes on
/// existing, with the numbers the binary ships. Whoever calls this owes the
/// player that sentence; this function cannot say it.
///
/// # Errors
///
/// If `name` has no file - an untouched built-in, or a pilot the list has got
/// out of step with - or if the file cannot be removed.
pub fn delete_pilot(dir: &Path, name: &str) -> Result<()> {
    let path = dir.join(format!("{name}.toml"));
    if !path.is_file() {
        bail!("{name} has no file of its own to delete");
    }
    std::fs::remove_file(&path).with_context(|| format!("deleting {}", path.display()))
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
    roll_chance: Option<[f32; 2]>,
    roll_floor: Option<[f32; 2]>,
    roll_airtime: Option<[f32; 2]>,
}

/// The hard limits every axis is checked against, whatever a file asks for.
///
/// **`commitment`'s upper bound is the most important number in this module.**
/// It is the only thing standing between a hand-written file and an opponent
/// that corners faster than the physics allows, which is not a faster driver -
/// it is a driver in the wall, on someone else's machine, in a race they did
/// not author. See [`Pilot::MAX_COMMITMENT`].
pub fn limit(axis: &str) -> (f32, f32) {
    match axis {
        "commitment" => (0.5, Pilot::MAX_COMMITMENT),
        // A bias is a fraction of the corridor, and the corridor's own edge is
        // the backstop; more than all of it is meaningless rather than
        // dangerous.
        "line_bias" | "wander" | "inside" | "courtesy" | "defence" | "caution" | "ram"
        | "trigger" => (-1.0, 1.0),
        // Both are fractions: a probability per airborne window, and a share
        // of the shield pool. A floor of one is a pilot that never rolls,
        // which is a thing somebody may reasonably want to write.
        //
        // **No floor-of-the-floor, unlike `commitment` above, and the two
        // failures are not the same shape.** A `commitment` over the cap is a
        // craft that corners faster than the physics allows - broken, on
        // someone else's machine, in a race they did not author. A
        // `roll_floor` of zero is a pilot that spends its own shield down to
        // the recovered `cost < shield` and gets destroyed by the first wall:
        // a worse opponent, which is the author's own business, and every
        // recovered gate still holds around it. Measured, so this is a
        // decision rather than an oversight - at floors near zero an
        // aggressive Ace finished `09_Track` on 6.5 of 95, which is why no
        // built-in goes near it. See `docs/gameplay/ai.md`.
        "roll_chance" | "roll_floor" => (0.0, 1.0),
        // Seconds of flight. Ten is longer than any jump on the disc, so the
        // top of this range is also a way of saying "never".
        "roll_airtime" => (0.0, 10.0),
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

/// Every axis this format understands, in the frozen draw order
/// [`Pilot::spans`] uses, paired with the accessor that reads it off a
/// resolved [`Pilot`].
///
/// **The one list an axis is added to.** The in-game editor's `AXIS` row and
/// [`template`] both walk this rather than repeating the names by hand, so
/// there is one place - not three - that falls out of step with
/// [`Pilot::spans`] when a new axis lands. `axis_names_cover_every_draw`
/// checks the count against it directly, which is what turns a landed axis
/// nobody wired here into a failing test instead of a silent gap.
pub type Axis = (&'static str, fn(&Pilot) -> Span);

/// See [`Axis`].
pub const AXES: &[Axis] = &[
    ("line_bias", |p| p.line_bias),
    ("wander", |p| p.wander),
    ("wander_period", |p| p.wander_period),
    ("look", |p| p.look),
    ("commitment", |p| p.commitment),
    ("patience", |p| p.patience),
    ("trail", |p| p.trail),
    ("width", |p| p.width),
    ("inside", |p| p.inside),
    ("courtesy", |p| p.courtesy),
    ("defence", |p| p.defence),
    ("caution", |p| p.caution),
    ("ram", |p| p.ram),
    ("provocation_ticks", |p| p.provocation_ticks),
    ("trigger", |p| p.trigger),
    // Draws 17, 18 and 19, appended after `trigger` in that order when the
    // AI barrel roll landed. This list is display order, not draw order -
    // but keeping the two agreeing costs nothing and makes a future
    // append obviously correct. See `crates/ai/src/pilot.rs`.
    ("roll_chance", |p| p.roll_chance),
    ("roll_floor", |p| p.roll_floor),
    ("roll_airtime", |p| p.roll_airtime),
];

/// The accessor for one axis, or `None` if `axis` is not one [`AXES`] names.
///
/// Distinct from calling [`limit`] directly: `limit` has a catch-all fallback
/// range for the multiplier axes, so it cannot by itself tell a real axis from
/// a typo - this can, and every write path below checks it first.
#[must_use]
pub fn axis_accessor(axis: &str) -> Option<fn(&Pilot) -> Span> {
    AXES.iter()
        .find(|(name, _)| *name == axis)
        .map(|(_, accessor)| *accessor)
}

/// Edits one axis of a pilot file **without disturbing anything else in it**.
///
/// Reads `text` as a document rather than through [`parse`], mutates only the
/// array `axis` names, and writes the rest back byte for byte through
/// `toml_edit` - see this module's own doc for why that is not negotiable.
/// A `low`/`high` given backwards is reordered and both ends are clamped into
/// [`limit`]'s range for `axis` **here**, at the point of editing, rather than
/// by writing a file [`parse`] would reject on the next launch.
///
/// The written number is `low`/`high`'s own `f32::to_string` - the shortest
/// text that reads back to the same bits - never a `f32`-to-`f64` conversion:
/// `1.05_f32 as f64` is `1.0499999523162842`, and writing that would vandalise
/// a hand-authored `1.05` one axis over from the comment this function exists
/// to keep.
///
/// # Errors
///
/// If `axis` is not one of [`AXES`] - a typo would otherwise silently clamp
/// against [`limit`]'s fallback range rather than failing - or if `text` does
/// not parse as TOML.
pub fn set_axis(text: &str, axis: &str, low: f32, high: f32) -> Result<String> {
    if axis_accessor(axis).is_none() {
        bail!("{axis:?} is not a pilot axis");
    }
    let (min, max) = limit(axis);
    let mut low = low.clamp(min, max);
    let mut high = high.clamp(min, max);
    if low > high {
        std::mem::swap(&mut low, &mut high);
    }

    let mut doc: toml_edit::DocumentMut = text.parse().context("parsing pilot file")?;
    let low_value: toml_edit::Value = format!("{low}")
        .parse()
        .expect("a formatted f32 is a valid TOML float");
    let high_value: toml_edit::Value = format!("{high}")
        .parse()
        .expect("a formatted f32 is a valid TOML float");

    let existing = doc
        .get_mut(axis)
        .and_then(toml_edit::Item::as_array_mut)
        .filter(|array| array.len() == 2);
    match existing {
        // Replaced in place, which keeps the array's own decor - the comma
        // and space between the two numbers, and any comment sharing the
        // line - and touches only the two numbers themselves.
        Some(array) => {
            array.replace(0, low_value);
            array.replace(1, high_value);
        }
        // No existing array to preserve the shape of: this axis was not in
        // the file at all, or was malformed enough not to look like one.
        // Either way a fresh two-element array, in this format's own default
        // style, is the honest thing to write.
        None => {
            let mut array = toml_edit::Array::new();
            array.push_formatted(low_value);
            array.push_formatted(high_value);
            array.fmt();
            doc[axis] = toml_edit::Item::Value(toml_edit::Value::Array(array));
        }
    }

    Ok(doc.to_string())
}

/// A brand-new pilot file, spelling out every axis [`AXES`] names off `pilot`
/// explicitly.
///
/// **Every axis, not just the ones that differ from `balanced`.** A file that
/// left the rest out would parse fine on its own - [`parse`]'s whole point is
/// that an absent axis falls back to `balanced` - but that is exactly wrong
/// for a file created from an existing pilot: writing only the one axis a
/// player just edited on, say, `aggressive` would leave every *other* axis
/// silently pulled from `balanced` instead of kept at what `aggressive` itself
/// flies. So this is what [`set_axis`] starts from the first time a pilot with
/// no file yet is saved, not an empty document.
///
/// Used for both halves of the editor: retuning a built-in for the first time,
/// and create-from-template.
#[must_use]
pub fn template(pilot: &Pilot) -> String {
    let mut out = String::from(
        "# Written by the in-game pilot editor. Every axis is a range, [low, high];\n\
         # each craft flying this pilot draws its own value inside it.\n\n",
    );
    let lean = match pilot.lean {
        Lean::Either => "either",
        Lean::Left => "left",
        Lean::Right => "right",
    };
    out.push_str(&format!("lean = {lean:?}\n"));
    for (name, accessor) in AXES {
        let span = accessor(pilot);
        out.push_str(&format!("{name} = [{}, {}]\n", span.low, span.high));
    }
    out
}

/// Reads one pilot's raw text, for the editor to hand to [`set_axis`].
///
/// **No filesystem beyond the one read.** Distinct from [`load_from`], which
/// parses every file in a directory into a checked [`Entry`]: the editor wants
/// the bytes as written, comments included, not what they resolve to.
///
/// # Errors
///
/// If the file cannot be read - including "does not exist yet", which is the
/// ordinary case for a built-in with no file: the caller falls back to
/// [`template`] rather than treating this as fatal.
pub fn read_pilot_text(dir: &Path, name: &str) -> Result<String> {
    let path = dir.join(format!("{name}.toml"));
    std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
}

/// Writes one pilot's text to disk, creating `dir` if this is the first pilot
/// saved on this machine.
///
/// **Never panics on a write failure.** A missing config directory is not the
/// only way this can fail - the directory can exist and not be writable,
/// which [`directory`] cannot see coming - so both are an [`Err`] the caller
/// must show the player, the same way a race that fails to load is reported
/// rather than unwound.
///
/// # Errors
///
/// If `dir` cannot be created, or the file cannot be written.
pub fn write_pilot(dir: &Path, name: &str, text: &str) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let path = dir.join(format!("{name}.toml"));
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))
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
            roll_chance: span("roll_chance", file.roll_chance, base.roll_chance)?,
            roll_floor: span("roll_floor", file.roll_floor, base.roll_floor)?,
            roll_airtime: span("roll_airtime", file.roll_airtime, base.roll_airtime)?,
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

// Moved to its own file under the 200-line cap on an inline `#[cfg(test)]`
// module (`scripts/check-file-size.py`) once the editor's own tests joined
// these - `use super::*;` still reaches every private item.
#[cfg(test)]
mod tests;
