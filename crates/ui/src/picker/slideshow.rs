//! The picture behind `Track Creation`'s hexagonal window: a slideshow of
//! the circuit's own stills, authored per circuit in the circuit's own
//! `screen.xml`.
//!
//! **Where it comes from.** `TrackSelection_ApplySelection` hands the newly
//! selected `PI_Track` definition to `TrackDefinition_EnterScreenState`,
//! which - the first time - loads `<location>\screen.xml` into the
//! definition's own state machine (`screen_zone.xml` in Zone mode on a
//! circuit that is `availableInZone`, `MP_Screen.xml` in split screen) and
//! then transitions to the state named `Info` (`Zone` in Zone mode). See
//! `docs/ghidra/functions/psp-pulse-usa/race-box-screens.md` and
//! `docs/ui/selection-screens.md`.
//!
//! **What the file is.** A chain of named `Screen`s linked by
//! `<Redirect delay="2"><Default goto="..."/></Redirect>`, each nested one
//! level deeper inside an anonymous `Screen` that carries one `Image` - so
//! the deeper the state, the more stills are on screen, stacked with a
//! shadow card under each one after the first:
//!
//! ```text
//! Info -> Info2 -> Info3 -> Info4 -> Info3 Close -> Info2 Close -> Info
//!  1        2        3        4         3              2            1 still
//! ```
//!
//! two seconds a step on the PSP. **The hexagon is the still's own shape**:
//! `image_0N.mip` is a 256x128 render cropped to a hexagon with its white
//! border baked in, and `track_sel_shadow.mip` the dark hexagon drawn under
//! each card but the first. Nothing here is a frame, a mask or a flythrough:
//! the window shows stills, and the "camera moving down the corridor" a
//! two-frames-a-second-apart capture read into it was two different stills.
//!
//! The `Src` values are authored as `%s\FE\image_01.mip`, with the `%s` the
//! circuit's own `location` - the same formatter `%s\FE\%s.vex` goes
//! through for the outline. Substituted at read time here.

use crate::frontend::{Draw, Placed};
use crate::screen::{Image, Screens, argb_to_rgba, parse};

/// One state of the chain: what is on screen while it holds, and where the
/// chain goes next.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    /// The `Screen`'s `name`.
    pub name: String,
    /// Every still on screen in this state, ancestors first - which is the
    /// paint order, the deepest card on top.
    pub images: Vec<Image>,
    /// Seconds this state holds before its redirect fires; zero when the
    /// state has no timed redirect, which makes it terminal.
    pub delay: f32,
    /// The state the redirect goes to.
    pub goto: Option<String>,
}

/// The `<Mode3D><Model>` the same file places: the outline ribbon's own
/// authored pose. **Read and carried, not yet drawn from** - the outline is
/// framed by `oag_game::preview::orbit_for`'s capture-read orbit today, and
/// turning these numbers into that camera needs the `Mode3D` projection
/// (`nearZ`/`farZ`, an `OriginX`/`OriginY` in an unmeasured space) read
/// first. Kept so the next reader starts from the disc's numbers rather
/// than from the capture.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    /// The `Src`, `%s` substituted - `<location>\FE\forward.vex`.
    pub src: String,
    /// `x`, `y`, `z`.
    pub position: [f32; 3],
    /// `RotX`, `RotY`, radians as authored.
    pub rotation: [f32; 2],
    /// The `Mode3D`'s `OriginX`/`OriginY`.
    pub origin: [f32; 2],
    /// The `Mode3D`'s `nearZ`/`farZ`.
    pub depth: [f32; 2],
}

/// A circuit's `screen.xml`, read as the state machine it is.
#[derive(Debug, Clone)]
pub struct Slideshow {
    states: Vec<State>,
    /// The chain from the start state, in order, until it revisits a state
    /// or runs out of redirects.
    path: Vec<usize>,
    /// Where in `path` the revisit lands, when the chain is a cycle.
    cycle_from: Option<usize>,
    /// The file's `Mode3D` model, when it authors one.
    pub model: Option<Model>,
}

impl Slideshow {
    /// Reads `xml` (already expanded) for the chain starting at `start` -
    /// `Info`, or `Zone` in Zone mode - with `location` substituted for
    /// every `%s`. `None` when the file authors no state of that name.
    ///
    /// `globals` are the front end's own, because a per-entity `screen.xml`
    /// declares none and still refers to them: Pure's stills each carry
    /// `Color="FEGlobals->ShipColor"` / `TrackColor`, and without the table
    /// those resolve to nothing and the still is tinted white - which on
    /// Pure's white front end is not a wrong colour but an invisible
    /// picture. Pulse's stills name no colour and are unaffected.
    #[must_use]
    pub fn read(xml: &str, location: &str, start: &str, globals: &[(&str, &str)]) -> Option<Self> {
        let screens = Screens::from_xml_with_fallback_globals(xml, globals);
        let root = parse(xml);
        let mut states = Vec::new();
        let mut model = None;
        for child in &root.children {
            walk(child, &[], &screens, location, &mut states, &mut model);
        }
        let start = states.iter().position(|state| state.name == start)?;
        let (path, cycle_from) = chain(&states, start);
        Some(Self {
            states,
            path,
            cycle_from,
            model,
        })
    }

    /// Every state the file authors, in document order.
    #[must_use]
    pub fn states(&self) -> &[State] {
        &self.states
    }

    /// Every still any state on the chain shows, each once, in first-use
    /// order - what a caller has to load before [`Self::draws`] can draw.
    #[must_use]
    pub fn sources(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for &index in &self.path {
            for image in &self.states[index].images {
                if !out.contains(&image.src) {
                    out.push(image.src.clone());
                }
            }
        }
        out
    }

    /// The state on screen `seconds` after entering the start state.
    ///
    /// A chain that closes on itself repeats from where it closed; one that
    /// ends stays on its last state. A state with no delay is terminal too,
    /// rather than a zero-length step that would spin.
    #[must_use]
    pub fn at(&self, seconds: f32) -> &State {
        let mut t = seconds.max(0.0);
        if let Some(from) = self.cycle_from {
            let prefix: f32 = self.path[..from]
                .iter()
                .map(|&index| self.states[index].delay)
                .sum();
            let cycle: f32 = self.path[from..]
                .iter()
                .map(|&index| self.states[index].delay)
                .sum();
            if t >= prefix && cycle > 0.0 {
                t = prefix + (t - prefix) % cycle;
            }
        }
        let mut current = self.path[0];
        for &index in &self.path {
            current = index;
            let delay = self.states[index].delay;
            if delay <= 0.0 || t < delay {
                return &self.states[index];
            }
            t -= delay;
        }
        &self.states[current]
    }

    /// The stills on screen `seconds` in, as sprites out of the sheet
    /// `sprites` answers for. A still the sheet does not hold is left out,
    /// the same rule every other image on the screen follows.
    #[must_use]
    pub fn draws(&self, seconds: f32, sprites: &dyn Fn(&str) -> Option<Placed>) -> Vec<Draw> {
        self.at(seconds)
            .images
            .iter()
            .filter_map(|image| {
                let placed = sprites(&image.src)?;
                Some(Draw::Sprite {
                    rect: [
                        image.x,
                        image.y,
                        image.width.unwrap_or(placed.width as f32),
                        image.height.unwrap_or(placed.height as f32),
                    ],
                    uv: super::image_uv(image, placed),
                    color: argb_to_rgba(image.color),
                })
            })
            .collect()
    }
}

/// Follows the redirects from `start`: the states visited in order, and
/// where the walk closed on itself if it did.
fn chain(states: &[State], start: usize) -> (Vec<usize>, Option<usize>) {
    let mut path = vec![start];
    let mut current = start;
    loop {
        let Some(goto) = states[current].goto.as_deref() else {
            return (path, None);
        };
        let Some(next) = states.iter().position(|state| state.name == goto) else {
            return (path, None);
        };
        if let Some(at) = path.iter().position(|&index| index == next) {
            return (path, Some(at));
        }
        path.push(next);
        current = next;
    }
}

/// Collects the states under `node`, each with the stills of every screen
/// enclosing it. A named `Screen`'s own stills are on screen with it too.
fn walk(
    node: &crate::screen::Node,
    inherited: &[Image],
    screens: &Screens,
    location: &str,
    out: &mut Vec<State>,
    model: &mut Option<Model>,
) {
    if !node.name.eq_ignore_ascii_case("Screen") {
        // A container (`Viewport`, `Mode3D`) between two screens: walk
        // through it, the way `Screens::collect` does.
        if node.name.eq_ignore_ascii_case("Mode3D") && model.is_none() {
            *model = model_from(node, location);
        }
        for child in &node.children {
            walk(child, inherited, screens, location, out, model);
        }
        return;
    }
    let mut images = inherited.to_vec();
    collect_images(node, screens, location, &mut images);
    if let Some(name) = node.attr("name") {
        let redirect = node.children_named("Redirect").next();
        out.push(State {
            name: name.to_string(),
            images: images.clone(),
            delay: redirect
                .and_then(|r| r.value("delay"))
                .and_then(|d| d.trim().parse().ok())
                .unwrap_or(0.0),
            goto: redirect.and_then(|r| {
                r.children_named("Default")
                    .find_map(|d| d.attr("goto"))
                    .map(str::to_string)
            }),
        });
    }
    for child in &node.children {
        walk(child, &images, screens, location, out, model);
    }
}

/// The `Image` widgets directly on `node`, through any container that is
/// not itself a `Screen` - a nested screen's stills are its own.
fn collect_images(
    node: &crate::screen::Node,
    screens: &Screens,
    location: &str,
    out: &mut Vec<Image>,
) {
    for child in &node.children {
        let name = child.name.to_ascii_lowercase();
        match name.as_str() {
            "screen" | "redirect" | "values" | "mode3d" => {}
            "image" => {
                if let Some(src) = child.value("src") {
                    let mut image = screens.image_from_node(child, src, (0.0, 0.0));
                    image.src = substitute(&image.src, location);
                    out.push(image);
                }
            }
            _ => collect_images(child, screens, location, out),
        }
    }
}

fn model_from(node: &crate::screen::Node, location: &str) -> Option<Model> {
    let number = |n: &crate::screen::Node, key: &str| -> f32 {
        n.value(key)
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0.0)
    };
    let model = node.children_named("Model").next()?;
    let src = model.value("src")?;
    Some(Model {
        src: substitute(src, location),
        position: [number(model, "x"), number(model, "y"), number(model, "z")],
        rotation: [number(model, "RotX"), number(model, "RotY")],
        origin: [number(node, "OriginX"), number(node, "OriginY")],
        depth: [number(node, "nearZ"), number(node, "farZ")],
    })
}

/// `%s` is the circuit's `location`, the way the executable's own
/// `sprintf` fills it.
fn substitute(src: &str, location: &str) -> String {
    src.replace("%s", location)
}

#[cfg(test)]
mod tests;
