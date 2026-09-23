//! An authored `LodGroup`'s tiers, and the per-frame switch between them.
//!
//! Four pieces:
//!
//! - [`Lod`] is how a model is **built**: every tier kept and switched per
//!   frame at one of three [`ModelDetail`] presets, or every tier drawn at
//!   once as a diagnostic.
//! - [`ModelDetail`] is the player's `[render_profiles.<title>] model_detail`
//!   setting: one multiplier on the authored switch distances.
//! - [`LodGroups`] is what a built [`super::Model`] carries: each group's
//!   model-space position and switch distances, and which group and child
//!   every scene-tree node sits under.
//! - [`LodSwitch`] is one drawn instance's choice for the current frame -
//!   eight craft share a model and each is its own distance from the camera.
//!
//! # The rule, recovered
//!
//! Pulse's `LodGroup_SelectChild` (`0x0890be68`, confidence 85, see
//! `docs/formats/vex.md`, "the switch is found") runs every frame for every
//! group: it takes the group's authored position through the node's world
//! matrix and the view matrix, scales the view-space depth by the field of
//! view as `d = -z * fov_degrees / 65`, and enables the child whose index is
//! how many of the `child_count - 1` authored switch distances `d` has
//! reached. Only that child's subtree draws. [`LodGroups::child_at`] is that
//! function, and the one place its arithmetic lives.

use std::cell::Cell;

use oag_core::math::{Mat4, Vec3};
use oag_vex::vex;

use super::DrawCall;

/// The field of view, in degrees, at which a switch distance is a plain
/// view depth: `LodGroup_SelectChild` divides by the literal `65.0`, Pulse's
/// chase-camera field. At any other field the depth is scaled by
/// `fov / 65`, so a wider view - where the model reads smaller - switches
/// sooner. Read on the PSP binary; see the module doc.
pub const REFERENCE_FOV_DEGREES: f32 = 65.0;

/// Where a `LodGroup` payload's own position sits: three `f32`s after its
/// 64-byte identity transform.
const POSITION_AT: usize = 0x40;
/// The payload's `child_count` (`u32`).
const CHILD_COUNT_AT: usize = 0x50;
/// The first of the `child_count - 1` switch distances (`f32`), after the
/// 12-byte field `LodGroup_SelectChild` reads as a relocated pointer to them.
const DISTANCES_AT: usize = 0x60;

/// How far away a model switches to its coarser authored tiers - one
/// multiplier on the switch distances the disc authors, so the recovered
/// rule stays the only rule. `[render_profiles.<title>] model_detail`.
///
/// [`Self::Original`] is the measured behaviour; [`Self::High`] and
/// [`Self::Maximum`] are this project's own, for a machine that can afford
/// the fine tier further out - chosen, not measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum ModelDetail {
    /// The authored switch distances as `LodGroup_SelectChild` applies them.
    #[default]
    Original,
    /// Every switch distance doubled.
    High,
    /// Never switch: the finest tier at every distance.
    Maximum,
}

impl ModelDetail {
    /// The spelling used in a settings file, on a menu row and on `--lod`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::High => "high",
            Self::Maximum => "maximum",
        }
    }

    /// Every preset, for the menus and for error messages.
    pub const ALL: [Self; 3] = [Self::Original, Self::High, Self::Maximum];

    /// What every authored switch distance is multiplied by. Infinite for
    /// [`Self::Maximum`], which no finite depth reaches.
    #[must_use]
    pub fn scale(self) -> f32 {
        match self {
            Self::Original => 1.0,
            Self::High => 2.0,
            Self::Maximum => f32::INFINITY,
        }
    }
}

impl std::str::FromStr for ModelDetail {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|detail| detail.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| {
                format!("{text:?} is not a model detail preset; try original, high or maximum")
            })
    }
}

impl std::fmt::Display for ModelDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for ModelDetail {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<ModelDetail> for String {
    fn from(detail: ModelDetail) -> Self {
        detail.to_string()
    }
}

/// How [`super::build_with_textures`] treats an authored `LodGroup`
/// (class `0x2ee` on Pulse), and what `--lod` on `oag-game` and `oag-view`
/// takes.
///
/// Every variant but [`Self::Both`] builds the same model - every tier in
/// the buffer, and a [`LodGroups`] table beside it - and differs only in the
/// [`ModelDetail`] a caller switches it at. [`Self::Both`] builds no table,
/// so every tier draws at once, the coarse over the fine: a diagnostic view
/// of where the coarse tier sits, which the original never shows.
///
/// A consumer that never calls [`LodSwitch::select`] gets the finest tier
/// ([`LodSwitch::new`], [`LodGroups::shows_nearest`]) - what the original
/// shows up close, confirmed against PPSSPP frames of the player's hull and
/// a `16_Track` grandstand (`docs/formats/vex.md`, "the running original
/// does not draw tier 1 up close").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lod {
    /// The recovered switch at the authored distances. The default.
    #[default]
    Original,
    /// The switch at twice the authored distances - chosen, not measured.
    High,
    /// The finest tier always - chosen, not measured.
    Maximum,
    /// Every tier at once, coarse over fine - a diagnostic.
    Both,
}

impl Lod {
    /// The spelling used on the `--lod` command-line flag.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Original => "original",
            Self::High => "high",
            Self::Maximum => "maximum",
        }
    }

    /// Every mode, for error messages.
    pub const ALL: [Self; 4] = [Self::Original, Self::High, Self::Maximum, Self::Both];

    /// The switch preset this mode draws at, or `None` for [`Self::Both`],
    /// which switches nothing.
    #[must_use]
    pub fn detail(self) -> Option<ModelDetail> {
        match self {
            Self::Original => Some(ModelDetail::Original),
            Self::High => Some(ModelDetail::High),
            Self::Maximum => Some(ModelDetail::Maximum),
            Self::Both => None,
        }
    }
}

impl From<ModelDetail> for Lod {
    fn from(detail: ModelDetail) -> Self {
        match detail {
            ModelDetail::Original => Self::Original,
            ModelDetail::High => Self::High,
            ModelDetail::Maximum => Self::Maximum,
        }
    }
}

impl std::str::FromStr for Lod {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|lod| lod.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| {
                format!(
                    "{text:?} is not a level-of-detail mode; try original, high, maximum or both"
                )
            })
    }
}

impl std::fmt::Display for Lod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// One authored `LodGroup`, as the switch needs it.
#[derive(Debug, Clone, PartialEq)]
struct Group {
    /// The payload's position, carried into the model's own space through
    /// the node's world matrix (at time zero, for a group under an `Anim
    /// Transform` - none is authored on any disc measured).
    position: [f32; 3],
    /// The `child_count - 1` switch distances, in file order.
    distances: Vec<f32>,
    /// How many node-tree children the group has - the ceiling on the
    /// index [`LodGroups::child_at`] returns.
    children: u16,
}

/// Which group and which of its children a node draws under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Member {
    group: u16,
    child: u16,
}

/// Every `LodGroup` a model authors, and which of their children each of the
/// model's scene-tree nodes sits under - keyed by the node index a
/// [`DrawCall::node`] carries.
///
/// Empty on a model built with [`Lod::Both`], on any model with no
/// `LodGroup`, and on every model that is not one `.vex` (a ribbon, a
/// collision overlay, a [`super::merge`]). An empty table shows every draw.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LodGroups {
    groups: Vec<Group>,
    /// Indexed by node index; `None` for a node under no group.
    member: Vec<Option<Member>>,
}

impl LodGroups {
    /// Reads every `LodGroup` in `nodes` and tags every node under one.
    ///
    /// `classes` is the file's own table: a version whose `LodGroup` id is
    /// unrecovered yields an empty table, which shows every tier - the
    /// direction that never deletes real geometry. `anchors` and
    /// `anchor_world` are what the builder places meshes with, so a group's
    /// position lands in the same space as its meshes' vertices.
    ///
    /// A node under nested groups is tagged with the nearest; the census in
    /// `crates/render/tests/lod_switch_ground_truth.rs` finds no nesting on
    /// any disc.
    #[must_use]
    pub fn collect(
        data: &[u8],
        nodes: &[vex::Node],
        classes: vex::classes::Classes,
        anchors: &[vex::Anchored],
        anchor_world: &[Option<[f32; 16]>],
    ) -> Self {
        let Some(class) = classes.lod_group else {
            return Self::default();
        };
        let order = vex::byte_order(data);
        let mut group_of: Vec<Option<u16>> = vec![None; nodes.len()];
        let mut children = vec![0u16; nodes.len()];
        let mut child_index = vec![0u16; nodes.len()];
        for (i, node) in nodes.iter().enumerate() {
            if let Some(parent) = node.parent.filter(|&p| p < nodes.len()) {
                child_index[i] = children[parent];
                children[parent] = children[parent].saturating_add(1);
            }
        }
        let mut groups = Vec::new();
        for (i, node) in nodes.iter().enumerate() {
            if node.class_id != class {
                continue;
            }
            let payload = data.get(node.payload()).unwrap_or_default();
            let read = |at: usize| (payload.len() >= at + 4).then(|| order.f32(payload, at));
            let local = [
                read(POSITION_AT).unwrap_or(0.0),
                read(POSITION_AT + 4).unwrap_or(0.0),
                read(POSITION_AT + 8).unwrap_or(0.0),
            ];
            let declared = if payload.len() >= CHILD_COUNT_AT + 4 {
                order.u32(payload, CHILD_COUNT_AT)
            } else {
                1
            };
            let distances = (0..declared.saturating_sub(1) as usize)
                .map_while(|k| read(DISTANCES_AT + 4 * k))
                .collect();
            let anchored = anchors.get(i).copied();
            let mut position = anchored.map_or(local, |a| vex::transform_point(&a.local, local));
            if let Some(world) = anchored
                .and_then(|a| a.anchor)
                .and_then(|a| anchor_world.get(a).copied().flatten())
            {
                position = vex::transform_point(&world, position);
            }
            let Ok(index) = u16::try_from(groups.len()) else {
                break;
            };
            group_of[i] = Some(index);
            groups.push(Group {
                position,
                distances,
                children: children[i],
            });
        }
        if groups.is_empty() {
            return Self::default();
        }
        let member = (0..nodes.len())
            .map(|i| {
                let mut below = i;
                let mut parent = nodes[i].parent;
                while let Some(p) = parent.filter(|&p| p < nodes.len()) {
                    if let Some(group) = group_of[p] {
                        return Some(Member {
                            group,
                            child: child_index[below],
                        });
                    }
                    below = p;
                    parent = nodes[p].parent;
                }
                None
            })
            .collect();
        Self { groups, member }
    }

    /// Whether this model authors no switchable group at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// How many groups the model authors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.groups.len()
    }

    /// Group `group`'s authored switch distances, in file order.
    #[must_use]
    pub fn distances(&self, group: usize) -> &[f32] {
        self.groups.get(group).map_or(&[], |g| &g.distances)
    }

    /// Group `group`'s position in the model's own space.
    #[must_use]
    pub fn position(&self, group: usize) -> Option<[f32; 3]> {
        self.groups.get(group).map(|g| g.position)
    }

    /// The group and child a draw call's node sits under, or `None` for one
    /// under no group.
    #[must_use]
    pub fn member_of(&self, draw: &DrawCall) -> Option<(usize, u16)> {
        let node = usize::try_from(draw.node?).ok()?;
        let member = (*self.member.get(node)?)?;
        Some((usize::from(member.group), member.child))
    }

    /// Group `group`'s `d = -z * fov_degrees / 65` - the number
    /// [`Self::child_at`] compares against the authored distances. `0` for a
    /// group the model does not have.
    #[must_use]
    pub fn scaled_depth(&self, group: usize, model_view: Mat4, fov_degrees: f32) -> f32 {
        self.groups.get(group).map_or(0.0, |g| {
            let z = model_view.transform_point3(Vec3::from(g.position)).z;
            -z * fov_degrees / REFERENCE_FOV_DEGREES
        })
    }

    /// **`LodGroup_SelectChild`**: which child of group `group` the original
    /// enables with the model at `model_view` (view matrix times model
    /// matrix) and the field of view at `fov_degrees`, vertical, in the
    /// unit the projection is built from.
    ///
    /// `d = -z * fov_degrees / 65`, where `z` is the group's view-space
    /// depth; the child is how many of the authored distances, each
    /// multiplied by `detail`'s [`ModelDetail::scale`], `d` reaches
    /// (`d >= distance`). Behind the eye `d` is negative and the finest
    /// child is kept.
    #[must_use]
    pub fn child_at(
        &self,
        group: usize,
        model_view: Mat4,
        fov_degrees: f32,
        detail: ModelDetail,
    ) -> u16 {
        let Some(g) = self.groups.get(group) else {
            return 0;
        };
        let d = self.scaled_depth(group, model_view, fov_degrees);
        let scale = detail.scale();
        let reached = g
            .distances
            .iter()
            .filter(|&&distance| d >= distance * scale)
            .count();
        u16::try_from(reached)
            .unwrap_or(u16::MAX)
            .min(g.children.saturating_sub(1))
    }

    /// Whether `draw` is in the child [`Self::child_at`] enables for the
    /// instance at `model` seen from `eye` - [`LodSwitch`] without the
    /// storage, for an instance drawn through another's geometry (the
    /// ghost borrows a grid slot's hull).
    #[must_use]
    pub fn shows_at(&self, draw: &DrawCall, model: Mat4, eye: LodEye) -> bool {
        self.member_of(draw).is_none_or(|(group, child)| {
            self.child_at(group, eye.view * model, eye.fov_degrees, eye.detail) == child
        })
    }

    /// Whether `draw` is in the finest tier of whatever group it sits under,
    /// or under none - the picture a caller with no camera to switch by
    /// draws. See [`Lod`].
    #[must_use]
    pub fn shows_nearest(&self, draw: &DrawCall) -> bool {
        self.member_of(draw).is_none_or(|(_, child)| child == 0)
    }
}

/// The camera a frame's switch is made from: everything
/// [`LodGroups::child_at`] needs besides the instance's own model matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LodEye {
    /// The view matrix - unjittered, the one the frustum is built from.
    pub view: Mat4,
    /// The vertical field of view the projection is built from, in degrees.
    pub fov_degrees: f32,
    /// The player's preset.
    pub detail: ModelDetail,
}

/// One drawn instance's per-frame choice of child for each of its model's
/// groups.
///
/// Interior-mutable so a drawable holding it behind `&self` can be switched
/// in the frame that draws it; sized once when the instance is made, so
/// switching allocates nothing. Starts on the finest tier of every group.
#[derive(Debug, Default)]
pub struct LodSwitch {
    chosen: Box<[Cell<u16>]>,
}

impl LodSwitch {
    /// A switch for `groups`, every group on its finest child.
    #[must_use]
    pub fn new(groups: &LodGroups) -> Self {
        Self {
            chosen: (0..groups.len()).map(|_| Cell::new(0)).collect(),
        }
    }

    /// Chooses every group's child for this frame - see
    /// [`LodGroups::child_at`]. `model` is the instance's own model matrix.
    pub fn select(&self, groups: &LodGroups, model: Mat4, eye: LodEye) {
        let model_view = eye.view * model;
        for (group, chosen) in self.chosen.iter().enumerate() {
            chosen.set(groups.child_at(group, model_view, eye.fov_degrees, eye.detail));
        }
    }

    /// Puts every group back on its finest child.
    pub fn reset(&self) {
        for chosen in &self.chosen {
            chosen.set(0);
        }
    }

    /// Whether `draw` belongs to a child this frame's choice enables.
    #[must_use]
    pub fn shows(&self, groups: &LodGroups, draw: &DrawCall) -> bool {
        groups.member_of(draw).is_none_or(|(group, child)| {
            self.chosen
                .get(group)
                .map_or(child == 0, |chosen| chosen.get() == child)
        })
    }

    /// Which child group `group` is on this frame, for a report.
    #[must_use]
    pub fn chosen(&self, group: usize) -> Option<u16> {
        self.chosen.get(group).map(Cell::get)
    }
}

impl super::Model {
    /// Drops every draw call in a coarser `LodGroup` tier than the first and
    /// empties [`Self::lod_groups`]: the fixed picture for a caller with no
    /// camera to switch by - `oag-view`, the front end's ship preview - so
    /// its own draw loops need no switch of their own. The vertices stay;
    /// nothing indexes them.
    pub fn keep_nearest(&mut self) {
        let groups = std::mem::take(&mut self.lod_groups);
        for list in [
            &mut self.draws,
            &mut self.alpha_tested_draws,
            &mut self.transparent_draws,
        ] {
            list.retain(|draw| groups.shows_nearest(draw));
        }
    }

    /// [`Self::keep_nearest`] unless `lod` is [`Lod::Both`], which keeps
    /// every tier: what a viewer with no race camera draws under `--lod`.
    #[must_use]
    pub fn for_fixed_view(mut self, lod: Lod) -> Self {
        if lod != Lod::Both {
            self.keep_nearest();
        }
        self
    }
}

#[cfg(test)]
mod tests;
