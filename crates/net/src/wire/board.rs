//! The final board of an online race: the server's own results, carried in [`super::Control::End`]
//! and through the lobby to every member of the room.
//!
//! The clients derive the same board from their confirmed state, which the tick hash proves equal
//! to the server's. This is the server's copy: what the room screen shows after the race, what a
//! client checks its own against, and where the people's names come from (the race itself only
//! knows a slot is a person). Chosen, not measured: the original has no network race.

use super::{Malformed, Reader, put_string};

/// One craft's line of the board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardRow {
    /// The place, 1 for the winner (`oag_race::places`, gaps kept as the HUD shows them).
    pub place: u8,
    /// The grid slot, which is the seat for a person.
    pub slot: u8,
    /// The person's name, empty for a craft the AI flew.
    pub name: String,
    /// Laps completed.
    pub laps: u16,
    /// The race clock's ticks at the finish, `None` for a craft still racing when the race ended.
    pub finish_ticks: Option<u32>,
    /// The craft's quickest lap in ticks, `None` if it never completed one.
    pub best_lap_ticks: Option<u32>,
    /// Craft this one destroyed (the Eliminator's count, zero elsewhere).
    pub eliminations: u16,
}

/// The board: every craft on the grid, first place first. Empty when the race ended before
/// anyone could be placed (everyone left, a limit, a crash).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Board {
    /// The rows in place order.
    pub rows: Vec<BoardRow>,
}

const NONE: u32 = u32::MAX;

fn opt(value: Option<u32>) -> u32 {
    value.unwrap_or(NONE)
}

fn some(value: u32) -> Option<u32> {
    (value != NONE).then_some(value)
}

impl Board {
    /// Whether there is nothing to show.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The row a person's seat flew.
    #[must_use]
    pub fn seat(&self, slot: usize) -> Option<&BoardRow> {
        self.rows
            .iter()
            .find(|row| usize::from(row.slot) == slot && !row.name.is_empty())
    }

    /// The board's rows without the names: what two ends compare, since only the server knows
    /// the names.
    #[must_use]
    pub fn same_race_as(&self, other: &Self) -> bool {
        self.rows.len() == other.rows.len()
            && self.rows.iter().zip(&other.rows).all(|(a, b)| {
                (a.place, a.slot, a.laps, a.finish_ticks, a.best_lap_ticks)
                    == (b.place, b.slot, b.laps, b.finish_ticks, b.best_lap_ticks)
            })
    }

    pub(crate) fn put(&self, out: &mut Vec<u8>) {
        out.push(u8::try_from(self.rows.len()).unwrap_or(u8::MAX));
        for row in self.rows.iter().take(usize::from(u8::MAX)) {
            out.push(row.place);
            out.push(row.slot);
            put_string(out, &row.name);
            out.extend_from_slice(&row.laps.to_le_bytes());
            out.extend_from_slice(&opt(row.finish_ticks).to_le_bytes());
            out.extend_from_slice(&opt(row.best_lap_ticks).to_le_bytes());
            out.extend_from_slice(&row.eliminations.to_le_bytes());
        }
    }

    pub(crate) fn take(r: &mut Reader<'_>) -> Result<Self, Malformed> {
        let count = r.u8()?;
        let rows = (0..count)
            .map(|_| {
                Ok(BoardRow {
                    place: r.u8()?,
                    slot: r.u8()?,
                    name: r.string()?,
                    laps: r.u16()?,
                    finish_ticks: some(r.u32()?),
                    best_lap_ticks: some(r.u32()?),
                    eliminations: r.u16()?,
                })
            })
            .collect::<Result<Vec<_>, Malformed>>()?;
        Ok(Self { rows })
    }

    /// The board as one line of hex, for the race child to print to the lobby.
    #[must_use]
    pub fn to_line(&self) -> String {
        let mut out = Vec::new();
        self.put(&mut out);
        super::to_hex(&out)
    }

    /// Parses [`Self::to_line`].
    ///
    /// # Errors
    ///
    /// [`Malformed`] for text that is not hex or not a board.
    pub fn from_line(line: &str) -> Result<Self, Malformed> {
        let bytes = super::from_hex(line)?;
        let mut r = Reader { bytes: &bytes };
        let board = Self::take(&mut r)?;
        r.end()?;
        Ok(board)
    }
}
