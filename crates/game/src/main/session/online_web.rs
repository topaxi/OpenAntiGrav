//! Online play has no transport in the browser (`oag_net::wt` is native only): the ONLINE row
//! declines and nothing here runs. The native half is `online.rs`.

use crate::overlay::OnlineField;

use super::Session;

/// Never built on the web.
pub(crate) struct Online;

impl Session {
    pub(crate) fn open_online(&mut self) {}

    pub(crate) fn open_online_create(&mut self) {}

    pub(crate) fn open_online_join(&mut self) {}

    pub(crate) fn online_creating(&self) -> bool {
        false
    }

    pub(crate) fn online_picking_ship(&self) -> bool {
        false
    }

    pub(crate) fn open_online_room_from_settings(&mut self) {}

    pub(crate) fn open_online_list(&mut self) {}

    pub(crate) fn sync_online(&mut self) {}

    pub(crate) fn poll_online(&mut self) {}

    pub(crate) fn tick_online(&mut self, _pointer: &oag_ui::pointer::Pointer) {}

    pub(crate) fn escape_online(&mut self) {}

    pub(crate) fn online_menus_opened(&mut self) {}

    pub(crate) fn online_field_entered(&mut self, _field: OnlineField, _text: &str) {}
}
