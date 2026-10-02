//! Which circuits the Race Box offers: the disc's own `<Unlock Grid="...">`
//! rows, evaluated against the campaign progress this build persists.
//!
//! **Pulse only.** `TrackSelection_PopulateList` (`0x088edf3c`) keeps a
//! `PI_Track` only when `Definition_IsUnlocked` (`0x0888e29c`) passes - a
//! locked circuit is **absent from the list, not greyed** (decompile; and the
//! 2026-09-29 live capture, where the list went from 3 circuits to 24 with the
//! dev byte set). A named `<Unlock Grid="GridN">` row evaluates
//! `Unlock_GridPointsMet` (`0x0888ebd8`): the named grid's own medal points
//! reach its `RequiredPoints`. See `docs/formats/race-setup.md`, "Two unlock
//! axes", and `docs/ui/selection-screens.md`.
//!
//! The predicate is [`oag_tables::race_campaign::grid_points_met`], called
//! directly rather than through the campaign screen's `grid_is_unlocked`: that
//! one answers "is this *grid* open" and short-circuits `true` for `grid0`
//! (shipped with no lock), which would open every `Grid0`-gated circuit on a
//! fresh profile.
//!
//! **Not modelled, and open:** the mode-gated per-track byte at `+0x16e` that
//! `TrackSelection_PopulateList` also tests when its cached `Mode == 6`, and
//! the always-fails byte at `+0x99`. Neither is tied to an attribute
//! `Definition.xml` carries (`availableInZone` is on 16 circuits, the byte on
//! three), so nothing here guesses at them.
//!
//! `--unlock-all` ([`Gate::open`]) is **ours**: a developer and capture escape
//! that mirrors, as context only, the original's own dev byte at profile
//! `+0x45f`. It is not a game feature.

use std::sync::atomic::{AtomicBool, Ordering};

use oag_tables::race_campaign::{self, Grid, Medal};

use crate::catalogue::Track;
use crate::records::{self, Store};

/// A campaign medal as the campaign tables name it. The one conversion,
/// shared by the campaign screens and this gate.
#[must_use]
pub fn to_campaign_medal(medal: records::Medal) -> Medal {
    match medal {
        records::Medal::Gold => Medal::Gold,
        records::Medal::Silver => Medal::Silver,
        records::Medal::Bronze => Medal::Bronze,
    }
}

/// The best medal `records` holds for a campaign cell of `title`, in the
/// shape [`race_campaign::grid_points_met`] and `Grid::points_earned` take.
pub fn campaign_medal_of<'a>(
    records: &'a Store,
    title: &'a str,
) -> impl Fn(&str) -> Option<Medal> + 'a {
    move |cell| {
        records
            .campaign_medal(title, cell)?
            .best_medal
            .map(to_campaign_medal)
    }
}

static UNLOCK_ALL: AtomicBool = AtomicBool::new(false);

/// Turns every gate off for this process - `--unlock-all`. **Ours**, for
/// captures and tests; a process-wide switch because the live race box and the
/// `--menu-page` stills must agree without a flag threaded through both.
pub fn set_unlock_all(on: bool) {
    UNLOCK_ALL.store(on, Ordering::Relaxed);
}

/// Whether [`set_unlock_all`] is on.
#[must_use]
pub fn unlock_all() -> bool {
    UNLOCK_ALL.load(Ordering::Relaxed)
}

/// What gates a title's circuits.
#[derive(Debug, Clone, Default)]
pub struct Gate {
    /// The campaign grids the `<Unlock>` names resolve against. `None` means
    /// nothing gates: a title that authors no `<Unlock>` on its circuits, or
    /// `--unlock-all`.
    grids: Option<Vec<Grid>>,
}

impl Gate {
    /// Nothing gates: every circuit is offered.
    #[must_use]
    pub fn open() -> Self {
        Self { grids: None }
    }

    /// Circuits gate on `grids`' own medal points.
    #[must_use]
    pub fn on_grids(grids: Vec<Grid>) -> Self {
        Self { grids: Some(grids) }
    }

    /// Whether `track` is offered given `records` for the title `title`.
    #[must_use]
    pub fn offers(&self, track: &Track, records: &Store, title: &str) -> bool {
        let Some(grids) = self.grids.as_deref() else {
            return true;
        };
        if unlock_all() {
            return true;
        }
        match track.unlock_grid.as_deref() {
            None => true,
            Some(name) => {
                race_campaign::grid_points_met(grids, name, &campaign_medal_of(records, title))
            }
        }
    }
}

impl Gate {
    /// The gate `title`'s circuits answer to, read off its own archives.
    ///
    /// Pulse names its campaign grids in `<Unlock Grid="...">`; no other title
    /// authors a circuit `<Unlock>` this build reads (Pure has none, HD and
    /// Omega are not wired), so they get [`Self::open`]. A Pulse source whose
    /// grids will not read is logged and left open - there is nothing to gate
    /// against - rather than locking every circuit away.
    #[must_use]
    pub fn read(title: &str, archives: &mut oag_assets::Archives) -> Self {
        if title != oag_pulse::TITLE.name {
            return Self::open();
        }
        match crate::campaign::read_grids(archives, oag_pulse::campaign::DEFINITION_ENTRY) {
            Ok(grids) => Self::on_grids(grids),
            Err(error) => {
                log::warn!("{error:#} - no campaign grids, so no circuit is locked");
                Self::open()
            }
        }
    }
}
