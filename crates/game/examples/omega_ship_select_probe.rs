//! Scratch probe: the teams Omega's Ship Select offers, per team what its
//! honeycomb rows resolve to, and the size of the hull its preview draws.
//!
//! ```sh
//! cargo run -p oag-game --example omega_ship_select_probe -- data/extracted/ps4
//! ```

fn main() -> anyhow::Result<()> {
    let source = std::env::args().nth(1).expect("a source path");
    let options = oag_game::boot::Options {
        source,
        language: None,
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-omega-ship-select-probe"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(1),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    };
    let (shell, _, title) = oag_game::boot::load_shell(&options)?;
    println!("{}: {} team(s)", title.name, shell.teams.len());
    for team in &shell.teams {
        let ids: Vec<&str> = title
            .race
            .team_variants_for(&team.id)
            .map(|table| table.variants.iter().map(|v| v.suffix).collect())
            .unwrap_or_default();
        let cells = team.hex_cells(ids.iter().copied());
        let open = cells.iter().filter(|cell| cell.open).count();
        println!(
            "  {:<12} {:<44} models {} open {} variants {:?}",
            team.id,
            team.location,
            team.models.len(),
            open,
            ids
        );
    }
    let mut archives =
        oag_source::title::open_source(&options.source, Vec::new(), Vec::new())?.archives;
    let Some(hull) = title
        .front_end
        .and_then(|front_end| front_end.ship_preview_hull)
    else {
        return Ok(());
    };
    for team in &shell.teams {
        let entry = format!(r"{}\{hull}", team.location);
        let mut model = oag_game::preview::psp2_scene::model(&mut archives, &entry)?;
        let raw = (model.centre, model.radius);
        oag_game::preview::frame_hull(&mut model);
        println!(
            "  {:<12} raw centre {:.2?} radius {:.2}; framed centre {:.2?} radius {:.2}",
            team.id, raw.0, raw.1, model.centre, model.radius
        );
    }
    Ok(())
}
