//! A race's effects, resolved once at load: each [`Trigger`] to the loaded
//! effect it plays, or nothing.
//!
//! A string lookup in [`oag_fx::psys::Library`] at each firing (some every
//! frame) would let a misspelt name draw nothing and say nothing. Firing sites index this by
//! [`Trigger`] instead: the name is read once, off the title's own table
//! ([`oag_title::Effects`]), and a trigger that does not exist is a compile
//! error. The [`Library`] stays for what only the
//! circuit's own data can name (placed scenery, weather) and for the sprite
//! sheet every loaded effect shares.

use oag_fx::psys::{Effect, Library};
use oag_title::Trigger;
use std::sync::Arc;

/// Each [`Trigger`]'s loaded effect.
#[derive(Debug, Clone)]
pub struct EffectHandles {
    slots: [Option<Arc<Effect>>; Trigger::COUNT],
    quake_look: oag_title::QuakeLook,
    own: crate::extension_effects::ModEffects,
}

impl Default for EffectHandles {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            quake_look: oag_title::QuakeLook::track(oag_title::Origin::Measured),
            own: crate::extension_effects::ModEffects::default(),
        }
    }
}

impl EffectHandles {
    /// Resolves every trigger `title` answers against what `library` loaded.
    /// A trigger the title leaves unread, or whose effect did not load, stays
    /// empty; the loader has already reported the missing name.
    #[must_use]
    pub fn resolve(title: &oag_title::Title, library: &Library) -> Self {
        let mut handles = Self {
            quake_look: title.effects.quake_look,
            ..Self::default()
        };
        for trigger in Trigger::ALL {
            handles.slots[trigger.index()] = title
                .effect_on(trigger)
                .and_then(|spec| library.get(spec.effect))
                .cloned();
        }
        handles
    }

    /// Adds the effects the race's mods ship (`fire-effect`'s own half).
    #[must_use]
    pub fn with_own(mut self, own: crate::extension_effects::ModEffects) -> Self {
        self.own = own;
        self
    }

    /// Plays `effect` on `trigger` where the title's own table names none (a lent burst, the
    /// player's `graphics.trail_hits`). `None` leaves the trigger as it was.
    #[must_use]
    pub fn lending(mut self, trigger: Trigger, effect: Option<Arc<Effect>>) -> Self {
        if effect.is_some() {
            self.slots[trigger.index()] = effect;
        }
        self
    }

    /// Plays nothing on `trigger`: a title's own trail-hit burst with the player's
    /// `graphics.trail_hits` off, on a title whose original never fires it.
    #[must_use]
    pub fn without(mut self, trigger: Trigger) -> Self {
        self.slots[trigger.index()] = None;
        self
    }

    /// The effect `index` of mod `mod_id`, if it loaded.
    #[must_use]
    pub fn own(&self, mod_id: &str, index: u16) -> Option<&crate::extension_effects::OwnEffect> {
        self.own.get(mod_id, index)
    }

    /// The effect `trigger` plays, if it loaded.
    #[must_use]
    pub fn get(&self, trigger: Trigger) -> Option<&Arc<Effect>> {
        self.slots[trigger.index()].as_ref()
    }

    /// How the title places the Quake wave's effect in its frame.
    #[must_use]
    pub fn quake_look(&self) -> oag_title::QuakeLook {
        self.quake_look
    }

    /// Puts `effect` on `trigger`. What a test uses to stand an effect in,
    /// the way [`Library::insert`] did before.
    pub fn insert(&mut self, trigger: Trigger, effect: Effect) {
        self.slots[trigger.index()] = Some(Arc::new(effect));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trigger_is_empty_until_its_effect_loaded() {
        let handles = EffectHandles::resolve(oag_pulse::TITLE, &Library::new());
        assert!(Trigger::ALL.iter().all(|&t| handles.get(t).is_none()));
    }
}
