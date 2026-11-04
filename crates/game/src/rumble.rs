//! Playing a rumble pattern on a real pad outside a race: the `--rumble-pattern`
//! probe, and the RUMBLE setting's mapping to a player's law.
//!
//! The probe is a feel test and a wiring proof for a test:
//! it opens the pad subsystem with no window, plays one pattern through
//! [`oag_input::rumble::Player`] and the `gilrs` backend, logs every level it
//! sent, and exits with the motors silent.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use oag_input::Pad;
use oag_input::rumble::feed::{Feed, Levels, Rules};
use oag_input::rumble::map::Set;
use oag_input::rumble::{Mode, MotorOutput, Motors, Player, SmallMotor, StepContext};
use oag_raceplay::Race;
use oag_tables::rumble::{Kind, Library};
use oag_title::rumble::{Rumble, Triggers};

/// The small-motor law the `RUMBLE` token plays on a title, or `None` for
/// nothing (`none`, or `original` on a title whose original has no patterns).
#[must_use]
pub fn law(token: &str, original: Rumble) -> Option<SmallMotor> {
    Mode::from_name(token)
        .unwrap_or_default()
        .law(original_law(original))
}

/// What the title's original does with the small motor, in the player's words.
#[must_use]
pub fn original_law(original: Rumble) -> Option<SmallMotor> {
    original.small_motor().map(|law| match law {
        oag_title::rumble::SmallMotor::OnOff => SmallMotor::OnOff,
        oag_title::rumble::SmallMotor::Sum => SmallMotor::Sum,
    })
}

/// What `--rumble-pattern` was asked to do.
#[derive(Debug, Clone, Copy)]
pub struct Probe<'a> {
    /// A disc pattern's name, or the path of a `.toml` pattern.
    pub pattern: &'a str,
    /// The source (disc image or folder) for a disc pattern.
    pub source: Option<&'a str>,
    /// The `[controls] rumble` token.
    pub token: &'a str,
    /// Seconds a looping pattern is held before it is finished.
    pub hold: f32,
}

/// Plays the pattern on every force-feedback pad, printing each level sent.
///
/// # Errors
///
/// No pattern, no force-feedback pad, or `RUMBLE` set to play nothing.
pub fn probe(probe: &Probe<'_>) -> Result<()> {
    let (library, original) = library(probe)?;
    let law = law(probe.token, original).with_context(|| {
        format!(
            "RUMBLE is {:?}, which plays nothing on this title; use ps3 or ps4",
            probe.token
        )
    })?;
    if let Some(p) = library.at(0) {
        println!(
            "pattern: {:?}, {} frames, loop {}..{}, large peak {} ({} non-zero), small on {}",
            p.kind,
            p.frames(),
            p.loop_start,
            p.loop_end,
            p.large.iter().max().copied().unwrap_or(0),
            p.large.iter().filter(|&&b| b != 0).count(),
            p.small.iter().filter(|&&b| b != 0).count(),
        );
    }
    let mut pad = Pad::new();
    pad.poll_players();
    println!("pads: {:?}", pad.names());
    let motors = pad.motor_pads();
    println!("force feedback: {motors:?}");
    if motors.is_empty() {
        bail!("no connected pad reports force feedback (see the log for gilrs' own message)");
    }
    let mut player = Player::new(library, law);
    let kind = player
        .library()
        .at(0)
        .map(|p| p.kind)
        .context("empty library")?;
    let held = (kind == Kind::Looping).then(|| player.hold(0)).flatten();
    if held.is_none() {
        player.one_shot(0, None);
    }
    let started = Instant::now();
    let ctx = StepContext {
        race: true,
        ..StepContext::default()
    };
    let mut last = Motors::OFF;
    let mut changes = 0u32;
    let mut finished = held.is_none();
    while player.active_voices() > 0 && started.elapsed() < Duration::from_secs(12) {
        let tick = Instant::now();
        pad.poll_players();
        if let Some(handle) = held
            && !finished
            && started.elapsed().as_secs_f32() >= probe.hold
        {
            player.finish(handle);
            finished = true;
        }
        let now = player.drive(ctx, 0, &mut pad);
        if now != last {
            changes += 1;
            println!(
                "{:7.3}s large {:3} small {:3}",
                started.elapsed().as_secs_f32(),
                now.large,
                now.small
            );
            last = now;
        }
        if let Some(rest) = Duration::from_micros(16_667).checked_sub(tick.elapsed()) {
            std::thread::sleep(rest);
        }
    }
    pad.silence();
    pad.poll_players();
    println!(
        "{changes} level changes in {:.2}s, motors silenced",
        started.elapsed().as_secs_f32()
    );
    Ok(())
}

fn library(probe: &Probe<'_>) -> Result<(Library, Rumble)> {
    let mut library = Library::new();
    if probe.pattern.ends_with(".toml") {
        let text = std::fs::read_to_string(probe.pattern)
            .with_context(|| format!("reading {}", probe.pattern))?;
        let pattern =
            oag_mod::rumble::parse(&text).map_err(|e| anyhow::anyhow!("{}: {e}", probe.pattern))?;
        library.insert("probe", pattern);
        return Ok((library, Rumble::Absent));
    }
    let source = oag_source::source::resolve(probe.source, None)?;
    let opened = oag_source::title::open_source(&source, Vec::new(), Vec::new())
        .with_context(|| format!("opening {source}"))?;
    let mut archives = opened.archives;
    library.insert(
        probe.pattern,
        oag_raceplay::rumble_library::disc_pattern(&mut archives, probe.pattern)?,
    );
    Ok((library, opened.title.rumble))
}

/// The trigger set `token` plays on a title whose original has `original`, with
/// the small-motor law, or `None` for nothing (`docs/gameplay/rumble.md`,
/// "Player and device"). `ps3` and `ps4` are the same on every title; `original`
/// is the title's own and plays nothing where it has none.
#[must_use]
pub fn plays(token: &str, original: Rumble) -> Option<(Set, SmallMotor)> {
    let mode = Mode::from_name(token).unwrap_or_default();
    let law = mode.law(original_law(original))?;
    let set = match mode {
        Mode::None => return None,
        Mode::Ps3 => Set::Ps3,
        Mode::Ps4 => Set::Ps4,
        Mode::Original => match original.triggers()? {
            Triggers::Hd => Set::Ps3,
            Triggers::Omega => Set::Ps4,
        },
    };
    Some((set, law))
}

/// A race's pad rumble: one [`Feed`] per local person, each on the pad
/// assigned to that person's slot.
///
/// Rumble is an output of the race (ADR-0018): [`Self::tick`] reads the cues
/// the tick raised before the sound drains them, and levels off the world, and
/// writes to the pads. Nothing here is state a replay or a hash sees.
#[derive(Debug)]
pub struct Rumbler {
    seats: Vec<Seat>,
}

/// One person's feed and what carries from tick to tick.
#[derive(Debug)]
struct Seat {
    slot: usize,
    feed: Feed,
    /// Last tick's speed towards the ground, the touchdown speed's source.
    falling: f32,
}

impl Rumbler {
    /// The race's feeds, or `None` where the title ships no patterns, the
    /// setting plays nothing or no person flies here.
    #[must_use]
    pub fn new(race: &Race, token: &str, original: Rumble) -> Option<Self> {
        let (set, law) = plays(token, original)?;
        let library = race.rumble_library();
        if library.is_empty() {
            return None;
        }
        let rules = Rules {
            set,
            drop_position: Mode::from_name(token) == Some(Mode::Original) && race.people() <= 1,
        };
        let seats: Vec<Seat> = (0..oag_gameplay::MAX_PLAYERS)
            .filter(|&slot| race.sim.world.person_flown(slot) && race.heard_here(slot))
            .map(|slot| Seat {
                slot,
                feed: Feed::new(library.clone(), law, rules),
                falling: 0.0,
            })
            .collect();
        (!seats.is_empty()).then_some(Self { seats })
    }

    /// One simulation tick. Call it after `Race::tick` and before the sound
    /// drains the cues.
    pub fn tick(&mut self, race: &Race, paused: bool, out: &mut dyn MotorOutput) {
        let dt = race.dt();
        for seat in &mut self.seats {
            let slot = seat.slot;
            for event in race.pending_cues() {
                seat.feed.event(event, slot);
            }
            seat.feed.levels(levels_of(race, slot));
            seat.feed
                .wall(race.wall_contact_of(slot).map(|c| (c.size, c.side)), dt);
            let ship = race.sim.world.ships.get(slot);
            let physics = ship.map(|s| &s.physics);
            // Touching down: grounded now and not on the tick before. The speed
            // is the last airborne tick's, since the hover response has already
            // taken the rest out of this one.
            let touching = physics.is_some_and(|p| p.grounded > 0.0 && p.grounded_prev <= 0.0);
            seat.feed
                .touchdown(if touching { seat.falling } else { 0.0 }, dt);
            seat.falling = physics.map_or(0.0, |p| -p.body.linear_velocity.dot(p.body.up()));
            let ctx = StepContext {
                race: true,
                paused,
                listener: physics.map_or([0.0; 3], |p| p.body.position.to_array()),
            };
            seat.feed.drive(ctx, slot, out);
        }
    }

    /// Voices started per pattern name across every seat, in first-fired order.
    #[must_use]
    pub fn fired(&self) -> Vec<(&'static str, u32)> {
        let mut total: Vec<(&'static str, u32)> = Vec::new();
        for seat in &self.seats {
            for &(name, count) in seat.feed.fired() {
                match total.iter_mut().find(|(n, _)| *n == name) {
                    Some((_, c)) => *c += count,
                    None => total.push((name, count)),
                }
            }
        }
        total
    }

    /// Ends every voice and silences the pads: a race left.
    pub fn stop(&mut self, out: &mut dyn MotorOutput) {
        for seat in &mut self.seats {
            seat.feed.clear();
        }
        out.silence();
    }
}

/// The held conditions of `slot`'s ship this tick.
fn levels_of(race: &Race, slot: usize) -> Levels {
    let world = &race.sim.world;
    Levels {
        mag_strip: race.over_magstrip(slot),
        autopilot: world
            .ships
            .get(slot)
            .is_some_and(|ship| ship.autopilot_timer > 0.0),
        leech_beam: race.leach_beam_on(slot),
    }
}
