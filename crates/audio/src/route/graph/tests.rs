use super::*;

const DEFAULT: &str = r#"{"name":"speaker_sink"}"#;

fn route(device: i32, name: &str, port_type: &str) -> DeviceRoute {
    DeviceRoute {
        device,
        name: name.into(),
        port_type: port_type.into(),
    }
}

fn set_routes(graph: &mut Graph, id: u32, routes: Vec<DeviceRoute>) {
    for route in routes {
        graph.set_card_route(id, route);
    }
}

fn sink(name: &str, device_id: u32, card_device: i32) -> Sink {
    Sink {
        name: name.into(),
        device_id: Some(device_id),
        card_device: Some(card_device),
        ..Sink::default()
    }
}

#[test]
fn nothing_known_is_unknown() {
    assert_eq!(Graph::default().route(), Route::default());
    let mut graph = Graph::default();
    graph.set_default_sink(Some(DEFAULT));
    assert_eq!(graph.route().kind, RouteKind::Unknown);
}

#[test]
fn the_port_type_of_the_sinks_own_route_decides() {
    let mut graph = Graph::default();
    graph.set_sink(10, sink("speaker_sink", 1, 11));
    graph.set_sink(11, sink("headphone_sink", 1, 12));
    set_routes(
        &mut graph,
        1,
        vec![
            route(10, "[Out] SPDIF", "spdif"),
            route(11, "[Out] Speaker", "speaker"),
            route(12, "[Out] Headphones", "headphones"),
        ],
    );
    graph.set_default_sink(Some(DEFAULT));
    assert_eq!(graph.route().kind, RouteKind::Speakers);
    assert_eq!(
        graph.route().device.as_deref(),
        Some("speaker_sink|[Out] Speaker")
    );
    graph.set_default_sink(Some(r#"{"name":"headphone_sink"}"#));
    assert_eq!(graph.route().kind, RouteKind::Headphones);
}

#[test]
fn plugging_a_jack_into_the_same_sink_changes_the_route() {
    let mut graph = Graph::default();
    graph.set_sink(10, sink("speaker_sink", 1, 0));
    graph.set_default_sink(Some(DEFAULT));
    set_routes(
        &mut graph,
        1,
        vec![route(0, "analog-output-speaker", "speaker")],
    );
    let before = graph.route();
    set_routes(
        &mut graph,
        1,
        vec![route(0, "analog-output-headphones", "headphones")],
    );
    let after = graph.route();
    assert_eq!(before.kind, RouteKind::Speakers);
    assert_eq!(after.kind, RouteKind::Headphones);
    assert_ne!(before.device, after.device);
}

#[test]
fn removing_the_default_sink_or_clearing_the_key_is_unknown() {
    let mut graph = Graph::default();
    graph.set_sink(10, sink("speaker_sink", 1, 0));
    set_routes(&mut graph, 1, vec![route(0, "r", "headphones")]);
    graph.set_default_sink(Some(DEFAULT));
    assert_eq!(graph.route().kind, RouteKind::Headphones);
    graph.remove_sink(10);
    assert_eq!(graph.route(), Route::default());
    graph.set_sink(10, sink("speaker_sink", 1, 0));
    graph.set_default_sink(None);
    assert_eq!(graph.route(), Route::default());
    graph.set_default_sink(Some("not json"));
    assert_eq!(graph.route(), Route::default());
}

#[test]
fn a_route_for_another_device_of_the_card_is_not_used() {
    let mut graph = Graph::default();
    graph.set_sink(10, sink("speaker_sink", 1, 5));
    set_routes(&mut graph, 1, vec![route(6, "hdmi-output-0", "hdmi")]);
    graph.set_default_sink(Some(DEFAULT));
    assert_eq!(graph.route().kind, RouteKind::Unknown);
}

#[test]
fn bluetooth_without_a_route_is_a_headset_unless_it_says_speaker() {
    let mut graph = Graph::default();
    let mut bt = Sink {
        name: "speaker_sink".into(),
        bus: "bluetooth".into(),
        ..Sink::default()
    };
    graph.set_sink(10, bt.clone());
    graph.set_default_sink(Some(DEFAULT));
    assert_eq!(graph.route().kind, RouteKind::Headphones);
    bt.form_factor = "speaker".into();
    graph.set_sink(10, bt);
    assert_eq!(graph.route().kind, RouteKind::Speakers);
}

#[test]
fn the_cards_form_factor_is_read_when_the_sink_has_none() {
    let mut graph = Graph::default();
    graph.set_sink(10, sink("speaker_sink", 1, 0));
    graph.set_card_props(1, "headset".into(), "usb".into());
    graph.set_default_sink(Some(DEFAULT));
    assert_eq!(graph.route().kind, RouteKind::Headphones);
    graph.remove_card(1);
    assert_eq!(graph.route().kind, RouteKind::Unknown);
}

fn headphone_graph() -> Graph {
    let mut graph = Graph::default();
    graph.set_default_sink(Some(DEFAULT));
    graph.set_sink(10, sink("speaker_sink", 1, 11));
    graph.set_card_route(1, route(11, "headphones-out", "headphones"));
    graph
}

#[test]
fn startup_sends_the_settled_route_not_the_transient_unknown() {
    let mut announcer = Announcer::new();
    let mut graph = Graph::default();
    let mut sent = Vec::new();
    // The sink arrives before its card's route: the raw answer is Unknown.
    graph.set_default_sink(Some(DEFAULT));
    graph.set_sink(10, sink("speaker_sink", 1, 11));
    assert_eq!(graph.route().kind, RouteKind::Unknown);
    sent.extend(announcer.publish(&graph));
    sent.extend(announcer.round_done(&graph));
    graph.set_card_route(1, route(11, "headphones-out", "headphones"));
    sent.extend(announcer.publish(&graph));
    sent.extend(announcer.round_done(&graph));
    assert_eq!(
        sent.len(),
        1,
        "one announcement, no Unknown first: {sent:?}"
    );
    assert_eq!(sent[0].kind, RouteKind::Headphones);
}

#[test]
fn no_pipewire_objects_still_settles_on_unknown() {
    let mut announcer = Announcer::new();
    let graph = Graph::default();
    assert!(announcer.round_done(&graph).is_none());
    assert_eq!(announcer.round_done(&graph), Some(Route::default()));
}

#[test]
fn a_later_change_still_switches_at_once() {
    let mut announcer = Announcer::new();
    let mut graph = headphone_graph();
    announcer.round_done(&graph);
    assert_eq!(
        announcer.round_done(&graph).map(|r| r.kind),
        Some(RouteKind::Headphones)
    );
    assert!(announcer.publish(&graph).is_none());
    graph.set_card_route(1, route(11, "speaker-out", "speaker"));
    assert_eq!(
        announcer.publish(&graph).map(|r| r.kind),
        Some(RouteKind::Speakers)
    );
}
