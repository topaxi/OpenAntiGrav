//! Scratch probe, the reproducer for the finding recorded in
//! `handover/2048s-track-vex-parses-but-two-binary-formats-changed.md`
//! (2026-09-01 addendum): Race Remix's RACE REMIX page is the first thing to
//! ever offer a 2048 team through a live picker, and every one of the 17 ids
//! `crate::remix::catalogue` builds for it fails to resolve - for two
//! different, already-documented reasons.
//!
//! `cargo run -p oag-game --example remix_2048_roster_probe`

fn main() -> anyhow::Result<()> {
    let catalogue = oag_game::remix::catalogue("data/extracted/vita/PCSF00007")?;
    let archives = oag_2048::open("data/extracted/vita/PCSF00007")?;

    println!(
        "{} team(s) offered by remix::catalogue:",
        catalogue.teams.len()
    );
    for choice in &catalogue.teams {
        // What `race::load` actually builds today: `craft_title.race.handling_dir`
        // (`oag_2048::race::HANDLING_DIR`, `Data\HandlingStats`) joined with the
        // catalogue's own id, unconditionally - the same join every other title
        // uses successfully.
        let naive = format!(r"Data\HandlingStats\{}\handlingstats.xml", choice.value);
        let naive_ok = archives.locate(&naive).is_some();

        // What actually resolves for the native roster: the id needs a second,
        // numbered level (`<team>\<1..4>`, fighter/agility/speed/prototype -
        // recovered and named in the handover thread this probe is evidence
        // for), which the plugin definition's own `PI_Team` location never
        // states - so `crate::catalogue::teams` cannot build it either.
        let numbered_ok = (1..=4).any(|n| {
            let path = format!(r"Data\HandlingStats\{}\{n}\handlingstats.xml", choice.value);
            archives.locate(&path).is_some()
        });

        // What actually resolves for the HD-derived roster: a different tree
        // entirely, `oag_2048::race::HD_SHIP_DIR`
        // (`Data\art\published\hdships`), which `RaceDefaults.handling_dir`
        // cannot name at the same time as the native roster's
        // `Data\HandlingStats` - "one `ship_dir` string cannot address both of
        // 2048's trees", per the same thread.
        let hd_derived = format!(
            r"Data\art\published\hdships\{}\handlingstats.xml",
            choice.value
        );
        let hd_derived_ok = archives.locate(&hd_derived).is_some();

        println!(
            "  {:>15} naive:{naive_ok:<5} numbered-variant:{numbered_ok:<5} hd-derived-tree:{hd_derived_ok:<5}",
            choice.value
        );
    }
    Ok(())
}
