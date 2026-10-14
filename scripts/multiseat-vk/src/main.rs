//! `vk <seat> <keymap-file> <script>`: a `zwp_virtual_keyboard_v1` on the named `wl_seat`, for testing
//! a game's per-seat keyboards on a private headless compositor (`docs/rendering/multi-window.md`,
//! "Who sits where"). Standalone: not a workspace member, build it from this directory with
//! `cargo build`. The keymap is `xkbcli compile-keymap --layout us > us.xkb`.
//!
//! `script` is semicolon separated: `down N`, `up N` (N an evdev keycode, X is 45, Left 105) and
//! `sleep MS`. Start it with a sleep of a second: a virtual keyboard's first key reaches a client
//! only after the client has bound the seat's new `wl_keyboard`.
use std::os::fd::AsFd;
use std::time::{Duration, Instant};
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

#[derive(Default)]
struct State {
    seats: Vec<(wl_seat::WlSeat, String)>,
    manager: Option<ZwpVirtualKeyboardManagerV1>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(state: &mut Self, reg: &wl_registry::WlRegistry, ev: wl_registry::Event, _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        if let wl_registry::Event::Global { name, interface, version } = ev {
            match interface.as_str() {
                "wl_seat" => {
                    let seat = reg.bind::<wl_seat::WlSeat, _, _>(name, version.min(7), qh, ());
                    state.seats.push((seat, String::new()));
                }
                "zwp_virtual_keyboard_manager_v1" => {
                    state.manager = Some(reg.bind(name, 1, qh, ()));
                }
                _ => {}
            }
        }
    }
}
impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(state: &mut Self, seat: &wl_seat::WlSeat, ev: wl_seat::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let wl_seat::Event::Name { name } = ev {
            if let Some(e) = state.seats.iter_mut().find(|(s, _)| s == seat) {
                e.1 = name;
            }
        }
    }
}
impl Dispatch<ZwpVirtualKeyboardManagerV1, ()> for State {
    fn event(_: &mut Self, _: &ZwpVirtualKeyboardManagerV1, _: wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_manager_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
impl Dispatch<ZwpVirtualKeyboardV1, ()> for State {
    fn event(_: &mut Self, _: &ZwpVirtualKeyboardV1, _: wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (want, keymap_path, script) = (&args[1], &args[2], &args[3]);
    let conn = Connection::connect_to_env().expect("connect");
    let mut queue = conn.new_event_queue();
    let qh = queue.handle();
    conn.display().get_registry(&qh, ());
    let mut state = State::default();
    queue.roundtrip(&mut state).unwrap();
    queue.roundtrip(&mut state).unwrap();
    let names: Vec<_> = state.seats.iter().map(|s| s.1.clone()).collect();
    eprintln!("seats: {names:?}");
    let seat = state.seats.iter().find(|(_, n)| n == want).unwrap_or_else(|| panic!("no seat {want}")).0.clone();
    let manager = state.manager.clone().expect("no virtual keyboard manager");
    let kb = manager.create_virtual_keyboard(&seat, &qh, ());
    let mut text = std::fs::read(keymap_path).unwrap();
    text.push(0);
    let file = std::fs::File::open(keymap_path).unwrap();
    // the size includes the terminating NUL, which the file lacks: map a copy that has one
    let tmp = std::env::temp_dir().join(format!("vk-keymap-{}", std::process::id()));
    std::fs::write(&tmp, &text).unwrap();
    drop(file);
    let f = std::fs::File::open(&tmp).unwrap();
    kb.keymap(1, f.as_fd(), text.len() as u32);
    queue.roundtrip(&mut state).unwrap();
    let t0 = Instant::now();
    for step in script.split(';') {
        let parts: Vec<&str> = step.split_whitespace().collect();
        match parts.as_slice() {
            ["down", n] => kb.key(t0.elapsed().as_millis() as u32, n.parse().unwrap(), 1),
            ["up", n] => kb.key(t0.elapsed().as_millis() as u32, n.parse().unwrap(), 0),
            ["sleep", ms] => {
                queue.roundtrip(&mut state).unwrap();
                std::thread::sleep(Duration::from_millis(ms.parse().unwrap()));
            }
            [] => {}
            other => panic!("bad step {other:?}"),
        }
        conn.flush().unwrap();
    }
    queue.roundtrip(&mut state).unwrap();
    let _ = std::fs::remove_file(tmp);
}
