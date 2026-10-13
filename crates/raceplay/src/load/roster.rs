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
/// directory - see [`oag_livery`], which also records what is recovered
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

/// Overlays [`crate::Options::grid_teams`] onto `slot_teams` - the
/// disc's own authored AI grid, where an event carries one, in place of
/// [`oag_livery::teams_for_slots`]'s draw.
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

/// Puts each seated person's team in their own slot and keeps every team on
/// the grid once.
///
/// `picks[i]` is slot `i + 1`'s; slot `0` is the player's and untouched. The
/// grid was drawn for one person ([`oag_livery::teams_for_slots`] keeps the
/// player's team out of the opponents' and flies every other team once), so a
/// seated person who took a team the draw gave an opponent would meet
/// themselves: the opponent that held it takes a team a person's pick
/// displaced instead. **A person may take the player's own team, or another
/// person's**, and then two craft wear it - the draw never offered it to an
/// opponent, so there is nobody to move (**chosen, not measured**). No picks is
/// no change at all, and the draw's own report line stands.
pub(super) fn apply_seat_picks(
    slot_teams: &mut [String],
    picks: &[crate::SeatPick],
    report: &mut Vec<String>,
) {
    let humans = (1 + picks.len()).min(slot_teams.len());
    if humans <= 1 {
        return;
    }
    let mut spare: std::collections::VecDeque<String> =
        slot_teams[1..humans].iter().cloned().collect();
    for (offset, pick) in picks.iter().enumerate().take(humans - 1) {
        slot_teams[offset + 1].clone_from(&pick.team);
    }
    let taken: Vec<String> = slot_teams[..humans].to_vec();
    spare.retain(|team| !taken.contains(team));
    for slot in humans..slot_teams.len() {
        if taken.contains(&slot_teams[slot])
            && let Some(team) = spare.pop_front()
        {
            slot_teams[slot] = team;
        }
    }
    report.push(format!(
        "seat picks: slot(s) 1..{humans} fly {} (their own choice); the opponents keep every other team once",
        picks
            .iter()
            .map(|pick| pick.team.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ));
}

/// Everything the grid's own slot list decides: who flies where, what each
/// one draws, and HD's per-slot trail flag.
pub(super) struct Grid {
    /// Which team flies each slot, the player first - Pulse's own draw, see
    /// [`oag_livery::teams_for_slots`].
    pub slot_teams: Vec<String>,
    /// Wipeout HD's red-trail flag per slot; see [`hd_trail_red`].
    pub hd_trail_red: [f32; oag_gameplay::MAX_SHIPS],
    /// One hull, plume, nozzle and shield per slot.
    pub liveries: Vec<oag_livery::Livery>,
}

/// The shell tint a craft of `craft_title`, loaded from a disc of `platform`,
/// takes. The craft's source, not the track's: the shell is loaded from the
/// craft's archives (`livery::shield::shell`), so a Race Remix with a PSP craft
/// on a PS2 circuit must still tint.
pub(super) fn shield_palette(
    craft_title: &oag_title::Title,
    platform: oag_assets::Platform,
) -> oag_render::shield::Palette {
    match craft_title.looks.shield_palette.on(platform) {
        oag_title::ShieldPalette::Hd => oag_render::shield::HD_PALETTE,
        oag_title::ShieldPalette::Ps2Pulse => oag_render::shield::PS2_PULSE_PALETTE,
        oag_title::ShieldPalette::Pulse => oag_render::shield::PULSE_PALETTE,
    }
}

/// Fills the grid: slot teams out of `available`, then a [`oag_livery`] per
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
/// [`crate::Options::skin`], which repaints the model this slot just
/// built. The second is resolved here against what the source's own definition
/// declares for the team - see [`oag_livery::ship_skin`], and note that
/// **which** skin a race flies is this project's choice rather than the
/// original's.
///
/// # Errors
///
/// Propagates [`oag_livery::load`]: only the player's own hull failing to
/// read or build, which is not a race.
pub(super) fn grid(
    archives: &mut oag_assets::Archives,
    craft_title: &'static oag_title::Title,
    options: &crate::Options,
    team: &str,
    available: &[String],
    hull_variant: Option<&str>,
    report: &mut Vec<String>,
) -> anyhow::Result<Grid> {
    let skin = super::skin::resolve(
        archives,
        craft_title,
        team,
        options.skin.as_deref(),
        options.mode,
        report,
    );
    let zone_liveries = super::zone_livery::resolve(archives, craft_title, options.mode, report);
    let seed = options.seed.unwrap_or(crate::SEED);
    let pool = super::family::pool(craft_title.race, team, available);
    let mut slot_teams = oag_livery::teams_for_slots(team, &pool, oag_gameplay::MAX_SHIPS, seed);
    apply_grid_teams(&mut slot_teams, &options.grid_teams, report);
    apply_seat_picks(&mut slot_teams, &options.seat_picks, report);
    super::family::follow_player(
        craft_title.race,
        &mut slot_teams,
        options.seat_picks.len(),
        report,
    );
    let seat_skins: Vec<Option<String>> = options
        .seat_picks
        .iter()
        .map(|pick| {
            super::skin::resolve(
                archives,
                craft_title,
                &pick.team,
                pick.skin.as_deref(),
                options.mode,
                report,
            )
        })
        .collect();
    let seat_paints: Vec<oag_livery::Paint<'_>> = options
        .seat_picks
        .iter()
        .zip(&seat_skins)
        .map(|(pick, skin)| oag_livery::Paint {
            hull_variant: pick.hull_variant.as_deref(),
            skin: skin.as_deref(),
        })
        .collect();
    let liveries = oag_livery::load(
        archives,
        &slot_teams,
        &oag_livery::LoadContext {
            race: craft_title.race,
            mode: options.mode,
            flare: craft_title.flare,
            hull_overlay: oag_fx::hull_overlay::DRAWN
                && craft_title.looks.hull_overlay.applies_everywhere(),
            hull_shine: oag_render::shine::DRAWN
                && options.hull_shine
                && craft_title.looks.hull_shine.applies_everywhere(),
            hull_wreck: options.hull_wreck
                && craft_title
                    .looks
                    .hull_wreck
                    .applies(archives.layout.platform),
            absorb_shell: craft_title.looks.absorb_shell.applies_everywhere(),
            zone_liveries: &zone_liveries,
        },
        hull_variant,
        skin.as_deref(),
        &seat_paints,
        report,
    )?;
    report.push(format!(
        "grid liveries: {} - Pulse's roster draw (RaceSession_DrawAiRoster, grid.md) off the race \
         seed {seed:#x}; the original seeds it from the wall clock",
        slot_teams.join(", ")
    ));
    Ok(Grid {
        hd_trail_red: hd_trail_red(&slot_teams),
        slot_teams,
        liveries,
    })
}

#[cfg(test)]
mod tests {
    use super::{apply_grid_teams, apply_seat_picks};
    use crate::SeatPick;

    fn pick(team: &str) -> SeatPick {
        SeatPick {
            team: team.to_string(),
            hull_variant: None,
            skin: None,
        }
    }

    /// The draw gave slot 3 the team a person then took: slot 3 takes the team
    /// that person's slot held, so every team is still flown once.
    #[test]
    fn a_seat_pick_moves_the_opponent_that_held_it() {
        let mut slot_teams = slots(&["p0", "a", "b", "c", "d"]);
        let mut report = Vec::new();
        apply_seat_picks(&mut slot_teams, &[pick("c")], &mut report);
        assert_eq!(slot_teams, slots(&["p0", "c", "b", "a", "d"]));
        assert!(report[0].contains("slot(s) 1..2 fly c"), "{}", report[0]);
    }

    /// A pick nobody held (a team the draw left off the grid) just lands.
    #[test]
    fn a_seat_pick_the_grid_never_held_just_lands() {
        let mut slot_teams = slots(&["p0", "a", "b", "c"]);
        apply_seat_picks(&mut slot_teams, &[pick("z")], &mut Vec::new());
        assert_eq!(slot_teams, slots(&["p0", "z", "b", "c"]));
    }

    /// Two people may take the same team, and may take the player's.
    #[test]
    fn people_may_share_a_team_and_the_opponents_do_not_double_up() {
        let mut slot_teams = slots(&["p0", "a", "b", "c", "d"]);
        apply_seat_picks(&mut slot_teams, &[pick("p0"), pick("c")], &mut Vec::new());
        assert_eq!(slot_teams, slots(&["p0", "p0", "c", "a", "d"]));
        let mut both = slots(&["p0", "a", "b", "c"]);
        apply_seat_picks(&mut both, &[pick("c"), pick("c")], &mut Vec::new());
        assert_eq!(both[..3], slots(&["p0", "c", "c"])[..], "{both:?}");
        assert!(!both[3..].contains(&"c".to_string()), "{both:?}");
    }

    /// One person is an ordinary race: no change, no report line.
    #[test]
    fn no_seat_picks_changes_nothing() {
        let mut slot_teams = slots(&["p0", "a", "b"]);
        let mut report = Vec::new();
        apply_seat_picks(&mut slot_teams, &[], &mut report);
        assert_eq!(slot_teams, slots(&["p0", "a", "b"]));
        assert!(report.is_empty());
    }

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
