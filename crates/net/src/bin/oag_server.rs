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
//! It opens no window, no GPU device and no audio device, and links none: built
//! with this crate's default features it takes `oag-raceplay` without `view`
//! (rule 6 of `just check-deps`).
//!
//! ```sh
//! cargo run -p oag-net --bin oag-server -- --people 1
//! # prints: cert-hash <64 hex digits>; then, in another terminal:
//! cargo run -p oag-game -- <the same image> --connect 127.0.0.1:4433 --cert-hash <64 hex digits>
//! ```
//!
//! `oag-server lobby` is the online lobby (`docs/networking/online-lobby.md`): it lists rooms
//! for every title it has an image for and runs one race at a time as a child,
//! `oag-server race`, which reads its job off standard input.
//!
//! See `docs/networking/README.md`, "Running a local server".

#[path = "oag_server/lobby_cmd.rs"]
mod lobby_cmd;
#[path = "oag_server/race_cmd.rs"]
mod race_cmd;

use std::net::SocketAddr;

use anyhow::{Context, Result};
use clap::Parser;
use oag_net::host::Host;
use oag_net::serve::Pace;
use oag_net::wire::RaceParams;
use web_time::Duration;

#[derive(clap::Subcommand, Debug)]
enum Cmd {
    /// The online lobby: rooms for every title it has an image for, one race at a time.
    Lobby(lobby_cmd::Args),
    /// One race for the lobby (not run by hand): reads its job from standard input.
    Race(race_cmd::Args),
}

#[derive(Parser, Debug)]
#[command(
    about = "The headless authoritative race server (WebTransport)",
    args_conflicts_with_subcommands = true
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
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
    /// The team a person flies, as the game's `--team` takes it. Repeat it for each seat in
    /// order (`--team a --team b`: seat 0 flies a, seat 1 b); a seat past the list flies the
    /// last one given.
    #[arg(long = "team")]
    teams: Vec<String>,
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
    match cli.cmd {
        Some(Cmd::Lobby(args)) => return lobby_cmd::run(&args),
        Some(Cmd::Race(args)) => return race_cmd::run(&args),
        None => {}
    }
    oag_log::tool::start("oag-server", "warn,oag=info,oag_server=info", &cli.log);

    let source = oag_source::source::resolve(cli.image.as_deref(), None)?;
    let mut params = RaceParams {
        title: String::new(),
        track: cli.track.clone(),
        team: cli.teams.last().cloned().unwrap_or_default(),
        teams: cli.teams.clone(),
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
    let pace = Pace {
        lead_ticks: cli.lead_ticks,
        max_ticks: cli.max_ticks,
    };
    oag_net::serve::run(
        &mut host,
        pace,
        &mut || {
            listener
                .accept()
                .map(|link| Box::new(link) as Box<dyn oag_net::link::Link>)
        },
        &|| false,
        &mut |line| println!("{line}"),
    );
    Ok(())
}
