//! The table a finished race leaves behind: who came where, on which lap, at
//! what time, as plain data.
//!
//! The overlay that draws it over the frozen scene is `oag_game::scoreboard`;
//! this half is what a race builds once it ends and what a tournament adds up,
//! so it lives with [`crate::Race`]. See that module for why a row names a grid
//! slot and nothing else.

/// One craft, as the caller knows it before the board is ordered.
///
/// Taken rather than a `&World` so that [`build`] is a pure function of the
/// numbers and can be tested without a disc, a track or a physics step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Craft {
    /// Grid slot, zero-based. Slot 0 is the player.
    pub slot: u8,
    /// Race position, 1-based, as `oag_race::places` ordered it.
    pub place: u8,
    /// The lap this craft was on, 1-based, exactly as `oag_race::Standing`
    /// means it.
    pub lap: u32,
    /// The tick it crossed the line for the last time, or `None` if it had not.
    pub finish_tick: Option<u64>,
    /// Its own quickest lap, in ticks, or `None` if it never completed one.
    pub best_lap_ticks: Option<u32>,
}

/// One line of the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Race position, 1-based.
    pub place: u8,
    /// Grid slot, zero-based.
    pub slot: u8,
    /// Laps completed, which is one less than the lap being driven - and capped
    /// at the target for a craft that has finished, whose lap counter is one
    /// past it.
    pub laps_completed: u32,
    /// The tick it finished on, or `None` for a craft still racing when the
    /// race ended.
    pub finish_tick: Option<u64>,
    /// Its own quickest lap, in ticks, or `None` if it never completed one.
    ///
    /// The craft's own clock (`oag_race::Standing::best_lap_ticks`) rather than
    /// the player's, which is what makes this a column rather than a footer.
    pub best_lap_ticks: Option<u32>,
    /// Whether this is the player's row.
    pub player: bool,
}

impl Row {
    /// Whether this craft finished the race.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.finish_tick.is_some()
    }
}

/// A finished race's results, ordered by place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    /// Every craft that was on the grid, first place first.
    pub rows: Vec<Row>,
    /// Laps the race ran to, or `None` for a mode that never ends on its own.
    pub laps_target: Option<u32>,
    /// The tick the race ended on - the player's own last crossing.
    pub tick: u64,
}

impl Board {
    /// The player's row, which is the one a caller usually wants.
    #[must_use]
    pub fn player(&self) -> Option<&Row> {
        self.rows.iter().find(|row| row.player)
    }

    /// Whether there is a *place* to show at all.
    ///
    /// **The HUD's own rule, deliberately the same one.** A place is a position
    /// among opponents, and a time trial, a speed lap and a Zone run all grid
    /// the player alone: `oag_hud::Readout` omits its position widget on a
    /// field of one because `1 / 1` is arithmetic rather than a standing, and a
    /// results table that printed `POS 1` under it would contradict the screen
    /// the player was looking at a tick earlier.
    #[must_use]
    pub fn shows_place(&self) -> bool {
        self.rows.len() > 1
    }
}

/// Orders the field into a board.
///
/// The ordering is **not** redone here: `crafts` carries the place
/// `oag_race::places` already assigned, and this sorts on it, so the table and
/// the HUD's position readout cannot disagree. Two craft cannot share a place -
/// that function breaks every tie, down to the slot index - so the sort is
/// total.
#[must_use]
pub fn build(crafts: &[Craft], laps_target: Option<u32>, tick: u64) -> Board {
    let mut rows: Vec<Row> = crafts
        .iter()
        .map(|craft| Row {
            place: craft.place,
            slot: craft.slot,
            // A finished craft is on `target + 1` by the time its finish tick is
            // written, so completed laps would read one too many without the cap.
            laps_completed: match (craft.finish_tick, laps_target) {
                (Some(_), Some(target)) => target,
                _ => craft.lap.saturating_sub(1),
            },
            finish_tick: craft.finish_tick,
            best_lap_ticks: craft.best_lap_ticks,
            player: craft.slot == 0,
        })
        .collect();
    rows.sort_by_key(|row| (row.place, row.slot));
    Board {
        rows,
        laps_target,
        tick,
    }
}
