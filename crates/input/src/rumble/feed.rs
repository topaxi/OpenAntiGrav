//! One local pad's rumble: the [`Player`] fed by the map.
//!
//! A [`Feed`] turns a race's per-tick output into voices. Edges arrive as the
//! [`CueEvent`]s the simulation already raises for sound; held conditions
//! arrive as a [`Levels`] set recomputed every tick, so a seek, a restart or a
//! lost cue cannot leave a motor running (`handover/gameplay/controller-rumble.md`
//! is the design; `docs/gameplay/rumble.md` the law).

use oag_gameplay::cue::CueEvent;

use super::contact::{LANDING, Selector, WALL};
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
    /// The wrong-way warning is up.
    pub wrong_way: bool,
    /// The ship is blowing up.
    pub exploding: bool,
    /// The weapon reticle has taken its lock.
    pub locked_on: bool,
}

impl Levels {
    fn get(self, level: Level) -> bool {
        match level {
            Level::MagStrip => self.mag_strip,
            Level::Autopilot => self.autopilot,
            Level::LeechBeam => self.leech_beam,
            Level::WrongWay => self.wrong_way,
            Level::Exploding => self.exploding,
            Level::LockedOn => self.locked_on,
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
    /// How many voices each pattern has started, by name, in first-fired
    /// order: what a run reports.
    fired: Vec<(&'static str, u32)>,
    selector: Selector,
}

impl Feed {
    /// A feed over `library`, the small motor reading as `law`.
    #[must_use]
    pub fn new(library: Library, law: SmallMotor, rules: Rules) -> Self {
        let ids = ROWS.iter().map(|r| library.index_of(r.pattern)).collect();
        let selector = Selector::new(&library);
        Self {
            player: Player::new(library, law),
            rules,
            ids,
            held: vec![None; ROWS.len()],
            previous: Levels::default(),
            fired: Vec::new(),
            selector,
        }
    }

    /// Voices started per pattern name, in first-fired order.
    #[must_use]
    pub fn fired(&self) -> &[(&'static str, u32)] {
        &self.fired
    }

    fn note(&mut self, name: &'static str) {
        match self.fired.iter_mut().find(|(n, _)| *n == name) {
            Some((_, count)) => *count += 1,
            None => {
                log::info!("rumble: first voice of {name}");
                self.fired.push((name, 1));
            }
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
        let plays: Vec<(usize, Scope, &'static str)> = self
            .rows()
            .filter(|(_, row, _)| row.trigger == Trigger::Cue(event.cue))
            .map(|(_, row, id)| (id, row.scope, row.pattern))
            .collect();
        for (id, scope, name) in plays {
            match scope {
                Scope::OwnShip if mine => {
                    if self.player.one_shot(id, None) {
                        self.note(name);
                    }
                }
                Scope::World if event.point.is_some() => {
                    let at = if self.rules.drop_position {
                        None
                    } else {
                        event.point.map(|p| p.to_array())
                    };
                    if self.player.one_shot(id, at) {
                        self.note(name);
                    }
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
                    if self.player.one_shot(id, None) {
                        self.note(row.pattern);
                    }
                }
                Trigger::Edge(level, Edge::Fall) if !now.get(level) && previous.get(level) => {
                    if self.player.one_shot(id, None) {
                        self.note(row.pattern);
                    }
                }
                Trigger::Level(level) => {
                    let Shape::Held(end) = row.shape else {
                        continue;
                    };
                    if now.get(level) {
                        if self.held[i].is_none() {
                            self.held[i] = self.player.hold(id);
                            if self.held[i].is_some() {
                                self.note(row.pattern);
                            }
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

    /// This tick's wall contact `(size, side)`, `None` with none, over `dt`
    /// seconds (`docs/gameplay/rumble.md`, "Contact and landing").
    pub fn wall(&mut self, reading: Option<(f32, f32)>, dt: f32) {
        if let Some(band) = self.selector.contact(&mut self.player, reading, dt) {
            self.note(WALL[band]);
        }
    }

    /// The speed the craft meets the ground at this tick, 0 when it is not
    /// touching down.
    pub fn touchdown(&mut self, v: f32, dt: f32) {
        if let Some(band) = self.selector.landing(&mut self.player, v, dt) {
            self.note(LANDING[band]);
        }
    }

    /// Ends every voice: a race left, a pad lost.
    pub fn clear(&mut self) {
        self.player.stop_all();
        self.held.iter_mut().for_each(|h| *h = None);
        self.previous = Levels::default();
        self.selector.clear();
    }

    /// One pad update, sent to `seat`'s pad.
    pub fn drive(&mut self, ctx: StepContext, seat: usize, out: &mut dyn MotorOutput) -> Motors {
        self.player.drive(ctx, seat, out)
    }
}

#[cfg(test)]
mod tests;
