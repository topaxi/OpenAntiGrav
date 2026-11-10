use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Context as _;

use super::*;

/// Failures only the page can tell: they happen before the module loads, or
/// are a trap the module cannot report about itself.
const PAGE_ONLY: [&str; 7] = [
    "trap",
    "out-of-memory",
    "no-webgpu",
    "no-isolation",
    "module-load",
    "wasm-unsupported",
    "wasm-memory",
];

fn source() -> wgpu::ErrorSource {
    Box::new(std::fmt::Error)
}

#[test]
fn a_gpu_error_is_told_by_its_variant() {
    assert_eq!(
        gpu_error(&wgpu::Error::OutOfMemory { source: source() }),
        Kind::GpuOutOfMemory
    );
    assert_eq!(
        gpu_error(&wgpu::Error::Validation {
            source: source(),
            description: String::new(),
        }),
        Kind::GpuValidation
    );
    assert_eq!(
        gpu_error(&wgpu::Error::Internal {
            source: source(),
            description: String::new(),
        }),
        Kind::GpuInternal
    );
}

#[test]
fn a_device_the_game_destroyed_is_not_a_failure() {
    assert_eq!(device_lost(wgpu::DeviceLostReason::Destroyed), None);
    assert_eq!(
        device_lost(wgpu::DeviceLostReason::Unknown),
        Some(Kind::GpuLost)
    );
}

#[test]
fn an_error_keeps_its_kind_under_more_context() {
    let tagged: anyhow::Result<()> = Err(anyhow::anyhow!("requestAdapter resolved null"))
        .context(Failure::new(Kind::NoAdapter, "no WebGPU adapter"))
        .context("starting the web build");
    let error = tagged.unwrap_err();
    assert_eq!(kind_of(&error), Kind::NoAdapter);
    assert!(format!("{error:#}").contains("requestAdapter resolved null"));
    assert_eq!(kind_of(&anyhow::anyhow!("anything else")), Kind::Startup);
}

#[test]
fn a_cut_short_image_is_unreadable_and_a_foreign_one_unsupported() {
    let cut = anyhow::Error::new(std::io::Error::from(std::io::ErrorKind::UnexpectedEof))
        .context("reading sector 40");
    assert_eq!(disc_error(&cut), Kind::DiscUnreadable);
    assert_eq!(
        disc_error(&anyhow::anyhow!("not a recognised image")),
        Kind::DiscUnsupported
    );
}

/// The `KINDS` rows of `web/fault.js`: lines of the form `  "tag": {`.
fn page_rows() -> BTreeSet<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/fault.js");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let table = text
        .split("const KINDS = {")
        .nth(1)
        .and_then(|rest| rest.split("\n};").next())
        .expect("web/fault.js has its KINDS table");
    table
        .lines()
        .filter_map(|line| {
            let tag = line.strip_prefix("  \"")?.strip_suffix("\": {")?;
            Some(tag.to_string())
        })
        .collect()
}

#[test]
fn every_kind_the_module_can_send_has_a_message_on_the_page() {
    let rows = page_rows();
    for kind in Kind::ALL {
        assert!(
            rows.contains(kind.tag()),
            "web/fault.js has no KINDS row for {:?} ({:?}): the page would show the generic one",
            kind,
            kind.tag()
        );
    }
}

#[test]
fn the_page_has_no_row_nothing_can_send() {
    let sent: BTreeSet<String> = Kind::ALL
        .iter()
        .map(|kind| kind.tag().to_string())
        .chain(PAGE_ONLY.iter().map(|tag| (*tag).to_string()))
        .collect();
    assert_eq!(page_rows(), sent);
}

#[test]
fn tags_are_distinct() {
    let tags: BTreeSet<_> = Kind::ALL.iter().map(|kind| kind.tag()).collect();
    assert_eq!(tags.len(), Kind::ALL.len());
}
