//! What a driver can see of the rest of the grid.
//!
//! # Plain numbers the caller measured
//!
//! This crate depends on `oag-core` and `oag-physics` only, not `oag-race`
//! (race positions) or `oag-gameplay` (the grid), so a driver is *handed* what
//! it can see, as it is handed the line rather than a decoded track node.
//! `oag_game::Race::field_for` builds it. Fixed size, `Copy`, no allocation.
//!
//! # Two geometries, because they disagree
//!
//! [`Rival::gap`] and [`Rival::offset`] are **along the track**: right for
//! blocking and yielding, and meaningful round a bend. [`Rival::range`] and
//! [`Rival::cos_bearing`] are straight-line, what a projectile flies through.
//! On a corner they differ, and the wrong one defends against a craft not
//! really behind or fires a rocket into a wall.
//!
//! The bearing is a **cosine, never an angle**:
//! `docs/architecture/determinism.md` forbids the transcendental.

/// One rival, as the driver sees it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rival {
    /// Which grid slot, so a caller can act on the driver's choice.
    pub slot: u8,
    /// Along-track gap, positive when the rival is **ahead**.
    /// Out of `oag_race::Standing::distance`, so it counts laps and is directly
    /// comparable between craft.
    pub gap: f32,
    /// How far across the rival sits, positive to **this** craft's right.
    pub offset: f32,
    /// Relative speed along **this craft's own forward axis**, signed so that
    /// positive is closing whichever side the rival is on: the raw difference of
    /// forward speeds means opposite things ahead and behind.
    ///
    /// A velocity projection, **not** the derivative of [`Rival::gap`] (which is
    /// round the track): they agree on a straight and drift apart through a
    /// corner, and the true derivative would need a tick of gap history in the
    /// snapshot.
    pub closing: f32,
    /// Straight-line distance, nose to nose.
    pub range: f32,
    /// Cosine of the bearing off this craft's nose. One is dead ahead.
    pub cos_bearing: f32,
}

/// Something laid on the track that a driver would rather not touch: a mine or
/// bomb today.
///
/// **Plain numbers the caller measured**, like [`Rival`]: this crate must not
/// know what a weapon is. `oag_game::Race::field_for` decides what counts as a
/// threat (it knows the charge's authored `trigger_radius`) and hands over two
/// distances.
///
/// # What a successful dodge is
///
/// **Passing outside the charge's trigger radius, so it never goes off.** Not
/// surviving the blast: a mine's blast radius is wider than a Pulse lane, so
/// leaving *that* would swerve into the scenery and still take the hit. The
/// trigger radius is a few units, a dodge a craft can make, which is why the
/// caller measures against it and not the blast radius.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Hazard {
    /// How far ahead it sits along this craft's own forward axis, in units.
    /// Always positive: the caller drops anything level or behind.
    pub distance: f32,
    /// How far across it sits, positive to **this** craft's right.
    /// How far across it sits, positive to **this** craft's right: the same axis
    /// and sign as [`Rival::offset`].
    pub offset: f32,
}

/// A pad on the track a driver would like to pass over.
///
/// **Plain numbers the caller measured**, like [`Hazard`]: this crate does not
/// know what a pad hands out or which mode wants one. `oag_game::Race::field_for`
/// decides whether a craft wants a pad (today: an opponent with an empty weapon
/// slot, in an Eliminator) and hands over where it is **relative to the line**,
/// a target fixed in the corridor rather than one that moves as the craft swings
/// toward it. See [`crate::Driver`]'s `toward_pad`.
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
    pub ahead: Option<Rival>,
    pub behind: Option<Rival>,
    /// A craft close enough alongside to touch, neither ahead nor behind while
    /// it is there.
    pub alongside: Option<Rival>,
    /// The nearest laid charge worth steering around. Not a rival: a mine does
    /// not close, have a place or get blocked. See [`Hazard`].
    pub hazard: Option<Hazard>,
    /// A pad worth swinging over, if the caller wants this craft to have one.
    /// `None` in every race but an Eliminator, which keeps every other mode's
    /// line byte-identical. See [`Pad`].
    pub pad: Option<Pad>,
    /// This craft's own race position, `1`-based.
    ///
    /// Zero when nothing places it (no closed ring, or the tick before the first
    /// fix). **Zero is not first**: it stops every craft reading tick one as
    /// having just been overtaken.
    pub place: u8,
}

impl Field {
    /// Nobody else on the circuit: what a synthetic test, a line with no race
    /// around it, or an unknown ring sees. Drives exactly the line it drove
    /// before it could see anything.
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

    /// Zero is "nobody has placed this craft yet", not first place.
    #[test]
    fn an_unplaced_craft_reads_as_zero_rather_than_first() {
        assert_eq!(Field::EMPTY.place, 0);
    }
}
