//! `oag-server lobby`: the online lobby, one process for every title it has an image for.
//!
//! ```sh
//! oag-server lobby --image data/images/pulse-psp-eu.chd --image data/images/hdfury-ps3-eu.iso \
//!     --listen '[::]:60000' --race-listen '[::]:60001'
//! # prints: lobby cert-hash <64 hex digits>
//! ```
//!
//! The lobby listens on `--listen`; the race it runs is a child of this binary
//! (`oag-server race`) listening on `--race-listen`. Both are UDP (WebTransport over QUIC).

use std::net::SocketAddr;

use anyhow::{Context, Result, bail};
use oag_net::online::race_proc::{ChildRunner, Limits};
use oag_net::online::rooms::Config as RoomsConfig;
use oag_net::online::server::{Config, Server};
use web_time::Duration;

#[derive(clap::Args, Debug)]
pub struct Args {
    /// The address the lobby listens on (UDP). Port 60000 is chosen, not standard.
    #[arg(long, default_value = "[::]:60000")]
    listen: SocketAddr,
    /// The address the race child listens on (UDP), one race at a time. Port 0 picks one.
    #[arg(long, default_value = "[::]:60001")]
    race_listen: SocketAddr,
    /// A disc image or unpacked folder to host, repeatable; its title is read from it.
    /// The first title found is the default when none is given.
    #[arg(long = "image")]
    images: Vec<String>,
    /// Another name the lobby's certificate is valid for, repeatable. Clients pin the hash,
    /// so none is needed to connect.
    #[arg(long = "cert-name")]
    cert_names: Vec<String>,
    /// The fewest people START needs.
    #[arg(long, default_value_t = oag_net::wire::online::MIN_PLAYERS)]
    min_players: u8,
    /// Milliseconds between a room getting the game slot and its race starting loading.
    #[arg(long, default_value_t = 3000)]
    countdown_ms: u32,
    /// Seconds the people have to arrive and load once the race is up.
    #[arg(long, default_value_t = 60)]
    join_timeout: u64,
    /// Seconds the race child has to start accepting.
    #[arg(long, default_value_t = 120)]
    load_timeout: u64,
    /// A race is stopped after this many ticks (60 a second). Default 30 minutes.
    #[arg(long)]
    max_ticks: Option<u32>,
    /// Write the lobby's certificate hash to this file as well as printing it.
    #[arg(long)]
    hash_out: Option<std::path::PathBuf>,
    /// The base of the races' seeds.
    #[arg(long, default_value_t = 1)]
    seed: u64,
    #[command(flatten)]
    log: oag_log::tool::LogArgs,
}

pub fn run(args: &Args) -> Result<()> {
    oag_log::tool::start("oag-server-lobby", "warn,oag=info,oag_net=info", &args.log);
    let mut images = Vec::new();
    let sources = if args.images.is_empty() {
        vec![oag_source::source::resolve(None, None)?]
    } else {
        args.images.clone()
    };
    for source in sources {
        let Some(title) = oag_source::title::identify(&source) else {
            bail!("{source} is not a disc or folder of a title this build knows");
        };
        println!("hosting {} from {source}", title.name);
        images.push((title.name.to_string(), source));
    }
    let titles: Vec<String> = images.iter().map(|(t, _)| t.clone()).collect();
    let limits = Limits {
        join_timeout: Duration::from_secs(args.join_timeout),
        max_ticks: args.max_ticks.unwrap_or(Limits::default().max_ticks),
    };
    let runner = ChildRunner {
        program: std::env::current_exe().context("finding this binary to run races with")?,
        images,
        listen: args.race_listen,
        limits,
        load_timeout: Duration::from_secs(args.load_timeout),
    };
    let mut server = Server::bind(
        Config {
            listen: args.listen,
            cert_names: args.cert_names.clone(),
            rooms: RoomsConfig {
                min_players: args.min_players,
                countdown_ms: args.countdown_ms,
                titles,
                seed: args.seed,
            },
        },
        Box::new(runner),
    )?;
    println!("lobby listening on {}", server.local_addr());
    println!("race port {}", args.race_listen.port());
    println!("lobby cert-hash {}", server.cert_hash_hex());
    if let Some(path) = &args.hash_out {
        std::fs::write(path, format!("{}\n", server.cert_hash_hex()))
            .with_context(|| format!("writing {}", path.display()))?;
    }
    server.serve(&|| false);
    Ok(())
}
