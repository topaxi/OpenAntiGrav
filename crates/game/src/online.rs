//! `--online`: this game as a client of an online lobby (`oag-server lobby`), a screen-less
//! first path. It connects to the lobby, makes or joins a room, picks a ship, waits for START
//! (pressing it itself when it made the room), and returns the [`Connect`] for the race the
//! lobby hands it to, which goes the way `--connect` does: the lobby's host, the race port,
//! the pinned certificate and the seat token.
//!
//! The menus' ONLINE screens (Omega's `OnlineMainMenu`, `Game List`, `MP Select`,
//! `GameLobby`) are open work; `docs/networking/online-lobby.md` has their order.

use anyhow::{Result, anyhow, bail};
use oag_net::link::Link;
use oag_net::online::client::LobbyClient;
use oag_net::wire::online::{Edition, Phase, RoomSettings};
use web_time::{Duration, Instant};

use crate::net_play::Connect;

/// How long the whole lobby flow may take before it gives up (a room that never fills).
/// Chosen.
const GIVE_UP: Duration = Duration::from_secs(600);

/// What to do in the lobby.
#[derive(Debug, Clone)]
pub enum Action {
    /// Make a room with these settings and press START when [`Plan::start_at`] are ready.
    Create(RoomSettings),
    /// Join this room, or the first joinable one.
    Join(Option<u32>),
}

/// What `--online` asked for.
#[derive(Debug, Clone)]
pub struct Plan {
    /// The lobby, `host:port`.
    pub addr: String,
    /// The lobby certificate's SHA-256.
    pub cert_hash: [u8; 32],
    /// The name to go by.
    pub name: String,
    /// What this machine can race: its title, the image's platform and its mods' fingerprint,
    /// which the lobby compares with the server's to show it only rooms it can join.
    pub titles: Vec<Edition>,
    /// Make a room or join one.
    pub action: Action,
    /// The team to fly, `--team`'s spelling.
    pub team: String,
    /// The owner presses START when this many are ready.
    pub start_at: usize,
}

/// Runs the lobby flow to the hand-off.
///
/// # Errors
///
/// The lobby refused something (it names why), the link closed, or nothing happened in ten
/// minutes.
pub fn run(plan: &Plan) -> Result<Connect> {
    let link = oag_net::wt::connect(&plan.addr, plan.cert_hash)?;
    let mut lobby = LobbyClient::connect(
        Box::new(link) as Box<dyn Link>,
        &plan.name,
        &plan.titles,
        "",
    );
    let started = Instant::now();
    let mut stage = 0;
    let mut asked_list = None::<Instant>;
    let mut seen = String::new();
    let mut pick_sent = false;
    let mut start_sent = false;
    loop {
        lobby
            .poll()
            .map_err(|why| anyhow!("the lobby closed: {why}"))?;
        if let Some((code, text)) = lobby.refused.take() {
            bail!("the lobby refused: {code:?}: {text}");
        }
        if started.elapsed() > GIVE_UP {
            bail!("no race within {GIVE_UP:?}");
        }
        if let Some(handoff) = lobby.take_handoff() {
            let host = plan
                .addr
                .rsplit_once(':')
                .map_or(plan.addr.as_str(), |(host, _)| host);
            println!("online: the race is on {host}:{}", handoff.port);
            keep_lobby_open(lobby);
            return Ok(Connect {
                addr: format!("{host}:{}", handoff.port),
                cert_hash: handoff.cert_hash,
                shim: None,
                bot: false,
                name: handoff.name,
                token: handoff.token,
            });
        }
        if stage == 0 && lobby.welcomed() {
            println!(
                "online: welcomed as {:?}, the server races {:?}",
                lobby.name, lobby.served
            );
            stage = 1;
            match &plan.action {
                Action::Create(settings) => {
                    lobby.create(settings.clone());
                    stage = 2;
                }
                Action::Join(Some(room)) => {
                    lobby.join(*room);
                    stage = 2;
                }
                Action::Join(None) => {}
            }
        }
        if stage == 1 {
            // Join the first room that will have us.
            if asked_list.is_none_or(|at| at.elapsed() > Duration::from_millis(500)) {
                lobby.list();
                asked_list = Some(Instant::now());
            }
            if let Some(room) = lobby.rooms.iter().find(|r| r.joinable) {
                lobby.join(room.id);
                stage = 2;
            }
        }
        if let Some(room) = lobby.room.clone() {
            let line = format!(
                "room {}: {} in it, {:?}{}",
                room.id,
                room.members.len(),
                room.phase,
                if room.note.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", room.note)
                }
            );
            if line != seen {
                println!("online: {line}");
                seen = line;
            }
            if !pick_sent {
                lobby.ready(&plan.team, true);
                pick_sent = true;
            }
            let ready = room.members.iter().filter(|m| m.ready).count();
            let owner = room.members.first().is_some_and(|m| m.owner)
                && room.members.first().map(|m| &m.name) == lobby.name.as_ref();
            if owner
                && matches!(plan.action, Action::Create(_))
                && !start_sent
                && ready >= plan.start_at
                && ready == room.members.len()
                && room.phase == Phase::Open
            {
                lobby.start();
                start_sent = true;
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The lobby link stays open for the whole race (the lobby pushes the end of the race on it,
/// and a closed link would have the lobby drop this person from their room): a thread polls
/// it until the process exits and says when the room is open again.
fn keep_lobby_open(mut lobby: LobbyClient) {
    let spawned = std::thread::Builder::new()
        .name("oag-lobby".into())
        .spawn(move || {
            let mut note = String::new();
            while lobby.poll().is_ok() {
                if let Some(room) = &lobby.room
                    && room.phase == Phase::Open
                    && room.note != note
                {
                    println!("online: back in the room: {}", room.note);
                    note.clone_from(&room.note);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        });
    if let Err(e) = spawned {
        log::warn!("online: the lobby link is not kept open: {e}");
    }
}
