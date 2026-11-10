//! A closed surface standing a fixed distance off a triangle mesh.
//!
//! The mesh's distance field is sampled on a regular grid and the surface where it crosses
//! `distance` is extracted with one vertex per straddling cell and one quad per straddling grid
//! edge (naive surface nets). Only the surface facing the outside is kept: a pocket further than
//! `distance` from every triangle but enclosed by the mesh is not part of the shell.

use oag_core::math::Vec3;

/// The most grid points a surface is built over; a mesh needing more gives none.
const MAX_POINTS: usize = 4_000_000;

pub(super) struct Surface {
    pub(super) positions: Vec<Vec3>,
    /// Unit, smooth across the surface, all on one side of it.
    pub(super) normals: Vec<Vec3>,
    pub(super) indices: Vec<u32>,
}

struct Grid {
    origin: Vec3,
    cell: f32,
    size: [usize; 3],
}

impl Grid {
    fn index(&self, [x, y, z]: [usize; 3]) -> usize {
        (z * self.size[1] + y) * self.size[0] + x
    }

    fn point(&self, [x, y, z]: [usize; 3]) -> Vec3 {
        self.origin + Vec3::new(x as f32, y as f32, z as f32) * self.cell
    }
}

/// The surface `distance` off `triangles`, on a grid of `cell`-sized cubes, or `None` for no
/// triangles or a mesh too large for the grid.
pub(super) fn around(triangles: &[[Vec3; 3]], distance: f32, cell: f32) -> Option<Surface> {
    let first = triangles.first()?[0];
    let (low, high) = triangles
        .iter()
        .flatten()
        .fold((first, first), |(low, high), p| (low.min(*p), high.max(*p)));
    let margin = distance + 2.0 * cell;
    let origin = low - Vec3::splat(margin);
    let extent = high - low + Vec3::splat(2.0 * margin);
    let size = extent.to_array().map(|e| (e / cell).ceil() as usize + 1);
    if size.iter().product::<usize>() > MAX_POINTS {
        return None;
    }
    let grid = Grid { origin, cell, size };

    // Signed so the shell is where it crosses zero: the distance to the mesh less `distance`,
    // exact within the band a straddling edge can reach and a constant beyond it.
    let mut field = vec![2.0 * cell; size.iter().product()];
    let reach = distance + 2.0 * cell;
    for triangle in triangles {
        let [a, b, c] = *triangle;
        if (b - a).cross(c - a).length_squared() < 1e-12 {
            continue;
        }
        let from = (a.min(b).min(c) - Vec3::splat(reach) - origin) / cell;
        let to = (a.max(b).max(c) + Vec3::splat(reach) - origin) / cell;
        let span = |axis: usize| {
            let from = from.to_array()[axis].floor().max(0.0) as usize;
            let to = (to.to_array()[axis].ceil() as usize).min(size[axis] - 1);
            from..=to
        };
        for z in span(2) {
            for y in span(1) {
                for x in span(0) {
                    let at = [x, y, z];
                    let p = grid.point(at);
                    let value = (p - closest(p, triangle)).length() - distance;
                    let slot = &mut field[grid.index(at)];
                    *slot = slot.min(value);
                }
            }
        }
    }
    enclose_pockets(&grid, &mut field);

    let (positions, cells) = cell_vertices(&grid, &field);
    let mut indices = Vec::new();
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for axis in 0..3 {
        let (b, c) = ((axis + 1) % 3, (axis + 2) % 3);
        for z in 0..size[2] {
            for y in 0..size[1] {
                for x in 0..size[0] {
                    let at = [x, y, z];
                    let mut next = at;
                    next[axis] += 1;
                    if next[axis] >= size[axis] || at[b] == 0 || at[c] == 0 {
                        continue;
                    }
                    let outside = field[grid.index(at)] > 0.0;
                    if outside == (field[grid.index(next)] > 0.0) {
                        continue;
                    }
                    let corner = |db: usize, dc: usize| {
                        let mut cell = at;
                        cell[b] -= db;
                        cell[c] -= dc;
                        cells[cell_index(&size, cell)]
                    };
                    let mut quad = [corner(1, 1), corner(0, 1), corner(0, 0), corner(1, 0)];
                    if quad.contains(&u32::MAX) {
                        continue;
                    }
                    if outside {
                        quad.reverse();
                    }
                    for [i, j, k] in [[0, 1, 2], [0, 2, 3]] {
                        let [p, q, r] = [quad[i], quad[j], quad[k]];
                        let face = (positions[q as usize] - positions[p as usize])
                            .cross(positions[r as usize] - positions[p as usize]);
                        for vertex in [p, q, r] {
                            normals[vertex as usize] += face;
                        }
                        indices.extend([p, q, r]);
                    }
                }
            }
        }
    }
    if indices.is_empty() {
        return None;
    }
    for normal in &mut normals {
        *normal = normal.normalize_or_zero();
    }
    Some(Surface {
        positions,
        normals,
        indices,
    })
}

/// Turns every region further than the distance from the mesh that the grid's corner cannot
/// reach into inside, so no surface is drawn around it.
fn enclose_pockets(grid: &Grid, field: &mut [f32]) {
    let mut outside = vec![false; field.len()];
    let mut pending = vec![[0usize; 3]];
    outside[0] = true;
    while let Some(at) = pending.pop() {
        for axis in 0..3 {
            for step in [-1isize, 1] {
                let Some(moved) = at[axis].checked_add_signed(step) else {
                    continue;
                };
                if moved >= grid.size[axis] {
                    continue;
                }
                let mut next = at;
                next[axis] = moved;
                let index = grid.index(next);
                if field[index] > 0.0 && !outside[index] {
                    outside[index] = true;
                    pending.push(next);
                }
            }
        }
    }
    for (value, outside) in field.iter_mut().zip(outside) {
        if *value > 0.0 && !outside {
            *value = -grid.cell;
        }
    }
}

fn cell_index(size: &[usize; 3], [x, y, z]: [usize; 3]) -> usize {
    (z * (size[1] - 1) + y) * (size[0] - 1) + x
}

/// One vertex per cell the surface passes through, at the mean of where it crosses the cell's
/// edges, and each cell's vertex index (`u32::MAX` for none).
fn cell_vertices(grid: &Grid, field: &[f32]) -> (Vec<Vec3>, Vec<u32>) {
    const EDGES: [[usize; 2]; 12] = [
        [0, 1],
        [2, 3],
        [4, 5],
        [6, 7],
        [0, 2],
        [1, 3],
        [4, 6],
        [5, 7],
        [0, 4],
        [1, 5],
        [2, 6],
        [3, 7],
    ];
    let size = grid.size;
    let mut positions = Vec::new();
    let mut cells = vec![u32::MAX; (size[0] - 1) * (size[1] - 1) * (size[2] - 1)];
    for z in 0..size[2] - 1 {
        for y in 0..size[1] - 1 {
            for x in 0..size[0] - 1 {
                let corner = |n: usize| [x + (n & 1), y + ((n >> 1) & 1), z + (n >> 2)];
                let mut sum = Vec3::ZERO;
                let mut crossings = 0.0;
                for [a, b] in EDGES {
                    let (va, vb) = (field[grid.index(corner(a))], field[grid.index(corner(b))]);
                    if (va > 0.0) == (vb > 0.0) {
                        continue;
                    }
                    let (pa, pb) = (grid.point(corner(a)), grid.point(corner(b)));
                    sum += pa + (pb - pa) * (va / (va - vb));
                    crossings += 1.0;
                }
                if crossings > 0.0 {
                    cells[cell_index(&size, [x, y, z])] = positions.len() as u32;
                    positions.push(sum / crossings);
                }
            }
        }
    }
    (positions, cells)
}

/// The point of the triangle nearest `p`.
pub(super) fn closest(p: Vec3, &[a, b, c]: &[Vec3; 3]) -> Vec3 {
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let scale = 1.0 / (va + vb + vc);
    a + ab * (vb * scale) + ac * (vc * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box_triangles(half: Vec3) -> Vec<[Vec3; 3]> {
        let corner = |n: usize| {
            Vec3::new(
                if n & 1 == 0 { -half.x } else { half.x },
                if n & 2 == 0 { -half.y } else { half.y },
                if n & 4 == 0 { -half.z } else { half.z },
            )
        };
        [
            [0, 1, 3, 2],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 3, 7, 6],
            [0, 2, 6, 4],
            [1, 3, 7, 5],
        ]
        .iter()
        .flat_map(|q| {
            [
                [corner(q[0]), corner(q[1]), corner(q[2])],
                [corner(q[0]), corner(q[2]), corner(q[3])],
            ]
        })
        .collect()
    }

    #[test]
    fn the_surface_stands_the_distance_off_every_triangle_and_is_closed() {
        let triangles = box_triangles(Vec3::new(1.0, 0.25, 2.0));
        let surface = around(&triangles, 0.2, 0.05).expect("a surface");
        for p in &surface.positions {
            let nearest = triangles
                .iter()
                .map(|t| (*p - closest(*p, t)).length())
                .fold(f32::MAX, f32::min);
            assert!(
                (nearest - 0.2).abs() < 0.03,
                "{p:?} is {nearest} off the box"
            );
            // Outside the box: the pocket inside it, 0.2 from every face, has no surface.
            assert!(
                p.x.abs() > 1.0 || p.y.abs() > 0.25 || p.z.abs() > 2.0,
                "{p:?}"
            );
        }
        // Closed: every edge is shared by exactly two triangles, once in each direction.
        let mut edges = std::collections::HashMap::new();
        for t in surface.indices.chunks_exact(3) {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                *edges.entry((a, b)).or_insert(0i32) += 1;
                *edges.entry((b, a)).or_insert(0i32) -= 1;
            }
        }
        assert!(edges.values().all(|n| *n == 0), "an open or doubled edge");
        // The normals all point away from the box.
        for (p, n) in surface.positions.iter().zip(&surface.normals) {
            let nearest = triangles
                .iter()
                .map(|t| closest(*p, t))
                .min_by(|a, b| (*p - *a).length().total_cmp(&(*p - *b).length()))
                .expect("a triangle");
            assert!(n.dot(*p - nearest) > 0.0, "{p:?} {n:?}");
        }
    }

    #[test]
    fn a_single_sheet_gets_a_shell_on_both_sides() {
        let sheet = [[
            Vec3::new(-1.0, 0.0, -1.0),
            Vec3::new(1.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, 1.0),
        ]];
        let surface = around(&sheet, 0.2, 0.05).expect("a surface");
        assert!(surface.positions.iter().any(|p| p.y > 0.15));
        assert!(surface.positions.iter().any(|p| p.y < -0.15));
    }

    #[test]
    fn no_triangles_give_no_surface() {
        assert!(around(&[], 0.2, 0.05).is_none());
    }
}
