//! How a title's races pay loyalty, and where that came from.
//!
//! The totals themselves are kept per team across titles ([`crate::team`]); this
//! says which **earning law** a title's finished race is paid under. Two laws are
//! recovered (`docs/architecture/persistence.md`, "Loyalty"): Pulse's
//! (`Race_ComputeLoyaltyAward`) and HD's, which Omega's own executable repeats.

use crate::effects::Origin;

/// A recovered earning law.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoyaltyLaw {
    /// Wipeout Pulse's: laps 15/30/10 by mode, kills 15 (30 in Eliminator), zones 10.
    Pulse,
    /// Wipeout HD / Fury's, as Omega's executable repeats it.
    Hd,
}

/// How a title earns loyalty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoyaltyEarning {
    /// The law a finished race is paid under.
    pub law: LoyaltyLaw,
    /// [`Origin::Measured`] when this title's own executable carries the law,
    /// [`Origin::InheritedFrom`] when it carries none and runs another title's.
    pub origin: Origin,
    /// Whether the title's EndRace flow already banks the award (Pulse's and HD's
    /// screens do, because they show it). `false` means the session banks it when
    /// the race result is saved.
    pub banked_by_endrace: bool,
}
