//! The viewer's incoming-weapon warnings: which opponent fire raises one.
//!
//! The law is `Weapon_AnnounceIncoming` (`0x08871ddc`) and the HUD half is
//! `oag_hud::incoming`; `docs/ghidra/functions/psp-pulse-usa/hud-incoming-warnings.md`.
//! A fire is judged once, at the moment it happens, from the two craft's along-track
//! progress, and the icon it earns waits in `RaceView::incoming_raises` for the HUD's
//! next advance. Nothing here touches `World`, so no hash moves.

use super::*;

impl Race {
    /// Records that `slot` fired `weapon` this tick: raises the viewer's warning when the
    /// along-track gap `progress(viewer) - progress(slot)` is inside the weapon's window.
    ///
    /// One call per fire request (**chosen, not measured**: the original announces on every
    /// `Weapon_RequestFire` press, including a re-press mid-cluster, while this engine's driver
    /// re-rolls its mine drop every tick, so a drop is announced when it starts; the Cannon has no
    /// press edge here, so the first round of a burst stands in for one). Skipped when the firer is the viewer, when the title
    /// raises none ([`oag_title::HudArt::incoming_warnings`]), outside the modes that raise
    /// them (Arcade, Tournament, Eliminator, Head2Head), and in any race with more than one
    /// person (the original's multiplayer modes never raise; a split-screen or network
    /// seat's own warnings are not wired). Every racing craft is a valid pair: the
    /// original's own refusal of a pair (its flag byte) is unread.
    pub(super) fn note_incoming_fire(&mut self, slot: usize, weapon: oag_tables::weapons::Weapon) {
        if slot == self.player_slot() || !self.incoming_warnings_drawn() {
            return;
        }
        let viewer = self.player_slot();
        let ships = &self.sim.world.ships;
        let (Some(here), Some(there)) = (
            ships[viewer].standing.progress,
            ships[slot].standing.progress,
        ) else {
            return;
        };
        let length = self.sim.course.as_deref().map(oag_race::Course::length);
        let gap =
            oag_weapons::projectile::missile::wrapped_gap(here - there, length.unwrap_or(0.0));
        if let Some(icon) = oag_hud::incoming::classify(weapon, gap) {
            self.view.incoming_raises.push(icon);
        }
    }

    /// Whether the viewer's warnings are drawn in this race: the title raises them, there is
    /// one view and one person, and the mode is one of those that raise them. Every raise,
    /// the original's and a mod's, goes through it.
    fn incoming_warnings_drawn(&self) -> bool {
        self.view.huds[self.player_slot()].incoming_enabled
            && self.viewport_count() <= 1
            && self.people() <= 1
            && self.view.seat.is_none()
            && matches!(
                self.sim.world.mode(),
                oag_race::Mode::SingleRace
                    | oag_race::Mode::Tournament
                    | oag_race::Mode::Eliminator
                    | oag_race::Mode::Head2Head
            )
    }

    /// A simulation mod's `raise-warning`: lights `icon`'s group on the viewer's HUD for
    /// `slot`, as a weapon fired at the viewer would. Dropped for a slot that is not the
    /// viewer and where the warnings are not drawn ([`Self::incoming_warnings_drawn`]).
    #[cfg(feature = "extensions")]
    pub(super) fn raise_mod_warning(&mut self, slot: usize, icon: oag_ext_abi::WarningIcon) {
        use oag_ext_abi::WarningIcon as W;
        use oag_hud::incoming::Icon;
        if slot == self.player_slot() && self.incoming_warnings_drawn() {
            self.view.incoming_raises.push(match icon {
                W::Rocket => Icon::Rocket,
                W::Missile => Icon::Missile,
                W::Quake => Icon::Quake,
                W::Cannon => Icon::Cannon,
                W::Plasma => Icon::Plasma,
                W::LeachBeam => Icon::LeachBeam,
                W::Repulser => Icon::Repulser,
                W::Shuriken => Icon::Shuriken,
                W::Mine => Icon::Mine,
                W::Bomb => Icon::Bomb,
            });
        }
    }
}
