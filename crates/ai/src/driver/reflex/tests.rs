//! What a [`Reflex`] is asserted to do.
//!
//! Its own file under `scripts/check-file-size.py`'s 200-line cap on an inline
//! `#[cfg(test)]` module.

use super::*;

/// A rival in a given slot, with nothing else measured: every assertion here is
/// about *whether* a channel is visible, never about what is in it.
fn rival(slot: u8) -> Rival {
    Rival {
        slot,
        ..Rival::default()
    }
}

fn ahead(slot: u8) -> Field {
    Field {
        ahead: Some(rival(slot)),
        ..Field::EMPTY
    }
}

/// Runs `field` for `ticks` and returns what the driver could act on at the end.
fn run(reflex: &mut Reflex, field: &Field, latency: u16, ticks: u16) -> Field {
    let mut seen = Field::EMPTY;
    for _ in 0..ticks {
        reflex.advance(field, latency);
        seen = reflex.filter(field);
    }
    seen
}

/// The whole of the guarantee that this axis is opt-in: a caller that has not
/// chosen a difficulty - the closed-loop harness, a replay, `oag-trace` - gets
/// the driver that was here before reaction latency existed.
#[test]
fn a_latency_of_zero_notices_on_the_tick_the_rival_arrives() {
    let mut reflex = Reflex::IDLE;
    let field = ahead(3);
    reflex.advance(&field, 0);
    assert_eq!(reflex.filter(&field).ahead, Some(rival(3)));
}

/// A latency of `n` hides the rival for exactly `n` ticks and shows it on the
/// next one - the clock is not off by one either way.
#[test]
fn a_rival_is_invisible_for_exactly_the_latency() {
    for latency in [1_u16, 2, 6, 24] {
        let mut reflex = Reflex::IDLE;
        let field = ahead(3);
        for tick in 1..=latency {
            reflex.advance(&field, latency);
            assert_eq!(
                reflex.filter(&field).ahead,
                None,
                "latency {latency} showed the rival on tick {tick}"
            );
        }
        reflex.advance(&field, latency);
        assert_eq!(
            reflex.filter(&field).ahead,
            Some(rival(3)),
            "latency {latency} had still not noticed after {latency} ticks"
        );
    }
}

/// Once noticed, a rival stays noticed - the clock does not restart every tick
/// the caller measures the same craft again.
#[test]
fn a_noticed_rival_does_not_have_to_be_noticed_again() {
    let mut reflex = Reflex::IDLE;
    let field = ahead(3);
    assert_eq!(run(&mut reflex, &field, 6, 60).ahead, Some(rival(3)));
    assert_eq!(reflex.wait[AHEAD], 0);
    assert_eq!(reflex.pending[AHEAD], NOBODY);
}

/// A driver still covering a rival that has dropped away reads as a bug; one
/// late to spot a new arrival reads as a driver. See this module's header.
#[test]
fn a_rival_that_leaves_is_gone_at_once() {
    let mut reflex = Reflex::IDLE;
    let field = ahead(3);
    run(&mut reflex, &field, 24, 30);
    reflex.advance(&Field::EMPTY, 24);
    assert_eq!(reflex.filter(&Field::EMPTY).ahead, None);
    assert_eq!(reflex.seen[AHEAD], NOBODY);
}

/// The reason [`Reflex::pending`] is an array of its own: a driver part way
/// through noticing one craft has not part-noticed a different one.
#[test]
fn a_second_rival_taking_the_channel_over_starts_its_own_clock() {
    let mut reflex = Reflex::IDLE;
    let first = ahead(3);
    let second = ahead(5);
    // Four ticks into noticing craft 3, which leaves three of its six to run.
    run(&mut reflex, &first, 6, 4);
    // Craft 5 takes its place. Three more ticks would have finished the clock
    // it replaced, and are not enough for a clock of its own.
    assert_eq!(run(&mut reflex, &second, 6, 3).ahead, None);
    assert_eq!(run(&mut reflex, &second, 6, 4).ahead, Some(rival(5)));
}

/// A rival already being acted on is still acted on if it comes back while a
/// replacement's clock is running - noticing is per craft, not per channel.
#[test]
fn a_rival_already_noticed_is_not_re_noticed_when_it_returns() {
    let mut reflex = Reflex::IDLE;
    let first = ahead(3);
    let second = ahead(5);
    run(&mut reflex, &first, 6, 10);
    run(&mut reflex, &second, 6, 3);
    reflex.advance(&first, 6);
    assert_eq!(reflex.filter(&first).ahead, Some(rival(3)));
}

/// Three channels, three clocks. A craft alongside is not noticed by having
/// noticed one in front.
#[test]
fn the_three_channels_are_noticed_separately() {
    let mut reflex = Reflex::IDLE;
    let both = Field {
        ahead: Some(rival(3)),
        alongside: Some(rival(5)),
        ..Field::EMPTY
    };
    run(&mut reflex, &ahead(3), 30, 31);
    let seen = run(&mut reflex, &both, 30, 5);
    assert_eq!(seen.ahead, Some(rival(3)));
    assert_eq!(seen.alongside, None);
    assert_eq!(run(&mut reflex, &both, 30, 26).alongside, Some(rival(5)));
}

/// Slot zero is the player's, and [`NOBODY`] is deliberately not zero: a
/// zero-initialised reflex would claim to have noticed the player already.
#[test]
fn a_fresh_reflex_has_not_noticed_the_player() {
    let reflex = Reflex::default();
    assert_eq!(reflex, Reflex::IDLE);
    let field = ahead(0);
    assert_eq!(reflex.filter(&field).ahead, None);
}

/// The standings are not something seen out of a cockpit - see this module's
/// header for why the grudge is not on a reaction time.
#[test]
fn a_race_place_is_never_held_back() {
    let reflex = Reflex::IDLE;
    let field = Field {
        place: 4,
        ..ahead(3)
    };
    assert_eq!(reflex.filter(&field).place, 4);
}
