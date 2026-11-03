//! PipeWire as the source of the default output, event driven.
//!
//! One thread runs a PipeWire main loop. It watches the registry for the
//! `default` metadata (`default.audio.sink`), the audio sinks and the audio
//! devices, subscribes to each device's `Route` param, and feeds [`Graph`].
//! When the answer changes it goes down a channel that
//! [`RouteWatcher`](super::RouteWatcher) drains a frame at a time. No process
//! is spawned and nothing polls.
//!
//! A connection that cannot be made, or one that drops, ends the thread after
//! one [`Route::default`] (the rear cue); nothing is retried or logged beyond
//! one debug line.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc;

use libspa::param::ParamType;
use libspa::pod::deserialize::PodDeserializer;
use libspa::pod::{Pod, Value};
use pipewire::context::ContextRc;
use pipewire::device::Device;
use pipewire::main_loop::MainLoopRc;
use pipewire::metadata::Metadata;
use pipewire::node::Node;
use pipewire::types::ObjectType;

use super::Route;
use super::graph::{DeviceRoute, Graph, Sink};

/// `SPA_PARAM_ROUTE_*` keys and `SPA_DIRECTION_OUTPUT` (`spa/param/route.h`).
const ROUTE_DIRECTION: u32 = 2;
const ROUTE_DEVICE: u32 = 3;
const ROUTE_NAME: u32 = 4;
const ROUTE_INFO: u32 = 8;
const DIRECTION_OUTPUT: u32 = 1;

/// The running watcher thread's handle.
pub(super) struct Live {
    routes: mpsc::Receiver<Route>,
    quit: pipewire::channel::Sender<()>,
}

impl std::fmt::Debug for Live {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Live").finish_non_exhaustive()
    }
}

impl Live {
    /// Starts the thread. Never fails: a missing PipeWire is a first answer of
    /// [`Route::default`].
    pub fn start() -> Self {
        let (tx, routes) = mpsc::channel();
        let (quit, quit_rx) = pipewire::channel::channel::<()>();
        let spawned = std::thread::Builder::new()
            .name("audio-route-pipewire".into())
            .spawn({
                let tx = tx.clone();
                move || {
                    if let Err(why) = run(&tx, quit_rx) {
                        log::debug!("route: PipeWire unavailable: {why}");
                        let _ = tx.send(Route::default());
                    }
                }
            });
        if spawned.is_err() {
            let _ = tx.send(Route::default());
        }
        Self { routes, quit }
    }

    /// The newest route the thread has sent since the last call.
    pub fn latest(&self) -> Option<Route> {
        self.routes.try_iter().last()
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        let _ = self.quit.send(());
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Sink,
    Card,
    Metadata,
}

/// A bound proxy and its listener; dropping either stops the callbacks.
struct Bound {
    kind: Kind,
    _alive: Box<dyn std::any::Any>,
}

struct State {
    graph: RefCell<Graph>,
    sent: RefCell<Option<Route>>,
    bound: RefCell<HashMap<u32, Bound>>,
    tx: mpsc::Sender<Route>,
}

impl State {
    fn publish(&self) {
        let route = self.graph.borrow().route();
        if self.sent.borrow().as_ref() != Some(&route) {
            *self.sent.borrow_mut() = Some(route.clone());
            let _ = self.tx.send(route);
        }
    }
}

fn run(tx: &mpsc::Sender<Route>, quit_rx: pipewire::channel::Receiver<()>) -> anyhow::Result<()> {
    pipewire::init();
    let main_loop = MainLoopRc::new(None)?;
    let context = ContextRc::new(&main_loop, None)?;
    let core = context.connect_rc(None)?;
    let registry = core.get_registry_rc()?;
    let _quit = quit_rx.attach(main_loop.loop_(), {
        let main_loop = main_loop.clone();
        move |()| main_loop.quit()
    });
    let lost = Rc::new(RefCell::new(false));
    let _core_listener = core
        .add_listener_local()
        .error({
            let main_loop = main_loop.clone();
            let lost = Rc::clone(&lost);
            move |id, _seq, _res, message| {
                if id == pipewire::core::PW_ID_CORE {
                    log::debug!("route: PipeWire core error: {message}");
                    *lost.borrow_mut() = true;
                    main_loop.quit();
                }
            }
        })
        .register();

    let state = Rc::new(State {
        graph: RefCell::new(Graph::default()),
        sent: RefCell::new(None),
        bound: RefCell::new(HashMap::new()),
        tx: tx.clone(),
    });
    let _registry_listener = registry
        .add_listener_local()
        .global({
            let registry = registry.clone();
            let state = Rc::clone(&state);
            move |global| {
                let Some(props) = global.props else { return };
                let bound = match global.type_ {
                    ObjectType::Node if props.get("media.class") == Some("Audio/Sink") => {
                        registry.bind::<Node, _>(global).ok().map(|node| {
                            let listener = node
                                .add_listener_local()
                                .info({
                                    let state = Rc::clone(&state);
                                    let id = global.id;
                                    move |info| {
                                        if let Some(props) = info.props() {
                                            let sink = sink_of(props);
                                            state.graph.borrow_mut().set_sink(id, sink);
                                            state.publish();
                                        }
                                    }
                                })
                                .register();
                            Bound {
                                kind: Kind::Sink,
                                _alive: Box::new((node, listener)),
                            }
                        })
                    }
                    ObjectType::Device if props.get("media.class") == Some("Audio/Device") => {
                        registry.bind::<Device, _>(global).ok().map(|device| {
                            let id = global.id;
                            let listener = device
                                .add_listener_local()
                                .info({
                                    let state = Rc::clone(&state);
                                    move |info| {
                                        if let Some(props) = info.props() {
                                            let get =
                                                |k| props.get(k).unwrap_or_default().to_owned();
                                            state.graph.borrow_mut().set_card_props(
                                                id,
                                                get("device.form_factor"),
                                                get("device.bus"),
                                            );
                                            state.publish();
                                        }
                                    }
                                })
                                .param({
                                    let state = Rc::clone(&state);
                                    move |_seq, _ty, _index, _next, pod| {
                                        if let Some(route) = pod.and_then(route_of) {
                                            state.graph.borrow_mut().set_card_route(id, route);
                                            state.publish();
                                        }
                                    }
                                })
                                .register();
                            device.subscribe_params(&[ParamType::Route]);
                            Bound {
                                kind: Kind::Card,
                                _alive: Box::new((device, listener)),
                            }
                        })
                    }
                    ObjectType::Metadata if props.get("metadata.name") == Some("default") => {
                        registry.bind::<Metadata, _>(global).ok().map(|metadata| {
                            let listener = metadata
                                .add_listener_local()
                                .property({
                                    let state = Rc::clone(&state);
                                    move |_subject, key, _type, value| {
                                        if key == Some("default.audio.sink") {
                                            state.graph.borrow_mut().set_default_sink(value);
                                            state.publish();
                                        }
                                        0
                                    }
                                })
                                .register();
                            Bound {
                                kind: Kind::Metadata,
                                _alive: Box::new((metadata, listener)),
                            }
                        })
                    }
                    _ => None,
                };
                if let Some(bound) = bound {
                    state.bound.borrow_mut().insert(global.id, bound);
                }
            }
        })
        .global_remove({
            let state = Rc::clone(&state);
            move |id| {
                if let Some(gone) = state.bound.borrow_mut().remove(&id) {
                    match gone.kind {
                        Kind::Sink => state.graph.borrow_mut().remove_sink(id),
                        Kind::Card => state.graph.borrow_mut().remove_card(id),
                        Kind::Metadata => state.graph.borrow_mut().set_default_sink(None),
                    }
                    state.publish();
                }
            }
        })
        .register();

    main_loop.run();
    state.bound.borrow_mut().clear();
    if *lost.borrow() {
        anyhow::bail!("connection lost");
    }
    Ok(())
}

fn sink_of(props: &pipewire::spa::utils::dict::DictRef) -> Sink {
    let get = |key| props.get(key).unwrap_or_default().to_owned();
    Sink {
        name: get("node.name"),
        device_id: props.get("device.id").and_then(|v| v.parse().ok()),
        card_device: props
            .get("card.profile.device")
            .and_then(|v| v.parse().ok()),
        form_factor: get("device.form_factor"),
        bus: get("device.bus"),
    }
}

/// An output `Route` param as a [`DeviceRoute`]; `None` for an input route or
/// anything that is not one.
fn route_of(pod: &Pod) -> Option<DeviceRoute> {
    let (_, Value::Object(object)) = PodDeserializer::deserialize_any_from(pod.as_bytes()).ok()?
    else {
        return None;
    };
    let mut direction = None;
    let (mut device, mut name, mut port_type) = (None, String::new(), String::new());
    for property in object.properties {
        match (property.key, property.value) {
            (ROUTE_DIRECTION, Value::Id(id)) => direction = Some(id.0),
            (ROUTE_DEVICE, Value::Int(n)) => device = Some(n),
            (ROUTE_NAME, Value::String(s)) => name = s,
            (ROUTE_INFO, Value::Struct(fields)) => {
                let mut strings = fields.into_iter().filter_map(|f| match f {
                    Value::String(s) => Some(s),
                    _ => None,
                });
                while let (Some(key), Some(value)) = (strings.next(), strings.next()) {
                    if key == "port.type" {
                        port_type = value;
                    }
                }
            }
            _ => {}
        }
    }
    (direction == Some(DIRECTION_OUTPUT)).then_some(DeviceRoute {
        device: device?,
        name,
        port_type,
    })
}
