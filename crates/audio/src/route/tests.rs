use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::*;
use crate::spatial::{SpatialChoice, SpatialMode};

fn sinks(name: &str, port: &str, port_type: &str, form: Option<&str>, bus: &str) -> String {
    let form = form.map_or(String::new(), |f| {
        format!(r#""device.form_factor": "{f}","#)
    });
    format!(
        r#"[{{"name": "other", "active_port": "x", "ports": []}},
           {{"name": "{name}", "active_port": "{port}",
             "ports": [{{"name": "{port}", "type": "{port_type}"}}],
             "properties": {{{form} "device.bus": "{bus}"}}}}]"#
    )
}

#[test]
fn headphones_resolve_to_the_hrtf_and_everything_else_to_the_rear_cue() {
    let headphone_jack = from_pactl(
        "a",
        &sinks("a", "[Out] Headphones", "Headphones", None, "usb"),
    );
    let speakers = from_pactl("a", &sinks("a", "[Out] Speaker", "Speaker", None, "usb"));
    let unknown = from_pactl("a", &sinks("a", "analog-output", "Analog", None, "usb"));
    assert_eq!(headphone_jack.kind, RouteKind::Headphones);
    assert_eq!(speakers.kind, RouteKind::Speakers);
    assert_eq!(unknown.kind, RouteKind::Unknown);
    let auto = |route: &Route| SpatialChoice::Auto.resolve(route.kind);
    assert_eq!(auto(&headphone_jack), SpatialMode::Headphones);
    assert_eq!(auto(&speakers), SpatialMode::RearCue);
    assert_eq!(auto(&unknown), SpatialMode::RearCue);
}

#[test]
fn a_bluetooth_sink_and_a_form_factor_are_read() {
    let bt = from_pactl(
        "a",
        &sinks("a", "headset-output", "Bluetooth", None, "bluetooth"),
    );
    assert_eq!(bt.kind, RouteKind::Headphones);
    let bt_speaker = from_pactl(
        "a",
        &sinks("a", "p", "Bluetooth", Some("speaker"), "bluetooth"),
    );
    assert_eq!(bt_speaker.kind, RouteKind::Speakers);
    let by_form = from_pactl("a", &sinks("a", "p", "Analog", Some("headphone"), "pci"));
    assert_eq!(by_form.kind, RouteKind::Headphones);
}

#[test]
fn the_id_names_the_sink_and_its_port() {
    let jack = from_pactl(
        "deck",
        &sinks("deck", "headphones", "Headphones", None, "pci"),
    );
    let spk = from_pactl("deck", &sinks("deck", "speaker", "Speaker", None, "pci"));
    assert_ne!(jack.device, spk.device);
    assert_eq!(jack.device.as_deref(), Some("deck|headphones"));
}

#[test]
fn an_unreadable_answer_is_unknown() {
    assert_eq!(from_pactl("a", "not json"), Route::default());
    assert_eq!(
        from_pactl("missing", &sinks("a", "p", "Analog", None, "pci")),
        Route::default()
    );
}

#[test]
fn the_watcher_reports_a_change_once_and_never_waits() {
    let now = Arc::new(Mutex::new(Route {
        device: Some("a".into()),
        kind: RouteKind::Speakers,
    }));
    let seen = Arc::clone(&now);
    let mut watcher =
        RouteWatcher::with_probe(Arc::new(move || seen.lock().expect("lock").clone()), 1);
    let wait = |watcher: &mut RouteWatcher| {
        let start = Instant::now();
        loop {
            if let Some(route) = watcher.poll() {
                return Some(route);
            }
            if start.elapsed() > Duration::from_secs(5) {
                return None;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    };
    assert_eq!(wait(&mut watcher).expect("first").kind, RouteKind::Speakers);
    *now.lock().expect("lock") = Route {
        device: Some("b".into()),
        kind: RouteKind::Headphones,
    };
    let changed = wait(&mut watcher).expect("change");
    assert_eq!(changed.kind, RouteKind::Headphones);
    assert_eq!(changed.device.as_deref(), Some("b"));
    // Unchanged answers report nothing.
    for _ in 0..50 {
        assert!(watcher.poll().is_none());
        std::thread::sleep(Duration::from_millis(1));
    }
}
