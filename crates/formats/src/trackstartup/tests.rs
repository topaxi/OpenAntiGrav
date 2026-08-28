use super::*;

/// Talon's Junction's own file, trimmed to two billboards.
const SAMPLE: &str = r#"<?xml version="1.0" encoding="utf-8" ?>
<TrackStartup>
	<LevelFx>
		<UnderwaterSound type="1"/>
		<WindSound type="2"/>
	</LevelFx>
	<LoadSoundBank Filename="env9_talonsjunction.bnk"/>
	<Billboard num="1" type="landscape" location="Data\Billboards\HD_Adverts\Icaras\Looping_Background.vex"></Billboard>
	<Billboard num="8" type="landscape" location="Data\Billboards\HD_Adverts\321Go\321Go_StartFinish.vex"></Billboard>
	<!-- You can edit and add up to 8 of these billboard definitions -->
</TrackStartup>
"#;

#[test]
fn it_reads_the_bank_the_fx_and_every_billboard() {
    let m = TrackStartup::parse(SAMPLE);
    assert_eq!(m.sound_bank.as_deref(), Some("env9_talonsjunction.bnk"));
    assert_eq!((m.underwater_sound, m.wind_sound), (Some(1), Some(2)));
    assert_eq!(m.billboards.len(), 2);
    assert_eq!(m.billboards[0].num, 1);
    assert_eq!(m.billboards[0].kind, "landscape");
}

/// The manifest spells a path the disc's way and the archives spell it another;
/// a reader that kept the backslashes would find nothing.
#[test]
fn a_location_becomes_the_name_an_archive_matches() {
    let m = TrackStartup::parse(SAMPLE);
    assert_eq!(
        m.billboards[0].location(),
        Some("/Data/Billboards/HD_Adverts/Icaras/Looping_Background.vex")
    );
    assert_eq!(
        m.billboard(8).and_then(Billboard::location),
        Some("/Data/Billboards/HD_Adverts/321Go/321Go_StartFinish.vex"),
    );
    assert_eq!(m.billboard(4), None);
}

/// `num` is what the file says it addresses, so it is looked up rather than
/// positioned - a file listing 8 then 1 must not answer 1 for the first.
#[test]
fn a_billboard_is_found_by_its_own_num_not_its_position() {
    let m = TrackStartup::parse(
        r#"<TrackStartup>
             <Billboard num="8" type="landscape" location="a.vex"/>
             <Billboard num="1" type="landscape" location="b.vex"/>
           </TrackStartup>"#,
    );
    assert_eq!(m.billboard(8).and_then(Billboard::location), Some("/a.vex"));
    assert_eq!(m.billboard(1).and_then(Billboard::location), Some("/b.vex"));
}

/// Supplementary data: an entry this cannot use is dropped, not fatal, because
/// a circuit whose adverts do not load still races.
#[test]
fn an_unusable_entry_is_dropped_rather_than_failing_the_file() {
    let m = TrackStartup::parse(
        r#"<TrackStartup>
             <Billboard type="landscape" location="no-num.vex"/>
             <Billboard num="2" type="landscape"/>
             <Billboard num="3" type="landscape" location="good.vex"/>
             <SomethingNobodyHasSeen value="7"/>
           </TrackStartup>"#,
    );
    assert_eq!(m.billboards.len(), 1);
    assert_eq!(m.billboards[0].num, 3);
    assert!(m.sound_bank.is_none());
}

#[test]
fn an_empty_or_unrelated_document_yields_nothing() {
    for xml in ["", "<TrackStartup></TrackStartup>", "<html><body/></html>"] {
        let m = TrackStartup::parse(xml);
        assert_eq!(m, TrackStartup::default(), "{xml:?}");
    }
}

/// Pulse ships the same file `fexml`-shortened, with a `<code>` dictionary
/// up top instead of readable tag names - measured directly off
/// `Data\Environments\16_Track\TrackStartup.xml`, whose own dictionary
/// spells the mapping below. A reader that called [`fexml::parse`] on this
/// text unexpanded found zero billboards on every Pulse circuit; this is the
/// regression that catches it coming back.
#[test]
fn pulse_ships_the_same_file_fexml_shortened() {
    let shortened = concat!(
        "<code as=\"Billboard\" bs=\"type\" cs=\"num\" ds=\"location\" ",
        "es=\"LevelFx\" fs=\"TrackStartup\" gs=\"color\" hs=\"WindSound\" ",
        "is=\"LoadSoundBank\" js=\"Filename\"></code>\r\n",
        "<f>\r\n",
        "<a c=\"1\" b=\"landscape\" d=\"Data\\Billboards\\Pulse_Adverts\\goteki\\GOTEKI_LANDSCAPE_01.vex\"></a>\r\n",
        "<a c=\"3\" b=\"portrait\" g=\"grey\"></a>\r\n",
        "<a c=\"8\" b=\"landscape\" d=\"Data\\Environments\\321_Go\\321Go_StartFinish.vex\"></a>\r\n",
        "<e><h b=\"1\"/></e>\r\n",
        "<i j=\"TALONS_JUNCTION_ENV.bnk\"/>\r\n",
        "</f>\r\n",
    );
    let m = TrackStartup::parse(shortened);
    assert_eq!(m.sound_bank.as_deref(), Some("TALONS_JUNCTION_ENV.bnk"));
    assert_eq!(m.wind_sound, Some(1));
    assert_eq!(m.billboards.len(), 3);
    assert_eq!(
        m.billboard(1).and_then(Billboard::location),
        Some("/Data/Billboards/Pulse_Adverts/goteki/GOTEKI_LANDSCAPE_01.vex")
    );
    assert_eq!(
        m.billboard(3).map(|b| &b.fill),
        Some(&Fill::Colour("grey".to_string()))
    );
    assert_eq!(
        m.billboard(8).and_then(Billboard::location),
        Some("/Data/Environments/321_Go/321Go_StartFinish.vex")
    );
}

/// A slot with no model names a colour instead, and dropping it would lose one
/// entry in eight across the disc.
#[test]
fn a_slot_may_be_a_colour_rather_than_a_model() {
    let m = TrackStartup::parse(
        r#"<TrackStartup>
             <Billboard num="1" type="landscape" color="red"></Billboard>
             <Billboard num="2" type="landscape" location="Data\A\b.vex"/>
           </TrackStartup>"#,
    );
    assert_eq!(m.billboards.len(), 2);
    assert_eq!(m.billboards[0].fill, Fill::Colour("red".to_string()));
    assert_eq!(m.billboards[0].location(), None);
    assert_eq!(m.billboards[1].location(), Some("/Data/A/b.vex"));
}
