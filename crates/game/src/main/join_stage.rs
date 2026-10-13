//! The "press a button to join" screen over the menu stage: who has a seat and
//! what the screen shows of it. The flow that opens, answers and closes it is
//! `crate::session::join`; the lobby rules are `oag_input::lobby`'s and the
//! screen `oag_ui_screens::join`'s.
//!
//! Like [`crate::pure_chain_stage::PureChainStage`] it takes the tick whole while
//! open and is drawn over the page behind it.

use oag_input::lobby::Lobby;
use oag_ui_screens::join::{Join, Labels};

/// The screen, open.
pub(crate) struct JoinStage {
    lobby: Lobby,
    screen: Join,
}

impl JoinStage {
    /// An empty lobby on a grid of `oag_gameplay::MAX_PLAYERS` seats.
    pub(crate) fn new(labels: Labels) -> Self {
        Self {
            lobby: Lobby::new(),
            screen: Join::new(labels, oag_gameplay::MAX_PLAYERS),
        }
    }

    pub(crate) fn lobby(&self) -> &Lobby {
        &self.lobby
    }

    pub(crate) fn lobby_mut(&mut self) -> &mut Lobby {
        &mut self.lobby
    }

    pub(crate) fn screen(&self) -> &Join {
        &self.screen
    }

    pub(crate) fn screen_mut(&mut self) -> &mut Join {
        &mut self.screen
    }
}
