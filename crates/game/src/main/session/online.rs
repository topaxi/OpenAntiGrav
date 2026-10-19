//! Online play from the menus: the ONLINE row of the MULTIPLAYER page, the ONLINE page's
//! CREATE GAME and JOIN GAME, the room list and the room (**ours, chosen, not measured**; the
//! order is Omega's, `docs/networking/online-lobby.md` parts 1 and 2, the protocol is
//! `oag-net`'s, part 5).
//!
//! **Connect.** Omega's PSN login has no counterpart: ONLINE asks for the lobby's address, the
//! SHA-256 of its certificate and a name, one on-screen keyboard each, and remembers the
//! answers (`settings.online`), so the next visit is OK, OK, OK. The connection is made off the
//! frame thread; the lobby's welcome opens the ONLINE page.
//!
//! **Create.** CREATE GAME is the RACE page, Track Select and Ship Select, exactly as LAN's
//! HOST GAME walks them; the launch makes a room of those choices on the lobby instead of a
//! race. **Join.** JOIN GAME is Ship Select, then the lobby's list of rooms (only those this
//! machine can join are enabled); the pick travels as the ready. Both end in the room, on the
//! join screen in its lobby form: who is in, who is ready, the owner's START.
//!
//! **Race and back.** The lobby's hand-off becomes the ordinary network race (the same
//! `Pending` and launch LAN uses) with the seat token; the lobby connection stays open and is
//! polled in every stage, and when the menus reopen after the race the room is shown again.
//!
//! Everything runs on [`oag_ui_screens::join`], in the slot `stage.lan` also uses: only one of
//! the two is ever live.

use anyhow::Result;
use log::{error, info, warn};
use oag_game::net_play::Pending;
use oag_gameplay::input::Button;
use oag_net::online::client::{Handoff, LobbyClient};
use oag_game::online_screen::with_port;
use oag_net::wire::online::{Edition, RoomSettings};
use oag_thread::Task;
use oag_ui::menu::nav::Nav;
use oag_ui_screens::join::{Event, Join, Labels};
use oag_ui_screens::prompt;
use web_time::{Duration, Instant};

use crate::overlay::{OnlineField as Field, Prompt, Purpose};
use crate::stage::Stage;

use super::Session;

/// How often the room list is asked for again while it is on screen. **Chosen.**
const REFRESH: Duration = Duration::from_millis(1000);

/// The longest a name may be. **Chosen**: the lobby's own limit is checked server side, and a
/// refusal reads as a notice.
/// How long a refusal stays on the room screen. **Chosen.**
const NOTICE_FOR: Duration = Duration::from_secs(5);

const NAME_LIMIT: usize = 16;


/// Where online play is. Lives on the session because the lobby connection outlives the menu
/// stage: the race, and the menus after it, run while it does.
pub(crate) struct Online {
    step: Step,
    lobby: Option<LobbyClient>,
    /// The typed answers while the connect step runs.
    address: String,
    hash: String,
    name: String,
    /// What this machine can race, said in the hello.
    editions: Vec<Edition>,
    /// The ship picked for the room, sent as the ready.
    team: String,
    /// The settings of the room being made (CREATE), set when its launch is pressed.
    pick_sent: bool,
    asked_list: Option<Instant>,
    /// A message for the screen, shown until the next step replaces it.
    notice: Option<(String, Instant)>,
}

enum Step {
    /// Asking for `Field`.
    Typing,
    /// Dialling the lobby (off the frame thread).
    Connecting(Task<Result<oag_net::wt::WtLink>>),
    /// Said hello, waiting for the lobby's welcome (or its refusal).
    Hello,
    /// A message on screen with only BACK: the lobby closed, a race could not start.
    Notice,
    /// Welcomed: the ONLINE page is on screen.
    Hub,
    /// CREATE GAME's RACE page, Track Select and Ship Select.
    Settings,
    /// The room owner's ROOM SETTINGS: the same RACE page and pickers, seeded with the room's own
    /// settings, and START sends an edit instead of making a room.
    Editing,
    /// JOIN GAME's Ship Select.
    ShipPick,
    /// The room list.
    List,
    /// In a room.
    Room,
    /// The lobby handed a race over: dialling its port.
    Dialing(Task<Result<oag_net::wt::WtLink>>, Handoff),
    /// Said hello to the race, waiting for the welcome.
    Waiting(Box<Pending>),
}

impl Session {
    fn online_say(&self, id: &str, english: &str) -> String {
        self.shell
            .as_ref()
            .and_then(|shell| shell.strings.get(id))
            .unwrap_or(english)
            .to_string()
    }

    /// ONLINE on the MULTIPLAYER page: starts the connect step.
    pub(crate) fn open_online(&mut self) {
        if !matches!(self.stage, Stage::Menu(_)) || self.shell.is_none() {
            return;
        }
        let saved = self.settings.online.clone();
        let editions = vec![self.online_edition()];
        self.online = Some(Online {
            step: Step::Typing,
            lobby: None,
            address: saved.address,
            hash: saved.hash,
            name: if saved.name.is_empty() {
                super::lan::player_name()
            } else {
                saved.name
            },
            editions,
            team: String::new(),
            pick_sent: false,
            asked_list: None,
            notice: None,
        });
        self.open_online_prompt(Field::Address);
    }

    fn online_edition(&self) -> Edition {
        Edition {
            title: self.shell.as_ref().map_or("", |s| s.title.name).to_string(),
            platform: self
                .shell
                .as_ref()
                .map_or_else(String::new, |s| s.platform.to_string()),
            content: oag_game::mods::Mods::load()
                .ok()
                .map_or(0, |mods| mods.content_fingerprint()),
            build: oag_net::BUILD_VERSION.to_string(),
        }
    }

    fn open_online_prompt(&mut self, field: Field) {
        let Some(online) = &mut self.online else {
            return;
        };
        online.step = Step::Typing;
        let (title_id, title, hint_id, hint, initial, limit, keys) = match field {
            Field::Address => (
                "OAG_ONLINE_ADDRESS_TITLE",
                "LOBBY ADDRESS",
                "OAG_ONLINE_ADDRESS_HINT",
                "THE SERVER'S ADDRESS, e.g. race.example.org",
                online.address.clone(),
                60,
                prompt::ADDRESS_KEYS,
            ),
            Field::Hash => (
                "OAG_ONLINE_HASH_TITLE",
                "LOBBY CERTIFICATE",
                "OAG_ONLINE_HASH_HINT",
                "THE 64 DIGITS THE SERVER'S OWNER GIVES YOU",
                online.hash.clone(),
                64,
                "0123456789abcdef",
            ),
            Field::Name => (
                "OAG_ONLINE_NAME_TITLE",
                "YOUR NAME",
                "OAG_ONLINE_NAME_HINT",
                "HOW THE OTHER PLAYERS SEE YOU",
                online.name.clone(),
                NAME_LIMIT,
                "abcdefghijklmnopqrstuvwxyz0123456789-_",
            ),
        };
        let labels = prompt::Labels {
            title: self.online_say(title_id, title),
            delete: self.online_say("OAG_KEYBOARD_DELETE", "DEL"),
            accept: self.online_say("OAG_KEYBOARD_ACCEPT", "OK"),
            hint: self.online_say(hint_id, hint),
        };
        if let Stage::Menu(stage) = &mut self.stage {
            stage.prompt = Some(Prompt::typing(
                Purpose::Online(field),
                prompt::Keyboard::with_keys(labels, &initial, limit, keys),
            ));
        }
    }

    /// A typed answer was accepted: the next question, or the connection.
    pub(crate) fn online_field_entered(&mut self, field: Field, text: &str) {
        let text = text.trim().to_string();
        let Some(online) = &mut self.online else {
            return;
        };
        match field {
            Field::Address => {
                online.address = text;
                self.open_online_prompt(Field::Hash);
            }
            Field::Hash => {
                let hash = text.to_lowercase();
                match oag_net::wt::parse_hash(&hash) {
                    Ok(_) => {
                        online.hash = hash;
                        self.open_online_prompt(Field::Name);
                    }
                    Err(e) => {
                        let why = format!("{e:#}");
                        self.online_fail(&why);
                    }
                }
            }
            Field::Name => {
                online.name = text;
                self.online_connect();
            }
        }
    }

    fn online_connect(&mut self) {
        let Some(online) = &mut self.online else {
            return;
        };
        let (addr, hash) = (with_port(&online.address), oag_net::wt::parse_hash(&online.hash));
        let Ok(hash) = hash else {
            self.online_fail("the certificate hash is not 64 hex digits");
            return;
        };
        info!("online: dialling {addr}");
        online.step = Step::Connecting(Task::spawn("online-connect", move || {
            oag_net::wt::connect(&addr, hash)
        }));
        let hint = self.online_say("OAG_ONLINE_CONNECTING", "CONNECTING...");
        self.show_online_screen(self.online_waiting_screen(hint));
    }

    fn online_waiting_screen(&self, hint: String) -> Join {
        let strings = self.shell.as_ref().map(|shell| &shell.strings);
        let mut labels = strings.map_or_else(
            || Labels::read(&oag_ui::language::StringTable::from_xml("<Strings/>")),
            Labels::read_online_list,
        );
        labels.hint = hint;
        Join::new(labels, 0).as_list()
    }

    /// Back to the connect step's first question with `why` on the screen.
    fn online_fail(&mut self, why: &str) {
        warn!("online: {why}");
        let notice = format!("{} - {why}", self.online_say("OAG_ONLINE_FAILED", "COULD NOT CONNECT"));
        if let Some(online) = &mut self.online {
            online.lobby = None;
        }
        if let Stage::Menu(stage) = &mut self.stage {
            stage.lan = None;
        }
        self.open_online_prompt(Field::Address);
        if let Stage::Menu(stage) = &mut self.stage
            && let Some(prompt) = stage.prompt.as_mut()
        {
            prompt.set_note(Some(notice));
        }
    }

    fn show_online_screen(&mut self, screen: Join) {
        if let Stage::Menu(stage) = &mut self.stage {
            stage.lan = Some(screen);
        }
    }

    /// Leaves online play altogether: the connection closes and the screen goes.
    pub(crate) fn end_online(&mut self) {
        self.online = None;
        if let Stage::Menu(stage) = &mut self.stage {
            stage.lan = None;
        }
        self.resupply_race_modes();
    }

    /// CREATE GAME: the RACE page, pushed over the ONLINE page so BACK returns there.
    pub(crate) fn open_online_create(&mut self) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        if self.online.is_none() || !stage.menu.push("race") {
            return;
        }
        if let Some(online) = &mut self.online {
            online.step = Step::Settings;
        }
        self.resupply_race_modes();
    }

    /// JOIN GAME: Ship Select, then the list.
    pub(crate) fn open_online_join(&mut self) {
        let Some(online) = &mut self.online else {
            return;
        };
        online.step = Step::ShipPick;
        if !self.open_ship_picker() {
            // A title with no Ship Select: the settings' team stands.
            self.launch_from_settings();
        }
    }

    /// Whether the RACE page is CREATE GAME's settings step.
    pub(crate) fn online_creating(&self) -> bool {
        matches!(&self.online, Some(Online { step: Step::Settings | Step::Editing, .. }))
    }

    /// Whether JOIN GAME is in its Ship Select.
    pub(crate) fn online_picking_ship(&self) -> bool {
        matches!(&self.online, Some(Online { step: Step::ShipPick, .. }))
    }

    /// Ends a step whose page the menu has left by BACK, and the connection with the ONLINE
    /// page; cancels the connect step when its keyboard was.
    pub(crate) fn sync_online(&mut self) {
        let Some(online) = &self.online else {
            return;
        };
        let Stage::Menu(stage) = &self.stage else {
            return;
        };
        let page = stage.menu.page().id.as_str();
        let screens = stage.picker.is_some() || stage.lan.is_some() || stage.prompt.is_some();
        let gone = match &online.step {
            Step::Typing => stage.prompt.is_none(),
            Step::Hub => page != "online" && !screens,
            Step::Settings | Step::Editing => page != "race" && !screens,
            Step::ShipPick => !screens,
            _ => false,
        };
        if !gone {
            return;
        }
        if matches!(online.step, Step::Editing) {
            self.online_back_to_room();
            return;
        }
        let back_to_hub = matches!(online.step, Step::Settings | Step::ShipPick)
            && online.lobby.is_some();
        if back_to_hub {
            if let Some(online) = &mut self.online {
                online.step = Step::Hub;
            }
            self.resupply_race_modes();
        } else {
            self.end_online();
        }
    }

    /// The team a person goes into a room with: the settings' own, or - when a title with no
    /// Ship Select does not offer the stored one (2048 has no `Assegai`) - the title's own
    /// default, so a joiner who never saw a picker still has a pick to send as the ready.
    fn online_team(&self, options: &oag_raceplay::Options) -> String {
        let Some(shell) = self.shell.as_ref() else {
            return options.team.clone().unwrap_or_default();
        };
        oag_game::online_screen::room_team(options.team.as_deref(), shell.title.race.team, &shell.teams)
    }

    /// The RACE page's choices as a room's settings, `max_players` seats.
    fn room_settings_from(&self, options: &oag_raceplay::Options, max_players: u8) -> Option<RoomSettings> {
        let shell = self.shell.as_ref()?;
        Some(RoomSettings {
            title: shell.title.name.to_string(),
            track: options.track.clone().unwrap_or_default(),
            class: options.class.clone(),
            // A stored Eliminator or Head to Head (weapons, two people) is a Single Race here.
            mode: match options.mode {
                oag_race::Mode::Eliminator | oag_race::Mode::Head2Head => {
                    oag_race::Mode::SingleRace.name().to_string()
                }
                other => other.name().to_string(),
            },
            max_players,
            opponents: options.opponents,
            laps: options
                .laps_override
                .map_or(0, |laps| u8::try_from(laps).unwrap_or(u8::MAX)),
            // The room-only rows (`oag_game::online_screen::setting_rows`): a new room has
            // rolls on, the assist allowed (chosen: Omega's list names On first) and Omega's
            // own default zone target in Zone.
            no_barrel_rolls: false,
            pilot_assist: true,
            zone_target: if options.mode == oag_race::Mode::Zone {
                oag_game::online_screen::DEFAULT_ZONE_TARGET
            } else {
                0
            },
        })
    }

    /// The RACE page's launch while online: CREATE GAME's choices become a room, or the owner's
    /// ROOM SETTINGS become an edit of the room they are in.
    pub(crate) fn open_online_room_from_settings(&mut self) {
        let Some(options) = self.race_options.clone() else {
            return;
        };
        let editing = matches!(&self.online, Some(Online { step: Step::Editing, .. }));
        let max_players = self
            .online
            .as_ref()
            .and_then(|online| online.lobby.as_ref())
            .and_then(|lobby| lobby.room.as_ref())
            .filter(|_| editing)
            .map_or(oag_net::wire::online::MAX_PLAYERS, |room| room.settings.max_players);
        let Some(mut settings) = self.room_settings_from(&options, max_players) else {
            return;
        };
        // The RACE page has none of the room-only rows (laps, barrel rolls, Pilot Assist, the
        // zone target): an edit keeps the room's own, which the room screen's rows set.
        if editing
            && let Some(room) = self
                .online
                .as_ref()
                .and_then(|online| online.lobby.as_ref())
                .and_then(|lobby| lobby.room.as_ref())
        {
            settings.laps = room.settings.laps;
            settings.no_barrel_rolls = room.settings.no_barrel_rolls;
            settings.pilot_assist = room.settings.pilot_assist;
            // Changing the mode to Zone starts from Omega's default; Zone to Zone keeps the
            // room's choice, including none.
            if room.settings.mode == settings.mode {
                settings.zone_target = room.settings.zone_target;
            }
        }
        let team = self.online_team(&options);
        let Some(online) = &mut self.online else {
            return;
        };
        let Some(lobby) = &mut online.lobby else {
            return;
        };
        online.team = team;
        online.pick_sent = false;
        if editing {
            lobby.edit(settings);
        } else {
            lobby.create(settings);
        }
        online.step = Step::Room;
        // The room stands over the ONLINE page, not over the RACE page it was made on, so
        // BACK from the room lands on the page the player chose CREATE GAME from.
        if let Stage::Menu(stage) = &mut self.stage {
            for _ in 0..4 {
                if stage.menu.page().id == "online" {
                    break;
                }
                stage.menu.back();
            }
            let _ = stage.menu.take_nav();
        }
        self.show_room_screen();
    }

    /// ROOM SETTINGS: the RACE page again, showing the room's own mode, class and circuit; START
    /// sends the owner's changes as an edit.
    fn open_online_edit(&mut self) {
        let Some(room) = self
            .online
            .as_ref()
            .and_then(|online| online.lobby.as_ref())
            .and_then(|lobby| lobby.room.clone())
        else {
            return;
        };
        let track_id = self.shell.as_ref().and_then(|shell| {
            shell
                .tracks
                .iter()
                .find(|(track, _)| track.entry_name() == room.settings.track)
                .map(|(track, _)| track.id.clone())
        });
        self.settings.race.mode = room.settings.mode.clone();
        self.settings.race.class = room.settings.class.clone();
        if let Some(id) = &track_id {
            self.settings.race.track = id.clone();
        }
        if let Stage::Menu(stage) = &mut self.stage {
            stage.lan = None;
            if !stage.menu.push("race") {
                return;
            }
            use oag_ui::menu::Value;
            stage.menu.seed("race.mode", &Value::Text(room.settings.mode.clone()));
            stage.menu.seed("race.class", &Value::Text(room.settings.class.clone()));
            if let Some(id) = track_id {
                stage.menu.seed("race.track", &Value::Text(id));
            }
        }
        if let Some(online) = &mut self.online {
            online.step = Step::Editing;
        }
        self.resupply_race_modes();
    }

    /// One of the owner's rows on the room screen (laps, barrel rolls, Pilot Assist, the zone
    /// target) stepped by `by`, sent as an edit like MAX PLAYERS.
    fn online_step_setting(&mut self, setting: oag_ui_screens::join::Setting, by: i32) {
        let Some(lobby) = self.online.as_mut().and_then(|online| online.lobby.as_mut()) else {
            return;
        };
        let Some(room) = lobby.room.as_ref() else {
            return;
        };
        let mut settings = room.settings.clone();
        oag_game::online_screen::step_setting(&mut settings, setting, by);
        lobby.edit(settings);
    }

    /// MAX PLAYERS on the room screen: one more or fewer seat (`by`), never fewer than the people
    /// already seated, wrapping round at the ends. The server holds the number and tells everyone.
    fn online_step_max_players(&mut self, by: i32) {
        let Some(lobby) = self.online.as_mut().and_then(|online| online.lobby.as_mut()) else {
            return;
        };
        let Some(room) = lobby.room.as_ref() else {
            return;
        };
        let fewest = i32::try_from(room.members.len())
            .unwrap_or(0)
            .max(i32::from(oag_net::wire::online::MIN_PLAYERS));
        let most = i32::from(oag_net::wire::online::MAX_PLAYERS);
        let next = oag_game::online_screen::step_max_players(
            i32::from(room.settings.max_players),
            by,
            fewest,
            most,
        );
        let mut settings = room.settings.clone();
        settings.max_players = u8::try_from(next).unwrap_or(settings.max_players);
        lobby.edit(settings);
    }

    /// The pick is made on JOIN GAME: the list is next.
    pub(crate) fn open_online_list(&mut self) {
        let team = self
            .race_options
            .as_ref()
            .map(|options| self.online_team(options))
            .unwrap_or_default();
        let Some(online) = &mut self.online else {
            return;
        };
        online.team = team;
        online.step = Step::List;
        online.asked_list = None;
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let screen = Join::new(Labels::read_online_list(&shell.strings), 16).as_list();
        self.show_online_screen(screen);
    }

    fn show_room_screen(&mut self) {
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let screen = Join::new(
            Labels::read_online_room(&shell.strings),
            usize::from(oag_net::wire::online::MAX_PLAYERS),
        );
        self.show_online_screen(screen);
    }

    /// One tick of the open online screen: the pad and keys move and confirm, the pointer
    /// answers the same.
    pub(crate) fn tick_online(&mut self, pointer: &oag_ui::pointer::Pointer) {
        let presses: u32 = self
            .controls
            .device_presses()
            .iter()
            .fold(0, |all, press| all | press.buttons);
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let skin = &stage.skin;
        let Some(screen) = stage.lan.as_mut() else {
            return;
        };
        let mut events: Vec<Event> = Vec::new();
        if presses & Button::Down.bit() != 0 {
            screen.step(1);
        }
        if presses & Button::Up.bit() != 0 {
            screen.step(-1);
        }
        if presses & Button::Cross.bit() != 0 {
            events.extend(screen.confirm());
        }
        if presses & Button::Left.bit() != 0 {
            events.extend(screen.adjust(-1));
        }
        if presses & Button::Right.bit() != 0 {
            events.extend(screen.adjust(1));
        }
        if presses & Button::Start.bit() != 0 {
            if screen.is_list() {
                events.extend(screen.confirm());
            } else if screen.can_start() {
                events.push(Event::Start);
            }
        }
        if presses & Button::Circle.bit() != 0 {
            events.push(Event::Back);
        }
        events.extend(screen.pointer(pointer, skin));
        for event in events {
            if !matches!(&self.stage, Stage::Menu(stage) if stage.lan.is_some()) {
                break;
            }
            self.handle_online(event);
        }
    }

    /// Escape on the open online screen is BACK.
    pub(crate) fn escape_online(&mut self) {
        self.handle_online(Event::Back);
    }

    fn handle_online(&mut self, event: Event) {
        match event {
            Event::Moved => {}
            Event::Back => self.online_back(),
            Event::Start => {
                if let Some(online) = &mut self.online
                    && matches!(online.step, Step::Room)
                    && let Some(lobby) = &mut online.lobby
                {
                    lobby.start();
                    self.play_navs(Some(Nav::Accept));
                }
            }
            Event::Settings => {
                if matches!(&self.online, Some(Online { step: Step::Room, .. })) {
                    self.open_online_edit();
                    self.play_navs(Some(Nav::Accept));
                }
            }
            Event::Setting(setting, by) => {
                if matches!(&self.online, Some(Online { step: Step::Room, .. })) {
                    self.online_step_setting(setting, by);
                    self.play_navs(Some(Nav::Accept));
                }
            }
            Event::MaxPlayers(by) => {
                if matches!(&self.online, Some(Online { step: Step::Room, .. })) {
                    self.online_step_max_players(by);
                    self.play_navs(Some(Nav::Accept));
                }
            }
            // The empty row of the list makes a room of one's own.
            Event::Join => {
                if matches!(&self.online, Some(Online { step: Step::List, .. })) {
                    self.online_list_to_create();
                }
            }
            Event::Leave(chosen) => {
                if matches!(&self.online, Some(Online { step: Step::List, .. })) {
                    self.online_join_room(chosen);
                }
            }
        }
    }

    fn online_back(&mut self) {
        let Some(online) = &mut self.online else {
            return;
        };
        match online.step {
            Step::Room => {
                if let Some(lobby) = &mut online.lobby {
                    lobby.leave();
                }
                online.step = Step::Hub;
                if let Stage::Menu(stage) = &mut self.stage {
                    stage.lan = None;
                }
            }
            Step::List => {
                online.step = Step::Hub;
                if let Stage::Menu(stage) = &mut self.stage {
                    stage.lan = None;
                }
            }
            Step::Notice => {
                if online.lobby.as_mut().is_some_and(|lobby| lobby.poll().is_ok()) {
                    online.step = Step::Hub;
                    if let Stage::Menu(stage) = &mut self.stage {
                        stage.lan = None;
                    }
                } else {
                    self.end_online();
                }
            }
            Step::Connecting(_) | Step::Hello | Step::Dialing(..) | Step::Waiting(_) => {
                self.end_online();
            }
            _ => {}
        }
        self.play_navs(Some(Nav::Decline));
    }

    fn online_list_to_create(&mut self) {
        if let Stage::Menu(stage) = &mut self.stage {
            stage.lan = None;
        }
        self.open_online_create();
    }

    fn online_join_room(&mut self, chosen: usize) {
        let Some(online) = &mut self.online else {
            return;
        };
        let Some(lobby) = &mut online.lobby else {
            return;
        };
        let Some(room) = lobby.rooms.get(chosen) else {
            return;
        };
        if !room.joinable {
            self.play_navs(Some(Nav::Decline));
            return;
        }
        let id = room.id;
        lobby.join(id);
        online.pick_sent = false;
        online.step = Step::Room;
        self.show_room_screen();
        self.play_navs(Some(Nav::Accept));
    }

    /// Every tick, in every stage: reads the lobby, advances the connection and hand-off, and
    /// refreshes the screen.
    pub(crate) fn poll_online(&mut self) {
        let Some(mut online) = self.online.take() else {
            return;
        };
        let mut lost = None;
        let mut refused = None;
        if let Some(lobby) = &mut online.lobby {
            if let Err(why) = lobby.poll() {
                lost = Some(format!("{}: {why}", self.online_say("OAG_ONLINE_LOST", "THE LOBBY CLOSED")));
            }
            if let Some((_code, text)) = lobby.refused.take() {
                if matches!(online.step, Step::Hello) {
                    refused = Some(text);
                } else {
                    online.notice = Some((text.to_uppercase(), Instant::now()));
                    // A refused join leaves the room list where it was.
                    if matches!(online.step, Step::Room) && lobby.room.is_none() {
                        online.step = Step::List;
                    }
                }
            }
        }
        if let Some(why) = refused.or(lost) {
            let before_welcome = matches!(online.step, Step::Typing | Step::Connecting(_) | Step::Hello);
            self.online = Some(online);
            if before_welcome {
                self.online_fail(&why);
            } else {
                self.online_notice(why);
            }
            return;
        }
        let step = std::mem::replace(&mut online.step, Step::Hub);
        online.step = match step {
            Step::Connecting(mut task) if task.is_finished() => match task.join() {
                Some(Ok(Ok(link))) => {
                    online.lobby = Some(LobbyClient::connect(
                        Box::new(link),
                        &online.name,
                        &online.editions,
                        "",
                    ));
                    Step::Hello
                }
                other => {
                    let why = match other {
                        Some(Ok(Err(e))) => format!("{e:#}"),
                        _ => "the connection thread stopped".to_string(),
                    };
                    self.online = Some(online);
                    self.online_fail(&why);
                    return;
                }
            },
            Step::Dialing(mut task, handoff) if task.is_finished() => match task.join() {
                Some(Ok(Ok(link))) => Step::Waiting(Box::new(Pending::new(
                    Box::new(link),
                    &handoff.name,
                    &online.team,
                    &handoff.token,
                    self.net_bot,
                    "the online race",
                ))),
                other => {
                    let why = match other {
                        Some(Ok(Err(e))) => format!("{e:#}"),
                        _ => "the connection thread stopped".to_string(),
                    };
                    online.notice = Some((why.to_uppercase(), Instant::now()));
                    Step::Hub
                }
            },
            other => other,
        };
        self.online = Some(online);
        self.advance_online();
    }

    /// A message on a screen of its own with only BACK; the connection is dropped when it is
    /// the lobby that is gone (BACK then ends online play).
    fn online_notice(&mut self, text: String) {
        warn!("online: {text}");
        let Some(online) = &mut self.online else {
            return;
        };
        online.step = Step::Notice;
        if matches!(self.stage, Stage::Menu(_)) {
            let screen = self.online_waiting_screen(text.to_uppercase());
            self.show_online_screen(screen);
        }
    }

    /// Moves the steps that wait on something on, and refreshes what the screen says.
    fn advance_online(&mut self) {
        let Some(mut online) = self.online.take() else {
            return;
        };
        let mut hub = false;
        let mut launch = None;
        // The welcome opens the ONLINE page.
        if matches!(online.step, Step::Hello)
            && let Some(lobby) = &online.lobby
            && lobby.welcomed()
        {
            info!("online: welcomed as {:?}, the server races {:?}", lobby.name, lobby.served);
            hub = true;
        }
        // The hand-off dials the race.
        let mut dial = None;
        if let Some(lobby) = &mut online.lobby
            && lobby.has_handoff()
            && matches!(online.step, Step::Room)
            && let Some(handoff) = lobby.take_handoff()
        {
            dial = Some(handoff);
        }
        if let Some(handoff) = dial {
            let host = with_port(&online.address);
            let host = host.rsplit_once(':').map_or(host.as_str(), |(h, _)| h).to_string();
            let addr = format!("{host}:{}", handoff.port);
            let hash = handoff.cert_hash;
            info!("online: the race is on {addr}");
            online.step = Step::Dialing(
                Task::spawn("online-race-connect", move || oag_net::wt::connect(&addr, hash)),
                handoff,
            );
        }
        if let Step::Waiting(pending) = &mut online.step {
            match pending.poll() {
                Ok(Some(net)) => {
                    let mine = self.shell.as_ref().map_or("", |shell| shell.title.name);
                    match oag_net::race::check_title(net.params(), mine) {
                        Ok(()) => launch = Some(net),
                        Err(e) => {
                            online.notice = Some((format!("{e:#}").to_uppercase(), Instant::now()));
                            online.step = Step::Hub;
                        }
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    online.notice = Some((format!("{e:#}").to_uppercase(), Instant::now()));
                    online.step = Step::Hub;
                }
            }
        }
        // The pick is the ready, once in the room; an edit clears it and it goes again. It
        // carries the Pilot Assist level this person flies with if the room allows it.
        let assist = self.settings.controls.pilot_assist;
        if matches!(online.step, Step::Room)
            && let Some(lobby) = &mut online.lobby
        {
            let me = lobby.name.clone();
            let mine_ready = lobby
                .room
                .as_ref()
                .and_then(|room| room.members.iter().find(|m| Some(&m.name) == me.as_ref()))
                .map(|m| m.ready);
            if mine_ready == Some(false) && !online.team.is_empty() && !online.pick_sent {
                lobby.ready(&online.team, true, assist);
                online.pick_sent = true;
            }
            if mine_ready == Some(true) {
                online.pick_sent = false;
            }
        }
        // The list is asked for again now and then.
        if matches!(online.step, Step::List)
            && let Some(lobby) = &mut online.lobby
            && online.asked_list.is_none_or(|at| at.elapsed() > REFRESH)
        {
            lobby.list();
            online.asked_list = Some(Instant::now());
        }
        // The lobby sent this person out of the room.
        if matches!(online.step, Step::Room)
            && let Some(lobby) = &mut online.lobby
            && lobby.room.is_none()
            && let Some(reason) = lobby.left.take()
        {
            online.notice = Some((reason.to_uppercase(), Instant::now()));
            online.step = Step::Hub;
        }
        // What the screen shows.
        if online.notice.as_ref().is_some_and(|(_, at)| at.elapsed() > NOTICE_FOR) {
            online.notice = None;
        }
        let notice = online.notice.as_ref().map(|(text, _)| text.clone());
        let hub_notice = matches!(online.step, Step::Hub)
            .then(|| online.notice.take().map(|(text, _)| text))
            .flatten();
        let screen_rows = match (&online.step, &online.lobby) {
            (Step::List, Some(lobby)) => Some(Rows::Rooms(lobby.rooms.clone())),
            (Step::Room, Some(lobby)) => lobby.room.as_ref().map(|room| {
                let owner = room.members.first().is_some_and(|m| m.owner)
                    && room.members.first().map(|m| &m.name) == lobby.name.as_ref();
                Rows::Room(room.clone(), owner)
            }),
            _ => None,
        };
        let to_hub = matches!(online.step, Step::Hub) && online.lobby.is_some();
        self.online = Some(online);
        if hub {
            self.online_enter_hub();
        } else if let Some(text) = hub_notice {
            self.online_notice(text);
        } else if to_hub {
            self.online_leave_screens_for_hub();
        }
        if let Some(net) = launch {
            self.net = Some(net);
            if let Stage::Menu(stage) = &mut self.stage {
                stage.lan = None;
            }
            if let Err(e) = self.launch_race() {
                error!("cannot start the online race: {e:#}");
                self.online_back_to_room();
            }
            return;
        }
        let track_labels: Vec<(String, String)> = self.shell.as_ref().map_or_else(Vec::new, |shell| {
            shell
                .tracks
                .iter()
                .map(|(track, name)| (track.entry_name().to_string(), name.clone()))
                .collect()
        });
        let (Some(shell), Stage::Menu(stage)) = (self.shell.as_ref(), &mut self.stage) else {
            return;
        };
        // A refused join leaves the room form on screen over the list: the form follows the step.
        let wrong_form = stage.lan.as_ref().is_some_and(|screen| match &screen_rows {
            Some(Rows::Rooms(_)) => !screen.is_list(),
            Some(Rows::Room(..)) => screen.is_list(),
            None => false,
        });
        if wrong_form {
            stage.lan = Some(match &screen_rows {
                Some(Rows::Rooms(_)) => {
                    Join::new(Labels::read_online_list(&shell.strings), 16).as_list()
                }
                _ => Join::new(
                    Labels::read_online_room(&shell.strings),
                    usize::from(oag_net::wire::online::MAX_PLAYERS),
                ),
            });
        }
        if let Some(screen) = stage.lan.as_mut() {
            match screen_rows {
                Some(Rows::Rooms(rooms)) => {
                    oag_game::online_screen::fill_list(screen, &rooms, &shell.strings, &|entry| {
                        track_labels
                            .iter()
                            .find(|(known, _)| known == entry)
                            .map(|(_, name)| name.clone())
                    });
                    if let Some(notice) = notice {
                        screen.set_hint(notice);
                    }
                }
                Some(Rows::Room(room, owner)) => {
                    oag_game::online_screen::fill_room(screen, &room, owner, &shell.strings, &|entry| {
                        track_labels
                            .iter()
                            .find(|(known, _)| known == entry)
                            .map(|(_, name)| name.clone())
                    });
                    if let Some(notice) = notice {
                        screen.set_hint(notice);
                    }
                }
                None => {}
            }
        }
    }

    /// The welcome has come: remember the answers and open the ONLINE page.
    fn online_enter_hub(&mut self) {
        let Some(online) = &mut self.online else {
            return;
        };
        online.step = Step::Hub;
        self.settings.online = crate::settings::Online {
            address: online.address.clone(),
            hash: online.hash.clone(),
            name: online.name.clone(),
        };
        if let Err(e) = crate::settings::save(&self.settings) {
            warn!("online: cannot remember the lobby: {e:#}");
        }
        if let Stage::Menu(stage) = &mut self.stage {
            stage.lan = None;
            stage.menu.push("online");
        }
    }

    /// Out of a room or list back to the ONLINE page, with the notice on its subtitle line.
    fn online_leave_screens_for_hub(&mut self) {
        if let Stage::Menu(stage) = &mut self.stage
            && stage.lan.is_some()
        {
            stage.lan = None;
        }
    }

    /// Called as the menus reopen: a race that ended (or was left) lands back in the room.
    pub(crate) fn online_menus_opened(&mut self) {
        let Some(online) = &mut self.online else {
            return;
        };
        if !matches!(online.step, Step::Waiting(_) | Step::Dialing(..) | Step::Room) {
            return;
        }
        online.step = Step::Room;
        online.pick_sent = false;
        if let Stage::Menu(stage) = &mut self.stage {
            stage.menu.push("multiplayer");
            stage.menu.push("online");
        }
        self.show_room_screen();
    }

    fn online_back_to_room(&mut self) {
        if let Some(online) = &mut self.online {
            online.step = Step::Room;
        }
        self.show_room_screen();
    }
}

enum Rows {
    Rooms(Vec<oag_net::wire::online::RoomInfo>),
    Room(oag_net::wire::online::Snapshot, bool),
}
