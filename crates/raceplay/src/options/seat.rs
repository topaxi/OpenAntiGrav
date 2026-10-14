//! What a person seated at a split-screen slot picked for their own craft.

/// One seated person's craft, as Ship Select resolved it.
///
/// [`crate::Options::seat_picks`] holds one per person after the first: slot 0
/// takes [`crate::Options::team`], [`crate::Options::hull_variant`] and
/// [`crate::Options::skin`], these are the same three for slots 1 and up. The
/// team is the identity the title flies (a title with a variant family has
/// already combined the variant into it), exactly what the single-player path
/// writes into `Options::team`.
///
/// **Chosen, not measured.** The original's PS2 split-screen Team Selection
/// lets every player pick a team (`docs/ui/selection-screens.md`); how a pick
/// is carried into a race is this project's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatPick {
    /// The team id this slot flies.
    pub team: String,
    /// The hull stem override, as [`crate::Options::hull_variant`].
    pub hull_variant: Option<String>,
    /// The `PI_ModelSkin` name, as [`crate::Options::skin`].
    pub skin: Option<String>,
}

impl crate::Options {
    /// How many people fly this race: [`crate::Options::people`], else one per view
    /// ([`crate::Options::views`]), at least one. What [`crate::Setup::people`] is built from.
    #[must_use]
    pub fn people_flying(&self) -> u8 {
        if self.people == 0 {
            self.views.max(1)
        } else {
            self.people
        }
    }
}
