//! `Grid Selection`'s unlock box: the bracketed panel under the flyer that
//! says how many more points a tier needs and carries another grid's logo.
//! Split out of [`super`] under the 1,000-line rule; see [`UnlockBox`] for
//! what is measured.

use crate::campaign::GridSelection;
use crate::language::StringTable;

/// The widgets a locked tier does not draw: `POINTS ACHIEVED`, its figure,
/// `TOTAL POINTS AVAILABLE` and its figure (with their two bullet arrows,
/// dropped in the image loop). **Measured**, 2026-09-30, on RPCS3 frames of
/// `grid1`..`grid7` of a fresh profile - none shows either figure; `grid0`,
/// unlocked, shows both.
pub(super) const POINTS_GROUP: [&str; 4] = ["Medals Title", "Points", "Points Title", "TotPoints"];

/// Which of the unlock box's two texts a tier shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum UnlockKind {
    /// `RC_POINTSTOUNL`, `"%d MORE POINTS NEEDED TO UNLOCK:"` - on an
    /// unlocked tier, naming the **next** grid.
    ToUnlock,
    /// `RC_POINTS_NEEDED`, `"%d MORE POINTS NEEDED IN:"` - on a locked one,
    /// naming the **previous** grid.
    NeededIn,
}

/// What `Grid Selection`'s bracketed box under the flyer says and whose logo
/// it carries.
///
/// **Measured on RPCS3, 2026-09-30** (`grid-hd-t0..t7-settled`, a fresh
/// profile, all eight base tiers): `grid0` reads `10 MORE POINTS NEEDED TO
/// UNLOCK:` over `warped`'s logo; `grid1` reads `10 MORE POINTS NEEDED IN:`
/// over `uplift`'s; `grid2`..`grid7` read `13`, `16`, `19`, `22`, `25`, `30`
/// over their previous grid's logo. Every figure is the **previous** tier's
/// `RequiredPoints` less what it has earned (on `grid0` its own), so a
/// grid's `RequiredPoints` is what it takes to open the *next* one - the
/// reading [`GridSelection::tier_shows_lock`] already had. The two strings
/// are `entries.xml`'s `RC_POINTSTOUNL`/`RC_POINTS_NEEDED`; the widget's
/// authored `string="more points needed from previous"` is a placeholder the
/// code overwrites.
///
/// **Chosen, not measured**: an unlocked tier whose own requirement is
/// already met shows no box (nothing is left to unlock), and neither does
/// the last tier, which has no next grid. Only the fresh-profile frames
/// exist.
pub(super) struct UnlockBox<'a> {
    pub(super) kind: UnlockKind,
    pub(super) points: u32,
    /// The `FlyerName` whose `Logo.gtf` the box carries.
    pub(super) logo: Option<&'a str>,
}

impl UnlockBox<'_> {
    /// `RC_POINTS_NEEDED` with `%d` filled in - `RC_POINT_NEEDED`
    /// (`"1 MORE POINT NEEDED IN:"`) for one, which the string table ships
    /// beside it; **chosen, not measured** that the singular is selected
    /// this way, no frame has a figure of 1.
    pub(super) fn needed_in(&self, strings: &StringTable) -> String {
        if self.points == 1 {
            return strings.get_or_id("RC_POINT_NEEDED").to_string();
        }
        strings
            .get_or_id("RC_POINTS_NEEDED")
            .replacen("%d", &self.points.to_string(), 1)
    }
}

pub(super) fn unlock_box(model: &GridSelection) -> Option<UnlockBox<'_>> {
    let grids = model.grids();
    let index = model.index();
    let selected = grids.get(index)?;
    if model.selected_is_locked() {
        let previous = grids.get(index.checked_sub(1)?)?;
        return Some(UnlockBox {
            kind: UnlockKind::NeededIn,
            points: previous
                .required_points
                .saturating_sub(previous.points_earned),
            logo: previous.flyer_name.as_deref(),
        });
    }
    let next = grids.get(index + 1)?;
    let remaining = selected
        .required_points
        .saturating_sub(selected.points_earned);
    (remaining > 0).then_some(UnlockBox {
        kind: UnlockKind::ToUnlock,
        points: remaining,
        logo: next.flyer_name.as_deref(),
    })
}

#[cfg(test)]
mod tests;
