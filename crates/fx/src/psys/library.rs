//! The effects a caller has loaded, by name - split out of `psys.rs`, which
//! is past the 1,000-line rule and ratcheted.

use super::{Effect, Render, Sheet, StreakDraw};

/// The effects a caller has loaded, by name.
///
/// The generic half of playing the disc's own effects: every
/// `Data\Psys\*.POB` reaches a [`Stage`] the same way, so nothing about an
/// effect needs code of its own. What is *not* generic, and cannot be, is
/// **when** each one fires - that is per-effect reverse-engineering, and the
/// caller that recovered a trigger is the one that names the effect here.
///
/// A `Vec` rather than a map: a race loads a handful of effects, lookups
/// happen at trigger time rather than per particle, and the order stays the
/// caller's so a loader report reads the same way twice.
#[derive(Debug, Clone, Default)]
pub struct Library {
    effects: Vec<(String, std::sync::Arc<Effect>)>,
    /// Every loaded billboard's sprite, packed - see [`sprite`].
    sheet: Sheet,
}

impl Library {
    /// An empty library.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `effect` under `name`, replacing any effect already there.
    ///
    /// Places each billboard emitter's sprite on the library's [`Sheet`]
    /// first; one that does not fit keeps the procedural profile.
    pub fn insert(&mut self, name: &str, mut effect: Effect) {
        for spec in &mut effect.emitters {
            // A streak samples only where its strip is read - see [`streak`].
            let sampled = spec.render == Render::Billboard || spec.streak != StreakDraw::Procedural;
            if let (true, Some(sprite)) = (sampled, &spec.sprite) {
                spec.sheet_rect = self.sheet.place(sprite);
            }
        }
        let effect = std::sync::Arc::new(effect);
        match self.effects.iter_mut().find(|(key, _)| key == name) {
            Some(slot) => slot.1 = effect,
            None => self.effects.push((name.to_string(), effect)),
        }
    }

    /// The effect loaded under `name`, if it loaded at all.
    ///
    /// `None` is the normal answer in a headless test with no disc, and the
    /// answer a caller must handle by drawing nothing.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&std::sync::Arc<Effect>> {
        self.effects
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, effect)| effect)
    }

    /// How many effects loaded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Whether nothing loaded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Their names, in load order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.effects.iter().map(|(name, _)| name.as_str())
    }

    /// The sprites every loaded effect draws with, for
    /// [`Pipeline::sync_sheet`].
    #[must_use]
    pub fn sheet(&self) -> &Sheet {
        &self.sheet
    }
}

impl Effect {
    /// The names of the emitters whose sprite loaded but did not fit the
    /// [`Library`]'s sheet, which draw the procedural profile instead.
    pub fn unplaced_sprites(&self) -> impl Iterator<Item = &str> {
        self.emitters
            .iter()
            .filter(|spec| {
                let sampled =
                    spec.render == Render::Billboard || spec.streak != StreakDraw::Procedural;
                sampled && spec.sprite.is_some() && spec.sheet_rect.is_none()
            })
            .map(|spec| spec.name.as_str())
    }
}
