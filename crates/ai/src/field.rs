//! What a driver can see of the rest of the grid.
//!
//! # Plain numbers the caller measured
//!
//! This crate depends on `oag-core` and `oag-physics` and on nothing else - not
//! on `oag-race`, which owns race positions, and not on `oag-gameplay`, which
//! owns the grid. So a driver is *handed* what it can see rather than going and
//! looking, the same way it is handed the line it follows rather than a decoded
//! track node. `oag_game::Race::field_for` is what builds it.
//!
//! Fixed size, `Copy`, no allocation: it costs nothing to pass every tick.
//!
//! # Two geometries, because they disagree
//!
//! [`Rival::gap`] and [`Rival::offset`] are measured **along the track** - that
//! is what a decision about blocking or yielding wants, and it is what stays
//! meaningful when a rival is round a bend. [`Rival::range`] and
//! [`Rival::cos_bearing`] are the straight-line geometry, which is what a
//! projectile actually flies through. On a corner those two answers differ, and
//! a driver that used the wrong one would either defend against a craft that is
//! not really behind it or fire a rocket into a wall.
//!
//! The bearing is a **cosine and never an angle**:
//! `docs/architecture/determinism.md` forbids the transcendental, and the
//! caller has the dot product already.

/// One rival, as the driver sees it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rival {
    /// Which grid slot, so a caller can act on the driver's choice.
    pub slot: u8,
    /// Along-track gap, positive when the rival is **ahead**.
    ///
    /// Out of `oag_race::Standing::distance`, so it counts laps and is directly
    /// comparable between craft rather than being a guess from positions.
    pub gap: f32,
    /// How far across the rival sits, positive to **this** craft's right.
    pub offset: f32,
    /// Relative speed along **this craft's own forward axis**, signed so that
    /// positive is closing whichever side the rival is on.
    ///
    /// The sign flip is the point: the raw difference of forward speeds means
    /// opposite things for a craft ahead and one behind, so a driver reacting
    /// to "closing" would react to the wrong one half the time.
    ///
    /// It is a velocity projection and **not** the derivative of
    /// [`Rival::gap`], which is measured round the track. The two agree on a
    /// straight and drift apart through a corner; carrying the true derivative
    /// would mean keeping a tick of gap history, which is snapshot state this
    /// does not need.
    pub closing: f32,
    /// Straight-line distance, nose to nose.
    pub range: f32,
    /// Cosine of the bearing off this craft's nose. One is dead ahead.
    pub cos_bearing: f32,
}

/// The rivals a driver may react to, and its own place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Field {
    /// The nearest craft in front, within awareness range.
    pub ahead: Option<Rival>,
    /// The nearest craft behind.
    pub behind: Option<Rival>,
    /// A craft close enough alongside to touch, which is neither ahead nor
    /// behind for as long as it is there.
    pub alongside: Option<Rival>,
    /// This craft's own race position, `1`-based.
    ///
    /// Zero when nothing places it - a track with no closed ring, or the tick
    /// before the first fix. **Zero is not first**, and a driver reading it has
    /// to say so; it is what stops every craft on the grid reading tick one as
    /// having just been overtaken.
    pub place: u8,
}

impl Field {
    /// Nobody else on the circuit.
    ///
    /// What a synthetic test, a line with no race around it, and any track
    /// whose ring is unknown all see. A driver handed this drives exactly the
    /// line it drove before it could see anything.
    pub const EMPTY: Self = Self {
        ahead: None,
        behind: None,
        alongside: None,
        place: 0,
    };
}

impl Default for Field {
    fn default() -> Self {
        Self::EMPTY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_field_is_the_default() {
        assert_eq!(Field::default(), Field::EMPTY);
        assert!(Field::EMPTY.ahead.is_none());
        assert!(Field::EMPTY.behind.is_none());
        assert!(Field::EMPTY.alongside.is_none());
    }

    /// Zero is "nobody has placed this craft yet", and a driver that read it as
    /// first place would think every craft on the grid had just been overtaken
    /// on tick one.
    #[test]
    fn an_unplaced_craft_reads_as_zero_rather_than_first() {
        assert_eq!(Field::EMPTY.place, 0);
    }
}
