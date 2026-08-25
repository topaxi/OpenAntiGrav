use super::*;

#[test]
fn a_fully_claimed_blob_has_no_gaps() {
    let mut seen = Coverage::new(100);
    seen.claim(0, 100, "everything");
    assert!(seen.gaps(1).is_empty());
    assert_eq!(seen.claimed(), 100);
    assert!((seen.fraction() - 1.0).abs() < f64::EPSILON);
}

#[test]
fn a_gap_names_what_is_either_side_of_it() {
    let mut seen = Coverage::new(64);
    seen.claim(32, 32, "payload");
    seen.claim(0, 16, "header");
    let gaps = seen.gaps(1);
    assert_eq!(
        gaps,
        vec![Gap {
            at: 16,
            len: 16,
            after: "header",
            before: "payload",
        }]
    );
}

#[test]
fn claims_may_overlap_and_arrive_in_any_order() {
    let mut seen = Coverage::new(50);
    seen.claim(20, 20, "b");
    seen.claim(0, 30, "a");
    assert!(
        seen.gaps(1)
            == vec![Gap {
                at: 40,
                len: 10,
                after: "b",
                before: "end of file"
            }]
    );
    assert_eq!(seen.claimed(), 40, "the overlap counts once");
}

#[test]
fn a_run_shorter_than_the_minimum_is_forgiven() {
    // Structure padding is a handful of bytes; a field nobody read is a table.
    let mut seen = Coverage::new(80);
    seen.claim(0, 30, "a");
    seen.claim(32, 48, "b");
    assert!(seen.gaps(8).is_empty(), "2 bytes of alignment slack");
    assert_eq!(seen.gaps(1).len(), 1);
}

#[test]
fn a_claim_past_the_end_is_clamped_rather_than_refused() {
    let mut seen = Coverage::new(16);
    seen.claim(0, 1_000_000, "an optimistic length");
    assert!(seen.gaps(1).is_empty());
    assert_eq!(seen.claimed(), 16);

    seen.claim(64, 8, "entirely past the end");
    assert_eq!(seen.claimed(), 16, "and one wholly outside is dropped");
}

#[test]
fn an_empty_claim_does_not_name_a_gap_it_does_not_bound() {
    let mut seen = Coverage::new(40);
    seen.claim(0, 8, "header");
    seen.claim(20, 0, "a table with no rows");
    seen.claim(32, 8, "tail");
    let gaps = seen.gaps(1);
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0].before, "tail", "not the empty table");
}

#[test]
fn an_empty_blob_is_fully_covered() {
    let seen = Coverage::new(0);
    assert!(seen.is_empty());
    assert!(seen.gaps(1).is_empty());
    assert!((seen.fraction() - 1.0).abs() < f64::EPSILON);
}
