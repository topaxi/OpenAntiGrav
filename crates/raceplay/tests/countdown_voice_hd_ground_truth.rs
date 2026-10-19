//! HD's `ready` is a lone `0x08` that hands its voice to the child cue `321_GO`
//! (`speech.bnk` cue 0, one command, child index 2), so the walk must follow it.
//!
//! **`#[ignore]`d and never run in CI**: it needs HD's disc image. Run with
//! `just test-data`.
//!
//! Before the countdown loader walked `0x08` as a child, `ready` had no timeline
//! and played one of its fourteen flattened waveforms picked at random, a clip
//! from 0.8 s to 3.0 s long. The walk plays the authored path: `321_GO`'s goto at
//! command 1 jumps to its marker at command 14 and keys the 3.0 s stereo pair.

use oag_sound::sfx::Banks;

fn opened() -> Option<oag_source::title::Opened> {
    let path = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    Some(
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source"),
    )
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn ready_follows_its_child_cue_to_a_timeline() {
    let Some(mut opened) = opened() else {
        return;
    };
    let race = opened.title.race;
    let voice = race.countdown_voice.expect("HD voices its start");
    let mut banks = Banks::default();
    banks.load_countdown(&mut opened.archives, race.sounds.speech, voice);
    let line = |cue: &str| {
        banks
            .report
            .iter()
            .find(|l| l.starts_with(&format!("sfx: {cue} -> ")))
            .unwrap_or_else(|| panic!("{cue} is not in the report: {:?}", banks.report))
            .clone()
    };
    for cue in ["ready", "go"] {
        let line = line(cue);
        println!("{line}");
        assert!(
            line.contains("plays its timeline"),
            "{cue} must play its authored timeline, not a flat pick: {line}"
        );
    }
    assert!(
        line("ready").contains("1 variant(s) of 2 voice(s)"),
        "ready is 321_GO's tail: one stereo pair, {}",
        line("ready")
    );
}
