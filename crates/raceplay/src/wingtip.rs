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

/// Reads the title's [`oag_title::Title::wingtip`] texture and each craft's locators, reporting
/// what it found or why nothing draws. The texture and the locators are both needed: a craft
/// without a locator lays nothing and the report names it; a texture that does not decode
/// draws nothing.
pub(crate) fn load(
    archives: &mut oag_assets::Archives,
    config: Option<&Wingtip>,
    liveries: &[oag_livery::Livery],
    report: &mut Vec<String>,
) -> Option<(WingtipAssets, WingtipDraw)> {
    let config = *config?;
    let platform = archives.layout.platform;
    let texture = archives
        .read_name(config.texture)
        .ok()
        .and_then(|blob| crate::load::magstrip_wake::decode(&blob, platform));
    let Some(texture) = texture else {
        report.push(format!(
            "wingtip contrails: {} missing or unreadable - nothing draws",
            config.texture
        ));
        return None;
    };
    let mut anchors = [[None; 2]; MAX_SHIPS];
    for (slot, livery) in liveries.iter().enumerate().take(MAX_SHIPS) {
        anchors[slot] = livery.wingtip;
    }
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
            for side in 0..2 {
                let Some(local) = contrails.anchors[slot][side] else {
                    continue;
                };
                let tip = (model * local).w_axis.truncate();
                contrails.step_node(slot, side, craft, tip, rows, dt);
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
