//! The race phase of a server, once every seat has loaded: pacing the ticks on the wall
//! clock, refusing latecomers, ending the race and letting the last ticks arrive.
//!
//! `oag-server` and a game hosting a LAN race ([`crate::lan`]) both run it, so there is
//! one host loop. What differs is how they fill the seats (a fixed head count, or a lobby
//! that fixes it at START) and where they report, which is the `say` callback.

use web_time::{Duration, Instant};

use crate::host::{Host, Phase};
use crate::link::Link;

/// The tick, from the engine's one rate.
fn dt() -> Duration {
    Duration::from_secs_f64(f64::from(oag_core::tick::TickRate::DEFAULT.dt()))
}

/// How long a seat may send nothing during the race before its craft is handed to the AI.
/// Chosen: every client sends its input every frame, so this is many frames of nothing, and it
/// beats QUIC's 30 s idle timeout by a wide margin.
pub const SILENCE: Duration = Duration::from_secs(8);

/// How the race phase is paced and bounded.
#[derive(Debug, Clone, Copy)]
pub struct Pace {
    /// Ticks between telling the clients the race starts and running tick 0, so their
    /// first inputs are already here. Chosen.
    pub lead_ticks: u16,
    /// Stop after this many ticks even if the race has not finished.
    pub max_ticks: Option<u32>,
}

impl Default for Pace {
    fn default() -> Self {
        Self {
            lead_ticks: 6,
            max_ticks: None,
        }
    }
}

/// Starts the race on `host` (every seat ready) and runs it to its end: every person
/// finished, every person gone, `pace.max_ticks`, or `stop` returning true.
///
/// `accept` yields connections that arrive meanwhile, which are refused by name by the
/// host; `say` receives the log lines.
pub fn run(
    host: &mut Host,
    pace: Pace,
    accept: &mut dyn FnMut() -> Option<Box<dyn Link>>,
    stop: &dyn Fn() -> bool,
    say: &mut dyn FnMut(&str),
) {
    host.start(pace.lead_ticks);
    say(&format!("every seat is ready: {:?}", host.seat_names()));
    let dt = dt();
    let started = Instant::now() + dt * u32::from(pace.lead_ticks);
    let mut last_log = 0;
    loop {
        // A late connection is refused by name rather than left hanging.
        while let Some(link) = accept() {
            host.join(link);
        }
        host.poll();
        host.drop_silent(SILENCE);
        let over = if stop() {
            Some("the host stopped")
        } else {
            let due = started + dt * host.tick_number();
            let now = Instant::now();
            if now < due {
                std::thread::sleep((due - now).min(Duration::from_millis(2)));
                continue;
            }
            host.tick();
            let tick = host.tick_number();
            if tick - last_log >= 60 {
                last_log = tick;
                say(&second_line(host, tick));
            }
            if host.finished() {
                Some("every person has finished")
            } else if host.deserted() {
                Some("every person has left")
            } else if pace.max_ticks.is_some_and(|max| tick >= max) {
                Some("--max-ticks reached")
            } else {
                None
            }
        };
        if let Some(reason) = over {
            say(&format!("tick {}: {reason}", host.tick_number()));
            for line in board_lines(host) {
                say(&line);
            }
            host.end(reason);
            break;
        }
    }
    // Let the end and the last ticks reach the clients before the process goes.
    let linger = Instant::now();
    while linger.elapsed() < Duration::from_secs(1) && host.phase() == Phase::Over {
        host.poll();
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// One line a second: per seat, the ticks its input arrived in time, the ticks the last
/// input was repeated, inputs that came too late, and how many ticks ahead its input is
/// arriving.
fn second_line(host: &mut Host, tick: u32) -> String {
    let hash = host.history().last().map_or(0, |record| record.hash);
    let seats: Vec<String> = host
        .take_counts()
        .iter()
        .enumerate()
        .map(|(slot, counts)| match counts {
            Some(c) => format!(
                "seat {slot}: fresh {} repeated {} late {} lead {}",
                c.fresh, c.repeated, c.late, c.lead
            ),
            None => format!("seat {slot}: empty"),
        })
        .collect();
    format!("tick {tick:>6} hash {hash:#018x} | {}", seats.join(" | "))
}

/// The server's results, the race's truth, one line a craft.
fn board_lines(host: &Host) -> Vec<String> {
    let Some(board) = host.race().results() else {
        return Vec::new();
    };
    board
        .rows
        .iter()
        .enumerate()
        .map(|(place, row)| {
            format!(
                "board {}: slot {}{} laps {} finish tick {:?} best lap ticks {:?}",
                place + 1,
                row.slot,
                if row.player { " (person)" } else { "" },
                row.laps_completed,
                row.finish_tick,
                row.best_lap_ticks
            )
        })
        .collect()
}
