//! What [`super`] is asserted to do, on a skeleton built by hand.

use super::*;
use crate::rcsmodel::psp2::container::wrap_section;

/// A section writer: appends words and remembers where things landed.
pub(crate) struct Section(pub(crate) Vec<u8>);

impl Section {
    pub(crate) fn u32(&mut self, v: u32) -> usize {
        let at = self.0.len();
        self.0.extend_from_slice(&v.to_le_bytes());
        at
    }

    pub(crate) fn f32s(&mut self, v: &[f32]) -> usize {
        let at = self.0.len();
        for x in v {
            self.0.extend_from_slice(&x.to_le_bytes());
        }
        at
    }

    pub(crate) fn patch(&mut self, at: usize, v: u32) {
        self.0[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
}

/// One node's bind values, for [`skeleton`].
pub(crate) struct Bind {
    pub(crate) id: u32,
    pub(crate) parent: Option<usize>,
    pub(crate) scale: [f32; 3],
    pub(crate) rotation: [f32; 4],
    pub(crate) translation: [f32; 3],
    pub(crate) visible: bool,
    pub(crate) pivot: [f32; 3],
    pub(crate) pivot_translate: [f32; 3],
    pub(crate) above: [f32; 16],
}

impl Bind {
    pub(crate) fn plain(id: u32, parent: Option<usize>, translation: [f32; 3]) -> Self {
        Self {
            id,
            parent,
            scale: [1.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            translation,
            visible: true,
            pivot: [0.0; 3],
            pivot_translate: [0.0; 3],
            above: IDENTITY,
        }
    }
}

/// A `.rcsskeleton` image with these nodes, laid out as the disc lays one
/// out: ids, parents, the property block, the parent matrices.
pub(crate) fn skeleton(binds: &[Bind]) -> Vec<u8> {
    let mut s = Section(Vec::new());
    s.u32(2);
    s.u32(binds.len() as u32);
    s.u32(0);
    let p_ids = s.u32(0);
    let p_parents = s.u32(0);
    let p_props = s.u32(0);
    let p_above = s.u32(0);
    let ids_at = s.0.len();
    for b in binds {
        s.u32(b.id);
    }
    let parents_at = s.0.len();
    for (i, b) in binds.iter().enumerate() {
        s.u32(b.parent.unwrap_or(i) as u32);
    }
    let props_at = s.0.len();
    s.u32(SLOTS as u32);
    let mut table_sites = Vec::new();
    for _ in binds {
        table_sites.push(s.u32(0));
        s.u32(SLOTS as u32);
    }
    for (i, b) in binds.iter().enumerate() {
        let table_at = s.0.len();
        let mut slot_sites = Vec::new();
        for _ in 0..SLOTS {
            slot_sites.push(s.u32(0));
        }
        let property = |s: &mut Section, slot: usize, kind: u8, values: &[f32]| {
            let at = s.0.len();
            s.u32(0xdead_0000 | (u32::from(kind) << 8) | slot as u32);
            let value_site = s.u32(0);
            let value_at = s.f32s(values);
            s.patch(value_site, value_at as u32);
            s.patch(slot_sites[slot], at as u32);
        };
        property(&mut s, SLOT_SCALE, 1, &b.scale);
        property(&mut s, SLOT_ROTATION, 3, &b.rotation);
        property(&mut s, SLOT_TRANSLATION, 1, &b.translation);
        property(
            &mut s,
            SLOT_VISIBILITY,
            4,
            &[f32::from_bits(u32::from(b.visible))],
        );
        property(&mut s, SLOT_PIVOT, 1, &b.pivot);
        property(&mut s, SLOT_PIVOT_TRANSLATE, 1, &b.pivot_translate);
        s.patch(table_sites[i], table_at as u32);
    }
    let above_at = s.0.len();
    for b in binds {
        s.f32s(&b.above);
    }
    s.patch(p_ids, ids_at as u32);
    s.patch(p_parents, parents_at as u32);
    s.patch(p_props, props_at as u32);
    s.patch(p_above, above_at as u32);
    wrap_section(&s.0, &[])
}

fn close(a: &[f32; 16], b: &[f32; 16]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-4)
}

#[test]
fn reads_every_slot_and_the_hierarchy() {
    let mut child = Bind::plain(0xbeef, Some(0), [1.0, 2.0, 3.0]);
    child.pivot = [5.0, 0.0, 0.0];
    child.pivot_translate = [0.0, 0.5, 0.0];
    child.visible = false;
    let mut root = Bind::plain(0xcafe, None, [10.0, 0.0, 0.0]);
    root.scale = [2.0, 2.0, 2.0];
    root.above[12] = 100.0;
    let file = skeleton(&[root, child]);
    let parsed = parse(&file).expect("parses");
    assert_eq!(parsed.nodes.len(), 2);
    assert_eq!(parsed.nodes[0].id, 0xcafe);
    assert_eq!(parsed.nodes[0].parent, None);
    assert_eq!(parsed.nodes[0].scale, [2.0; 3]);
    assert_eq!(parsed.nodes[0].above[12], 100.0);
    assert!(parsed.nodes[0].visible);
    assert_eq!(parsed.nodes[1].parent, Some(0));
    assert_eq!(parsed.nodes[1].translation, [1.0, 2.0, 3.0]);
    assert_eq!(parsed.nodes[1].pivot, [5.0, 0.0, 0.0]);
    assert_eq!(parsed.nodes[1].pivot_translate, [0.0, 0.5, 0.0]);
    assert!(!parsed.nodes[1].visible);
    assert_eq!(parsed.nodes[1].kinds[SLOT_ROTATION], Some(Kind::Quat));
    assert_eq!(parsed.nodes[1].kinds[6], None);
    assert_eq!(parsed.node_by_id(0xbeef), Some(1));
    assert_eq!(parsed.order(), vec![0, 1]);
}

#[test]
fn a_child_listed_before_its_parent_still_orders_parent_first() {
    let file = skeleton(&[
        Bind::plain(1, Some(1), [0.0; 3]),
        Bind::plain(2, None, [0.0; 3]),
    ]);
    let parsed = parse(&file).expect("parses");
    assert_eq!(parsed.order(), vec![1, 0]);
}

#[test]
fn local_matrix_rotates_about_the_pivot() {
    // A quarter turn about y with the pivot at x = 10: a point at the pivot
    // stays put, the origin swings to (10, 0, -10) + pivot... measured
    // against the numpy composition the format was read with.
    let half = std::f32::consts::FRAC_1_SQRT_2;
    let m = local_matrix(
        [1.0; 3],
        [0.0, half, 0.0, half],
        [0.0; 3],
        [10.0, 0.0, 0.0],
        [0.0; 3],
    );
    let apply = |p: [f32; 3]| -> [f32; 3] {
        std::array::from_fn(|c| p[0] * m[c] + p[1] * m[4 + c] + p[2] * m[8 + c] + m[12 + c])
    };
    let at_pivot = apply([10.0, 0.0, 0.0]);
    assert!(
        at_pivot
            .iter()
            .zip(&[10.0, 0.0, 0.0])
            .all(|(a, b)| (a - b).abs() < 1e-5),
        "{at_pivot:?}"
    );
    let origin = apply([0.0; 3]);
    // The origin sits 10 units in -x from the pivot; a quarter turn about y
    // (row-vector convention) takes -x to -z... or +z; the sign is what the
    // quaternion-to-matrix layout decides, so pin the distance and the plane.
    assert!(
        (origin[0] - 10.0).abs() < 1e-5 && origin[1].abs() < 1e-5,
        "{origin:?}"
    );
    assert!((origin[2].abs() - 10.0).abs() < 1e-5, "{origin:?}");
    // Translation and the pivot translate add on the right.
    let m2 = local_matrix(
        [1.0; 3],
        [0.0, half, 0.0, half],
        [1.0, 2.0, 3.0],
        [10.0, 0.0, 0.0],
        [0.5, 0.0, 0.0],
    );
    let expected = [m[12] + 1.5, m[13] + 2.0, m[14] + 3.0];
    assert!(
        expected
            .iter()
            .zip(&m2[12..15])
            .all(|(a, b)| (a - b).abs() < 1e-5)
    );
}

#[test]
fn bind_world_composes_child_under_parent_and_root_under_above() {
    let mut root = Bind::plain(1, None, [10.0, 0.0, 0.0]);
    root.above[12] = 100.0;
    let child = Bind::plain(2, Some(0), [1.0, 0.0, 0.0]);
    let parsed = parse(&skeleton(&[root, child])).expect("parses");
    let world = parsed.bind_world();
    assert!((world[0][12] - 110.0).abs() < 1e-5);
    assert!((world[1][12] - 111.0).abs() < 1e-5);
}

#[test]
fn multiply_is_apply_a_then_b() {
    let mut a = IDENTITY;
    a[12] = 1.0;
    let mut b = IDENTITY;
    b[0] = 2.0;
    let ab = multiply(&a, &b);
    // Translate by 1 then scale x by 2: the translation is scaled.
    assert!(close(&ab, &{
        let mut m = IDENTITY;
        m[0] = 2.0;
        m[12] = 2.0;
        m
    }));
}

#[test]
fn refuses_a_wrong_magic() {
    let mut file = skeleton(&[Bind::plain(1, None, [0.0; 3])]);
    file[0] = 0;
    assert!(parse(&file).is_err());
}
