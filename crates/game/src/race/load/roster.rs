//! Which teams the grid may fly, when the caller named none itself.

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
        .and_then(|blob| oag_formats::fexml::text(&blob).map_err(|error| error.to_string()))
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
