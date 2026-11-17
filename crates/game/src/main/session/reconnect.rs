//! The RECONNECTING overlay of a network race whose link is gone, and giving up on it.

use oag_ui::frontend::Draw;

use super::Session;

impl Session {
    /// The overlay while `NetPlay` is dialling for its seat; empty otherwise.
    pub(crate) fn reconnect_screen(&self, grid: (f32, f32)) -> Vec<Draw> {
        let Some(seconds) = self
            .net
            .as_ref()
            .and_then(oag_game::net_play::NetPlay::reconnect_seconds_left)
        else {
            return Vec::new();
        };
        let say = |id: &str, english: &str| {
            self.shell
                .as_ref()
                .and_then(|shell| shell.strings.get(id))
                .unwrap_or(english)
                .to_string()
        };
        oag_game::net_play::reconnect::draw(
            grid,
            &say("OAG_NET_RECONNECTING", "RECONNECTING"),
            seconds,
            &say("OAG_NET_RECONNECT_HINT", "PRESS BACK TO LEAVE THE RACE"),
        )
    }

    /// BACK while reconnecting leaves the race at once; whether it did.
    pub(crate) fn give_up_reconnecting(&mut self) -> bool {
        match self.net.as_ref() {
            Some(net) if net.reconnecting() => {
                net.give_up();
                true
            }
            _ => false,
        }
    }
}
