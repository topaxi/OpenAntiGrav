//! The wingtip contrails: the two white ribbons a craft lays from its `wingtip` locators while a
//! speed pad (or a Turbo) acts, Wipeout HD's `ShipWingtipVortices` node and `VortexLine` element
//! (`docs/ghidra/functions/ps3-hdfury-eu/wingtip-vortices.md`, `docs/rendering/hd-wingtip-contrails.md`).
//!
//! Render-side state like [`crate::missile_smoke`]'s: nothing here reaches
//! [`oag_gameplay::World`] or a state hash. The law is [`oag_fx::fat_line`]'s `Vortex`; this is
//! the trigger, the per-locator timers, the pool and the assets. A title draws it when
//! [`oag_title::Title::wingtip`] is set (HD, and 2048 and Omega by lineage), or on any title
//! when the player's `graphics.wingtip_contrails` lends another source's.

use oag_core::math::{Mat4, Vec3};
use oag_fx::exhaust::FlareTexture;
use oag_fx::fat_line::{Ribbon, Spec, Vortex};
use oag_gameplay::MAX_SHIPS;
use oag_mesh::mesh::GpuVertex;
use oag_physics::damage::CraftState;
use oag_physics::hover::TARGET_GLOBAL_SCALE;
use oag_title::wingtip::Wingtip;

use crate::WingtipContrails;

use crate::Race;

/// What the contrails draw with: the ribbon's texture and the title's config, which sizes the
/// scene's buffer.
#[derive(Debug, Clone)]
pub struct WingtipAssets {
    pub texture: FlareTexture,
    pub config: Wingtip,
}

/// Each slot's `[left, right]` `wingtip` locators in its hull's model space, with the config
/// they are laid by. `Some` only where the texture loaded and a craft carries a locator.
#[derive(Debug, Clone, Copy)]
pub struct WingtipDraw {
    pub config: Wingtip,
    pub anchors: [[Option<Mat4>; 2]; MAX_SHIPS],
}

/// Reads the contrails' config, texture and each craft's locators, reporting what it found or why
/// nothing draws.
///
/// A title that carries the class ([`oag_title::Title::wingtip`]) reads its own texture and its
/// hulls' `wingtip` locators. Any other title (Pulse and Pure) draws them only with the player's
/// `graphics.wingtip_contrails`, lent by a source that carries the class ([`lend`]).
pub(crate) fn load(
    archives: &mut oag_assets::Archives,
    craft_title: &oag_title::Title,
    options: &crate::Options,
    liveries: &[oag_livery::Livery],
    report: &mut Vec<String>,
) -> Option<(WingtipAssets, WingtipDraw)> {
    let (config, texture, anchors) = match craft_title.wingtip {
        Some(config) => {
            let texture = texture(archives, config, report)?;
            (*config, texture, own_anchors(liveries))
        }
        None => {
            let (config, texture) = lend(options, craft_title, report)?;
            (config, texture, lent_anchors(liveries))
        }
    };
    let with = anchors
        .iter()
        .filter(|pair| pair.iter().any(Option::is_some))
        .count();
    if with == 0 {
        report.push(format!(
            "wingtip contrails: {} {}x{}, but no craft carries a wingtip locator - nothing draws",
            config.texture, texture.width, texture.height
        ));
        return None;
    }
    report.push(format!(
        "wingtip contrails: {} {}x{}, {with} of {} craft carry wingtip locators",
        config.texture,
        texture.width,
        texture.height,
        liveries.len().min(MAX_SHIPS)
    ));
    Some((
        WingtipAssets { texture, config },
        WingtipDraw { config, anchors },
    ))
}

fn texture(
    archives: &mut oag_assets::Archives,
    config: &Wingtip,
    report: &mut Vec<String>,
) -> Option<FlareTexture> {
    let platform = archives.layout.platform;
    let decoded = archives
        .read_name(config.texture)
        .ok()
        .and_then(|blob| crate::load::magstrip_wake::decode(&blob, platform));
    if decoded.is_none() {
        report.push(format!(
            "wingtip contrails: {} missing or unreadable - nothing draws",
            config.texture
        ));
    }
    decoded
}

fn own_anchors(liveries: &[oag_livery::Livery]) -> [[Option<Mat4>; 2]; MAX_SHIPS] {
    let mut anchors = [[None; 2]; MAX_SHIPS];
    for (slot, livery) in liveries.iter().enumerate().take(MAX_SHIPS) {
        anchors[slot] = livery.wingtip;
    }
    anchors
}

/// A title with no `wingtip` locators of its own (Pulse, Pure) lays the contrails from the
/// wing-tip `Ship Collision Fx` locators: on HD the two coincide on every hull checked (Feisar
/// and Zone `ship_collision_fx2` is the `wingtip` point), and Pulse's and Pure's hulls author
/// the same lateral pair at the same places (Assegai: Pulse `(2.69, -1.11, -2.19)` against HD's
/// `wingtip_left` `(2.78, -1.40, -2.02)`).
///
/// **Chosen, not measured**: on each side the locator of greatest `|x|`, the rearmost of those
/// within `0.1` of it, as a pure translation. Neither title authors a wing-tip locator, so no
/// hull's pick is the original's.
fn lent_anchors(liveries: &[oag_livery::Livery]) -> [[Option<Mat4>; 2]; MAX_SHIPS] {
    let mut anchors = [[None; 2]; MAX_SHIPS];
    for (slot, livery) in liveries.iter().enumerate().take(MAX_SHIPS) {
        anchors[slot] = [1.0_f32, -1.0].map(|side| wing_tip(&livery.collision_fx, side));
    }
    anchors
}

fn wing_tip(anchors: &[oag_livery::SparkAnchor], side: f32) -> Option<Mat4> {
    let on_side = || anchors.iter().filter(|a| a.position.x * side > 0.5);
    let widest = on_side()
        .map(|a| a.position.x.abs())
        .fold(0.0_f32, f32::max);
    on_side()
        .filter(|a| a.position.x.abs() > widest - 0.1)
        .min_by(|a, b| a.position.z.total_cmp(&b.position.z))
        .map(|a| Mat4::from_translation(a.position))
}

/// The contrails lent to a title that has none: the config from `lender`'s title, the texture from
/// its archives.
fn lend(
    options: &crate::Options,
    craft_title: &oag_title::Title,
    report: &mut Vec<String>,
) -> Option<(Wingtip, FlareTexture)> {
    let WingtipContrails::On { lender } = &options.wingtip_contrails else {
        return None;
    };
    let Some(lender) = lender else {
        report.push(format!(
            "wingtip contrails: switched on, but no source that carries them (Wipeout HD, 2048 or \
             Omega) is mounted, so {} lays none",
            craft_title.name
        ));
        return None;
    };
    let opened = match oag_source::title::open_source(lender, Vec::new(), Vec::new()) {
        Ok(opened) => opened,
        Err(why) => {
            report.push(format!(
                "wingtip contrails: {lender} would not open ({why}), so none are laid"
            ));
            return None;
        }
    };
    let (mut archives, from) = (opened.archives, opened.title);
    let Some(config) = from.wingtip else {
        report.push(format!(
            "wingtip contrails: {} carries none, so {} lays none",
            from.name, craft_title.name
        ));
        return None;
    };
    let texture = texture(&mut archives, config, report)?;
    report.push(format!(
        "wingtip contrails: {} read off {}, laid from the wing-tip collision locators; an \
         enhancement of this project's, {} authors none",
        config.texture, from.name, craft_title.name
    ));
    Some((*config, texture))
}

/// One locator's node: its timer, and the pool element it holds while the timer runs.
#[derive(Debug, Clone, Copy, Default)]
struct Node {
    timer: f32,
    element: Option<usize>,
}

/// The pool, and every craft's two nodes.
#[derive(Debug, Clone)]
pub(crate) struct Contrails {
    config: Wingtip,
    anchors: [[Option<Mat4>; 2]; MAX_SHIPS],
    nodes: [[Node; 2]; MAX_SHIPS],
    pool: Vec<Option<Ribbon>>,
}

/// The element's config for [`oag_fx::fat_line`].
fn spec(config: &Wingtip) -> Spec {
    let line = config.line;
    Spec {
        width: line.width,
        lifetime: line.lifetime,
        u_per_unit: line.u_per_unit,
        chains: line.chains as usize,
        capacity: line.capacity as usize,
        phase_step: line.phase_step,
        radius: line.radius,
        ramp: line.ramp,
    }
}

impl Contrails {
    pub(crate) fn new(draw: WingtipDraw) -> Self {
        Self {
            config: draw.config,
            anchors: draw.anchors,
            nodes: [[Node::default(); 2]; MAX_SHIPS],
            pool: vec![None; draw.config.pool],
        }
    }

    /// Elements alive, held or ageing out.
    #[must_use]
    pub(crate) fn live(&self) -> usize {
        self.pool.iter().flatten().count()
    }

    fn acquire(&mut self) -> Option<usize> {
        let free = self.pool.iter().position(Option::is_none)?;
        let vortex = Vortex {
            drift: self.config.drift,
            colour: self.config.colour,
        };
        self.pool[free] = Some(Ribbon::new_vortex(spec(&self.config), vortex));
        Some(free)
    }
}

/// What one craft's nodes read each tick.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Craft {
    /// The speed pad's force time is above zero, or a Turbo runs.
    pub triggered: bool,
    /// Not in the destroyed sequence (the original's states 4 to 6).
    pub flying: bool,
    /// `|dot(velocity, forward)|`.
    pub speed: f32,
}

impl Contrails {
    /// `ShipWingtipVortices_Update` for one locator: refill the timer while the trigger holds,
    /// and while it is above zero take an element, push this tick's nodes at `tip` and run the
    /// timer down; else let the element go (its nodes live out their life).
    ///
    /// `rows` are the locator's world rows as the original stores them, carrying the craft's
    /// scale: `[row1, row2]`.
    fn step_node(
        &mut self,
        slot: usize,
        side: usize,
        craft: Craft,
        tip: Vec3,
        rows: [Vec3; 2],
        dt: f32,
    ) {
        let config = self.config;
        if craft.triggered && craft.flying {
            self.nodes[slot][side].timer = config.refill;
        }
        let ramp = config.speed.at(craft.speed);
        let node = self.nodes[slot][side];
        if node.timer <= 0.0 {
            self.nodes[slot][side].element = None;
            return;
        }
        let element = match node.element {
            Some(element) => Some(element),
            None => self.acquire(),
        };
        self.nodes[slot][side].element = element;
        if let Some(ribbon) = element.and_then(|i| self.pool[i].as_mut()) {
            // `ramp * timer * (1 / 1.3)`, the strength the whole element is drawn at.
            let strength = ramp * node.timer * (1.0 / config.refill);
            ribbon.push_vortex(tip, rows[0], rows[1], side == 0, strength);
        }
        self.nodes[slot][side].timer -= dt;
    }

    fn age(&mut self, dt: f32) {
        for element in &mut self.pool {
            if let Some(ribbon) = element.as_mut() {
                ribbon.advance(dt);
                if ribbon.is_empty() {
                    *element = None;
                }
            }
        }
    }
}

/// The original's object rows for a craft with this orientation: row 1 points **down** and row 2
/// forward, both carrying the craft scale (`g_craft_scale`, `0.75`).
///
/// Row 1 down is the convention every HD object matrix reads in
/// (`crate::rocket_smoke::rocket_rows`), measured on the craft's wingtip nodes: at `theta = 0`
/// chain 0 drifts `-row1`, up on a level track.
#[must_use]
fn craft_rows(up: Vec3, forward: Vec3) -> [Vec3; 2] {
    [-up * TARGET_GLOBAL_SCALE, forward * TARGET_GLOBAL_SCALE]
}

impl Race {
    /// One tick of every craft's two wingtip nodes, then the ribbons' ageing: the order the
    /// original runs them in (a node's update pushes, `RibbonEffects_UpdatePools` ages).
    ///
    /// The trigger is `ShipState::pad_timer` (the pad force window the original's
    /// `entry+0x2e8` is) or `turbo_timer` (the original's flag `0x20`; **code-only, the Turbo
    /// half was never watched live** - `wingtip-vortices.md`). The destroyed sequence (states
    /// 4 to 6) is [`CraftState::Destroyed`] and [`CraftState::Eliminated`].
    pub(crate) fn advance_wingtip(&mut self) {
        let dt = self.sim.dt;
        let Some(mut contrails) = self.view.wingtip.take() else {
            return;
        };
        for slot in 0..usize::from(self.sim.world.ship_count).min(MAX_SHIPS) {
            let ship = &self.sim.world.ships[slot];
            let body = &ship.physics.body;
            let craft = Craft {
                triggered: ship.physics.pad_timer > 0.0 || ship.physics.turbo_timer > 0.0,
                flying: ship.active && ship.physics.craft_state == CraftState::Racing,
                speed: body.linear_velocity.dot(body.forward()).abs(),
            };
            let rows = craft_rows(body.up(), body.forward());
            let model = self.ship_model_matrix_of(slot);
            for index in 0..2 {
                let Some(local) = contrails.anchors[slot][index] else {
                    continue;
                };
                let tip = (model * local).w_axis.truncate();
                // The original's `left` node (side flag `1`) is the craft's physical left, which
                // an HD hull's `wingtip_left` lands on in this engine; read off the world so the
                // side holds for a locator that carries no name.
                let left = (tip - body.position).dot(body.right()) < 0.0;
                contrails.step_node(slot, usize::from(!left), craft, tip, rows, dt);
            }
        }
        contrails.age(dt);
        self.view.wingtip = Some(contrails);
    }

    /// Every live element's triangle list, built for a camera at `eye`. Empty where the race
    /// draws no contrails.
    #[must_use]
    pub fn wingtip_vertices(&self, eye: Vec3) -> Vec<GpuVertex> {
        let mut out = Vec::new();
        if let Some(contrails) = &self.view.wingtip {
            for ribbon in contrails.pool.iter().flatten() {
                ribbon.extend_vortex_vertices(&mut out, eye);
            }
        }
        out
    }

    /// How many contrail elements are alive, held or ageing out.
    #[must_use]
    pub fn wingtip_live(&self) -> usize {
        self.view.wingtip.as_ref().map_or(0, Contrails::live)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_livery::SparkAnchor;

    fn anchor(x: f32, y: f32, z: f32) -> SparkAnchor {
        SparkAnchor {
            position: Vec3::new(x, y, z),
            up: Vec3::Y,
        }
    }

    /// Pulse's Triakis authors collision locators at `|x|` 2.65 twice a side (`z` -5.23 and 0.52)
    /// and a nose pair at 2.17: the widest, rearmost one is the wing tip, as on HD's Triakis
    /// (`wingtip_left` at `z = -5.44`).
    #[test]
    fn the_wing_tip_is_the_widest_rearmost_locator_on_each_side() {
        let hull = [
            anchor(2.65, -1.08, -5.23),
            anchor(2.65, -0.82, 0.52),
            anchor(-2.65, -0.82, 0.52),
            anchor(-2.65, -1.08, -5.23),
            anchor(2.17, -1.24, 6.96),
            anchor(-2.17, -1.24, 6.96),
        ];
        let tip = |side| wing_tip(&hull, side).expect("a tip").w_axis.truncate();
        assert_eq!(tip(1.0), Vec3::new(2.65, -1.08, -5.23));
        assert_eq!(tip(-1.0), Vec3::new(-2.65, -1.08, -5.23));
    }

    /// A hull with locators only on the centreline has no wing tip: nothing is invented.
    #[test]
    fn a_hull_with_no_lateral_locator_has_no_wing_tip() {
        let hull = [anchor(0.0, 0.0, 7.0), anchor(0.4, -0.5, 5.8)];
        assert!(wing_tip(&hull, 1.0).is_none() && wing_tip(&hull, -1.0).is_none());
    }

    /// The setting off, or on with no lender, lends nothing; on with no lender says why.
    #[test]
    fn a_title_with_none_lays_them_only_with_the_setting_and_a_lender() {
        let mut report = Vec::new();
        let off = crate::Options::default();
        assert!(lend(&off, oag_pulse::TITLE, &mut report).is_none() && report.is_empty());
        let on = crate::Options {
            wingtip_contrails: WingtipContrails::On { lender: None },
            ..crate::Options::default()
        };
        assert!(lend(&on, oag_pulse::TITLE, &mut report).is_none());
        assert!(
            report[0].contains("no source that carries them"),
            "{report:?}"
        );
    }
}
