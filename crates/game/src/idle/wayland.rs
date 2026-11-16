//! `idle-inhibit-unstable-v1` over the connection winit already holds.
//!
//! winit 0.30 does not expose the protocol, and an inhibitor must be created from the window's
//! own `wl_surface`, which only exists on winit's connection. So the display and surface are
//! wrapped from the raw handles instead of opening a second connection.

use log::debug;
use wayland_backend::client::{Backend, ObjectId};
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, delegate_noop};
use wayland_protocols::wp::idle_inhibit::zv1::client::zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1;
use wayland_protocols::wp::idle_inhibit::zv1::client::zwp_idle_inhibitor_v1::ZwpIdleInhibitorV1;
use winit::raw_window_handle::{
    HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use winit::window::Window;

use super::Backend as IdleBackend;

struct State;

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(State: ignore ZwpIdleInhibitManagerV1);
delegate_noop!(State: ZwpIdleInhibitorV1);

struct Inhibit {
    conn: Connection,
    qh: QueueHandle<State>,
    manager: ZwpIdleInhibitManagerV1,
    surface: WlSurface,
    inhibitor: Option<ZwpIdleInhibitorV1>,
}

impl IdleBackend for Inhibit {
    fn hold(&mut self, on: bool) {
        match (on, self.inhibitor.take()) {
            (true, None) => {
                self.inhibitor = Some(self.manager.create_inhibitor(&self.surface, &self.qh, ()));
            }
            (true, Some(held)) => self.inhibitor = Some(held),
            (false, Some(held)) => held.destroy(),
            (false, None) => {}
        }
        let _ = self.conn.flush();
    }
}

impl Drop for Inhibit {
    fn drop(&mut self) {
        self.hold(false);
        self.manager.destroy();
        let _ = self.conn.flush();
    }
}

/// The backend for `window`, or `None` when it is not a Wayland window or the compositor has no
/// idle inhibit manager (logged once per window, at debug).
pub(super) fn open(window: &Window) -> Option<Box<dyn IdleBackend>> {
    let RawDisplayHandle::Wayland(display) = window.display_handle().ok()?.as_raw() else {
        return None;
    };
    let RawWindowHandle::Wayland(surface) = window.window_handle().ok()?.as_raw() else {
        return None;
    };
    // SAFETY: both pointers come from winit's live Wayland window, which the caller holds for as
    // long as the returned backend (the session drops it before the window). The display is
    // wrapped as foreign, so dropping our `Backend` never disconnects winit's; `from_ptr` checks
    // the object's interface name before trusting the surface pointer. This is the only unsafe
    // in the feature: the protocol cannot be reached without the window's own `wl_surface`.
    #[allow(unsafe_code)]
    let (backend, id) = unsafe {
        let backend = Backend::from_foreign_display(display.display.as_ptr().cast());
        let id = ObjectId::from_ptr(WlSurface::interface(), surface.surface.as_ptr().cast());
        (backend, id)
    };
    let conn = Connection::from_backend(backend);
    let surface = WlSurface::from_id(&conn, id.ok()?).ok()?;
    let (globals, queue) = registry_queue_init::<State>(&conn).ok()?;
    let qh = queue.handle();
    let Ok(manager) = globals.bind::<ZwpIdleInhibitManagerV1, _, _>(&qh, 1..=1, ()) else {
        debug!("idle inhibit: the compositor has no zwp_idle_inhibit_manager_v1");
        return None;
    };
    debug!("idle inhibit: zwp_idle_inhibit_manager_v1 bound");
    Some(Box::new(Inhibit {
        conn,
        qh,
        manager,
        surface,
        inhibitor: None,
    }))
}
