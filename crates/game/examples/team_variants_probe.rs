//! Scratch probe: does `VariantJoin::combine` actually produce a resolving
//! id for every team and every variant on both titles that carry the axis?

fn check(source: &str, title: &'static oag_title::Title) -> anyhow::Result<()> {
    let Some(team_variants) = title.race.team_variants else {
        println!("{}: no team_variants", title.name);
        return Ok(());
    };
    let opened = oag_source::title::open_source(source, Vec::new(), Vec::new())?;
    let archives = opened.archives;
    println!("{}:", title.name);
    for team in team_variants.teams {
        for variant in team_variants.variants {
            let id = team_variants.join.combine(team, variant.suffix);
            let entry = format!(r"{}\{id}\handlingstats.xml", title.race.handling_dir);
            let resolves = archives.locate(&entry).is_some();
            println!("  {id:<24} ({:<16}) resolves: {resolves}", variant.label);
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    check("data/extracted/vita/PCSF00007", oag_2048::TITLE)?;
    check("data/images/hdfury-ps3-eu-dec.iso", oag_hd::TITLE)?;
    Ok(())
}
