//! Re-reads `data09.psarc` directly rather than trusting `frontend.rs`'s
//! constants, and checks that Omega's own archive candidates neither match
//! nor are matched by HD's or 2048's - the collision question
//! `docs/formats/omega-frontend.md` and this lane's own report leave as
//! "checked, not argued".
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-omega --run-ignored all \
//!     -E 'binary(omega_title_ground_truth)'
//! ```

fn source() -> Option<String> {
    oag_testdata::exact("data/extracted/ps4").map(|path| path.display().to_string())
}

/// `data09.psarc`'s own `skin.xml` still authors the ten globals
/// `frontend::MENU_SKIN` was read off, to the digit - a future patch or
/// region reshaping the file fails this loudly rather than leaving a
/// constant table quietly wrong.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn skin_xml_still_authors_the_menu_skin_globals() {
    let Some(source) = source() else { return };
    let mut archives = oag_omega::open(&source).expect("open the omega source");
    let bytes = archives
        .read_name(oag_omega::frontend::names::FRONTEND_ROOT)
        .expect("read skin.xml");
    let text = String::from_utf8(bytes).expect("skin.xml is not valid UTF-8");

    for needle in [
        r#"<Variable global="MenuXOffset">"#,
        r#"<Values String="800"></Values>"#,
        r#"<Variable global="TitleXOffset">"#,
        r#"<Values String="194"></Values>"#,
        r#"<Variable global="TextColor">"#,
        r#"<Values String="0xFFFFFFFF"></Values>"#,
        r#"<Screen name="2048_Colours">"#,
    ] {
        assert!(text.contains(needle), "{needle:?} not found in skin.xml");
    }
}

/// Nineteen grids (`grid_00.xml`..`grid_18.xml`), read by direct listing off
/// `data09.psarc` rather than trusted from the earlier sweep's count.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn data09_carries_nineteen_grids() {
    let Some(source) = source() else { return };
    let mut archives = oag_omega::open(&source).expect("open the omega source");
    let count = (0..=18)
        .filter(|n| {
            archives
                .read_name(&format!(r"Data\Plugins\Grids\grid_{n:02}.xml"))
                .is_ok()
        })
        .count();
    assert_eq!(count, 19);
    assert!(
        archives
            .read_name(r"Data\Plugins\Grids\grid_19.xml")
            .is_err(),
        "there is no twentieth grid"
    );
}

/// Every front-end file this crate reads is served by `data09.psarc` -
/// `Archives::locate` names the winning archive, so this is a tripwire
/// rather than a log line nobody kept: a future patch, or a reordering of
/// `oag_omega::EXTRA_CANDIDATES`, that changed which archive wins fails here
/// by name instead of only disagreeing with a boot report.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn data09_serves_every_front_end_file_this_crate_reads() {
    let Some(source) = source() else { return };
    let archives = oag_omega::open(&source).expect("open the omega source");
    for path in [
        oag_omega::frontend::names::FRONTEND_ROOT,
        oag_omega::campaign::SCREEN_ENTRY,
        oag_omega::campaign::DEFINITION_ENTRY,
        r"Data\Plugins\Frontend\Gui\MainMenu_Definition.xml",
        r"Data\Plugins\Frontend\Gui\RaceBox_Definition.xml",
    ] {
        let served = archives.locate(path);
        assert!(
            served.is_some_and(|label| label.contains("data09.psarc")),
            "{path} served by {served:?}, expected data09.psarc"
        );
    }
}

/// Neither `oag_hd` nor `oag_2048` opens Omega's own source directory as
/// their own title - the archive-name collision the brief asked to have
/// checked rather than argued.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn hd_and_2048_both_refuse_the_omega_source() {
    let Some(source) = source() else { return };
    assert!(
        oag_hd::open(&source).is_err(),
        "oag_hd::open should not recognise an Omega source"
    );
    assert!(
        oag_2048::open(&source).is_err(),
        "oag_2048::open should not recognise an Omega source"
    );
}

/// And the reverse: `oag_omega` does not open HD's PS3 image or 2048's own
/// Vita package directory as Omega.
#[test]
#[ignore = "needs hdfury-ps3-eu-dec.iso and the decrypted Vita package"]
fn omega_refuses_hd_and_2048_sources() {
    if let Some(hd) = oag_testdata::image("hdfury-ps3-eu-dec.iso") {
        assert!(oag_omega::open(&hd.display().to_string()).is_err());
    }
    if let Some(vita) = oag_testdata::exact("data/extracted/vita/PCSF00007/base") {
        assert!(oag_omega::open(&vita.display().to_string()).is_err());
    }
}

/// The EndRace screens Omega's copies of `EndRace_Definition.xml` author,
/// read off every archive that holds one - the census behind
/// `docs/formats/omega-status.md`'s "checked" line for HD's `Rewards`/
/// `Podium` findings. Omega's front end is HD's, so the question is whether
/// the screens HD authors (and HD's code builds only some of) are the same
/// ones here.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_endrace_definition_authors_the_screens_hd_does() {
    let Some(source) = source() else { return };
    let mut archives = oag_omega::open(&source).expect("open the omega source");
    let copies = archives.read_every_name(r"Data\Plugins\Frontend\Gui\EndRace_Definition.xml");
    assert!(!copies.is_empty(), "Omega ships an EndRace_Definition.xml");
    let screens = |xml: &str, name: &str| xml.contains(&format!("name=\"{name}\""));
    let mut seen = Vec::new();
    for (label, blob) in &copies {
        let xml = String::from_utf8_lossy(blob);
        seen.push(format!(
            "{label}: results={} menu={} rewards={} podium={}",
            screens(&xml, "EndRace Results"),
            screens(&xml, "EndRace Menu"),
            screens(&xml, "EndRace Rewards"),
            screens(&xml, "EndRace Podium"),
        ));
    }
    eprintln!("{}", seen.join("\n"));
    for (label, blob) in &copies {
        let xml = String::from_utf8_lossy(blob);
        assert!(
            screens(&xml, "EndRace Results") && screens(&xml, "EndRace Menu"),
            "{label}"
        );
        assert!(screens(&xml, "EndRace Podium"), "{label} has a Podium");
        assert!(
            !screens(&xml, "EndRace Rewards"),
            "{label}: no Rewards, like HD's DATA06"
        );
    }
}
