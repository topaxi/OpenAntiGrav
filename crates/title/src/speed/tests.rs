//! What the ladder type promises, proved without reaching for any title's
//! data: this crate holds vocabulary, so the fixtures below are shaped like
//! the two measured ladders rather than imported from `oag-pulse`/`oag-pure`.

use super::*;

/// The four rungs every title measured so far shares.
const FOUR: SpeedClasses = SpeedClasses {
    names: SpeedClasses::PULSE_LADDER,
};

/// Wipeout Pure's shape: the same four with `VECTOR` ahead of them.
const FIVE: SpeedClasses = SpeedClasses {
    names: &["VECTOR", "VENOM", "FLASH", "RAPIER", "PHANTOM"],
};

/// The axis exists because the two disagree in **cardinality**, so that is
/// what gets asserted rather than any single ladder's contents.
#[test]
fn the_two_measured_ladders_disagree_in_length() {
    assert_eq!(FOUR.names().len(), 4);
    assert_eq!(FIVE.names().len(), 5);
    assert_eq!(FIVE.names()[0], SpeedClasses::VECTOR);
    assert!(!FOUR.names().contains(&SpeedClasses::VECTOR));
}

/// A union over one title is that title's own ladder, unchanged - the case a
/// machine holding a single disc actually hits.
#[test]
fn a_union_of_one_ladder_is_that_ladder() {
    assert_eq!(SpeedClasses::union([FOUR]), FOUR.names().to_vec());
    assert_eq!(SpeedClasses::union([FIVE]), FIVE.names().to_vec());
}

/// The union grows when a title that authors more rungs is available, and
/// **shrinks back when it is not**. This is the pair that proves the row is
/// data-driven rather than a fixed list with a filter bolted on.
#[test]
fn the_union_grows_and_shrinks_with_what_is_available() {
    let without_pure = SpeedClasses::union([FOUR]);
    let with_pure = SpeedClasses::union([FOUR, FIVE]);

    assert_eq!(without_pure.len(), 4);
    assert_eq!(with_pure.len(), 5);
    assert!(!without_pure.contains(&SpeedClasses::VECTOR));
    assert!(with_pure.contains(&SpeedClasses::VECTOR));
}

/// The merge respects each ladder's own slowest-first ordering whatever order
/// the titles arrive in - so `VECTOR` lands at the **front** even when the
/// four-rung ladder was merged first and has no rung below Venom to hang it
/// off.
#[test]
fn the_union_puts_vector_first_whichever_title_is_seen_first() {
    let expected = FIVE.names().to_vec();
    assert_eq!(SpeedClasses::union([FOUR, FIVE]), expected);
    assert_eq!(SpeedClasses::union([FIVE, FOUR]), expected);
}

/// Merging the same ladder twice - two pressings of one title on the same
/// machine - adds nothing and reorders nothing.
#[test]
fn the_union_is_idempotent() {
    assert_eq!(SpeedClasses::union([FIVE, FIVE]), FIVE.names().to_vec());
    assert_eq!(
        SpeedClasses::union([FOUR, FIVE, FOUR, FIVE]),
        FIVE.names().to_vec()
    );
}

/// A union over nothing is empty rather than a borrowed default. A machine
/// with no readable source offers no classes, which is the honest answer.
#[test]
fn a_union_of_no_ladders_is_empty() {
    assert!(SpeedClasses::union([]).is_empty());
}

/// `VECTOR` is authored by Pure and is **not** offered, because nothing
/// downstream can name it. The filter is what keeps a measured-but-unusable
/// rung out of a menu instead of racing it on another class's tuning.
#[test]
fn vector_is_authored_but_not_selectable() {
    assert!(!SpeedClasses::is_selectable(SpeedClasses::VECTOR));
    assert!(!SpeedClasses::is_selectable("vector"));

    for name in SpeedClasses::PULSE_LADDER {
        assert!(
            SpeedClasses::is_selectable(name),
            "{name} should be offered"
        );
    }
}

/// The consequence, stated as its own case because it is the thing a reader
/// will want to check: **the offered union is four on every configuration
/// today**, with or without Pure. When `oag_physics::SpeedClass` can name a
/// fifth rung, this test is the one that should change.
#[test]
fn the_offered_union_is_four_with_or_without_pure() {
    let offered = |ladders: Vec<SpeedClasses>| -> Vec<&'static str> {
        SpeedClasses::union(ladders)
            .into_iter()
            .filter(|name| SpeedClasses::is_selectable(name))
            .collect()
    };

    assert_eq!(offered(vec![FOUR]), SpeedClasses::PULSE_LADDER.to_vec());
    assert_eq!(
        offered(vec![FOUR, FIVE]),
        SpeedClasses::PULSE_LADDER.to_vec()
    );
}

/// `selectable` and `is_selectable` cannot drift apart.
#[test]
fn selectable_agrees_with_the_predicate() {
    assert_eq!(FIVE.selectable().count(), 4);
    assert_eq!(FOUR.selectable().count(), 4);
    assert!(FIVE.selectable().all(SpeedClasses::is_selectable));
}
