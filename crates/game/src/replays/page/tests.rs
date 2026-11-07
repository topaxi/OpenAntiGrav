use std::path::PathBuf;

use oag_replay::Header;
use oag_replay::header::{Fork, Setup};

use super::*;

fn entry(name: &str, folder: Folder, head: Head, unix: u64) -> Entry {
    Entry {
        path: PathBuf::from(name),
        folder,
        unix,
        n: 1,
        bytes: 150_000,
        head,
    }
}

fn header(mode: &str) -> Header {
    let mut header = Header::new(
        "Wipeout Pulse",
        r"Data\Environments\16_Track\track.vex",
        mode,
        "Venom",
        "feisar",
        1,
    );
    header.ticks = 18_000;
    header.setup = Some(Setup {
        mode: mode.into(),
        people: 1,
        ..Setup::default()
    });
    header
}

fn modes() -> Vec<Choice> {
    vec![Choice::labelled("single_race", "SINGLE RACE")]
}

fn with<R>(refused: &BTreeMap<PathBuf, String>, run: impl FnOnce(&Context<'_>) -> R) -> R {
    let strings = StringTable::default();
    let modes = modes();
    let circuit = |_mode: oag_race::Mode, entry: &str| {
        entry
            .contains("16_Track")
            .then(|| "Talon's Junction".to_string())
    };
    run(&Context {
        title: "Wipeout Pulse",
        strings: &strings,
        modes: &modes,
        circuit: Box::new(circuit),
        refused,
        recording: true,
    })
}

#[test]
fn the_calendar_is_right_at_the_edges() {
    assert_eq!(civil(0), (1970, 1, 1, 0, 0));
    assert_eq!(civil(951_782_400), (2000, 2, 29, 0, 0), "a leap day");
    assert_eq!(civil(1_794_084_439), (2026, 11, 7, 20, 47));
    assert_eq!(civil(4_102_444_799), (2099, 12, 31, 23, 59));
}

#[test]
fn a_list_row_says_when_and_where_and_a_kept_one_is_marked() {
    let refused = BTreeMap::new();
    with(&refused, |cx| {
        let auto = entry(
            "a.oagr",
            Folder::Auto,
            Head::Read(Box::new(header("single_race"))),
            1_794_084_439,
        );
        let saved = entry(
            "b.oagr",
            Folder::Saved,
            Head::Read(Box::new(header("single_race"))),
            1_794_084_439,
        );
        assert_eq!(choice(&auto, cx).label, "07 20:47");
        assert!(
            choice(&saved, cx).label.starts_with("*07 20:47"),
            "{}",
            choice(&saved, cx).label
        );
        assert!(choice(&auto, cx).unavailable.is_none());
    });
}

#[test]
fn a_file_the_list_will_not_play_is_dimmed_with_its_reason() {
    let mut refused = BTreeMap::new();
    refused.insert(
        PathBuf::from("r.oagr"),
        "the tick-zero hash disagrees".to_string(),
    );
    with(&refused, |cx| {
        let why = |e: &Entry| choice(e, cx).unavailable.unwrap_or_default();
        assert_eq!(
            why(&entry("n.oagr", Folder::Auto, Head::Newer, 1)),
            "RECORDED BY A NEWER VERSION"
        );
        assert_eq!(
            why(&entry("d.oagr", Folder::Auto, Head::Damaged, 1)),
            "DAMAGED FILE"
        );
        let mut other = header("single_race");
        other.title = "Wipeout Pure".into();
        assert_eq!(
            why(&entry(
                "o.oagr",
                Folder::Auto,
                Head::Read(Box::new(other)),
                1
            )),
            "RECORDED ON WIPEOUT PURE: OPEN THAT DISC"
        );
        let mut bare = header("single_race");
        bare.setup = None;
        assert!(
            why(&entry(
                "s.oagr",
                Folder::Auto,
                Head::Read(Box::new(bare)),
                1
            ))
            .starts_with("CANNOT BE REBUILT: NO LAUNCH")
        );
        let rebuilt = entry(
            "r.oagr",
            Folder::Auto,
            Head::Read(Box::new(header("single_race"))),
            1,
        );
        assert_eq!(
            why(&rebuilt),
            "CANNOT BE REBUILT HERE: the tick-zero hash disagrees"
        );
    });
}

#[test]
fn a_tick_zero_refusal_reads_as_a_sentence_not_a_hash() {
    let mut refused = BTreeMap::new();
    refused.insert(PathBuf::from("r.oagr"), TICK_ZERO.to_string());
    with(&refused, |cx| {
        let rebuilt = entry(
            "r.oagr",
            Folder::Auto,
            Head::Read(Box::new(header("zone"))),
            1,
        );
        let why = choice(&rebuilt, cx).unavailable.expect("dimmed");
        assert!(why.starts_with("NOT THE RACE THAT WAS RECORDED"), "{why}");
        assert!(!why.contains("0x"), "{why}");
    });
}

#[test]
fn only_the_proved_modes_are_claimed() {
    for mode in ["time_trial", "speed_lap", "single_race"] {
        assert!(claimed(&header(mode)), "{mode}");
    }
    for mode in ["zone", "eliminator", "tournament"] {
        assert!(!claimed(&header(mode)), "{mode}");
    }
    let mut two = header("head_to_head");
    assert!(!claimed(&two), "one person is not the proved setup");
    two.setup.as_mut().unwrap().people = 2;
    assert!(claimed(&two));
    two.title = "wipeout pulse".into();
    assert!(
        claimed(&two),
        "a header spells the title as the records key does, lower case"
    );
    two.title = "Wipeout Pure".into();
    assert!(!claimed(&two), "the proved setup is Pulse's");
}

#[test]
fn the_detail_lines_lead_with_the_finish_and_name_what_is_not_verified() {
    let refused = BTreeMap::new();
    with(&refused, |cx| {
        let mut h = header("zone");
        h.finish = Some(oag_replay::header::Finish {
            place: 3,
            ticks: 7_200,
        });
        h.fork = Some(Fork {
            tick: 3_600,
            ..Fork::default()
        });
        let e = entry(
            "z.oagr",
            Folder::Saved,
            Head::Read(Box::new(h)),
            1_794_084_439,
        );
        let lines = lines(&e, cx);
        assert_eq!(lines[0], ("CIRCUIT".into(), "TALON'S JUNCTION".into()));
        assert_eq!(lines[1], ("FINISH".into(), "3RD  2.00.00".into()));
        assert_eq!(lines[2].0, "RACE");
        let note = &lines.iter().find(|(label, _)| label == "NOTE").unwrap().1;
        assert!(
            note.contains("SAVED") && note.contains("TAKEN OVER"),
            "{note}"
        );
        let unfinished = entry(
            "u.oagr",
            Folder::Auto,
            Head::Read(Box::new(header("single_race"))),
            1,
        );
        assert_eq!(super::lines(&unfinished, cx)[1].1, "NOT FINISHED");
        assert!(super::lines(&entry("d.oagr", Folder::Auto, Head::Damaged, 1), cx).is_empty());
    });
}

#[test]
fn an_empty_list_is_one_dimmed_value_that_says_why() {
    let refused = BTreeMap::new();
    with(&refused, |cx| {
        let on = choices(&[], cx);
        assert_eq!(on.len(), 1);
        assert_eq!(on[0].label, "NONE YET");
        assert_eq!(
            on[0].unavailable.as_deref(),
            Some("EVERY RACE YOU DRIVE IS RECORDED HERE")
        );
    });
    let strings = StringTable::default();
    let circuit = |_: oag_race::Mode, _: &str| None;
    let cx = Context {
        title: "Wipeout Pulse",
        strings: &strings,
        modes: &[],
        circuit: Box::new(circuit),
        refused: &refused,
        recording: false,
    };
    assert!(
        choices(&[], &cx)[0]
            .unavailable
            .as_deref()
            .unwrap()
            .contains("RECORDING IS OFF")
    );
}

#[test]
fn ordinals_follow_the_english_rule() {
    assert_eq!(
        [1, 2, 3, 4, 11, 12, 13, 21, 22].map(ordinal),
        [
            "1ST", "2ND", "3RD", "4TH", "11TH", "12TH", "13TH", "21ST", "22ND"
        ]
    );
}
