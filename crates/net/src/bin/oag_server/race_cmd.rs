//! `oag-server race`: the lobby's race child. One race, then it exits.
//!
//! Reads one hex line (`RaceJob`) off standard input, loads the race, listens, prints
//! `listening on ADDR` and `cert-hash HEX` for the lobby to read, seats the people the job
//! names by their seat tokens, runs the race and prints `race-over REASON`. A failure prints
//! `error: WHY` on standard error and exits 2, which is what the lobby shows the room.

use std::io::Read;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result, anyhow};
use oag_net::host::Host;
use oag_net::online::race_proc::{Limits, run_race};
use oag_net::wire::online::RaceJob;
use web_time::Duration;

#[derive(clap::Args, Debug)]
pub struct Args {
    /// The disc image or unpacked folder to race from.
    #[arg(long)]
    image: String,
    /// The address to listen on.
    #[arg(long)]
    listen: SocketAddr,
    /// Seconds the people have to arrive and load.
    #[arg(long, default_value_t = 60)]
    join_timeout: u64,
    /// Stop after this many ticks even if the race has not finished.
    #[arg(long)]
    max_ticks: Option<u32>,
    #[command(flatten)]
    log: oag_log::tool::LogArgs,
}

pub fn run(args: &Args) -> Result<()> {
    oag_log::tool::start("oag-server-race", "warn,oag=info", &args.log);
    match race(args) {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::exit(2);
        }
    }
}

fn race(args: &Args) -> Result<()> {
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .context("reading the job")?;
    let job = RaceJob::from_line(&line).map_err(|e| anyhow!("the job: {e}"))?;
    let (race, title) = oag_net::race::load(&job.params, &args.image)?;
    println!(
        "racing {title} from {}: {} craft, {} people, seed {}",
        args.image, race.sim.world.ship_count, job.params.people, job.params.seed
    );
    let mut host = Host::new(race, job.params.clone());
    let identity = oag_net::wt::Identity::self_signed(&[])?;
    let mut listener = oag_net::wt::Listener::bind(args.listen, &identity)?;
    println!("listening on {}", listener.local_addr());
    println!("cert-hash {}", identity.hash_hex());
    let limits = Limits {
        join_timeout: Duration::from_secs(args.join_timeout),
        max_ticks: args.max_ticks.unwrap_or(Limits::default().max_ticks),
    };
    watch_misses();
    let lobby_gone = lobby_watch();
    let ended = run_race(
        &mut host,
        &mut listener,
        &job.seats,
        limits,
        &|| lobby_gone.load(Ordering::Relaxed),
        &mut |line| println!("{line}"),
    )
    .map_err(|e| anyhow!("{e}"))?;
    println!("race-over {ended}");
    Ok(())
}

/// The lobby holds this process's standard input open for the whole race, so its end of file
/// means the lobby is gone (it exited or was killed): the race stops rather than outliving it
/// and holding the race port.
fn lobby_watch() -> Arc<AtomicBool> {
    let gone = Arc::new(AtomicBool::new(false));
    let flag = gone.clone();
    std::thread::spawn(move || {
        let mut sink = [0u8; 256];
        let mut stdin = std::io::stdin();
        while matches!(stdin.read(&mut sink), Ok(n) if n > 0) {}
        flag.store(true, Ordering::Relaxed);
    });
    gone
}

/// A server running from an extraction answers a read it lacks with an error a reader may turn
/// into an absent asset, which would make its race differ from the clients' silently. Say so,
/// as it happens, on the log and on standard error (the lobby shows the room the child's last
/// lines).
fn watch_misses() {
    std::thread::spawn(|| {
        loop {
            for miss in oag_assets::record::take_misses() {
                eprintln!("warning: the extraction lacks a read the race made: {miss}");
                log::warn!("the extraction lacks a read the race made: {miss}");
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    });
}
