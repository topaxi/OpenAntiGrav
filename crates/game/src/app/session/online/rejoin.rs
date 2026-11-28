//! REJOIN RACE: the offer to take the seat of an online race back after the game was closed or
//! crashed under it (`crate::rejoin_record`, `docs/networking/server-roadmap.md`, V1).
//!
//! ONLINE looks for a fresh record before it asks for a lobby. If there is one the race
//! server is dialled first (the record only says the race was running a short while ago, the
//! server answering says it still is), and only then is REJOIN RACE offered, with DECLINE,
//! which deletes the record. Yes dials again and goes through the race the lobby's hand-off
//! does, with the token the record kept. A server that does not answer, or refuses the seat,
//! deletes the record and goes on to the ordinary lobby questions.

use log::{info, warn};
use oag_net::online::client::Handoff;
use oag_thread::Task;
use oag_ui_screens::prompt;

use crate::app::overlay::{Prompt, Purpose};
use crate::app::stage::Stage;

use super::{Online, Session, Step};

impl Session {
    /// Starts the offer when a fresh record of this title's race exists; whether it did.
    pub(super) fn begin_rejoin_offer(&mut self) -> bool {
        let Some(file) = crate::rejoin_record::path() else {
            return false;
        };
        let title = self.shell.as_ref().map_or("", |shell| shell.title.name);
        let Some(rejoin) =
            crate::rejoin_record::load_from(&file, title, crate::rejoin_record::now())
        else {
            return false;
        };
        let (addr, hash) = (rejoin.addr.clone(), rejoin.cert_hash);
        info!("online: looking for the race {addr} to rejoin");
        let Some(online) = &mut self.online else {
            return false;
        };
        online.offer = Some(rejoin);
        online.step = Step::Probing(Task::spawn("online-rejoin-probe", move || {
            oag_net::wt::connect(&addr, hash)
        }));
        let hint = self.online_say("OAG_ONLINE_REJOIN_LOOKING", "LOOKING FOR YOUR RACE...");
        self.show_online_screen(self.online_waiting_screen(hint));
        true
    }

    /// The probe came back: the confirm if the race answered, the lobby questions if not.
    pub(super) fn rejoin_probed(&mut self, answered: bool) {
        if !answered {
            self.rejoin_gone();
            return;
        }
        if let Stage::Menu(stage) = self.stack.top_mut() {
            stage.lan = None;
        }
        let confirm = prompt::Confirm::opened_on_yes(prompt::ConfirmLabels {
            title: self.online_say("OAG_ONLINE_REJOIN_TITLE", "REJOIN RACE"),
            message: self.online_say(
                "OAG_ONLINE_REJOIN_ASK",
                "YOU WERE IN AN ONLINE RACE THAT IS STILL RUNNING. TAKE YOUR SEAT BACK?",
            ),
            yes: self.online_say("OAG_ONLINE_REJOIN_YES", "REJOIN RACE"),
            no: self.online_say("OAG_ONLINE_REJOIN_NO", "DECLINE"),
        });
        if let Some(online) = &mut self.online {
            online.step = Step::Offered;
        }
        if let Stage::Menu(stage) = self.stack.top_mut() {
            stage.prompt = Some(Prompt::asking(Purpose::RejoinRace, confirm));
        }
    }

    /// The offer was answered: yes dials the race, no deletes the record.
    pub(crate) fn rejoin_answered(&mut self, yes: bool) {
        let offer = self.online.as_mut().and_then(|online| online.offer.take());
        let Some(rejoin) = offer else {
            return;
        };
        if !yes {
            info!("online: the offer to rejoin was declined");
            Self::forget_rejoin();
            self.open_online_prompt(super::Field::Address);
            return;
        }
        let port = rejoin
            .addr
            .rsplit_once(':')
            .and_then(|(_, port)| port.parse::<u16>().ok());
        let Some(port) = port else {
            self.rejoin_gone();
            return;
        };
        let (addr, hash) = (rejoin.addr.clone(), rejoin.cert_hash);
        let Some(online) = &mut self.online else {
            return;
        };
        online.address.clone_from(&rejoin.addr);
        // A player-hosted race takes this seat's inputs only signed with the profile's key.
        if online.key.is_none() {
            online.key = crate::identity::load_or_create()
                .map(std::sync::Arc::new)
                .map_err(|e| warn!("online: no player key for the rejoined race: {e:#}"))
                .ok();
        }
        online.name.clone_from(&rejoin.name);
        online.team.clone_from(&rejoin.team);
        online.rejoined = true;
        online.step = Step::Dialing(
            Task::spawn("online-rejoin-connect", move || {
                oag_net::wt::connect(&addr, hash)
            }),
            Handoff {
                host: String::new(),
                port,
                cert_hash: rejoin.cert_hash,
                token: rejoin.token,
                name: rejoin.name,
                params: [0; 32],
                host_key: [0; 32],
            },
        );
        let hint = self.online_say("OAG_ONLINE_CONNECTING", "CONNECTING...");
        self.show_online_screen(self.online_waiting_screen(hint));
    }

    /// The race is not there any more (it answered no dial, or refused the seat): the record is
    /// deleted and the ordinary questions follow, with the reason on screen.
    pub(super) fn rejoin_gone(&mut self) {
        Self::forget_rejoin();
        if let Some(online) = &mut self.online {
            online.rejoined = false;
            online.offer = None;
        }
        let why = self.online_say("OAG_ONLINE_REJOIN_GONE", "YOUR RACE IS OVER");
        self.open_online_prompt(super::Field::Address);
        if let Stage::Menu(stage) = self.stack.top_mut()
            && let Some(prompt) = stage.prompt.as_mut()
        {
            prompt.set_note(Some(why));
        }
    }

    fn forget_rejoin() {
        if let Some(file) = crate::rejoin_record::path() {
            crate::rejoin_record::clear_at(&file);
        } else {
            warn!("online: no state directory to forget the seat in");
        }
    }
}

impl Online {
    /// Whether this is a rejoin: a race dialled from a record, with no lobby behind it.
    pub(super) fn rejoining(&self) -> bool {
        self.rejoined && self.lobby.is_none()
    }
}
