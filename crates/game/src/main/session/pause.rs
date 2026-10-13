//! Who may resume a paused race.
//!
//! **Only the player who paused**, in a local split race (maintainer's rule,
//! 2026-10-13): anyone seated can pause, and the pause belongs to them until they
//! press Start again or Circle to leave the race. A pause nobody pressed for (the
//! window losing focus, `Session::window_away`) has no owner, and any player may
//! lift it. With devices merged - every race that is not a split - every press is
//! slot 0's, so a one-player race behaves as it always did.

/// The pause state the session keeps: `None` while the race runs, `Some(owner)`
/// while paused, `owner` being the slot that paused or `None` for a pause nobody
/// pressed for.
pub(crate) type Paused = Option<Option<usize>>;

/// What a Start press does: `presses` is [`oag_input::Controls::take_presses`]'s
/// bitmask. Pauses on anyone's press, owned by the lowest slot that pressed;
/// resumes only on the owner's press, or anyone's for an unowned pause.
pub(crate) fn start(paused: Paused, presses: u8) -> Paused {
    if presses == 0 {
        return paused;
    }
    match paused {
        None => Some(Some(presses.trailing_zeros() as usize)),
        Some(owner) if may_act(owner, presses) => None,
        Some(owner) => Some(owner),
    }
}

/// Whether a Circle press leaves the paused race: the same owner rule as
/// resuming, so another player cannot quit someone else's pause either.
pub(crate) fn leaves(paused: Paused, presses: u8) -> bool {
    matches!(paused, Some(owner) if may_act(owner, presses))
}

impl super::Session {
    /// The session's pause as [`Paused`].
    pub(crate) fn pause_state(&self) -> Paused {
        self.paused.then_some(self.pause_owner)
    }
}

fn may_act(owner: Option<usize>, presses: u8) -> bool {
    match owner {
        Some(slot) => presses & (1 << slot) != 0,
        None => presses != 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P1: u8 = 0b01;
    const P2: u8 = 0b10;

    #[test]
    fn whoever_pauses_owns_the_pause() {
        assert_eq!(start(None, P2), Some(Some(1)));
        assert_eq!(start(None, P1 | P2), Some(Some(0)), "lowest slot on a tie");
        assert_eq!(start(None, 0), None);
    }

    #[test]
    fn only_the_owner_resumes() {
        let paused = Some(Some(1));
        assert_eq!(start(paused, P1), paused, "player 1 cannot resume player 2's pause");
        assert_eq!(start(paused, P2), None);
        assert_eq!(start(paused, P1 | P2), None, "the owner's press counts in a tie");
    }

    #[test]
    fn only_the_owner_leaves() {
        let paused = Some(Some(1));
        assert!(!leaves(paused, P1));
        assert!(leaves(paused, P2));
        assert!(!leaves(None, P1 | P2), "a running race is not left by Circle");
    }

    #[test]
    fn an_unowned_pause_is_anyones() {
        assert_eq!(start(Some(None), P2), None);
        assert!(leaves(Some(None), P1));
    }

    #[test]
    fn one_player_pauses_and_resumes_as_before() {
        let paused = start(None, P1);
        assert_eq!(paused, Some(Some(0)));
        assert_eq!(start(paused, P1), None);
    }
}
