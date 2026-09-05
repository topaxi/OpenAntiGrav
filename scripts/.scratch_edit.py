import pathlib

# 1. One place that spells "what ladder does this file actually carry", so the
#    two callers that report a missing rung do not each grow a formatter.
p = pathlib.Path("crates/formats/src/handling.rs")
s = p.read_text()
old = """    /// Whether this file carries exactly Pulse's four-class ladder."""
new = '''    /// The rungs this file authors, in ladder order, as a readable list.
    ///
    /// For the one thing a caller does when [`Self::class_named`] returns
    /// `None`: say what the file *does* carry. A message naming only the rung
    /// that was missing sends the reader to the wrong file.
    #[must_use]
    pub fn ladder(&self) -> String {
        self.classes
            .iter()
            .map(|block| block.raw_name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Whether this file carries exactly Pulse's four-class ladder.'''
assert old in s
s = s.replace(old, new, 1)
p.write_text(s)

# 2. `race::load` uses it, and the global-class note gets terser.
p = pathlib.Path("crates/game/src/race/load.rs")
s = p.read_text()
old = '''    //
    // Looked up by **name**, not by an enum discriminant, so a title whose
    // ladder is not Pulse's is served by its own file: Wipeout Pure authors a
    // fifth `<GlobalClass name="VECTOR">` and this reaches it. A rung the file
    // does not author is `None` and is reported - never quietly filled from a
    // neighbouring rung, which would be racing on borrowed numbers.
    let global_class = global
        .as_ref()
        .and_then(|global| global.class_named(&options.class));
    if global.is_some() && global_class.is_none() {
        report.push(format!(
            "{}: authors no <GlobalClass name=\\"{}\\"> - this run gets no speed \\
             pads, unscaled gravity and no weapon-pad debounce rather than \\
             another class's numbers",
            handling::GLOBAL_ENTRY,
            options.class
        ));
    }
    let pad_tunables = match global_class {'''
new = '''    //
    // Looked up by **name**, so a title whose ladder is not Pulse's is served by
    // its own file: Pure authors a fifth `<GlobalClass name="VECTOR">` and this
    // reaches it. A rung the file does not author is `None` and is reported -
    // never filled from a neighbouring rung, which is racing on borrowed
    // numbers.
    let global_class = global
        .as_ref()
        .and_then(|global| global.class_named(&options.class));
    if global.is_some() && global_class.is_none() {
        report.push(format!(
            "{}: no <GlobalClass name=\\"{}\\">, so no speed pads, unscaled \\
             gravity and no weapon-pad debounce",
            handling::GLOBAL_ENTRY,
            options.class
        ));
    }
    let pad_tunables = match global_class {'''
assert old in s
s = s.replace(old, new, 1)

old_h = '''    // `None` where this team's file does not author the requested rung, which
    // is a race that cannot be set up rather than one to run on a substitute:
    // every other rung's numbers are somebody else's tuning.
    let handling = handling_for(&stats, &options.class, pad_tunables, special).with_context(|| {
        format!(
            "{stats_name} authors no <Class name=\\"{}\\"> - it carries {}",
            options.class,
            stats
                .classes
                .iter()
                .map(|block| block.raw_name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;'''
new_h = '''    // `None` where this team's file does not author the requested rung: a race
    // that cannot be set up, rather than one run on somebody else's tuning.
    let handling = handling_for(&stats, &options.class, pad_tunables, special).with_context(
        || {
            let (class, ladder) = (&options.class, stats.ladder());
            format!("{stats_name} authors no <Class name=\\"{class}\\"> - it carries {ladder}")
        },
    )?;'''
assert old_h in s
s = s.replace(old_h, new_h, 1)
p.write_text(s)

# 3. `oag-trace`: one validator instead of three copies, and the same helper.
p = pathlib.Path("crates/trace/src/main.rs")
s = p.read_text()
old_v = """    // Spell-checked against every measured ladder, then resolved by name
    // against the file that authored the rung - see `oag_title::SpeedClasses`.
    let class = args.class.trim();
    anyhow::ensure!(
        oag_title::SpeedClasses::is_measured_name(class),
        "{:?} is not a speed class",
        args.class
    );"""
new_v = """    let class = checked_class(&args.class)?;"""
n = s.count(old_v)
assert n == 3, n
s = s.replace(old_v, new_v)

old_e = '''    let handling = handling_for(&stats, class, pad_tunables, special).with_context(|| {
        format!(
            "{stats_name} authors no <Class name=\\"{class}\\"> - it carries {}",
            stats
                .classes
                .iter()
                .map(|block| block.raw_name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;'''
new_e = '''    let handling = handling_for(&stats, class, pad_tunables, special).with_context(|| {
        format!(
            "{stats_name} authors no <Class name=\\"{class}\\"> - it carries {}",
            stats.ladder()
        )
    })?;'''
assert old_e in s
s = s.replace(old_e, new_e, 1)

# the validator itself, placed just before `fn load`
old_load = """/// captures, so the layout appears only in an error's context.
fn load("""
new_load = """/// Spell-checks `--class` against every measured ladder.
///
/// Not the check that decides a race: which rungs *this* disc authors is
/// settled by the files themselves, in [`load`]. This one exists so a typo is
/// a message about the command line rather than a failure eight seconds in.
/// See `oag_title::SpeedClasses::MEASURED`.
fn checked_class(raw: &str) -> Result<&str> {
    let class = raw.trim();
    anyhow::ensure!(
        oag_title::SpeedClasses::is_measured_name(class),
        "{raw:?} is not a speed class"
    );
    Ok(class)
}

/// captures, so the layout appears only in an error's context.
fn load("""
assert old_load in s
s = s.replace(old_load, new_load, 1)
p.write_text(s)
print("ok")
