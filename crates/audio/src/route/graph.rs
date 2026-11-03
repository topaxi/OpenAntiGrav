//! The part of the PipeWire graph that decides what the default output is.
//!
//! [`Graph`] is a reducer: the adapter in `live` feeds it what the registry
//! reports (the default sink, the audio sinks, the audio devices and their
//! active routes) and asks [`Graph::route`] after each event. It touches no
//! PipeWire object, so the event sequences are unit-tested.
//!
//! A sink node does not carry its port type. The device that owns it does, in
//! its active `Route` params, and one device can own several sinks (a USB card
//! with speaker, headphone and S/PDIF sinks). A sink's `card.profile.device`
//! equals the `device` field of the route it plays through.

use std::collections::BTreeMap;

use super::{Route, RouteKind, names_headphones, names_speakers};

/// One active output route of an audio device (`SPA_PARAM_Route`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeviceRoute {
    /// The route's `device` field: which of the card's devices plays through it.
    pub device: i32,
    /// `[Out] Headphones`, `analog-output-headphones`, `headset-output`, ...
    pub name: String,
    /// The `port.type` entry of the route's info (`headphones`, `speaker`, ...).
    pub port_type: String,
}

/// An `Audio/Sink` node.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Sink {
    pub name: String,
    /// `device.id`: the registry id of the owning device.
    pub device_id: Option<u32>,
    /// `card.profile.device`: which of that device's outputs this sink is.
    pub card_device: Option<i32>,
    pub form_factor: String,
    pub bus: String,
}

/// An audio device and the output routes it is playing through, one per device.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Card {
    pub form_factor: String,
    pub bus: String,
    pub routes: Vec<DeviceRoute>,
}

/// The registry state that decides the default output.
#[derive(Debug, Default)]
pub(super) struct Graph {
    default_sink: Option<String>,
    sinks: BTreeMap<u32, Sink>,
    cards: BTreeMap<u32, Card>,
}

impl Graph {
    /// The `default` metadata's `default.audio.sink` value (`{"name":"..."}`),
    /// or `None` when the key was cleared.
    pub fn set_default_sink(&mut self, value: Option<&str>) {
        self.default_sink = value.and_then(|json| {
            serde_json::from_str::<serde_json::Value>(json)
                .ok()?
                .get("name")?
                .as_str()
                .map(str::to_owned)
        });
    }

    pub fn set_sink(&mut self, id: u32, sink: Sink) {
        self.sinks.insert(id, sink);
    }

    pub fn remove_sink(&mut self, id: u32) {
        self.sinks.remove(&id);
    }

    /// The device's own form factor and bus; keeps the routes it already had.
    pub fn set_card_props(&mut self, id: u32, form_factor: String, bus: String) {
        let card = self.cards.entry(id).or_default();
        card.form_factor = form_factor;
        card.bus = bus;
    }

    /// A route param arrived: the card's route for `route.device` is now this
    /// one. PipeWire reports one param per device of the card, so this replaces
    /// by `device` and leaves the others.
    pub fn set_card_route(&mut self, id: u32, route: DeviceRoute) {
        let routes = &mut self.cards.entry(id).or_default().routes;
        match routes.iter_mut().find(|r| r.device == route.device) {
            Some(slot) => *slot = route,
            None => routes.push(route),
        }
    }

    pub fn remove_card(&mut self, id: u32) {
        self.cards.remove(&id);
    }

    /// The default output now. [`Route::default`] when there is no default
    /// sink, or the default sink is not (yet) a node the registry reported.
    pub fn route(&self) -> Route {
        let Some(name) = self.default_sink.as_deref() else {
            return Route::default();
        };
        let Some(sink) = self.sinks.values().find(|s| s.name == name) else {
            return Route::default();
        };
        let card = sink.device_id.and_then(|id| self.cards.get(&id));
        let route = card.and_then(|card| {
            card.routes
                .iter()
                .find(|r| Some(r.device) == sink.card_device)
        });
        let device = Some(match route {
            Some(route) => format!("{name}|{}", route.name),
            None => name.to_owned(),
        });
        let form_factors = [
            Some(sink.form_factor.as_str()),
            card.map(|c| c.form_factor.as_str()),
        ];
        let port_types = route.map(|r| r.port_type.as_str());
        let words = || {
            port_types
                .into_iter()
                .chain(form_factors.into_iter().flatten())
        };
        let bluetooth = sink.bus == "bluetooth" || card.is_some_and(|c| c.bus == "bluetooth");
        let kind = if words().any(names_headphones) {
            RouteKind::Headphones
        } else if words().any(names_speakers) {
            RouteKind::Speakers
        } else if bluetooth {
            // A Bluetooth sink with no port type or form factor is
            // overwhelmingly a headset.
            RouteKind::Headphones
        } else {
            RouteKind::Unknown
        };
        Route { device, kind }
    }
}

/// Decides when a [`Route`] is worth sending. The registry announces its
/// objects one by one, so the graph passes through half-built answers (a sink
/// with no route yet reads as [`RouteKind::Unknown`]) before the real one. The
/// announcer says nothing until [`Self::SETTLE_ROUNDS`] server roundtrips have
/// completed, then sends the graph's answer and afterwards every change.
#[derive(Debug)]
pub(super) struct Announcer {
    rounds_left: u8,
    sent: Option<Route>,
}

impl Announcer {
    /// The first roundtrip delivers the registry's globals and the binds'
    /// requests; the second delivers the bound objects' `info` and `Route`
    /// replies.
    pub const SETTLE_ROUNDS: u8 = 2;

    pub fn new() -> Self {
        Self {
            rounds_left: Self::SETTLE_ROUNDS,
            sent: None,
        }
    }

    pub fn settled(&self) -> bool {
        self.rounds_left == 0
    }

    /// A roundtrip completed. Returns the route to send when that settles the
    /// startup.
    pub fn round_done(&mut self, graph: &Graph) -> Option<Route> {
        self.rounds_left = self.rounds_left.saturating_sub(1);
        self.publish(graph)
    }

    /// Gives up waiting (the roundtrip could not be requested).
    pub fn settle_now(&mut self, graph: &Graph) -> Option<Route> {
        self.rounds_left = 0;
        self.publish(graph)
    }

    /// The route to send now: nothing while settling, then only a change.
    pub fn publish(&mut self, graph: &Graph) -> Option<Route> {
        if !self.settled() {
            return None;
        }
        let route = graph.route();
        (self.sent.as_ref() != Some(&route)).then(|| {
            self.sent = Some(route.clone());
            route
        })
    }
}

#[cfg(test)]
mod tests;
