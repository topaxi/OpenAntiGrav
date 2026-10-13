//! The broadphase in front of [`super::TriangleSoup`]'s narrowphase.
//!
//! A bounding-volume hierarchy over the soup's triangle boxes, and the same tree over a
//! world's collider boxes. It is an acceleration only: [`Bvh::visit`] reports exactly the
//! items whose own box overlaps the query box, the same set the linear scan's per-item
//! reject passed, so the narrowphase sees the same candidates and the caller restores the
//! scan's ascending-index order. Built once, from data alone, with a total order on every
//! comparison, so the tree is identical on every platform.

use super::Aabb;
use oag_core::math::Vec3;

/// Triangles a leaf holds before it splits.
const LEAF: usize = 4;

/// Deep enough for any soup the format's `u32` indices can address with a median split.
const STACK: usize = 40;

#[derive(Debug, Clone)]
struct Node {
    bounds: Aabb,
    /// A leaf: first entry in `order`. An interior node: the right child's index (the left
    /// child is always the next node).
    start: u32,
    /// Triangles in a leaf, or `0` for an interior node.
    count: u32,
}

#[derive(Debug, Clone, Default)]
pub(super) struct Bvh {
    nodes: Vec<Node>,
    /// Triangle indices, grouped by leaf.
    order: Vec<u32>,
    /// Each triangle's own box, by triangle index.
    boxes: Vec<Aabb>,
}

impl Bvh {
    pub(super) fn build(boxes: Vec<Aabb>) -> Self {
        let mut order: Vec<u32> = (0..boxes.len() as u32).collect();
        let mut nodes = Vec::new();
        if !order.is_empty() {
            build_node(&boxes, &mut order, 0, &mut nodes);
        }
        Self {
            nodes,
            order,
            boxes,
        }
    }

    /// Calls `f` with the index of every triangle whose box overlaps `query`, in no
    /// particular order.
    pub(super) fn visit(&self, query: &Aabb, mut f: impl FnMut(u32)) {
        if self.nodes.is_empty() {
            return;
        }
        let mut stack = [0u32; STACK];
        let mut top = 1;
        while top > 0 {
            top -= 1;
            let index = stack[top] as usize;
            let node = &self.nodes[index];
            if !query.overlaps(&node.bounds) {
                continue;
            }
            if node.count == 0 {
                stack[top] = node.start;
                stack[top + 1] = index as u32 + 1;
                top += 2;
            } else {
                let first = node.start as usize;
                for &tri in &self.order[first..first + node.count as usize] {
                    if query.overlaps(&self.boxes[tri as usize]) {
                        f(tri);
                    }
                }
            }
        }
    }
}

/// Item indices a query collected, sorted ascending: the linear scan's order. Held inline
/// up to [`Indices::INLINE`], so the common query allocates nothing.
pub(super) struct Indices {
    inline: [u32; Self::INLINE],
    len: usize,
    spill: Vec<u32>,
}

impl Indices {
    const INLINE: usize = 64;

    /// Every item of `bvh` whose box overlaps `query`, ascending.
    pub(super) fn overlapping(bvh: &Bvh, query: &Aabb) -> Self {
        let mut out = Self {
            inline: [0; Self::INLINE],
            len: 0,
            spill: Vec::new(),
        };
        bvh.visit(query, |index| {
            if out.len < Self::INLINE {
                out.inline[out.len] = index;
                out.len += 1;
            } else {
                if out.spill.is_empty() {
                    out.spill.extend_from_slice(&out.inline);
                }
                out.spill.push(index);
            }
        });
        if out.spill.is_empty() {
            out.inline[..out.len].sort_unstable();
        } else {
            out.spill.sort_unstable();
        }
        out
    }

    pub(super) fn as_slice(&self) -> &[u32] {
        if self.spill.is_empty() {
            &self.inline[..self.len]
        } else {
            &self.spill
        }
    }
}

fn centre(b: &Aabb, axis: usize) -> f32 {
    (b.min[axis] + b.max[axis]) * 0.5
}

fn build_node(boxes: &[Aabb], order: &mut [u32], offset: usize, nodes: &mut Vec<Node>) {
    let mut bounds = Aabb::EMPTY;
    let mut centres = Aabb::EMPTY;
    for &tri in order.iter() {
        let b = &boxes[tri as usize];
        bounds.expand(b.min);
        bounds.expand(b.max);
        centres.expand(Vec3::new(centre(b, 0), centre(b, 1), centre(b, 2)));
    }

    let me = nodes.len();
    if order.len() <= LEAF {
        nodes.push(Node {
            bounds,
            start: offset as u32,
            count: order.len() as u32,
        });
        return;
    }
    nodes.push(Node {
        bounds,
        start: 0,
        count: 0,
    });

    let extent = centres.max - centres.min;
    let axis = if extent.x >= extent.y && extent.x >= extent.z {
        0
    } else if extent.y >= extent.z {
        1
    } else {
        2
    };
    let half = order.len() / 2;
    order.select_nth_unstable_by(half, |&a, &b| {
        centre(&boxes[a as usize], axis)
            .total_cmp(&centre(&boxes[b as usize], axis))
            .then(a.cmp(&b))
    });

    let (left, right) = order.split_at_mut(half);
    build_node(boxes, left, offset, nodes);
    nodes[me].start = nodes.len() as u32;
    build_node(boxes, right, offset + half, nodes);
}
