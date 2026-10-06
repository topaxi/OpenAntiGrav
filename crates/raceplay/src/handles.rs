//! A race's effects, resolved once at load: each [`Trigger`] to the loaded
//! effect it plays, or nothing.
//!
//! The firing sites used to look an effect up by its string in
//! [`oag_fx::psys::Library`] every time one fired (some every frame), so a
//! misspelt name drew nothing and said nothing. They index this by
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
}

impl Default for EffectHandles {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
        }
    }
}

impl EffectHandles {
    /// Resolves every trigger `title` answers against what `library` loaded.
    /// A trigger the title leaves unread, or whose effect did not load, stays
    /// empty; the loader has already reported the missing name.
    #[must_use]
    pub fn resolve(title: &oag_title::Title, library: &Library) -> Self {
        let mut handles = Self::default();
        for trigger in Trigger::ALL {
            handles.slots[trigger.index()] = title
                .effect_on(trigger)
                .and_then(|spec| library.get(spec.effect))
                .cloned();
        }
        handles
    }

    /// The effect `trigger` plays, if it loaded.
    #[must_use]
    pub fn get(&self, trigger: Trigger) -> Option<&Arc<Effect>> {
        self.slots[trigger.index()].as_ref()
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
