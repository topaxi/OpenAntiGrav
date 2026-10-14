//! LAN play from the menus: HOST GAME and JOIN GAME on the LAN page (**ours, chosen, not
//! measured**; the protocol is `oag-net`'s, `docs/networking/README.md`, "LAN play").
//!
//! **Host.** HOST GAME walks the RACE page, Track Select and Ship Select as a single-player
//! race does, then [`Session::open_lan_lobby`] starts an `oag_net::lan::Hosted` in this
//! process (the same authoritative host `oag-server` runs, on a thread, answering discovery)
//! and shows the lobby: the people waiting, the host first. The host plays as a client of its
//! own server over an in-process link, the one path a joiner takes, so it is seat 0 by
//! construction. START fixes the head count, the host thread loads the race and welcomes
//! everybody, and from the welcome on the race is the same as a `--connect` one.
//!
//! **Join.** JOIN GAME lists the hosts discovery finds (`oag_net::discovery`), with TYPE AN
//! ADDRESS for a host the broadcast does not reach (it asks that address directly, so the host
//! still answers with its certificate hash and nothing is typed but the address). Choosing a
//! host connects (off the frame thread), says hello and waits for the host's START. A host
//! that plays another title is refused by name before anything connects.
//!
//! **One local person per machine** in this cut: no split screen on a LAN client, and every
//! person flies the host's ship (the wire carries one team per race). Weapons are off, our AI
//! flies the opponents and the host's race is a Single Race, whatever the RACE page said
//! (`docs/networking/README.md`). Everything runs on the same [`oag_ui_screens::join`]
//! screen the LOCAL MULTIPLAYER join step uses, in its list form.

use log::{error, info, warn};
use oag_game::net_play::Pending;
use oag_gameplay::input::Button;
use oag_net::discovery::{Browser, Found};
use oag_net::lan::{Config, Hosted, Phase};
use oag_ui::menu::nav::Nav;
use oag_ui_screens::join::{Event, Join, Labels};
use oag_ui_screens::prompt;
use oag_thread::Task;

use crate::overlay::{Prompt, Purpose};
use crate::stage::Stage;

use super::Session;

/// Where LAN play is. Lives on the session because the host and the connection outlive the
/// menu stage: the race, and the end screens after it, run while they do.
pub(crate) struct Lan {
    step: Step,
}

enum Step {
    /// HOST GAME walking the RACE page, Track Select and Ship Select; nothing runs yet.
    Settings,
    /// The lobby: the host runs, and this machine is its first client.
    Lobby {
        hosted: Hosted,
        own: Pending,
        started: bool,
    },
    /// The host list.
    Browse { browser: Browser, found: Vec<Found> },
    /// Connecting to the chosen host (off the frame thread).
    Connecting {
        browser: Browser,
        task: Task<anyhow::Result<oag_net::wt::WtLink>>,
        to: Found,
    },
    /// Joined, waiting for the host's START.
    Waiting {
        browser: Browser,
        pending: Pending,
        to: Found,
    },
}

/// How many hosts the list holds before TYPE AN ADDRESS stops showing. **Chosen.**
pub(crate) const HOSTS_SHOWN: usize = 16;

/// Who this machine says it is: the login name, else "player". **Chosen.**
fn player_name() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .ok()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "player".into())
}

impl Session {
    fn lan_say(&self, id: &str, english: &str) -> String {
        self.shell
            .as_ref()
            .and_then(|shell| shell.strings.get(id))
            .unwrap_or(english)
            .to_string()
    }

    /// HOST GAME: the RACE page, pushed over the LAN page so BACK returns there.
    pub(crate) fn open_lan_host(&mut self) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        if !stage.menu.push("race") {
            warn!("lan: the menu has no RACE page to open");
            return;
        }
        self.lan = Some(Lan {
            step: Step::Settings,
        });
        self.resupply_race_modes();
    }

    /// Whether the RACE page is HOST GAME's settings step, whose launch opens the lobby
    /// instead of a race.
    pub(crate) fn lan_hosting_settings(&self) -> bool {
        matches!(
            self.lan,
            Some(Lan {
                step: Step::Settings
            })
        )
    }

    /// Ends the settings step once the menu has left the RACE page by BACK (and no Track or
    /// Ship Select is open over it).
    pub(crate) fn sync_lan(&mut self) {
        if !self.lan_hosting_settings() {
            return;
        }
        let Stage::Menu(stage) = &self.stage else {
            return;
        };
        if stage.menu.page().id != "race" && stage.picker.is_none() && stage.lan.is_none() {
            self.lan = None;
            self.resupply_race_modes();
        }
    }

    /// The host's launch: `self.race_options` is the race the pages describe. Starts the host
    /// and opens the lobby.
    pub(crate) fn open_lan_lobby(&mut self) {
        let Some(options) = self.race_options.clone() else {
            return;
        };
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let title = shell.title.name.to_string();
        let track_label = options
            .track
            .as_deref()
            .and_then(|entry| {
                shell
                    .tracks
                    .iter()
                    .find(|(track, _)| track.entry_name() == entry)
                    .map(|(_, name)| name.clone())
            })
            .unwrap_or_default();
        if options.mode != oag_race::Mode::SingleRace {
            warn!(
                "lan: a hosted race is a Single Race; {} becomes one",
                options.mode.name()
            );
        }
        let params = oag_net::wire::RaceParams {
            title: title.clone(),
            track: options.track.clone().unwrap_or_default(),
            team: options.team.clone().unwrap_or_default(),
            class: options.class.clone(),
            mode: oag_race::Mode::SingleRace.name().to_string(),
            difficulty: options.difficulty.name().to_string(),
            scheme: self.scheme.name().to_string(),
            seed: seed(),
            people: 1,
            opponents: options.opponents,
            weapons: false,
        };
        let name = player_name();
        let config = Config {
            source: options.source.clone(),
            params,
            name: name.clone(),
            track_label,
            port: oag_net::lan::DEFAULT_PORT,
        };
        let (hosted, own_link) = match Hosted::spawn(config) {
            Ok(started) => started,
            Err(e) => {
                error!("lan: cannot host: {e:#}");
                self.lan = None;
                return;
            }
        };
        info!(
            "lan: hosting on port {} (cert {}), discovery {}",
            hosted.port(),
            oag_net::wt::hex(&hosted.hash()),
            if hosted.announcing() { "on" } else { "off" }
        );
        let own = Pending::new(Box::new(own_link), &name, self.net_bot, "this machine");
        self.lan = Some(Lan {
            step: Step::Lobby {
                hosted,
                own,
                started: false,
            },
        });
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let labels = Labels::read_lan_lobby(&shell.strings);
        self.show_lan_screen(Join::new(labels, oag_gameplay::MAX_PLAYERS));
    }

    /// JOIN GAME: starts looking and shows the host list.
    pub(crate) fn open_lan_join(&mut self) {
        let browser = match Browser::new() {
            Ok(browser) => browser,
            Err(e) => {
                error!("lan: cannot look for games: {e}");
                return;
            }
        };
        self.lan = Some(Lan {
            step: Step::Browse {
                browser,
                found: Vec::new(),
            },
        });
        self.show_browse_screen(None);
    }

    fn show_browse_screen(&mut self, hint: Option<String>) {
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let labels = Labels::read_lan_list(&shell.strings);
        let mut screen = Join::new(labels, HOSTS_SHOWN).as_list();
        if let Some(hint) = hint {
            screen.set_hint(hint);
        }
        self.show_lan_screen(screen);
    }

    /// The screen while connecting and waiting for the host: the list's title and `hint`, and
    /// nothing to choose but BACK.
    fn show_waiting_screen(&mut self, hint: String) {
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let mut screen = Join::new(Labels::read_lan_list(&shell.strings), 0).as_list();
        screen.set_hint(hint);
        self.show_lan_screen(screen);
    }

    fn show_lan_screen(&mut self, screen: Join) {
        if let Stage::Menu(stage) = &mut self.stage {
            stage.lan = Some(screen);
        }
    }

    /// Leaves LAN play altogether: the host stops, the connection closes, the screen goes.
    pub(crate) fn end_lan(&mut self) {
        self.lan = None;
        self.net = None;
        if let Stage::Menu(stage) = &mut self.stage {
            stage.lan = None;
        }
        self.resupply_race_modes();
    }

    /// One tick of the open LAN screen: the pad and keys move and confirm, the pointer
    /// answers the same, then the state behind the screen advances.
    pub(crate) fn tick_lan(&mut self, pointer: &oag_ui::pointer::Pointer) {
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
        // START is START in the lobby wherever the cursor is, and a confirm in the list.
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
            self.handle_lan(event);
        }
        self.advance_lan();
    }

    /// Escape on the open LAN screen is BACK.
    pub(crate) fn escape_lan(&mut self) {
        self.handle_lan(Event::Back);
    }

    fn handle_lan(&mut self, event: Event) {
        match event {
            Event::Moved => {}
            Event::Back => {
                self.end_lan();
                self.play_navs(Some(Nav::Decline));
            }
            Event::Start => {
                if let Some(Lan {
                    step: Step::Lobby { hosted, started, .. },
                }) = &mut self.lan
                    && !*started
                {
                    *started = true;
                    hosted.start();
                    self.play_navs(Some(Nav::Accept));
                    if let Stage::Menu(stage) = &mut self.stage
                        && let Some(screen) = stage.lan.as_mut()
                    {
                        screen.set_hint("STARTING...".into());
                    }
                }
            }
            Event::Join => {
                if matches!(
                    self.lan,
                    Some(Lan {
                        step: Step::Browse { .. }
                    })
                ) {
                    self.open_lan_address_prompt();
                }
            }
            Event::Leave(chosen) => self.choose_lan_host(chosen),
        }
    }

    fn open_lan_address_prompt(&mut self) {
        let labels = prompt::Labels {
            title: self.lan_say("OAG_LAN_ADDRESS_TITLE", "HOST ADDRESS"),
            delete: self.lan_say("OAG_KEYBOARD_DELETE", "DEL"),
            accept: self.lan_say("OAG_KEYBOARD_ACCEPT", "OK"),
            hint: self.lan_say(
                "OAG_LAN_ADDRESS_HINT",
                "THE HOST COMPUTER'S ADDRESS, e.g. 192.168.1.20",
            ),
        };
        if let Stage::Menu(stage) = &mut self.stage {
            stage.prompt = Some(Prompt::typing(
                Purpose::LanAddress,
                prompt::Keyboard::with_keys(labels, "", 45, prompt::ADDRESS_KEYS),
            ));
        }
    }

    /// The typed address was accepted: the browser asks it directly from now on, and the host
    /// appears in the list when it answers.
    pub(crate) fn lan_address_entered(&mut self, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        let resolved = std::net::ToSocketAddrs::to_socket_addrs(&(text, 0))
            .ok()
            .and_then(|mut found| found.next());
        let Some(Lan {
            step: Step::Browse { browser, .. },
        }) = &mut self.lan
        else {
            return;
        };
        match resolved {
            Some(address) => {
                info!("lan: asking {} for a game", address.ip());
                browser.ask_also(address.ip());
            }
            None => {
                let hint = format!("{} {text}", self.lan_say("OAG_LAN_FAILED", "COULD NOT JOIN"));
                if let Stage::Menu(stage) = &mut self.stage
                    && let Some(screen) = stage.lan.as_mut()
                {
                    screen.set_hint(hint);
                }
            }
        }
    }

    /// A host row was chosen: refused by name when it plays another title, else connected to.
    fn choose_lan_host(&mut self, chosen: usize) {
        let mine = self.shell.as_ref().map(|shell| shell.title.name);
        let Some(Lan {
            step: Step::Browse { found, .. },
        }) = &self.lan
        else {
            return;
        };
        let Some(host) = found.get(chosen).cloned() else {
            return;
        };
        if mine.is_some_and(|mine| mine != host.advert.title) {
            let hint = self
                .lan_say("OAG_LAN_NOT_HERE", "THIS PLAYS %s, NOT THIS DISC")
                .replace("%s", &host.advert.title.to_uppercase());
            if let Stage::Menu(stage) = &mut self.stage
                && let Some(screen) = stage.lan.as_mut()
            {
                screen.set_hint(hint);
            }
            self.play_navs(Some(Nav::Decline));
            return;
        }
        let Some(Lan {
            step: Step::Browse { .. },
        }) = &self.lan
        else {
            return;
        };
        let Some(Lan { step }) = self.lan.take() else {
            return;
        };
        let Step::Browse { browser, .. } = step else {
            return;
        };
        let addr = host.addr.to_string();
        let hash = host.advert.hash;
        let task = Task::spawn("lan-connect", move || oag_net::wt::connect(&addr, hash));
        self.lan = Some(Lan {
            step: Step::Connecting {
                browser,
                task,
                to: host,
            },
        });
        self.show_waiting_screen(self.lan_say("OAG_LAN_CONNECTING", "CONNECTING..."));
        self.play_navs(Some(Nav::Accept));
    }

    /// Moves the state behind the screen on one step and refreshes what the screen says.
    fn advance_lan(&mut self) {
        let Some(Lan { step }) = self.lan.take() else {
            return;
        };
        let you = self.lan_say("OAG_LAN_YOU", "YOU");
        let (step, rows, hint) = match step {
            Step::Lobby {
                hosted,
                mut own,
                started,
            } => {
                let status = hosted.status();
                let mut names = status.names.clone();
                if let Some(first) = names.first_mut() {
                    first.push_str(&format!(" ({you})"));
                }
                match own.poll() {
                    Ok(Some(net)) => {
                        // The host's welcome has come: START was pressed and the race loaded.
                        self.net = Some(net);
                        self.lan = Some(Lan {
                            step: Step::Lobby {
                                hosted,
                                own,
                                started,
                            },
                        });
                        self.launch_lan_race();
                        return;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        error!("lan: {e:#}");
                        self.end_lan();
                        return;
                    }
                }
                if let Phase::Failed(why) = &status.phase {
                    error!("lan: the host stopped: {why}");
                    self.end_lan();
                    return;
                }
                let hint = started.then(|| "STARTING...".to_string());
                (
                    Step::Lobby {
                        hosted,
                        own,
                        started,
                    },
                    Rows::Seats(names),
                    hint,
                )
            }
            Step::Browse {
                mut browser,
                found: _,
            } => {
                let found = browser.poll();
                let rows = Rows::Hosts(found.clone());
                (Step::Browse { browser, found }, rows, None)
            }
            Step::Connecting {
                browser,
                mut task,
                to,
            } => {
                if !task.is_finished() {
                    (
                        Step::Connecting { browser, task, to },
                        Rows::Seats(Vec::new()),
                        None,
                    )
                } else {
                    match task.join() {
                        Some(Ok(Ok(link))) => {
                            let pending = Pending::new(
                                Box::new(link),
                                &player_name(),
                                self.net_bot,
                                &to.addr.to_string(),
                            );
                            (
                                Step::Waiting {
                                    browser,
                                    pending,
                                    to,
                                },
                                Rows::Seats(Vec::new()),
                                None,
                            )
                        }
                        failed => {
                            let why = match failed {
                                Some(Ok(Err(e))) => format!("{e:#}"),
                                _ => "the connection thread stopped".to_string(),
                            };
                            self.back_to_browse(browser, &why);
                            return;
                        }
                    }
                }
            }
            Step::Waiting {
                browser,
                mut pending,
                to,
            } => match pending.poll() {
                Ok(Some(net)) => {
                    let mine = self.shell.as_ref().map_or("", |shell| shell.title.name);
                    if let Err(e) = oag_net::race::check_title(net.params(), mine) {
                        self.back_to_browse(browser, &format!("{e:#}"));
                        return;
                    }
                    self.net = Some(net);
                    self.lan = Some(Lan {
                        step: Step::Waiting {
                            browser,
                            pending,
                            to,
                        },
                    });
                    self.launch_lan_race();
                    return;
                }
                Ok(None) => (
                    Step::Waiting {
                        browser,
                        pending,
                        to,
                    },
                    Rows::Seats(Vec::new()),
                    Some(self.lan_say(
                        "OAG_LAN_WAIT_HINT",
                        "WAITING FOR THE HOST TO START THE RACE",
                    )),
                ),
                Err(e) => {
                    self.back_to_browse(browser, &format!("{e:#}"));
                    return;
                }
            },
            Step::Settings => {
                self.lan = Some(Lan {
                    step: Step::Settings,
                });
                return;
            }
        };
        self.lan = Some(Lan { step });
        let mine = self.shell.as_ref().map_or("", |shell| shell.title.name);
        if let Stage::Menu(stage) = &mut self.stage
            && let Some(screen) = stage.lan.as_mut()
        {
            match rows {
                Rows::Seats(names) => screen.set_seats(names),
                Rows::Hosts(found) => {
                    screen.set_seats(
                        found
                            .iter()
                            .map(|host| {
                                // A host on another title says which, since it cannot be joined.
                                let other = (host.advert.title != mine)
                                    .then(|| format!("{}: ", host.advert.title.to_uppercase()))
                                    .unwrap_or_default();
                                format!("{other}{} ({})", host.advert.track.to_uppercase(), host.advert.people)
                            })
                            .collect(),
                    );
                    screen.set_row_labels(
                        found
                            .iter()
                            .map(|host| host.advert.name.to_uppercase())
                            .collect(),
                    );
                }
            }
            if let Some(hint) = hint {
                screen.set_hint(hint);
            }
        }
    }

    fn back_to_browse(&mut self, browser: Browser, why: &str) {
        warn!("lan: could not join: {why}");
        self.lan = Some(Lan {
            step: Step::Browse {
                browser,
                found: Vec::new(),
            },
        });
        let hint = format!("{} - {why}", self.lan_say("OAG_LAN_FAILED", "COULD NOT JOIN"));
        self.show_browse_screen(Some(hint));
    }

    /// The welcome has come: this machine's `self.net` is the race, and the launch is the
    /// ordinary one with the server's options (`Session::launch_race`).
    fn launch_lan_race(&mut self) {
        if let Stage::Menu(stage) = &mut self.stage {
            stage.lan = None;
        }
        if let Err(e) = self.launch_race() {
            error!("cannot start the LAN race: {e:#}");
            self.end_lan();
        }
    }
}

/// What the screen's rows hold after a step.
enum Rows {
    Seats(Vec<String>),
    Hosts(Vec<Found>),
}

/// A seed for the race, from the clock: it is outside the simulation, which only ever
/// sees it as a number the host chose. **Chosen.**
fn seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |since| since.as_nanos() as u64 & 0xFFFF_FFFF)
}
