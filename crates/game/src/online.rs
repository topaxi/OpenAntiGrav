//! `--online`: this game as a client of an online lobby (`oag-server lobby`), a screen-less
//! first path. It connects to the lobby, makes or joins a room, picks a ship, waits for START
//! (pressing it itself when it made the room), and returns the [`Connect`] for the race the
//! lobby hands it to, which goes the way `--connect` does: the lobby's host, the race port,
//! the pinned certificate and the seat token.
//!
//! The menus' ONLINE screens (Omega's `OnlineMainMenu`, `Game List`, `MP Select`,
//! `GameLobby`) are open work; `docs/networking/online-lobby.md` has their order.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow, bail};
use oag_net::link::Link;
use oag_net::online::client::{Handoff, LobbyClient};
use oag_net::online::player_host::{Event, PlayerHost};
use oag_net::wire::online::{Edition, Phase, RoomSettings};
use web_time::{Duration, Instant};

use crate::net_play::Connect;

/// What passes between the thread that keeps the lobby link and the race
/// ([`crate::net_play::NetPlay`]) in a `--online` run.
#[derive(Debug, Default)]
pub struct LobbyLine {
    /// The lobby closed the room under this person (its host left, or was banned), and why:
    /// said from outside the race, it ends the race.
    pub closed: Mutex<Option<String>>,
    /// This machine hosts the race and has not yet told the lobby how it ended: a process
    /// that is about to exit waits a moment for this, so the room gets its board.
    pub unreported: std::sync::atomic::AtomicBool,
}

/// A [`LobbyLine`] as its two ends hold it.
pub type LobbyWord = Arc<LobbyLine>;

/// This machine as the race host of a room it owns on a lobby: the listener the lobby probed,
/// the thread that runs each race the lobby sends, and what the lobby is told about it
/// (`docs/architecture/adr/0092-a-player-hosted-race-is-a-room-whose-owner-runs-the-race-host.md`).
#[derive(Debug)]
pub struct Hosting {
    host: PlayerHost,
    /// The title's disc image or folder, as this game opened it: what a hosted race loads.
    source: String,
    /// The loopback address of the listener's family, where this machine's own seat dials.
    own: IpAddr,
    /// The room whose race is up, and when the lobby was last told so.
    up: Option<(u32, Instant)>,
}

impl Hosting {
    /// Opens the hosting port (`port`, 0 for the game's own, [`oag_net::lan::DEFAULT_PORT`]) on
    /// every interface of the family the lobby link uses (`lobby_peer`), so the address the
    /// lobby sees this machine at is one the listener answers on.
    ///
    /// # Errors
    ///
    /// The port could not be bound.
    pub fn open(source: &str, port: u16, lobby_peer: IpAddr) -> Result<Self> {
        let v4 = match lobby_peer {
            IpAddr::V4(_) => true,
            IpAddr::V6(v6) => v6.to_ipv4_mapped().is_some(),
        };
        let (any, own): (IpAddr, IpAddr) = if v4 {
            (Ipv4Addr::UNSPECIFIED.into(), Ipv4Addr::LOCALHOST.into())
        } else {
            (Ipv6Addr::UNSPECIFIED.into(), Ipv6Addr::LOCALHOST.into())
        };
        let host = PlayerHost::bind(
            SocketAddr::new(any, port),
            oag_net::online::race_proc::Limits::default(),
        )?;
        Ok(Self {
            host,
            source: source.to_string(),
            own,
            up: None,
        })
    }

    /// The port the listener is on.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.host.port()
    }

    /// Offers this listener to `lobby` as the host of a new room with `settings`, signed with
    /// the profile's key for the lobby whose certificate hashes to `lobby_hash`.
    ///
    /// # Errors
    ///
    /// The lobby has not welcomed this client yet.
    pub fn offer(
        &self,
        lobby: &mut LobbyClient,
        lobby_hash: &[u8; 32],
        key: &oag_net::online::hostkey::PlayerKey,
        settings: RoomSettings,
    ) -> Result<()> {
        let token = lobby
            .token
            .clone()
            .ok_or_else(|| anyhow!("the lobby has not welcomed this machine"))?;
        let offer = key.offer(lobby_hash, &token, self.host.port(), self.host.hash());
        lobby.host_room(settings, offer);
        Ok(())
    }

    /// Every tick: a race the lobby sent is started, the lobby is told when it is up (and once a
    /// second after, the sign of life it expects) and when it is over. Whether an end was
    /// reported in this call.
    pub fn pump(&mut self, lobby: &mut LobbyClient) -> bool {
        let mut reported = false;
        if let Some((room, job)) = lobby.take_launch() {
            log::debug!("online: hosting room {room}'s race: {:?}", job.params);
            let source = self.source.clone();
            self.host.launch(
                room,
                job,
                Arc::new(move |params| oag_net::race::load(params, &source).map(|(race, _)| race)),
            );
        }
        while let Some(event) = self.host.poll() {
            match event {
                Event::Up(room) => {
                    lobby.host_up(room);
                    self.up = Some((room, Instant::now()));
                }
                Event::Over(room, ended) => {
                    println!("online: the hosted race is over: {}", ended.reason);
                    lobby.host_over(room, &ended.reason, ended.board);
                    self.up = None;
                    reported = true;
                }
            }
        }
        if let Some((room, told)) = &mut self.up
            && told.elapsed() >= Duration::from_secs(1)
        {
            lobby.host_up(*room);
            *told = Instant::now();
        }
        reported
    }

    /// The host's player left the race: its seats are told and the race ends.
    pub fn end_race(&self) {
        self.host.end_race();
    }
}

/// Where the race of `handoff` is dialled: this machine's own listener when it hosts the race,
/// else the address the hand-off names (a player host's), else the lobby's own host.
#[must_use]
pub fn race_addr(lobby_addr: &str, handoff: &Handoff, hosting: Option<&Hosting>) -> String {
    if let Some(hosting) = hosting {
        return SocketAddr::new(hosting.own, handoff.port).to_string();
    }
    let host = if handoff.host.is_empty() {
        lobby_addr
            .rsplit_once(':')
            .map_or(lobby_addr, |(host, _)| host)
    } else {
        handoff.host.as_str()
    };
    format!("{host}:{}", handoff.port)
}

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
    /// The Pilot Assist level to fly with, where the room allows it.
    pub assist: oag_physics::pilot_assist::Level,
    /// The owner presses START when this many are ready.
    pub start_at: usize,
    /// With [`Action::Create`]: this machine hosts the room's race. The title's source and the
    /// UDP port to listen on (0: the game's own).
    pub host: Option<(String, u16)>,
}

/// Runs the lobby flow to the hand-off.
///
/// # Errors
///
/// The lobby refused something (it names why), the link closed, or nothing happened in ten
/// minutes.
pub fn run(plan: &Plan) -> Result<Connect> {
    let link = oag_net::wt::connect(&plan.addr, plan.cert_hash)?;
    let lobby_peer = link.peer().ip();
    let mut hosting = None::<Hosting>;
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
        if let Some(hosting) = &mut hosting {
            let _ = hosting.pump(&mut lobby);
        }
        if let Some(handoff) = lobby.take_handoff() {
            let addr = race_addr(&plan.addr, &handoff, hosting.as_ref());
            println!("online: the race is on {addr}");
            let word = LobbyWord::default();
            word.unreported
                .store(hosting.is_some(), std::sync::atomic::Ordering::Relaxed);
            keep_lobby_open(lobby, hosting, word.clone());
            return Ok(Connect {
                addr,
                cert_hash: handoff.cert_hash,
                shim: None,
                bot: false,
                name: handoff.name,
                token: handoff.token,
                lobby: Some(word),
            });
        }
        if stage == 0 && lobby.welcomed() {
            println!(
                "online: welcomed as {:?}, the server races {:?}",
                lobby.name, lobby.served
            );
            stage = 1;
            match (&plan.action, &plan.host) {
                (Action::Create(settings), Some((source, port))) => {
                    let opened = Hosting::open(source, *port, lobby_peer)?;
                    let key = crate::identity::load_or_create()?;
                    println!(
                        "online: hosting on UDP port {}, asking the lobby to check it",
                        opened.port()
                    );
                    opened.offer(&mut lobby, &plan.cert_hash, &key, settings.clone())?;
                    hosting = Some(opened);
                    stage = 2;
                }
                _ => {}
            }
            match &plan.action {
                Action::Create(_) if plan.host.is_some() => {}
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
                "room {}{}: {} in it, {:?}{}",
                room.id,
                if room.player_hosted {
                    " (player-hosted)"
                } else {
                    ""
                },
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
                lobby.ready(&plan.team, true, plan.assist);
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
/// it until the process exits and says when the room is open again. A race this machine hosts
/// is reported to the lobby from here, and a room that closes under the race is `word`.
fn keep_lobby_open(mut lobby: LobbyClient, mut hosting: Option<Hosting>, word: LobbyWord) {
    let spawned = std::thread::Builder::new()
        .name("oag-lobby".into())
        .spawn(move || {
            let mut note = String::new();
            while lobby.poll().is_ok() {
                if let Some(hosting) = &mut hosting
                    && hosting.pump(&mut lobby)
                {
                    word.unreported
                        .store(false, std::sync::atomic::Ordering::Relaxed);
                }
                if let Some(reason) = lobby.left.take() {
                    println!("online: the lobby closed the room: {reason}");
                    *word
                        .closed
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(reason);
                }
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
