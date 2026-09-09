//! What the disc says about the vertex stride, and whether the searches agree.
//!
//! **The declaration is the oracle.** `vertex_decl` states a chunk's stride
//! outright, so the three searches in `rcsmodel::stride` can be graded against
//! it on every chunk that has one - and `rcsmodel::STRIDES`, the list of widths
//! they are allowed to answer, can be checked against the widths that actually
//! exist rather than against the ones that were known when it was written.
//!
//! That check is the point. `STRIDES` was `[14, 18, 22]` for as long as no
//! declaration was read; the disc carries seven widths, and a chunk of one of
//! the missing four with no declaration of its own could not be solved by any
//! search, because the answer was not on the ballot.
//!
//! `#[ignore]`d because it needs `data/images/hdfury-ps3-eu-dec.iso`; run with
//! `just test-data`.

mod rcsmodel_common;

use std::collections::BTreeMap;

use oag_rcs::rcsmodel;
use rcsmodel_common::image;

const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

#[derive(Default)]
struct Grades {
    /// Declared stride -> how many chunks declare it.
    declared: BTreeMap<usize, usize>,
    agree: usize,
    disagree: usize,
    /// Declares a stride, but no search answers.
    silent: usize,
    /// Surfaces with neither a declaration nor a solvable stride: geometry
    /// nothing can draw.
    unsolvable: usize,
    surfaces: usize,
}

fn grade() -> Grades {
    let image = image().expect("checked by the caller");
    let mut g = Grades::default();
    for archive in ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(blob) = open.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&blob) else {
                continue;
            };
            for chunk in &model.meshes {
                for surface in chunk.surfaces() {
                    g.surfaces += 1;
                    let solved = surface.solve_stride_without_a_box(&blob);
                    match surface.declared_stride() {
                        Some(truth) => {
                            *g.declared.entry(truth).or_default() += 1;
                            match solved {
                                Some(s) if s == truth => g.agree += 1,
                                Some(_) => g.disagree += 1,
                                None => g.silent += 1,
                            }
                        }
                        None if solved.is_none() => g.unsolvable += 1,
                        None => {}
                    }
                }
            }
        }
    }
    g
}

/// **Every stride the disc declares is a stride the searches may answer.**
///
/// The invariant that broke: a width missing from [`rcsmodel::STRIDES`] is a
/// chunk no search can ever solve. Asserted against the declarations rather
/// than against a list written by hand, so adding a title with an eighth width
/// fails here instead of silently losing its geometry.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn every_declared_stride_is_a_candidate_the_searches_may_answer() {
    if image().is_none() {
        return;
    }
    let g = grade();
    println!("declared strides: {:?}", g.declared);
    assert_eq!(
        g.declared.keys().copied().collect::<Vec<_>>(),
        vec![10, 14, 18, 22, 26, 34, 38],
        "the widths the disc's own vertex declarations carry",
    );
    for width in g.declared.keys() {
        assert!(
            rcsmodel::STRIDES.contains(width),
            "stride {width} is declared by {} chunk(s) and is not in STRIDES, so no search \
             can ever answer it",
            g.declared[width],
        );
    }
}

/// **Where a search answers a chunk that also declares, it is right.**
///
/// 6 disagreements in 58,823, which is 99.99%. Held as a ceiling on the
/// disagreements rather than a floor on the rate, because the rate hides how
/// few they are.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_stride_searches_agree_with_the_declaration() {
    if image().is_none() {
        return;
    }
    let g = grade();
    println!(
        "{} agree, {} disagree, {} declared but unanswered",
        g.agree, g.disagree, g.silent
    );
    assert!(
        g.disagree <= 6,
        "{} chunk(s) where a search contradicts the declaration, up from 6",
        g.disagree,
    );
    assert!(
        g.agree >= 58_000,
        "only {} chunk(s) where a search confirms the declaration, down from 58,817",
        g.agree,
    );
}

/// **How much geometry no rule can decode at all**, as a ratchet.
///
/// A surface with neither a declaration nor a solvable stride is skipped by
/// every consumer, silently. Widening `STRIDES` to the declared set took this
/// from 371 to 115; the rest are concentrated in the front-end track previews
/// and are a standing lead rather than a defect this asserts against.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_surfaces_no_rule_can_decode_do_not_grow() {
    if image().is_none() {
        return;
    }
    let g = grade();
    println!("{} of {} surface(s) undecodable", g.unsolvable, g.surfaces);
    assert_eq!(g.surfaces, 64_589);
    assert!(
        g.unsolvable <= 115,
        "{} surface(s) have neither a declared nor a solvable stride, up from 115",
        g.unsolvable,
    );
}
