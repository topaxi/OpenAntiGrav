//! The menu tree a title shows with its mods: what a boot builds and what a live MODS
//! switch or LANGUAGE pick builds again, so the two cannot differ.

use anyhow::Result;
use oag_ui::menu;
use oag_ui::strings;

use crate::mods::{Listing, Mods};
use crate::settings;

/// The menu tree for `title` with `mods`' rows, the MODS page's listing and the mod
/// campaigns and tracks: what a boot builds and what a live rebuild builds
/// again. Logs what each mod did.
///
/// # Errors
///
/// The vanilla tree or the extras are wrong, which is a build defect.
pub fn mods_menu(
    mods: &Mods,
    language: Option<&str>,
    title: &'static oag_title::Title,
    updated: bool,
) -> Result<(
    menu::Definition,
    Vec<Listing>,
    crate::campaign::custom::Installed,
)> {
    let built = mods.build(menu::BUILT_IN, language, Some(menu::For { title, updated }))?;
    for line in &built.report {
        log::info!("{line}");
    }
    let mut definition = built.definition;
    let mut listing = built.listing;
    // Surveyed only when a campaign names another disc: the same survey
    // Race Remix reads, by title slug.
    let discs = std::cell::OnceCell::new();
    let has_disc = |slug: &str| {
        discs
            .get_or_init(|| {
                crate::launcher::distinct_titles(&crate::launcher::survey(
                    &oag_source::source::candidates(),
                ))
                .iter()
                .filter_map(|candidate| match &candidate.state {
                    crate::launcher::State::Playable(title) => Some(title.slug()),
                    _ => None,
                })
                .collect::<Vec<_>>()
            })
            .iter()
            .any(|have| have == slug)
    };
    let installed = mods.installed_for(title, &has_disc);
    crate::campaign::custom::add_picker(
        &mut definition,
        &mut listing,
        &installed,
        title,
        &strings::project_table(language),
    );
    Ok((definition, listing, installed))
}

/// The title's trims of a tree [`mods_menu`] built: rows this title, platform or input
/// lacks, and the rows its own words rename. Boot and a live rebuild both run it, so a
/// rebuilt tree has the rows a booted one has.
pub fn finish_definition(
    definition: &mut menu::Definition,
    title: &'static oag_title::Title,
    strings: &oag_ui::language::StringTable,
) {
    // A no-op on every title but Wipeout Pure and Wipeout Pulse, whose
    // race defaults declare no variant axis at all - see
    // `menu::Definition::drop_unavailable_race_variant`.
    definition.drop_unavailable_race_variant(title);
    // The BOOST ZOOM row exists where a ring can be borrowed: Pulse and Pure.
    definition.drop_unavailable_boost_zoom(title);
    // And the TRAIL HITS row, where the title authors no such burst: Pulse and Pure.
    definition.drop_unavailable_trail_hits(title);
    definition.drop_unavailable_wingtip_contrails(title);
    // And the OWN SHIELD row, where the original draws a shield of its own look: HD, 2048, Omega.
    definition.drop_unavailable_own_shield(title);
    // And the HUD STYLE row, where the disc authors the list: HD and Omega.
    definition.drop_unavailable_hud_style(title);
    // And the DYNAMIC HUD row, where the options list it: Pulse.
    definition.drop_unavailable_dynamic_hud(title);
    definition.drop_unavailable_game_options(title);
    // And the PLAYER NAME and PLAYER TAG rows, where the title keeps them: see `oag_title::profile`.
    definition.drop_unavailable_player_rows(title);
    // And the TRAIL ORDER row, where the ribbon is the PSP preset: Pulse and Pure.
    definition.drop_unavailable_trail_order(title);
    definition.drop_unavailable_trail_length(title);
    // And the WEATHER row, where the weather is played: Pulse.
    definition.drop_unavailable_weather(title);
    // And the RUMBLE row's `original` value, where the original plays patterns: HD and Omega.
    definition.drop_unavailable_rumble_original(title);
    // And the three rows the race box's own screens pick, on a title
    // that authors them - see `menu::Definition::drop_rows_picked_on_screen`.
    definition.drop_rows_picked_on_screen(title);
    // The campaign row as the title's original words it (`SINGLE PLAYER` on
    // Pure) - see `menu::Definition::rename_campaign_row`.
    definition.rename_campaign_row(title, strings);
    // And the MULTIPLAYER row where the title's own is worded differently (Pulse's
    // `MULTIPLAYER & SHARING`) - see `menu::Definition::rename_multiplayer_row`.
    definition.rename_multiplayer_row(title, strings);
    // `auto` in the player's language; the other values are the setting's own
    // spellings.
    definition.relabel_choice_from(
        "audio.spatial_audio",
        "auto",
        strings,
        "OAG_AUDIO_SPATIAL_AUTO",
    );
    // A phone app is left, not quit - see `menu::Definition::drop_quit`.
    if oag_mobile::MOBILE {
        definition.drop_quit();
    }
    // A page has no monitor to pick and one present mode, and a desktop
    // has no canvas: see `settings::web`.
    if cfg!(target_arch = "wasm32") {
        definition.drop_settings(&settings::web::HIDDEN_ROWS);
        definition.drop_settings(&["display.window_size"]);
        // The browser's words for the two modes.
        definition.relabel_choice("display.window_mode", "borderless", "FULLSCREEN");
    } else {
        definition.drop_settings(&settings::web::WEB_ONLY_ROWS);
    }
    // A phone's window is the whole panel: its monitor, mode and size rows have no choice.
    definition.drop_settings(settings::platform::hidden_rows(oag_mobile::MOBILE));
    // Only a touchscreen has an overlay to set up.
    if !crate::touch_controls::available() {
        definition.drop_touch_controls();
    }
}

/// The tree, listing and mod content a live rebuild starts from: the mods read again off
/// `bases` (the search path) with the mods `switches` turn off turned off, `mods_menu` for `language`, then the title's trims
/// with `strings`, the new language's own words. What a boot builds, bar the disc reads.
///
/// # Errors
///
/// The mods cannot be searched, or the vanilla tree is wrong ([`mods_menu`]).
pub fn rebuilt(
    bases: &[std::path::PathBuf],
    switches: &crate::settings::Mods,
    language: Option<&str>,
    title: &'static oag_title::Title,
    updated: bool,
    strings: &oag_ui::language::StringTable,
) -> Result<(
    menu::Definition,
    Vec<Listing>,
    crate::campaign::custom::Installed,
)> {
    let loaded = Mods::load_from(bases, switches)?;
    let (mut definition, listing, installed) = mods_menu(&loaded, language, title, updated)?;
    finish_definition(&mut definition, title, strings);
    Ok((definition, listing, installed))
}
