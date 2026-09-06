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
    let roster = load_from(Path::new("/nonexistent/oag/pilots")).expect("absent is not an error");
    assert_eq!(roster.len(), 4);
    assert_eq!(roster.entries()[0].name, "balanced");
}

/// The count [`AXES`] carries has to track [`Pilot::spans`] exactly - not one
/// short, which is what a landed axis nobody wired here looks like: a menu
/// row and a template that silently cover fifteen of eighteen. Whoever adds
/// the next axis to `oag_ai::Pilot` fails this test if they forget the one
/// list it also belongs in here.
#[test]
fn axis_names_cover_every_draw() {
    assert_eq!(AXES.len(), Pilot::BALANCED.spans().len());
}

/// The whole point of `toml_edit` over `settings.rs`'s
/// `toml::to_string_pretty`: a hand-written comment must still be there after
/// the row it documents is edited and saved.
#[test]
fn editing_an_axis_keeps_the_hand_written_comment_above_it() {
    let text = "\
# How hard this pilot corners. Careful with this one.
commitment = [0.95, 1.00]

# Untouched axis, with its own note.
ram = [0.1, 0.2]
";
    let saved = set_axis(text, "commitment", 0.97, 1.02).expect("a valid edit");
    assert!(
        saved.contains("# How hard this pilot corners. Careful with this one."),
        "{saved}"
    );
    assert!(
        saved.contains("# Untouched axis, with its own note."),
        "{saved}"
    );
    // And the edit actually took: this is not merely "nothing was touched".
    assert!(saved.contains("[0.97, 1.02]"), "{saved}");

    // The saved text is still a pilot file the loader accepts, and it reads
    // back exactly the span that was written - the "won't write a file the
    // loader rejects next launch" half of the requirement.
    let entry = parse("re-read", &saved).expect("still a valid pilot file");
    assert_eq!(entry.pilot.commitment, Span::new(0.97, 1.02));
    assert_eq!(entry.pilot.ram, Span::new(0.1, 0.2));
}

/// `1.05_f32 as f64` is `1.0499999523162842`. Writing that instead of `1.05`
/// is the same hostility as eating a comment, one line over - the number a
/// player wrote is gone even though the file still "round-trips".
#[test]
fn editing_an_axis_writes_the_f32s_own_short_text_not_an_f64_expansion() {
    let saved = set_axis("commitment = [0.9, 1.0]\n", "commitment", 0.9, 1.05).expect("valid");
    assert!(saved.contains("1.05"), "{saved}");
    assert!(!saved.contains("1.0499999523162842"), "{saved}");
}

/// An axis absent from the file is appended, so editing something `balanced`
/// alone provides still ends up written down explicitly.
#[test]
fn editing_an_axis_not_yet_in_the_file_adds_it() {
    let saved = set_axis("commitment = [0.9, 1.0]\n", "ram", 0.2, 0.4).expect("valid");
    let entry = parse("added", &saved).expect("still a valid pilot file");
    assert_eq!(entry.pilot.ram, Span::new(0.2, 0.4));
}

/// A typo must fail rather than silently clamp against `limit`'s fallback
/// range for the multiplier axes.
#[test]
fn editing_an_unknown_axis_is_rejected_by_name() {
    let error =
        set_axis("look = [0.9, 1.0]\n", "commitment_typo", 0.5, 0.6).expect_err("must be rejected");
    assert!(format!("{error:#}").contains("commitment_typo"));
}

/// The editor clamps at the point of editing rather than writing a file
/// [`parse`] would reject on the next launch.
#[test]
fn editing_an_axis_clamps_into_its_own_limit() {
    let saved = set_axis("commitment = [0.9, 1.0]\n", "commitment", 0.9, 5.0).expect("valid");
    let entry = parse("clamped", &saved).expect("clamped, so still valid");
    assert_eq!(entry.pilot.commitment.high, Pilot::MAX_COMMITMENT);
}

/// A two-handle control that got dragged past itself still saves a span the
/// right way round, not one [`parse`] rejects as "runs backwards".
#[test]
fn editing_an_axis_with_low_and_high_swapped_reorders_them() {
    let saved = set_axis("look = [0.9, 1.0]\n", "look", 1.1, 0.9).expect("valid");
    let entry = parse("swapped", &saved).expect("reordered, so still valid");
    assert_eq!(entry.pilot.look, Span::new(0.9, 1.1));
}

/// [`template`] is what a first edit of a built-in, or create-from-template,
/// starts from - every axis has to be there explicitly, or the other
/// seventeen would quietly fall back to `balanced` the moment this is saved.
#[test]
fn a_template_names_every_axis_explicitly() {
    let text = template(&Pilot::AGGRESSIVE);
    let entry = parse("templated", &text).expect("a template is a valid pilot file");
    assert_eq!(entry.pilot, Pilot::AGGRESSIVE);
    for (name, _) in AXES {
        assert!(text.contains(name), "{name} missing from template:\n{text}");
    }
}

/// Reading a pilot with no file on disk yet - the ordinary case for a
/// built-in nobody has retuned - is a plain [`anyhow::Error`], not a panic,
/// so the caller can fall back to [`template`].
#[test]
fn reading_a_missing_pilot_file_is_an_error_not_a_panic() {
    let scratch = Scratch::with(&[]);
    assert!(read_pilot_text(&scratch.0, "nobody-home").is_err());
}

/// The write path creates the directory on the first pilot saved on this
/// machine, the same as `settings::save` does for the settings file.
#[test]
fn writing_a_pilot_creates_the_directory_if_it_is_missing() {
    let scratch = Scratch::with(&[]);
    // A directory `Scratch` never created - `write_pilot` has to make it.
    let dir = scratch.0.join("nested");
    write_pilot(&dir, "winston", "look = [0.9, 1.0]\n").expect("a fresh directory is not fatal");
    assert_eq!(
        std::fs::read_to_string(dir.join("winston.toml")).expect("the file exists"),
        "look = [0.9, 1.0]\n"
    );
}
