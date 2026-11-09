//! What Track Select and Ship Select would open on, read off the disc ahead of the tap.
//!
//! A selection screen's first picture has nothing earlier to stay up, so the frame that opens it
//! takes the read whole; waited for there, it is a third of a slow menu frame on the Steam Deck.
//! So while a menu frame runs, a worker reads what each of the two screens would open on now, and
//! the opening frame finds it landed. It costs one speculative read per change of pick, which
//! the screen checks ([`crate::picker_stage::OpeningRead`]): a read that guessed wrong is dropped
//! and the screen reads for itself, as it does for one the tap beat. See
//! `docs/rendering/frame-thread-cpu.md` ("A picker's opening read").
//!
//! An open screen's own kind is left alone: its Moved events read the same entries already, and
//! the screen it leads to (or back to) is the one worth having ready.
//!
//! Natively only: this needs the menus' source, which the web build does not open on a worker.

use oag_ui_screens::picker::Kind;

use crate::picker_stage::{LiveryAxis, OpeningRead, PreviewSource, SharedArchives};
use crate::stage::Stage;

use super::Session;
use super::picker::{ShipScreen, TrackScreen};

/// The picks a read was started for.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Asked {
    source: Option<super::menu_source::Key>,
    race: oag_game::pending_race::PendingRace,
    language: Option<String>,
    /// The pick on the screen's own axis: the circuit, or the team and its livery.
    picks: Vec<String>,
}

#[derive(Default)]
struct Slot {
    asked: Option<Asked>,
    read: Option<OpeningRead>,
}

/// One slot per screen.
#[derive(Default)]
pub(crate) struct OpeningReads {
    track: Slot,
    ship: Slot,
    /// The screen open on the last frame this looked: a frame that changed it has work enough.
    open: Option<Kind>,
}

impl OpeningReads {
    /// The read for a screen of `kind`, for the screen to adopt.
    pub(super) fn take(&mut self, kind: Kind) -> Option<OpeningRead> {
        let slot = self.slot(kind);
        slot.asked = None;
        slot.read.take()
    }

    fn slot(&mut self, kind: Kind) -> &mut Slot {
        match kind {
            Kind::Track => &mut self.track,
            Kind::Ship => &mut self.ship,
        }
    }
}

impl Session {
    /// Whether the reads of the screens that could open next have landed.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn opening_reads_landed(&self) -> bool {
        let Stage::Menu(stage) = &self.stage else {
            return false;
        };
        let open = stage.picker.as_ref().map(|picker| picker.model.kind());
        let reads = &self.opening_reads;
        [(Kind::Track, &reads.track), (Kind::Ship, &reads.ship)].iter().all(|(kind, slot)| {
            open == Some(*kind)
                || (slot.asked.is_some() && slot.read.as_ref().is_none_or(OpeningRead::landed))
        })
    }

    /// Starts the read of whichever screen's opening changed. Off a menu, drops what it holds:
    /// a race can change the records the livery row reads.
    pub(crate) fn refresh_opening_reads(&mut self) {
        let Stage::Menu(stage) = &self.stage else {
            self.opening_reads = OpeningReads::default();
            return;
        };
        let open = stage.picker.as_ref().map(|picker| picker.model.kind());
        // Working out a screen and starting its read costs a frame 1 to 2 ms on the Deck. Not on
        // the frame that just changed screen, and one a frame, so it never adds to a slow one.
        if std::mem::replace(&mut self.opening_reads.open, open) != open {
            return;
        }
        for kind in [Kind::Track, Kind::Ship] {
            let asked = self.asked(kind);
            if open != Some(kind) && self.opening_reads.slot(kind).asked.as_ref() != Some(&asked) {
                self.start_opening_read(kind, asked);
                return;
            }
        }
    }

    fn asked(&self, kind: Kind) -> Asked {
        let race = &self.settings.race;
        Asked {
            source: self.menu_source_key(),
            race: self.pending_race(),
            language: self.settings.language.clone(),
            picks: match kind {
                Kind::Track => vec![race.track.clone()],
                Kind::Ship => vec![
                    race.team.clone(),
                    race.skin.clone(),
                    race.variant.clone(),
                    race.variant_chosen.to_string(),
                ],
            },
        }
    }

    fn start_opening_read(&mut self, kind: Kind, asked: Asked) {
        // Not asked until the source is there to read from: the next frame tries again.
        let Some((archives, gate)) = self.landed_menu_source() else {
            return;
        };
        let screen = match kind {
            Kind::Track => self.track_screen(&gate).map(|screen| {
                let TrackScreen { model, sources, layout, .. } = screen;
                (model, sources, layout, LiveryAxis::Variant)
            }),
            // The team gate is the one fact left out, since it reads the disc: a locked team's
            // screen lists fewer entries, and its read then fails to match and is redone.
            Kind::Ship => self.ship_screen(0, &[], &self.records).map(|screen| {
                let ShipScreen { model, sources, layout, axis } = screen;
                (model, sources, layout, axis)
            }),
        };
        let read = screen.and_then(|(model, sources, layout, axis)| {
            self.read_opening(&model, &layout, axis, &sources, &archives)
        });
        *self.opening_reads.slot(kind) = Slot { asked: Some(asked), read };
    }

    fn read_opening(
        &self,
        model: &oag_ui_screens::picker::Picker,
        layout: &oag_ui_screens::picker::Layout,
        axis: LiveryAxis,
        sources: &[PreviewSource],
        archives: &SharedArchives,
    ) -> Option<OpeningRead> {
        let base = &self.shell.as_ref()?.sprites;
        let previews = self.picker_previews()?;
        Some(OpeningRead::start(model, layout, axis, sources, &previews, archives, base))
    }
}
