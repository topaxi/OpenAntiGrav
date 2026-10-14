//! `oag-server`: the headless authoritative race server.
//!
//! Loads a title's race the way the game does (`oag_net::race::load`), listens
//! for WebTransport sessions with a self-signed certificate minted for this run,
//! seats `--people` clients, and once every one has loaded the same race (the
//! tick-0 state hash, checked), runs the race at 60 Hz on the wall clock: each
//! tick it takes each person's buffered input (repeating the last when there is
//! none), steps the simulation, and sends every client the inputs it applied and
//! the state hash it reached. The simulation itself never reads the clock; the
//! clock only paces when the next tick runs.
//!
//! It opens no window, no GPU device and no audio device. It links
//! `oag-raceplay`, which links wgpu and winit, and never initialises either.
//!
//! ```sh
//! cargo run -p oag-net --bin oag-server -- --people 1
//! # prints: cert-hash <64 hex digits>; then, in another terminal:
//! cargo run -p oag-game -- --connect 127.0.0.1:4433 --cert-hash <64 hex digits>
//! ```
//!
//! See `docs/networking/README.md`, "Running a local server".

use std::net::SocketAddr;

use anyhow::{Context, Result};
use clap::Parser;
use oag_net::host::{Host, Phase};
use oag_net::wire::RaceParams;
use web_time::{Duration, Instant};

/// The tick, from the engine's one rate.
fn dt() -> Duration {
    Duration::from_secs_f64(f64::from(oag_core::tick::TickRate::DEFAULT.dt()))
}

#[derive(Parser, Debug)]
#[command(about = "The headless authoritative race server (WebTransport)")]
struct Cli {
    /// The address to listen on. Port 4433 is chosen, not standard.
    #[arg(long, default_value = "127.0.0.1:4433")]
    listen: SocketAddr,
    /// The disc image or unpacked folder to race from; the same search as the
    /// game's when absent.
    #[arg(long)]
    image: Option<String>,
    /// The circuit, as the game's `--track` takes it; the title's default when absent.
    #[arg(long, default_value = "")]
    track: String,
    /// The team every person flies, as the game's `--team` takes it.
    #[arg(long, default_value = "")]
    team: String,
    /// The speed class, as the title spells it.
    #[arg(long, default_value = "VENOM")]
    class: String,
    /// The race mode (`oag_race::Mode::name`).
    #[arg(long, default_value = "single_race")]
    mode: String,
    /// The opponents' difficulty.
    #[arg(long, default_value = "elite")]
    difficulty: String,
    /// The control scheme every client flies (one per race in this engine).
    #[arg(long, default_value = "veteran")]
    scheme: String,
    /// The world seed. Chosen default.
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// How many people race, each from their own client, in grid slots `0..N`.
    #[arg(long, default_value_t = 1)]
    people: u8,
    /// No AI on the grid, only the people.
    #[arg(long)]
    no_opponents: bool,
    /// Ticks between telling the clients the race starts and running tick 0, so
    /// their first inputs are already here. Chosen.
    #[arg(long, default_value_t = 6)]
    lead_ticks: u16,
    /// Write the certificate hash to this file as well as printing it.
    #[arg(long)]
    hash_out: Option<std::path::PathBuf>,
    /// Another name the certificate is valid for (a LAN address), repeatable.
    #[arg(long = "cert-name")]
    cert_names: Vec<String>,
    /// Stop after this many ticks even if the race has not finished.
    #[arg(long)]
    max_ticks: Option<u32>,
    #[command(flatten)]
    log: oag_log::tool::LogArgs,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    oag_log::tool::start("oag-server", "warn,oag=info,oag_server=info", &cli.log);

    let source = oag_source::source::resolve(cli.image.as_deref(), None)?;
    let mut params = RaceParams {
        title: String::new(),
        track: cli.track.clone(),
        team: cli.team.clone(),
        class: cli.class.clone(),
        mode: cli.mode.clone(),
        difficulty: cli.difficulty.clone(),
        scheme: cli.scheme.clone(),
        seed: cli.seed,
        people: cli.people.clamp(1, 8),
        opponents: !cli.no_opponents,
        // Weapons are not server events yet (ADR-0069, condition 2).
        weapons: false,
    };
    let (race, title) = oag_net::race::load(&params, &source)?;
    params.title = title.to_string();
    println!(
        "racing {title} from {source}: {} craft, {} people, weapons off, seed {}",
        race.sim.world.ship_count, params.people, params.seed
    );
    let mut host = Host::new(race, params);
    println!("fingerprint {:#018x}", host.fingerprint());

    let identity = oag_net::wt::Identity::self_signed(&cli.cert_names)?;
    let mut listener = oag_net::wt::Listener::bind(cli.listen, &identity)?;
    println!("listening on {}", listener.local_addr());
    println!("cert-hash {}", identity.hash_hex());
    if let Some(path) = &cli.hash_out {
        std::fs::write(path, format!("{}\n", identity.hash_hex()))
            .with_context(|| format!("writing {}", path.display()))?;
    }

    // The lobby: seat people until every seat has loaded the race.
    while !host.all_ready() {
        while let Some(link) = listener.accept() {
            println!("connection from {}", link.peer());
            host.join(Box::new(link));
        }
        host.poll();
        std::thread::sleep(Duration::from_millis(5));
    }
    host.start(cli.lead_ticks);
    println!("every seat is ready: {:?}", host.seat_names());

    let dt = dt();
    let started = Instant::now() + dt * u32::from(cli.lead_ticks);
    let mut last_log = 0;
    loop {
        // A late connection is refused by name rather than left hanging.
        while let Some(link) = listener.accept() {
            host.join(Box::new(link));
        }
        host.poll();
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
            log_second(&mut host, tick);
        }
        let over = if host.finished() {
            Some("every person has finished")
        } else if host.deserted() {
            Some("every person has left")
        } else if cli.max_ticks.is_some_and(|max| tick >= max) {
            Some("--max-ticks reached")
        } else {
            None
        };
        if let Some(reason) = over {
            println!("tick {tick}: {reason}");
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
    Ok(())
}

/// One line a second: per seat, the ticks its input arrived in time, the ticks
/// the last input was repeated, inputs that came too late, and how many ticks
/// ahead its input is arriving.
fn log_second(host: &mut Host, tick: u32) {
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
    println!("tick {tick:>6} hash {hash:#018x} | {}", seats.join(" | "));
}
