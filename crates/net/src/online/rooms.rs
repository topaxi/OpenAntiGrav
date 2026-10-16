//! The rooms of an online lobby as a state machine with no I/O: who is in which room, who
//! owns it, the ready and start gate, and the one game slot with the rooms waiting for it.
//!
//! A message goes in ([`Rooms::handle`]), replies and room snapshots collect in an outbox
//! ([`Rooms::take_outbox`]), the clock is [`Rooms::tick`] (milliseconds the caller counts),
//! and a race to bring up comes out of [`Rooms::take_launch`]. The service in
//! [`super::server`] only moves bytes and processes; every rule is here.
//!
//! Chosen, not measured, after Omega's lobby (`docs/networking/online-lobby.md`): the
//! owner is the person who made the room and the next in line when they leave; START
//! needs [`Config::min_players`] people and every pick made; a room that pressed START
//! while a race runs waits in a FIFO and is told its place; the room is back to `Open`
//! when its race ends, picks cleared, its members in it.

use std::collections::{BTreeMap, VecDeque};

use crate::discovery::{Compat, Local, differs};
use crate::wire::online::{
    Edition, MAX_PLAYERS, MIN_PLAYERS, Member, Message, Phase, Refusal, RoomInfo, RoomSettings,
    Snapshot,
};
use crate::wire::{PROTOCOL, RaceParams};

/// One connected person, as the lobby knows them.
pub type ClientId = u32;

/// The longest name, in characters. Chosen.
pub const NAME_MAX: usize = 24;

/// How the rooms behave.
#[derive(Debug, Clone)]
pub struct Config {
    /// The fewest people START needs.
    pub min_players: u8,
    /// Milliseconds between a room getting the slot and its race being brought up.
    pub countdown_ms: u32,
    /// What this server races, per title (its images' own title and platform, no mods): a room
    /// can only be made in one of these, and a person sees it only if theirs matches.
    pub titles: Vec<Edition>,
    /// Mixed with the room id and launch count into the race's seed.
    pub seed: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            min_players: MIN_PLAYERS,
            countdown_ms: 3_000,
            titles: Vec::new(),
            seed: 1,
        }
    }
}

/// A race to bring up: the room that earned the slot, the race, and who sits where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    /// The room.
    pub room: u32,
    /// The race, seats in `seats` order.
    pub params: RaceParams,
    /// The people who race: seat `i` is `seats[i]`.
    pub seats: Vec<(ClientId, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Open,
    Queued,
    Countdown { until: u64, shown: u32 },
    Racing,
}

#[derive(Debug, Clone)]
struct Occupant {
    client: ClientId,
    team: String,
    ready: bool,
}

#[derive(Debug, Clone)]
struct Room {
    id: u32,
    settings: RoomSettings,
    /// Owner first, then in join order.
    members: Vec<Occupant>,
    stage: Stage,
    note: String,
}

#[derive(Debug, Clone)]
struct Person {
    name: String,
    titles: Vec<Edition>,
    room: Option<u32>,
}

/// Every room, every person, and the one game slot.
#[derive(Debug)]
pub struct Rooms {
    config: Config,
    people: BTreeMap<ClientId, Person>,
    rooms: Vec<Room>,
    /// Rooms that pressed START while the slot was busy (or not yet promoted), first in line first.
    queue: VecDeque<u32>,
    /// The room holding the slot: counting down or racing.
    active: Option<u32>,
    next_room: u32,
    launches: u64,
    outbox: Vec<(ClientId, Message)>,
    launch: Option<Launch>,
    now: u64,
}

/// A refusal and the line that goes with it.
type Fail = (Refusal, String);

fn refuse(code: Refusal, text: &str) -> Fail {
    (code, text.to_string())
}

impl Rooms {
    /// An empty lobby.
    #[must_use]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            people: BTreeMap::new(),
            rooms: Vec::new(),
            queue: VecDeque::new(),
            active: None,
            next_room: 1,
            launches: 0,
            outbox: Vec::new(),
            launch: None,
            now: 0,
        }
    }

    /// The titles this server races.
    #[must_use]
    pub fn titles(&self) -> Vec<String> {
        self.config.titles.iter().map(|e| e.title.clone()).collect()
    }

    /// A person connected, under `name`, able to race `titles`.
    ///
    /// # Errors
    ///
    /// [`Refusal::BadName`]: empty, too long, or another connected person has it.
    pub fn connect(&mut self, id: ClientId, name: &str, titles: Vec<Edition>) -> Result<(), Fail> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > NAME_MAX || name.chars().any(char::is_control)
        {
            return Err(refuse(
                Refusal::BadName,
                "a name is 1 to 24 characters with no control characters",
            ));
        }
        if self
            .people
            .values()
            .any(|p| p.name.eq_ignore_ascii_case(name))
        {
            return Err(refuse(Refusal::BadName, "that name is in use"));
        }
        // A build that differs on a title this server races is refused here, by name, instead of
        // at the race's tick-0 hash a minute later.
        if let Some(server) = self.config.titles.iter().find(|server| {
            titles.iter().any(|theirs| {
                theirs.title == server.title
                    && theirs.platform == server.platform
                    && theirs.build != server.build
            })
        }) {
            let theirs = titles
                .iter()
                .find(|theirs| theirs.title == server.title)
                .map_or("", |theirs| theirs.build.as_str());
            return Err(refuse(
                Refusal::Version,
                &format!(
                    "this server is build {}, yours is {theirs}: races only match between the same build",
                    server.build
                ),
            ));
        }
        self.people.insert(
            id,
            Person {
                name: name.to_string(),
                titles,
                room: None,
            },
        );
        Ok(())
    }

    /// A person's name, while connected.
    #[must_use]
    pub fn name(&self, id: ClientId) -> Option<&str> {
        self.people.get(&id).map(|p| p.name.as_str())
    }

    /// The room a person is in.
    #[must_use]
    pub fn room_of(&self, id: ClientId) -> Option<u32> {
        self.people.get(&id).and_then(|p| p.room)
    }

    /// The room holding the one game slot.
    #[must_use]
    pub fn active(&self) -> Option<u32> {
        self.active
    }

    /// Everything to send since the last call.
    pub fn take_outbox(&mut self) -> Vec<(ClientId, Message)> {
        std::mem::take(&mut self.outbox)
    }

    /// A race to bring up, once a room's countdown has run out.
    pub fn take_launch(&mut self) -> Option<Launch> {
        self.launch.take()
    }

    fn say(&mut self, to: ClientId, message: Message) {
        self.outbox.push((to, message));
    }

    fn room_index(&self, id: u32) -> Option<usize> {
        self.rooms.iter().position(|r| r.id == id)
    }

    /// Why `id` cannot race `title` on this server, `None` when it can: the person's edition
    /// of it against the server's, by the decision LAN discovery makes ([`differs`]).
    fn mismatch(&self, id: ClientId, title: &str) -> Option<Compat> {
        let server = self.config.titles.iter().find(|e| e.title == title)?;
        let Some(theirs) = self
            .people
            .get(&id)
            .and_then(|p| p.titles.iter().find(|e| e.title == title))
        else {
            return Some(Compat::OtherTitle);
        };
        let local = Local {
            protocol: PROTOCOL,
            build: &theirs.build,
            title: &theirs.title,
            platform: &theirs.platform,
            content: theirs.content,
        };
        let host = Local {
            protocol: PROTOCOL,
            build: &server.build,
            title: &server.title,
            platform: &server.platform,
            content: server.content,
        };
        differs(&local, &host)
    }

    fn can_race(&self, id: ClientId, title: &str) -> bool {
        self.mismatch(id, title).is_none()
    }

    fn content_refusal(&self, id: ClientId, title: &str) -> Fail {
        match self.mismatch(id, title) {
            Some(Compat::OtherVersion) => refuse(
                Refusal::Version,
                "your build differs from the server's, so the race would not match",
            ),
            Some(Compat::OtherContent) => refuse(
                Refusal::Content,
                "your mods differ from the server's, so the race would not match",
            ),
            _ => refuse(
                Refusal::Content,
                "you do not have this server's pressing of that title",
            ),
        }
    }

    fn snapshot(&self, index: usize) -> Snapshot {
        let room = &self.rooms[index];
        let phase = match room.stage {
            Stage::Open => Phase::Open,
            Stage::Queued => Phase::Queued {
                position: self
                    .queue
                    .iter()
                    .position(|&q| q == room.id)
                    .map_or(1, |p| u16::try_from(p + 1).unwrap_or(u16::MAX)),
            },
            Stage::Countdown { until, .. } => Phase::Countdown {
                millis: u32::try_from(until.saturating_sub(self.now)).unwrap_or(u32::MAX),
            },
            Stage::Racing => Phase::Racing,
        };
        Snapshot {
            id: room.id,
            settings: room.settings.clone(),
            members: room
                .members
                .iter()
                .enumerate()
                .map(|(i, m)| Member {
                    name: self
                        .people
                        .get(&m.client)
                        .map(|p| p.name.clone())
                        .unwrap_or_default(),
                    team: m.team.clone(),
                    ready: m.ready,
                    owner: i == 0,
                })
                .collect(),
            phase,
            note: room.note.clone(),
        }
    }

    fn push_room(&mut self, index: usize) {
        let snapshot = self.snapshot(index);
        let to: Vec<ClientId> = self.rooms[index].members.iter().map(|m| m.client).collect();
        for client in to {
            self.say(client, Message::Room(snapshot.clone()));
        }
    }

    fn push_queue(&mut self) {
        let queued: Vec<u32> = self.queue.iter().copied().collect();
        for id in queued {
            if let Some(index) = self.room_index(id) {
                self.push_room(index);
            }
        }
    }

    fn validate(&self, id: ClientId, settings: &mut RoomSettings) -> Result<(), Fail> {
        if !self.config.titles.iter().any(|e| e.title == settings.title) {
            return Err(refuse(
                Refusal::BadSettings,
                "this server does not race that title",
            ));
        }
        if !self.can_race(id, &settings.title) {
            return Err(self.content_refusal(id, &settings.title));
        }
        if oag_race::Mode::from_name(&settings.mode).is_none() {
            return Err(refuse(Refusal::BadSettings, "unknown race mode"));
        }
        settings.max_players = settings
            .max_players
            .clamp(self.config.min_players, MAX_PLAYERS);
        Ok(())
    }

    /// The rooms `id` may see: only ones in a title it can race.
    #[must_use]
    pub fn list(&self, id: ClientId) -> Vec<RoomInfo> {
        self.rooms
            .iter()
            .filter(|r| self.can_race(id, &r.settings.title))
            .map(|r| RoomInfo {
                id: r.id,
                owner: r
                    .members
                    .first()
                    .and_then(|m| self.people.get(&m.client))
                    .map(|p| p.name.clone())
                    .unwrap_or_default(),
                settings: r.settings.clone(),
                members: u8::try_from(r.members.len()).unwrap_or(u8::MAX),
                joinable: r.stage == Stage::Open
                    && r.members.len() < usize::from(r.settings.max_players),
            })
            .collect()
    }

    /// One message from a connected person.
    pub fn handle(&mut self, id: ClientId, message: Message) {
        if !self.people.contains_key(&id) {
            return;
        }
        let result = match message {
            Message::ListRooms => {
                let rooms = self.list(id);
                self.say(id, Message::Rooms(rooms));
                Ok(())
            }
            Message::CreateRoom(settings) => self.create(id, settings),
            Message::JoinRoom { room } => self.join(id, room),
            Message::Edit(settings) => self.edit(id, settings),
            Message::Ready { team, ready } => self.ready(id, &team, ready),
            Message::Start => self.start(id),
            Message::Leave => {
                if self.room_of(id).is_some() {
                    self.leave(id, "you left the room");
                    Ok(())
                } else {
                    Err(refuse(Refusal::NotInRoom, "you are not in a room"))
                }
            }
            other => Err(refuse(
                Refusal::BadSettings,
                &format!("a client cannot send {other:?}"),
            )),
        };
        if let Err((code, text)) = result {
            self.say(id, Message::Rejected { code, text });
        }
    }

    fn create(&mut self, id: ClientId, mut settings: RoomSettings) -> Result<(), Fail> {
        if self.room_of(id).is_some() {
            return Err(refuse(Refusal::InRoom, "leave your room first"));
        }
        self.validate(id, &mut settings)?;
        let room = Room {
            id: self.next_room,
            settings,
            members: vec![Occupant {
                client: id,
                team: String::new(),
                ready: false,
            }],
            stage: Stage::Open,
            note: String::new(),
        };
        self.next_room += 1;
        if let Some(person) = self.people.get_mut(&id) {
            person.room = Some(room.id);
        }
        self.rooms.push(room);
        self.push_room(self.rooms.len() - 1);
        Ok(())
    }

    fn join(&mut self, id: ClientId, room: u32) -> Result<(), Fail> {
        if self.room_of(id).is_some() {
            return Err(refuse(Refusal::InRoom, "leave your room first"));
        }
        let Some(index) = self.room_index(room) else {
            return Err(refuse(Refusal::NoSuchRoom, "no such room"));
        };
        let title = self.rooms[index].settings.title.clone();
        if !self.can_race(id, &title) {
            return Err(self.content_refusal(id, &title));
        }
        if self.rooms[index].stage != Stage::Open {
            return Err(refuse(Refusal::Started, "this game has already started"));
        }
        if self.rooms[index].members.len() >= usize::from(self.rooms[index].settings.max_players) {
            return Err(refuse(Refusal::Full, "this game is full"));
        }
        self.rooms[index].members.push(Occupant {
            client: id,
            team: String::new(),
            ready: false,
        });
        self.rooms[index].note = format!("{} joined", self.name(id).unwrap_or_default());
        if let Some(person) = self.people.get_mut(&id) {
            person.room = Some(room);
        }
        self.push_room(index);
        Ok(())
    }

    fn owned(&self, id: ClientId) -> Result<usize, Fail> {
        let Some(room) = self.room_of(id) else {
            return Err(refuse(Refusal::NotInRoom, "you are not in a room"));
        };
        let index = self
            .room_index(room)
            .ok_or_else(|| refuse(Refusal::NotInRoom, "no room"))?;
        if self.rooms[index].members.first().map(|m| m.client) != Some(id) {
            return Err(refuse(Refusal::NotOwner, "only the owner can do that"));
        }
        Ok(index)
    }

    fn edit(&mut self, id: ClientId, mut settings: RoomSettings) -> Result<(), Fail> {
        let index = self.owned(id)?;
        if self.rooms[index].stage != Stage::Open {
            return Err(refuse(
                Refusal::Started,
                "the settings are locked once START is pressed",
            ));
        }
        self.validate(id, &mut settings)?;
        let mut kicked = Vec::new();
        let room = &mut self.rooms[index];
        let changed = room.settings != settings;
        room.settings = settings.clone();
        if changed {
            for member in &mut room.members {
                member.ready = false;
            }
            room.note = "the owner changed the settings".to_string();
        }
        let members: Vec<ClientId> = room.members.iter().map(|m| m.client).collect();
        for client in members {
            if client != id && !self.can_race(client, &settings.title) {
                kicked.push(client);
            }
        }
        for client in kicked {
            self.leave(
                client,
                "the owner changed the settings and picked content that you do not have",
            );
        }
        // The room may have shrunk to the new size: the latest to join are asked to go.
        if let Some(index) = self.room_index(self.room_of(id).unwrap_or(0)) {
            let max = usize::from(self.rooms[index].settings.max_players);
            while self.rooms[index].members.len() > max {
                let last = self.rooms[index].members.last().map(|m| m.client);
                if let Some(last) = last {
                    self.leave(last, "the owner made the room smaller");
                }
            }
            self.push_room(index);
        }
        Ok(())
    }

    fn ready(&mut self, id: ClientId, team: &str, ready: bool) -> Result<(), Fail> {
        let Some(room) = self.room_of(id) else {
            return Err(refuse(Refusal::NotInRoom, "you are not in a room"));
        };
        let index = self
            .room_index(room)
            .ok_or_else(|| refuse(Refusal::NotInRoom, "no room"))?;
        if ready && team.is_empty() {
            return Err(refuse(Refusal::NotReady, "pick a ship first"));
        }
        match self.rooms[index].stage {
            Stage::Open => {}
            Stage::Queued | Stage::Countdown { .. } if !ready => {
                self.cancel_start(index, "a pick was taken back");
            }
            _ => return Err(refuse(Refusal::Started, "this game has already started")),
        }
        if let Some(member) = self.rooms[index]
            .members
            .iter_mut()
            .find(|m| m.client == id)
        {
            member.team = team.to_string();
            member.ready = ready;
        }
        self.push_room(index);
        Ok(())
    }

    fn start(&mut self, id: ClientId) -> Result<(), Fail> {
        let index = self.owned(id)?;
        let room = &self.rooms[index];
        if room.stage != Stage::Open {
            return Err(refuse(Refusal::Started, "this game has already started"));
        }
        if room.members.len() < usize::from(self.config.min_players) {
            return Err(refuse(
                Refusal::NeedPlayers,
                &format!(
                    "you cannot start this race until at least {} players are in the lobby",
                    self.config.min_players
                ),
            ));
        }
        let waiting: Vec<String> = room
            .members
            .iter()
            .filter(|m| !m.ready)
            .filter_map(|m| self.people.get(&m.client).map(|p| p.name.clone()))
            .collect();
        if !waiting.is_empty() {
            return Err(refuse(
                Refusal::NotReady,
                &format!("waiting for: {}", waiting.join(", ")),
            ));
        }
        self.rooms[index].stage = Stage::Queued;
        let room_id = self.rooms[index].id;
        self.queue.push_back(room_id);
        self.rooms[index].note = String::new();
        self.push_queue();
        Ok(())
    }

    fn cancel_start(&mut self, index: usize, why: &str) {
        let id = self.rooms[index].id;
        self.queue.retain(|&q| q != id);
        if self.active == Some(id) {
            self.active = None;
        }
        self.rooms[index].stage = Stage::Open;
        self.rooms[index].note = why.to_string();
        self.push_queue();
    }

    /// A person left, or their link closed. In a room that is racing they simply stop
    /// being a member of it.
    pub fn leave(&mut self, id: ClientId, reason: &str) {
        let Some(person) = self.people.get_mut(&id) else {
            return;
        };
        let Some(room) = person.room.take() else {
            return;
        };
        self.say(
            id,
            Message::Left {
                reason: reason.to_string(),
            },
        );
        let Some(index) = self.room_index(room) else {
            return;
        };
        let name = self.name(id).unwrap_or_default().to_string();
        let owner_left = self.rooms[index].members.first().map(|m| m.client) == Some(id);
        self.rooms[index].members.retain(|m| m.client != id);
        if self.rooms[index].members.is_empty() {
            // A racing room stays until its race is reported over, so the slot is not freed early.
            if self.rooms[index].stage != Stage::Racing {
                self.close_room(index);
            }
            return;
        }
        self.rooms[index].note = if owner_left {
            let next = self.rooms[index].members[0].client;
            let next_name = self.name(next).unwrap_or_default().to_string();
            format!("{name} left; {next_name} has been made host of this game")
        } else {
            format!("{name} left")
        };
        let below = self.rooms[index].members.len() < usize::from(self.config.min_players);
        if below
            && matches!(
                self.rooms[index].stage,
                Stage::Queued | Stage::Countdown { .. }
            )
        {
            self.cancel_start(index, "too few people are left to start");
        }
        self.push_room(index);
    }

    fn close_room(&mut self, index: usize) {
        let id = self.rooms[index].id;
        self.queue.retain(|&q| q != id);
        if self.active == Some(id) {
            self.active = None;
        }
        self.rooms.remove(index);
        self.push_queue();
    }

    /// A person's link closed: the same as leaving, and the name is free again.
    pub fn disconnect(&mut self, id: ClientId) {
        self.leave(id, "disconnected");
        self.people.remove(&id);
        self.outbox.retain(|(to, _)| *to != id);
    }

    /// The race of `room` ended (finished, crashed, timed out). The slot is free; the
    /// room is `Open` again with its people in it, picks cleared, and `reason` in its note.
    pub fn race_over(&mut self, room: u32, reason: &str) {
        if self.active == Some(room) {
            self.active = None;
        }
        let Some(index) = self.room_index(room) else {
            return;
        };
        if self.rooms[index].members.is_empty() {
            self.close_room(index);
            return;
        }
        self.rooms[index].stage = Stage::Open;
        self.rooms[index].note = reason.to_string();
        for member in &mut self.rooms[index].members {
            member.ready = false;
        }
        self.push_room(index);
    }

    /// The race for `room` could not be brought up (the runner failed): same as it ending.
    pub fn launch_failed(&mut self, room: u32, reason: &str) {
        self.race_over(room, &format!("the race could not start: {reason}"));
    }

    /// Whether the room's race is up (the hand-off was sent): its members read `Racing`.
    #[must_use]
    pub fn racing_members(&self, room: u32) -> Vec<ClientId> {
        self.room_index(room)
            .map(|i| self.rooms[i].members.iter().map(|m| m.client).collect())
            .unwrap_or_default()
    }

    /// Advances the clock to `now` (milliseconds): hands the free slot to the next room in
    /// line (skipping any that no longer qualifies), counts its start down, and produces the
    /// launch when the countdown is out.
    pub fn tick(&mut self, now: u64) {
        self.now = now;
        if self.active.is_none() {
            while let Some(&front) = self.queue.front() {
                let Some(index) = self.room_index(front) else {
                    self.queue.pop_front();
                    continue;
                };
                let room = &self.rooms[index];
                let ok = room.members.len() >= usize::from(self.config.min_players)
                    && room.members.iter().all(|m| m.ready);
                if ok {
                    self.queue.pop_front();
                    self.active = Some(front);
                    self.rooms[index].stage = Stage::Countdown {
                        until: now + u64::from(self.config.countdown_ms),
                        shown: u32::MAX,
                    };
                    self.push_queue();
                } else {
                    self.queue.pop_front();
                    self.rooms[index].stage = Stage::Open;
                    self.rooms[index].note = "the start was skipped: the room was not ready".into();
                    self.push_room(index);
                    self.push_queue();
                    continue;
                }
                break;
            }
        }
        let Some(active) = self.active else { return };
        let Some(index) = self.room_index(active) else {
            self.active = None;
            return;
        };
        if let Stage::Countdown { until, shown } = self.rooms[index].stage {
            if now >= until {
                self.rooms[index].stage = Stage::Racing;
                self.launch = Some(self.launch_for(index));
                self.push_room(index);
            } else {
                let seconds = u32::try_from((until - now).div_ceil(1000)).unwrap_or(u32::MAX);
                if seconds != shown {
                    self.rooms[index].stage = Stage::Countdown {
                        until,
                        shown: seconds,
                    };
                    self.push_room(index);
                }
            }
        }
    }

    fn launch_for(&mut self, index: usize) -> Launch {
        self.launches += 1;
        let room = &self.rooms[index];
        let seats: Vec<(ClientId, String)> = room
            .members
            .iter()
            .map(|m| {
                (
                    m.client,
                    self.people
                        .get(&m.client)
                        .map(|p| p.name.clone())
                        .unwrap_or_default(),
                )
            })
            .collect();
        let teams: Vec<String> = room.members.iter().map(|m| m.team.clone()).collect();
        let params = RaceParams {
            title: room.settings.title.clone(),
            track: room.settings.track.clone(),
            team: teams.first().cloned().unwrap_or_default(),
            teams,
            class: room.settings.class.clone(),
            mode: room.settings.mode.clone(),
            difficulty: "elite".to_string(),
            scheme: "veteran".to_string(),
            seed: self.config.seed ^ (u64::from(room.id) << 32) ^ self.launches,
            people: u8::try_from(seats.len()).unwrap_or(MAX_PLAYERS),
            opponents: room.settings.opponents,
            weapons: false,
        };
        Launch {
            room: room.id,
            params,
            seats,
        }
    }
}

#[cfg(test)]
mod tests;
