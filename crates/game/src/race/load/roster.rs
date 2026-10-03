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

/// Overlays [`crate::race::Options::grid_teams`] onto `slot_teams` - the
/// disc's own authored AI grid, where an event carries one, in place of
/// [`crate::livery::teams_for_slots`]'s "chosen, not measured" placement.
///
/// **Per-slot, not all-or-nothing.** `grid_teams[i]` is grid slot `i + 1`
/// (slot `0` stays the player's, always); a `None` entry, or an index past
/// what `grid_teams` carries, leaves that one slot exactly what
/// `teams_for_slots` already gave it - so an event that authors only some
/// of its seven slots (measured: two of `SP.xml`'s own events author only
/// their first) overrides only those, and the fallback still answers for
/// the rest, labelled here rather than left to look like measured data.
fn apply_grid_teams(
    slot_teams: &mut [String],
    grid_teams: &[Option<String>],
    report: &mut Vec<String>,
) {
    if grid_teams.is_empty() {
        return;
    }
    let ai_slots = oag_gameplay::MAX_SHIPS - 1; // slot 0 is always the player's own.
    let mut overridden = 0usize;
    for (offset, over) in grid_teams.iter().enumerate() {
        let slot = offset + 1;
        let Some(team_id) = over else { continue };
        let Some(dest) = slot_teams.get_mut(slot) else {
            continue;
        };
        *dest = team_id.clone();
        overridden += 1;
    }
    report.push(format!(
        "grid: {overridden} of {ai_slots} AI slot(s) authored by the event's own \
         M_PGRIDSHIPMODELDATA; {} left on the chosen, not measured, fallback",
        ai_slots - overridden,
    ));
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
    let mut slot_teams = crate::livery::teams_for_slots(team, available, oag_gameplay::MAX_SHIPS);
    apply_grid_teams(&mut slot_teams, &options.grid_teams, report);
    let liveries = crate::livery::load(
        archives,
        &slot_teams,
        &crate::livery::LoadContext {
            race: craft_title.race,
            mode: options.mode,
            flare: craft_title.flare,
            hull_overlay: oag_render::hull_overlay::DRAWN
                && craft_title.name == oag_pulse::TITLE.name,
            hull_shine: oag_render::shine::DRAWN
                && options.hull_shine
                && craft_title.name == oag_pulse::TITLE.name,
            hull_wreck: options.hull_wreck
                && craft_title.name == oag_pulse::TITLE.name
                && archives.layout.platform == oag_assets::Platform::Psp,
            absorb_shell: craft_title.name == oag_hd::TITLE.name,
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

/// The grid's own team roster, independent of a live [`Loaded`](crate::race::Loaded) race -
/// `grid`'s own [`available`]/[`crate::livery::teams_for_slots`]
/// pair, for a caller reached after the `Race` this leg built is already
/// gone. Used by a Tournament leg's `EndRace Results` standings
/// (`crate::main::session::endrace::build_endrace`), which needs to know
/// which team flew which grid slot to label a row - `slot_teams` itself is
/// never stored on `Loaded`/`Race`, so this is a second, independent call
/// rather than a threaded-through field.
///
/// **Deterministic in the same inputs.** `roster::available` reads the
/// title's own plugin definition off the disc (no randomness), and
/// `teams_for_slots` is a pure function of `(team, available, slots)` - so
/// this reproduces the exact roster a tournament's own first-leg launch
/// built, as long as the caller passes the same `team`/`opponent_teams`
/// every leg, which a Tournament's own relaunch (`Session::advance_tournament_leg`)
/// does: neither is touched between legs.
///
/// Here rather than in `load.rs` under the 1,000-line rule in
/// `scripts/check-file-size.py`; a move, with no behaviour change.
///
/// Not a `Race Remix` path - always the single archive set a Tournament
/// launch (campaign or Racebox) opens, never [`crate::remix::craft_of`]'s
/// split case.
pub fn slot_teams(
    archives: &mut oag_assets::Archives,
    craft_title: &'static oag_title::Title,
    team: &str,
    opponent_teams: &[String],
) -> Vec<String> {
    let mut report = Vec::new();
    let available = available(archives, craft_title, opponent_teams, &mut report);
    crate::livery::teams_for_slots(team, &available, oag_gameplay::MAX_SHIPS)
}

#[cfg(test)]
mod tests {
    use super::apply_grid_teams;

    fn slots(teams: &[&str]) -> Vec<String> {
        teams.iter().map(|t| (*t).to_string()).collect()
    }

    #[test]
    fn empty_grid_teams_changes_nothing() {
        let mut slot_teams = slots(&["a", "b", "c"]);
        let mut report = Vec::new();
        apply_grid_teams(&mut slot_teams, &[], &mut report);
        assert_eq!(slot_teams, slots(&["a", "b", "c"]));
        assert!(report.is_empty(), "no line when there is nothing to say");
    }

    /// The full-grid case: every AI slot (index 0 of `grid_teams` is grid
    /// slot 1 - slot 0 is always the player) gets overwritten, in order.
    #[test]
    fn every_authored_slot_overrides_its_own_ai_slot() {
        let mut slot_teams = slots(&["player", "x", "x", "x", "x", "x", "x", "x"]);
        let grid_teams = vec![
            Some("Feisar2048\\1".to_string()),
            Some("Qirex2048\\2".to_string()),
            Some("Feisar2048\\1".to_string()),
            Some("Qirex2048\\2".to_string()),
            Some("Feisar2048\\1".to_string()),
            Some("Qirex2048\\2".to_string()),
            Some("Feisar2048\\1".to_string()),
        ];
        let mut report = Vec::new();
        apply_grid_teams(&mut slot_teams, &grid_teams, &mut report);
        assert_eq!(
            slot_teams,
            slots(&[
                "player",
                "Feisar2048\\1",
                "Qirex2048\\2",
                "Feisar2048\\1",
                "Qirex2048\\2",
                "Feisar2048\\1",
                "Qirex2048\\2",
                "Feisar2048\\1",
            ])
        );
        assert!(report[0].contains("7 of 7"));
    }

    /// A `None` slot, and a slot the caller's `grid_teams` does not reach at
    /// all, both keep whatever `teams_for_slots` already placed there -
    /// never overwritten with anything invented.
    #[test]
    fn unresolved_and_unauthored_slots_keep_the_fallback() {
        let mut slot_teams = slots(&["player", "fallback-1", "fallback-2", "fallback-3"]);
        let grid_teams = vec![None, Some("Qirex2048\\2".to_string())];
        let mut report = Vec::new();
        apply_grid_teams(&mut slot_teams, &grid_teams, &mut report);
        assert_eq!(
            slot_teams,
            slots(&["player", "fallback-1", "Qirex2048\\2", "fallback-3"]),
            "slot 1 stayed on the fallback (None), slot 2 was overridden, \
             slot 3 was never named by grid_teams at all"
        );
        assert!(report[0].contains("1 of 7"), "{}", report[0]);
    }
}
