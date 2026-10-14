//! LAN play on the web: not there. The browser's own `WebTransport` is the transport the
//! network crate will use on a page (ADR-0070) and is not built yet, so the LAN page's rows
//! decline and nothing here runs. The native half is `lan.rs`.

use oag_ui::menu::nav::Nav;

use super::Session;

/// LAN play's state: none on the web.
pub(crate) struct Lan;

/// What the host list compares: nothing on the web.
pub(crate) struct LanFacts;

impl Session {
    pub(crate) fn open_lan_host(&mut self) {
        self.play_navs(Some(Nav::Decline));
    }

    pub(crate) fn open_lan_join(&mut self) {
        self.play_navs(Some(Nav::Decline));
    }

    pub(crate) fn lan_hosting_settings(&self) -> bool {
        false
    }

    pub(crate) fn lan_picking_ship(&self) -> bool {
        false
    }

    pub(crate) fn open_lan_browse(&mut self) {}

    pub(crate) fn sync_lan(&mut self) {}

    pub(crate) fn open_lan_lobby(&mut self) {}

    pub(crate) fn tick_lan(&mut self, _pointer: &oag_ui::pointer::Pointer) {}

    pub(crate) fn escape_lan(&mut self) {}

    pub(crate) fn lan_address_entered(&mut self, _text: &str) {}
}
