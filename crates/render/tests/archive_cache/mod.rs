//! Archive reads for the HD ground-truth binaries, each archive opened once.
//!
//! `mesh::read_blob(spec, name)` opens the disc image and parses the archive's
//! table of contents on every call. A PS3 circuit's scene asks for each of its
//! textures by name, so one `build_scene` with `read_blob` as its texture
//! closure opened `hdfury-ps3-eu-dec.iso` 153,293 times in
//! `hd_water_ground_truth`'s `no_other_circuit_is_reached_02` (2026-10-07,
//! `strace -e openat`), spending 76 s of user and 63 s of system time on it.
//! The race itself never does this: `oag_raceplay`'s loader reads through
//! `oag_assets::Archives::read_name`, which holds every archive open.
//!
//! [`Reads`] is that for a test: each spec's [`Container`] is opened on first
//! use and kept, and every entry read is kept too, because a scene asks for
//! the same texture once per material that binds it. What a test asserts is
//! unchanged; only how often the same bytes are fetched is.
//!
//! `tests/<dir>/mod.rs` is not a test target of its own, which is why this
//! is a directory (as `crates/rcs/tests/rcsmodel_common/` is).

#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::HashMap;

use oag_assets::Container;

/// Open archives and the entries already read out of them, by spec.
#[derive(Default)]
pub struct Reads {
    open: RefCell<HashMap<String, Option<Container>>>,
    read: RefCell<HashMap<(String, String), Option<Vec<u8>>>>,
}

impl Reads {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `mesh::read_blob(spec, name).ok()`, with the archive and the entry
    /// both kept for the next call.
    pub fn read(&self, spec: &str, name: &str) -> Option<Vec<u8>> {
        let key = (spec.to_string(), name.to_string());
        if let Some(hit) = self.read.borrow().get(&key) {
            return hit.clone();
        }
        let mut open = self.open.borrow_mut();
        let container = open
            .entry(spec.to_string())
            .or_insert_with(|| Container::open(spec).ok());
        let bytes = container.as_mut().and_then(|c| c.read_entry(name).ok());
        self.read.borrow_mut().insert(key, bytes.clone());
        bytes
    }

    /// The first of `specs` that holds `name`, and its bytes.
    pub fn read_any<'a>(&self, specs: &'a [String], name: &str) -> Option<(&'a str, Vec<u8>)> {
        specs
            .iter()
            .find_map(|spec| self.read(spec, name).map(|bytes| (spec.as_str(), bytes)))
    }
}

thread_local! {
    static READS: Reads = Reads::new();
}

/// [`Reads::read`] on one cache for the whole test.
///
/// `cargo nextest` runs every test in a process of its own, so this lives
/// exactly as long as the test that fills it and is never shared between two.
pub fn read(spec: &str, name: &str) -> Option<Vec<u8>> {
    READS.with(|reads| reads.read(spec, name))
}
