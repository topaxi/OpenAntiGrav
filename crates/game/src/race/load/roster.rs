//! Which teams the grid may fly, when the caller named none itself.

/// The team the player races as, resolved to `craft_title`'s own default
/// when the caller named none - the first point the title is known, and
/// craft content, so it follows `craft_title` rather than the track's own
/// title. See `race::Options::team`.
///
/// Here rather than in `load.rs` under the 1,000-line rule in
/// `scripts/check-file-size.py`; a move, with no behaviour change, and this
/// is the file that already answers "which team".
pub(super) fn resolve_team(
    asked: Option<&str>,
    craft_title: &'static oag_title::Title,
    report: &mut Vec<String>,
) -> String {
    match asked {
        Some(asked) => asked.to_string(),
        None => {
            report.push(format!(
                "no team named: {}'s own default, {}",
                craft_title.name, craft_title.race.team
            ));
            craft_title.race.team.to_string()
        }
    }
}

/// One hull, plume and nozzle per grid slot, each off its own team's
/// directory - see [`crate::livery`], which also records what is recovered
/// here (the paths) and what is this project's (which team flies which
/// slot). Slot 0 is the player's.
///
/// The caller's own `opponent_teams` wins, because only the caller can know
/// about mounted DLC packs - the front end passes the whole catalogue. When
/// it is empty the disc's own plugin definition is read here instead, which
/// is what makes `--race`, a capture and a test field a varied grid rather
/// than eight identical ships; an entry point that forgot to pass the list is
/// exactly how this was first found.
///
/// **The definition is `title`'s own**, and reaching for Pulse's constant
/// regardless of title is one of the two ways this same grid went uniform on
/// an HD source: HD names its game plugin where the PSP titles number it, so
/// the read missed. See [`oag_title::Title::plugin_definition`]. This is
/// craft content - a Race Remix passes `craft_title`'s archives here, not the
/// track's.
///
/// **The other way was `fexml::expand` rather than `fexml::text`**, and it is
/// the trap that page already records for the PS2 in-race HUD: shortening is
/// per *file*, not per platform, and HD's definition is plain `<?xml`.
/// `expand` refuses a file with no `<code>` dictionary, so a perfectly good
/// document came back as `NoDictionary` and the roster was empty again - with
/// the path already fixed. `text` decides from the blob.
///
/// Both failures used to read out as `0 team(s)`, which is why the reason is
/// reported now: an empty roster and an unreadable one are the same grid and
/// very different bugs.
pub(super) fn available(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    opponent_teams: &[String],
    report: &mut Vec<String>,
) -> Vec<String> {
    if !opponent_teams.is_empty() {
        return opponent_teams.to_vec();
    }
    let definition = title.plugin_definition;
    let declared = archives
        .read_name(definition)
        .map_err(|error| error.to_string())
        .and_then(|blob| oag_tables::fexml::text(&blob).map_err(|error| error.to_string()))
        .map(|xml| crate::catalogue::teams(&xml));
    let declared = match declared {
        Ok(declared) => {
            report.push(format!(
                "{definition}: {} team(s) for the grid, read here because the caller passed \
                 none",
                declared.len()
            ));
            declared
        }
        Err(error) => {
            report.push(format!(
                "{definition}: {error} - no team declared, so the whole grid wears the \
                 player's hull"
            ));
            Vec::new()
        }
    };
    declared.into_iter().map(|team| team.id).collect()
}

/// Wipeout HD's red-trail flag per grid slot, from each slot's own ship
/// directory.
///
/// The runtime sets it by the model-variant name
/// (`concept1`/`nitro`/`detonator`/`chrome_c1`), and on the disc those
/// variants live in `*_c1`, `*_n1` and `detonator` directories. See
/// `Loaded::hd_trail_red`.
///
/// Here rather than in `load.rs` under the 1,000-line rule in
/// `scripts/check-file-size.py`; a move, with no behaviour change, and this is
/// the file that already answers "which team flies which slot".
pub(super) fn hd_trail_red(slot_teams: &[String]) -> [f32; oag_gameplay::MAX_SHIPS] {
    std::array::from_fn(|slot| {
        let fury = slot_teams.get(slot).is_some_and(|team| {
            team.ends_with("_c1") || team.ends_with("_n1") || team == "detonator"
        });
        if fury { 1.0 } else { 0.0 }
    })
}

/// Everything the grid's own slot list decides: who flies where, what each
/// one draws, and HD's per-slot trail flag.
pub(super) struct Grid {
    /// Which team flies each slot, the player first. **This project's
    /// ordering, not the original's** - see [`crate::livery`].
    pub slot_teams: Vec<String>,
    /// Wipeout HD's red-trail flag per slot; see [`hd_trail_red`].
    pub hd_trail_red: [f32; oag_gameplay::MAX_SHIPS],
    /// One hull, plume, nozzle and shield per slot.
    pub liveries: Vec<crate::livery::Livery>,
}

/// Fills the grid: slot teams out of `available`, then a [`crate::livery`] per
/// slot off `archives`.
///
/// Here rather than in `load.rs` under the 1,000-line rule in
/// `scripts/check-file-size.py`, and this is the file that already answers
/// "which team flies which slot" - `available` above is its own first half. A
/// move with no behaviour change, taken when the player's paint job joined the
/// player's hull override as a second per-slot-0 pick.
///
/// **Two things here are the player's alone**, and both land on slot 0 because
/// nothing offers an opponent a choice of their own: `hull_variant`, which
/// swaps the model file ([`oag_title::race::HullVariant`]), and
/// [`crate::race::Options::skin`], which repaints the model this slot just
/// built. The second is resolved here against what the source's own definition
/// declares for the team - see [`crate::livery::ship_skin`], and note that
/// **which** skin a race flies is this project's choice rather than the
/// original's.
///
/// # Errors
///
/// Propagates [`crate::livery::load`]: only the player's own hull failing to
/// read or build, which is not a race.
pub(super) fn grid(
    archives: &mut oag_assets::Archives,
    craft_title: &'static oag_title::Title,
    options: &crate::race::Options,
    team: &str,
    available: &[String],
    hull_variant: Option<&str>,
    report: &mut Vec<String>,
) -> anyhow::Result<Grid> {
    let skin = crate::livery::ship_skin::resolve(
        archives,
        craft_title,
        team,
        options.skin.as_deref(),
        options.mode,
        report,
    );
    let slot_teams = crate::livery::teams_for_slots(team, available, oag_gameplay::MAX_SHIPS);
    let liveries = crate::livery::load(
        archives,
        &slot_teams,
        &crate::livery::LoadContext {
            race: craft_title.race,
            mode: options.mode,
            flare: craft_title.flare,
            lod: options.lod,
            hull_overlay: oag_render::hull_overlay::DRAWN
                && craft_title.name == oag_pulse::TITLE.name,
        },
        hull_variant,
        skin.as_deref(),
        report,
    )?;
    report.push(format!(
        "grid liveries: {} - which team flies which slot is this project's, not \
         the original's (livery.rs)",
        slot_teams.join(", ")
    ));
    Ok(Grid {
        hd_trail_red: hd_trail_red(&slot_teams),
        slot_teams,
        liveries,
    })
}
