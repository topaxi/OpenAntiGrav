//! One local pad's rumble: the [`Player`] fed by the map.
//!
//! A [`Feed`] turns a race's per-tick output into voices. Edges arrive as the
//! [`CueEvent`]s the simulation already raises for sound; held conditions
//! arrive as a [`Levels`] set recomputed every tick, so a seek, a restart or a
//! lost cue cannot leave a motor running (`handover/gameplay/controller-rumble.md`
//! is the design; `docs/gameplay/rumble.md` the law).

use oag_gameplay::cue::CueEvent;

use super::map::{Edge, End, Level, ROWS, Row, Scope, Set, Shape, Trigger};
use super::{Handle, MotorOutput, Motors, Player, SmallMotor, StepContext};
use oag_tables::rumble::Library;

/// Which held conditions are up this tick for one ship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Levels {
    /// Over a mag strip.
    pub mag_strip: bool,
    /// Autopilot engaged.
    pub autopilot: bool,
    /// A Leech beam is fastened to the ship.
    pub leech_beam: bool,
}

impl Levels {
    fn get(self, level: Level) -> bool {
        match level {
            Level::MagStrip => self.mag_strip,
            Level::Autopilot => self.autopilot,
            Level::LeechBeam => self.leech_beam,
        }
    }
}

/// What a feed plays and how.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rules {
    /// The trigger set that fires.
    pub set: Set,
    /// Drop a world event's position, as the original does outside split
    /// screen: the gain then measures from the world origin
    /// (`docs/gameplay/rumble.md`, "A one-player race drops the position").
    pub drop_position: bool,
}

/// One pad's player and the state the map keeps between ticks.
#[derive(Debug, Clone)]
pub struct Feed {
    player: Player,
    rules: Rules,
    /// Each row's pattern id in the player's library, or `None` where the
    /// title ships no such pattern.
    ids: Vec<Option<usize>>,
    /// The held voice of each [`Trigger::Level`] row.
    held: Vec<Option<Handle>>,
    previous: Levels,
}

impl Feed {
    /// A feed over `library`, the small motor reading as `law`.
    #[must_use]
    pub fn new(library: Library, law: SmallMotor, rules: Rules) -> Self {
        let ids = ROWS.iter().map(|r| library.index_of(r.pattern)).collect();
        Self {
            player: Player::new(library, law),
            rules,
            ids,
            held: vec![None; ROWS.len()],
            previous: Levels::default(),
        }
    }

    /// The player, for reading its voices.
    #[must_use]
    pub fn player(&self) -> &Player {
        &self.player
    }

    fn rows(&self) -> impl Iterator<Item = (usize, &'static Row, usize)> + '_ {
        ROWS.iter()
            .enumerate()
            .filter(|(_, row)| self.rules.set.fires(row.from))
            .filter_map(|(i, row)| self.ids[i].map(|id| (i, row, id)))
    }

    /// An edge the simulation raised. `own` is the ship this pad belongs to:
    /// a craft cue from another ship reaches no motor, and a world cue (it
    /// carries a point) reaches every pad.
    pub fn event(&mut self, event: &CueEvent, own: usize) {
        let mine = event.point.is_none() && usize::from(event.slot) == own;
        let plays: Vec<(usize, Scope)> = self
            .rows()
            .filter(|(_, row, _)| row.trigger == Trigger::Cue(event.cue))
            .map(|(_, row, id)| (id, row.scope))
            .collect();
        for (id, scope) in plays {
            match scope {
                Scope::OwnShip if mine => {
                    self.player.one_shot(id, None);
                }
                Scope::World if event.point.is_some() => {
                    let at = if self.rules.drop_position {
                        None
                    } else {
                        event.point.map(|p| p.to_array())
                    };
                    self.player.one_shot(id, at);
                }
                _ => {}
            }
        }
    }

    /// This tick's held conditions for the pad's ship.
    pub fn levels(&mut self, now: Levels) {
        let previous = self.previous;
        self.previous = now;
        let rows: Vec<(usize, &'static Row, usize)> = self.rows().collect();
        for (i, row, id) in rows {
            match row.trigger {
                Trigger::Edge(level, Edge::Rise) if now.get(level) && !previous.get(level) => {
                    self.player.one_shot(id, None);
                }
                Trigger::Edge(level, Edge::Fall) if !now.get(level) && previous.get(level) => {
                    self.player.one_shot(id, None);
                }
                Trigger::Level(level) => {
                    let Shape::Held(end) = row.shape else {
                        continue;
                    };
                    if now.get(level) {
                        if self.held[i].is_none() {
                            self.held[i] = self.player.hold(id);
                        }
                    } else if let Some(handle) = self.held[i].take() {
                        match end {
                            End::Finish => self.player.finish(handle),
                            End::Stop => self.player.stop(handle),
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Ends every voice: a race left, a pad lost.
    pub fn clear(&mut self) {
        self.player.stop_all();
        self.held.iter_mut().for_each(|h| *h = None);
        self.previous = Levels::default();
    }

    /// One pad update, sent to `seat`'s pad.
    pub fn drive(&mut self, ctx: StepContext, seat: usize, out: &mut dyn MotorOutput) -> Motors {
        self.player.drive(ctx, seat, out)
    }
}

#[cfg(test)]
mod tests;
