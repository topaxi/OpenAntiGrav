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

/// Something laid on the track that a driver would rather not touch.
///
/// A mine or a bomb today; anything that sits still and hurts, in principle.
///
/// **Plain numbers the caller measured**, like [`Rival`] and for the same
/// reason: this crate must not know what a weapon is. `oag_game::Race::field_for`
/// decides what counts as a threat - it is the half that knows the charge's
/// authored `trigger_radius` - and hands over two distances. A driver dodges
/// what it is told about and asks nothing else.
///
/// # What a successful dodge is
///
/// **Passing outside the charge's trigger radius, so it never goes off at all.**
/// Not "surviving the blast": a mine's blast radius is wider than a Pulse lane,
/// so a driver that tried to leave *that* would swerve into the scenery and
/// still take the hit. The trigger radius is a few units, which is a dodge a
/// craft can actually make. That is why the caller measures the threat against
/// the trigger radius and not the blast radius, and it is the whole reason this
/// term is worth having.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Hazard {
    /// How far ahead it sits along this craft's own forward axis, in units.
    ///
    /// Always positive - the caller drops anything level with or behind the
    /// craft, because there is nothing left to steer around.
    pub distance: f32,
    /// How far across it sits, positive to **this** craft's right.
    ///
    /// The same axis and sign as [`Rival::offset`], so the two terms that read
    /// it cannot disagree about which way is right.
    pub offset: f32,
}

/// A pad on the track a driver would like to pass over.
///
/// **Plain numbers the caller measured**, for the same reason as [`Hazard`]:
/// this crate does not know what a pad hands out or which mode wants one.
/// `oag_game::Race::field_for` decides whether a craft wants a pad at all
/// (today: an opponent with an empty weapon slot, in an Eliminator) and which
/// one, and hands over where it is **relative to the line**, not to the craft -
/// a target fixed in the corridor rather than one that moves as the craft
/// swings toward it. See [`crate::Driver`]'s `toward_pad`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pad {
    /// How far ahead along the line its centre sits, in units. Always positive.
    pub distance: f32,
    /// How far across the line its centre sits, positive to the line's right -
    /// the corridor's own [`crate::Frame::lateral`] axis and sign.
    pub offset: f32,
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
    /// The nearest laid charge worth steering around, if there is one.
    ///
    /// Not a rival, so not one of the three above: a mine does not close, does
    /// not have a place and cannot be blocked. See [`Hazard`].
    pub hazard: Option<Hazard>,
    /// A pad worth swinging over, if the caller wants this craft to have one.
    ///
    /// `None` in every race but an Eliminator, which is what keeps every other
    /// mode's line byte-identical - see [`Pad`].
    pub pad: Option<Pad>,
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
        hazard: None,
        pad: None,
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
        assert!(Field::EMPTY.hazard.is_none());
        assert!(Field::EMPTY.pad.is_none());
    }

    /// Zero is "nobody has placed this craft yet", and a driver that read it as
    /// first place would think every craft on the grid had just been overtaken
    /// on tick one.
    #[test]
    fn an_unplaced_craft_reads_as_zero_rather_than_first() {
        assert_eq!(Field::EMPTY.place, 0);
    }
}
