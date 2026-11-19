use super::*;

const DEFINITION: &str = r#"
<Screen name="Top">
<PI_News><Values NewsID="TKR_SONGS" NewsFormat="int" NewsClass="GenStats" NewsMember="69" NewsFrequency="0"></Values></PI_News>
<PI_News><Values NewsID="TKR_MENTIM" NewsFormat="time" NewsClass="GenStats" NewsMember="85" NewsFrequency="0"></Values></PI_News>
<PI_News><Values NewsID="TKR_SLTIM" NewsFormat="time" NewsClass="GenStats" NewsMember="13" NewsFrequency="0"></Values></PI_News>
<PI_News><Values NewsID="TKR_ENABS" NewsFormat="float" NewsClass="GenStats" NewsMember="24" NewsFrequency="0"></Values></PI_News>
<PI_News><Values NewsID="TKR_FAVMUS" NewsClass="Profile" NewsMember="3" NewsFrequency="0"></Values></PI_News>
<PI_News><Values NewsID="TKR_FYIZONE" NewsFrequency="1"></Values></PI_News>
<PI_News><Values NewsID="TKR_FYITRACK" NewsClass="Unlockable" NewsMember="0" NewsFrequency="-1"></Values></PI_News>
<PI_News><Values NewsID="TKR_TEAM" NewsClass="TeamStats" NewsMember="0" NewsFrequency="0"></Values></PI_News>
</Screen>"#;

const STRINGS: &str = r#"
<Entry ID="TKR_SONGS" String="You've listened to %d songs"></Entry>
<Entry ID="TKR_MENTIM" String="You spent %s in the menus"></Entry>
<Entry ID="TKR_SLTIM" String="Speed Lap time is %s"></Entry>
<Entry ID="TKR_NOSL" String="Try a Speed Lap"></Entry>
<Entry ID="TKR_ENABS" String="Energy %.2f units, 100%%"></Entry>
<Entry ID="TKR_FAVMUS" String="Your favourite is %s"></Entry>
<Entry ID="TKR_FYIZONE" String="Try Zone"></Entry>
<Entry ID="MSC_SEC" String="sec"></Entry>
<Entry ID="MSC_SECONDS" String="secs"></Entry>
<Entry ID="MSC_MIN" String="min"></Entry>
<Entry ID="MSC_MINS" String="mins"></Entry>
<Entry ID="MSC_HOUR" String="hour"></Entry>
<Entry ID="MSC_HOURS" String="hours"></Entry>"#;

#[derive(Default)]
struct Stats {
    slots: Vec<(u32, f32)>,
    favourite: Option<String>,
}

impl Statistics for Stats {
    fn general(&self, member: u32) -> f32 {
        self.slots
            .iter()
            .find(|(slot, _)| *slot == member)
            .map_or(0.0, |(_, value)| *value)
    }
    fn favourite(&self, member: u32) -> Option<String> {
        self.favourite.clone().filter(|_| member == 3)
    }
}

fn strings() -> StringTable {
    StringTable::from_xml(STRINGS)
}

#[test]
fn the_definition_reads_every_row_with_its_class_format_and_weight() {
    let items = parse(DEFINITION);
    assert_eq!(items.len(), 8);
    assert_eq!(items[0].member, 69);
    assert_eq!(items[1].format, Format::Time);
    assert_eq!(items[3].format, Format::Float);
    assert_eq!(items[4].class, Class::Profile);
    // A row that names no class is `GenStats` member 0, as the original's constructor zeroes it.
    assert_eq!(items[5].class, Class::GenStats);
    assert_eq!(items[5].member, 0);
    assert_eq!(items[5].weight(), 2, "NewsFrequency 1 is weight 2");
    assert_eq!(items[0].weight(), 1);
    assert_eq!(items[6].weight(), 0, "an unlock announcement is never drawn");
}

#[test]
fn a_statistic_of_zero_has_no_message() {
    let items = parse(DEFINITION);
    let none = Stats::default();
    assert_eq!(message(&items[0], &strings(), &none), None);
    assert_eq!(message(&items[3], &strings(), &none), None);
    assert_eq!(message(&items[4], &strings(), &none), None);
    // Member 0 of an item that names no class is the speed-pad count: nothing, until a pad is crossed.
    assert_eq!(message(&items[5], &strings(), &none), None);
}

#[test]
fn a_counted_statistic_fills_its_template() {
    let items = parse(DEFINITION);
    let stats = Stats {
        slots: vec![(69, 6.0), (24, 12.5), (0, 3.0)],
        favourite: Some("Fenix Funk 5".to_string()),
    };
    let strings = strings();
    assert_eq!(message(&items[0], &strings, &stats).unwrap(), "You've listened to 6 songs");
    assert_eq!(message(&items[3], &strings, &stats).unwrap(), "Energy 12.50 units, 100%");
    assert_eq!(message(&items[4], &strings, &stats).unwrap(), "Your favourite is Fenix Funk 5");
    assert_eq!(message(&items[5], &strings, &stats).unwrap(), "Try Zone");
}

#[test]
fn times_read_in_hours_minutes_and_seconds_singular_at_one() {
    let items = parse(DEFINITION);
    let strings = strings();
    let at = |seconds: f32| {
        let stats = Stats {
            slots: vec![(85, seconds)],
            favourite: None,
        };
        message(&items[1], &strings, &stats).unwrap()
    };
    assert_eq!(at(125.0), "You spent 2 mins 5 secs in the menus");
    assert_eq!(at(61.0), "You spent 1 min 1 sec in the menus");
    assert_eq!(at(3725.0), "You spent 1 hour 2 mins in the menus");
}

#[test]
fn an_untried_modes_time_item_says_to_try_it_and_other_time_items_say_nothing() {
    let items = parse(DEFINITION);
    let none = Stats::default();
    assert_eq!(message(&items[2], &strings(), &none).unwrap(), "Try a Speed Lap");
    assert_eq!(message(&items[1], &strings(), &none), None);
}

#[test]
fn the_draw_follows_the_weights_and_never_repeats_the_item_before() {
    let mut items = parse(DEFINITION);
    // The team row has no message here; a draw landing on it separates two equal draws, which the
    // original allows, so it would hide the law this test checks.
    items.retain(|item| item.class != Class::TeamStats);
    let stats = Stats {
        slots: vec![(69, 6.0), (85, 90.0), (24, 12.5), (0, 1.0)],
        favourite: Some("Fenix Funk 5".to_string()),
    };
    let mut rng = Rng::new(7);
    let plays = sequence(&items, &strings(), &stats, &mut rng, 4000);
    assert_eq!(plays.len(), 4000);
    assert!(plays.windows(2).all(|pair| pair[0] != pair[1]), "an item is never drawn twice running");
    let count = |text: &str| plays.iter().filter(|p| p.as_str() == text).count();
    // Every drawable item has a message here, so the weight-2 item is drawn about twice as
    // often as a weight-1 one, less the draws no-repeat steals from it.
    let zone = count("Try Zone");
    let songs = count("You've listened to 6 songs");
    assert!(zone > songs + songs / 2, "weight 2 beats weight 1: {zone} vs {songs}");
    assert_eq!(count("Your favourite is Fenix Funk 5") > 0, true);
}

#[test]
fn an_empty_statistic_is_skipped_and_the_draw_goes_again() {
    let items = parse(DEFINITION);
    let stats = Stats {
        slots: vec![(69, 6.0)],
        favourite: None,
    };
    let plays = sequence(&items, &strings(), &stats, &mut Rng::new(5), 200);
    // The untried Speed Lap tip and the song count are all that has a message.
    assert!(plays.iter().all(|p| p == "You've listened to 6 songs" || p == "Try a Speed Lap"));
    assert!(plays.iter().any(|p| p == "Try a Speed Lap"));
}

#[test]
fn nothing_to_say_is_an_empty_sequence_not_a_hang() {
    let items = parse(DEFINITION);
    let mut rng = Rng::new(1);
    let strings = StringTable::default();
    assert!(sequence(&items, &strings, &Stats::default(), &mut rng, 10).is_empty());
}

#[test]
fn one_seed_is_one_sequence() {
    let items = parse(DEFINITION);
    let stats = Stats {
        slots: vec![(69, 6.0), (85, 90.0)],
        favourite: None,
    };
    let run = |seed| sequence(&items, &strings(), &stats, &mut Rng::new(seed), 24);
    assert_eq!(run(3), run(3));
    assert_ne!(run(3), run(4));
}
