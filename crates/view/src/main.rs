//! Views assets decoded straight from a Wipeout disc image.
//!
//! ```sh
//! oag-view data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad
//! ```
//!
//! Left and right arrows change asset, Escape quits. With `--mesh` or
//! `--track`, arrow keys orbit the camera instead and `+`/`-` (or
//! PageUp/PageDown) zoom. Nothing is written to disk, and the disc image is
//! opened read-only.
//!
//! This is the first thing in the project with a window, and it exists to prove
//! the whole pipeline end to end: CHD, ISO 9660, WAD, LZSS, texture decode,
//! GPU. If it draws the right picture, every layer beneath it is right.

mod assets;
mod draws;
mod logging;
mod offscreen;
mod orbit;
mod ps3_mesh;

use anyhow::{Context, Result};
use clap::Parser;
use log::warn;
use std::path::PathBuf;
use std::sync::Arc;

// The mesh loader, the track-ribbon builder and the pipeline that draws them
// live in `oag-render`, so the game draws through the same code. What is left
// here is the viewer: the CLI, the window and the texture browser.
use oag_assets::Archive;
use oag_mesh::mesh;
use oag_mesh::mesh_render;
use oag_mesh::mesh_render::Anisotropy;
use oag_render::{collision, track};
use oag_vex::vex;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

#[derive(Parser, Debug)]
#[command(
    name = "oag-view",
    about = "View Wipeout assets from a disc image",
    version
)]
struct Cli {
    #[command(flatten)]
    log: oag_log::tool::LogArgs,

    /// `<image>:<path-on-disc>`, for example
    /// `data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad`.
    archive: String,

    /// A file of candidate names, one per line, used to label assets.
    #[arg(long)]
    names: Option<PathBuf>,

    /// Render one asset to a PNG and exit, without opening a window.
    ///
    /// Runs headless, so it works over SSH and in CI, and gives the render
    /// path something a test can assert on.
    #[arg(long)]
    screenshot: Option<PathBuf>,

    /// Which asset the screenshot uses.
    #[arg(long, default_value_t = 0)]
    index: usize,

    /// Render a `.vex` model instead of textures, by its name in the archive.
    #[arg(long)]
    mesh: Option<String>,

    /// Render a track's `Skycube` instead of its art meshes, by the `.vex` name
    /// in the archive.
    ///
    /// The sky is a separate model from the track's geometry even though it
    /// lives in the same file, because the game draws it camera-centred and
    /// without depth. Seeing it alone is how you tell a five-faced cube from a
    /// six-faced one.
    #[arg(long)]
    sky: Option<String>,

    /// Render a track's `Speedup Pad` geometry instead of its art meshes, by the
    /// `.vex` name in the archive.
    ///
    /// A pad is a `Mesh` subclass, so its geometry sits in the track file
    /// alongside everything else and is invisible in the art-mesh view - nothing
    /// distinguishes it once drawn. Seeing the pads alone is how you check that
    /// the transform chain placed them apart from each other and around a
    /// circuit, which is the half of the decode a screenshot of a race cannot
    /// show.
    #[arg(long)]
    pads: Option<String>,

    /// Skin a `--mesh`, `--sky` or `--pads` from an external texture set, by
    /// entry index or `0x`-prefixed name hash.
    ///
    /// PS2 models carry no textures of their own: their texture set is a separate archive entry, a
    /// nested WAD keyed by each `Texture` node's own declared name. A track's art mesh, its
    /// `Skycube` and its `Speedup Pad` geometry share one file and one set - see
    /// `mesh::build_sky`'s doc comment. In `oag-game`, which entry is which model's set is found by
    /// directory position (`ps2-texture.md`); here it is named by hand.
    #[arg(long)]
    textures: Option<String>,

    /// Camera yaw in radians, for `--mesh` and `--track`.
    #[arg(long, default_value_t = 0.9)]
    yaw: f32,

    /// With `--screenshot`, how far into their authored loops to sample the
    /// model's texture-transform tracks, in seconds.
    ///
    /// A window animates these from its own clock; a capture is one frame and
    /// has to be told which. Two captures at two times, differenced, is the
    /// only headless way to see that an animated surface moves at all - and
    /// the rates are authored per material, so a time that shows one surface
    /// mid-sweep may leave another at its first key.
    #[arg(long, default_value_t = 0.0)]
    anim_seconds: f32,

    /// Camera pitch in radians. Near zero looks along the ground, near 1.5
    /// straight down, which is what a track wants and a model does not.
    #[arg(long)]
    pitch: Option<f32>,

    /// Render a track's driveable spline instead of its art meshes, by the
    /// `.vex` name in the archive.
    ///
    /// Draws the track surface from the spline's own half-widths, coloured per
    /// visibility section, with the racing line and AI corridor at hover height
    /// above it.
    #[arg(long)]
    track: Option<String>,

    /// Render a track's collision soup - the geometry the physics world is
    /// actually made of - by the `.vex` name in the archive.
    ///
    /// Triangle outlines by default, coloured per collision class, so geometry
    /// behind a wall stays visible. Per-class counts and extents are printed,
    /// which is what makes this checkable without looking at the picture.
    #[arg(long)]
    collision: Option<String>,

    /// Draw `--collision` filled instead of as outlines.
    ///
    /// Reads better for a single surface, and hides everything inside a closed
    /// wall: the pipeline is opaque and depth-tested, so there is no seeing
    /// through it.
    #[arg(long)]
    solid: bool,

    /// Include `Cage` nodes in `--collision`.
    ///
    /// Off by default because a cage is not collidable - the original parses
    /// cage nodes and branches past them - so the default picture is what the
    /// physics world contains rather than what the file holds.
    #[arg(long)]
    cage: bool,

    /// Overlay the driveable ribbon on `--collision`, from the same file.
    ///
    /// The point of the collision view: the walls should enclose the ribbon.
    #[arg(long)]
    with_spline: bool,

    /// Anisotropic filtering level for `--mesh` and `--track`: off, 2x, 4x,
    /// 8x or 16x.
    #[arg(long, default_value_t = Anisotropy::default())]
    anisotropy: Anisotropy,

    /// Print every node of a `.vex` file - class, name, size, tree position -
    /// and exit, drawing nothing.
    ///
    /// The parser has always returned every node regardless of class; until this
    /// flag there was no way to look at the ones nothing decodes. That is how
    /// the effects classes stayed unexamined while `docs/formats/vex.md` listed
    /// their names.
    #[arg(long)]
    nodes: Option<String>,

    /// Restrict `--nodes` to one class, by decimal id or `0x`-prefixed hex.
    #[arg(long)]
    class: Option<String>,

    /// Hex-dump each shown node's payload under it, for classes nothing decodes.
    ///
    /// `--nodes` prints a payload's *size*, which is enough to tell a locator
    /// from a parameter block but not enough to read either. Pair with `--class`
    /// or the dump runs to the whole file.
    #[arg(long)]
    payload: bool,

    /// Stop each `--payload` dump after this many bytes. 0 dumps all of it.
    #[arg(long, default_value_t = 256)]
    payload_bytes: usize,

    /// Print every draw call of a `--mesh`, `--sky` or `--pads` model - list,
    /// node, material, texture ordinal, decoded texture, vertex colour - and
    /// the material chain behind it.
    ///
    /// A screenshot says a surface looks wrong; this says which draw made it.
    /// The line that usually matters is a draw that resolves no texture: those
    /// bind `mesh_render::build`'s white 1x1 and paint white whatever their
    /// material meant to paint.
    #[arg(long)]
    draws: bool,

    /// Draw only the parts of a `--mesh`, `--sky` or `--pads` model whose node
    /// name or texture label contains this text, case-insensitively.
    ///
    /// Node-level isolation: hide everything else and the remaining pixels can
    /// only have come from what is left, which is what turns `--draws`'
    /// attribution into a confirmation. The camera reframes onto what survives.
    #[arg(long)]
    only: Option<String>,
}

/// Hex-dumps one node payload, 16 bytes to a line with an ASCII gutter.
///
/// Deliberately the classic `hexdump -C` shape rather than anything cleverer:
/// the point is to diff two payloads by eye and to spot floats and embedded
/// strings, and every other tool a reader might paste this into expects it.
fn hexdump(bytes: &[u8], limit: usize) {
    let shown = if limit == 0 {
        bytes
    } else {
        &bytes[..bytes.len().min(limit)]
    };
    for (offset, chunk) in shown.chunks(16).enumerate() {
        let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        let text: String = chunk
            .iter()
            .map(|&b| if b.is_ascii_graphic() { b as char } else { '.' })
            .collect();
        println!("      {:04x}  {:<47}  {text}", offset * 16, hex.join(" "));
    }
    if shown.len() < bytes.len() {
        println!(
            "      ... {} more byte(s); raise --payload-bytes to see them",
            bytes.len() - shown.len()
        );
    }
}

/// Prints a `.vex` file's node tree, and a per-class census under it.
///
/// The census is the part worth reading: a count per class says what a file *is*
/// in one screen, and an unnamed class ID is a prompt to go back to the table in
/// [`oag_vex::vex::class_name`] rather than a defect.
fn report_nodes(data: &[u8], label: &str, only: Option<u32>, payload: Option<usize>) -> Result<()> {
    let nodes = vex::nodes(data).with_context(|| format!("walking {label}"))?;
    println!("{label}: {} node(s)", nodes.len());

    let shown: Vec<&vex::Node> = match only {
        Some(id) => vex::nodes_by_class(&nodes, id).collect(),
        None => nodes.iter().collect(),
    };
    if let Some(id) = only {
        let name = vex::class_name(id).unwrap_or("unnamed class");
        println!("  filtered to 0x{id:03x} {name}: {} node(s)", shown.len());
    }

    for node in &shown {
        // Indent by depth so the tree reads as a tree; the parent index is
        // printed as well because a filtered view loses the indentation's
        // meaning entirely.
        let indent = "  ".repeat(node.depth.min(16));
        let class = vex::class_name(node.class_id).unwrap_or("?");
        // A `Texture` node keeps its runtime path in the payload, and on a track
        // that is the only name it has: the node header is the short 32-byte
        // form with the name field zeroed. Falling back to it here is what makes
        // a track's texture list readable at all.
        let payload_name = (node.class_id == vex::CLASS_TEXTURE)
            .then(|| vex::texture_asset_path(&data[node.payload()]))
            .flatten();
        let name = payload_name
            .as_deref()
            .or(node.name.as_deref())
            .unwrap_or("");
        println!(
            "  {indent}0x{:03x} {class:<24} {name:<32} \
             {} byte(s), {} child(ren), parent {:?}",
            node.class_id, node.data_size, node.child_count, node.parent,
        );
        if let Some(limit) = payload {
            hexdump(&data[node.payload()], limit);
        }
    }

    // Census over the whole file, not over the filtered view: the point of it is
    // to say what is present, and a filter has already decided that.
    let mut census: Vec<(u32, usize)> = Vec::new();
    for node in &nodes {
        match census.iter_mut().find(|(id, _)| *id == node.class_id) {
            Some((_, count)) => *count += 1,
            None => census.push((node.class_id, 1)),
        }
    }
    census.sort_unstable_by_key(|(id, _)| *id);

    println!("  class census:");
    for (id, count) in census {
        let name = vex::class_name(id).unwrap_or("unnamed - not in the read table");
        println!("    0x{id:03x} {name:<28} {count:>5}");
    }
    Ok(())
}

/// Prints the per-class breakdown of a track's collision soup.
///
/// Extents are the interesting part, not the counts: the sweep-and-prune reading
/// in `docs/formats/collision.md` packs a 2,048-unit window, and this is the
/// cheapest place to measure the shipped geometry against it. The survey it
/// produced found seven of sixteen tracks reaching past ±1024 but not one
/// spanning more than 2,048, so a flagged reach is a finding about the packing's
/// origin, not about the file.
fn report_collision(nodes: &[oag_vex::collision::CollisionNode]) {
    for k in collision::stats(nodes) {
        let collidable = if collision::is_collidable(k.kind) {
            ""
        } else {
            " (not collidable)"
        };
        match k.bounds {
            None => println!("  {:<9} -{collidable}", k.kind.node_name()),
            Some((lo, hi)) => println!(
                "  {:<9} {:>3} node(s), {:>4} mesh(es), {:>7} tri, \
                 x {:>9.1}..{:<9.1} y {:>9.1}..{:<9.1} z {:>9.1}..{:<9.1}{collidable}",
                k.kind.node_name(),
                k.nodes,
                k.meshes,
                k.triangles,
                lo[0],
                hi[0],
                lo[1],
                hi[1],
                lo[2],
                hi[2],
            ),
        }
    }

    if let Some((lo, hi)) = collision::bounds_of(nodes, collision::is_collidable) {
        let reach = (0..3).fold(0.0f32, |m, i| m.max(lo[i].abs().max(hi[i].abs())));
        let span = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
        let widest = span.iter().fold(0.0f32, |m, &s| m.max(s));
        println!("  collidable extent: {reach:.1} units from the origin at furthest");
        println!(
            "  collidable span:   {:.1} x {:.1} x {:.1}, widest axis {widest:.1}",
            span[0], span[1], span[2]
        );

        // Not a rendering concern, but this is the only place the numbers get
        // measured, so say so rather than leaving them in a screenshot.
        //
        // Both are printed because they answer different questions about the
        // sweep-and-prune packing. `((int)coord + 0x400) * 2` in 12 bits covers a
        // **2048-unit window centred on the origin**, so the reach is what decides
        // whether raw world coordinates fit, and the span is what decides whether
        // coordinates *relative to the geometry's own box* would. A track that
        // busts the first and clears the second is evidence the broadphase does
        // not pack world space.
        if reach > 1024.0 {
            println!(
                "  NOTE: reach is past +/-1024, so raw world coordinates do not \
                 fit the packing in docs/formats/collision.md."
            );
        }
        if widest > 2048.0 {
            println!(
                "  NOTE: the widest span is past 2048, so no origin choice makes \
                 this track fit the packing."
            );
        }
    }
}

/// Applies `--draws` and `--only` to a freshly built model.
///
/// Both act on the same three draw lists and both want the file the model came
/// from (for node names), so they are one step rather than two scattered
/// through each `--mesh`/`--sky`/`--pads` branch.
fn inspect(model: mesh::Model, data: &[u8], cli: &Cli) -> mesh::Model {
    if cli.draws {
        draws::report(&model, data);
    }
    let Some(needle) = &cli.only else {
        return model;
    };
    let model = draws::isolate(model, data, needle);
    println!(
        "  --only {needle:?}: {} opaque, {} cutout, {} blend draw(s) kept",
        model.draws.len(),
        model.alpha_tested_draws.len(),
        model.transparent_draws.len()
    );
    model
}

/// Reads and decodes an external texture set out of the same archive.
fn load_texture_set(spec: &str, entry: &str) -> Result<mesh::Ps2TextureSet> {
    let mut archive = Archive::open(spec)?;
    let index = if let Some(hex) = entry.strip_prefix("0x") {
        let hash = u32::from_str_radix(hex, 16).context("parsing the entry hash")?;
        archive
            .index_of_hash(hash)
            .with_context(|| format!("{hex} is not in {}", archive.label()))?
    } else if let Ok(index) = entry.parse::<usize>() {
        index
    } else {
        archive
            .index_of_name(entry)
            .with_context(|| format!("{entry} is not in {}", archive.label()))?
    };
    let blob = archive.read(index)?;
    mesh::Ps2TextureSet::parse(&blob).with_context(|| format!("entry {index} is not a texture set"))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    logging::start(&cli.log);

    if let Some(name) = &cli.nodes {
        let only = match &cli.class {
            None => None,
            Some(text) => Some(
                match text.strip_prefix("0x") {
                    Some(hex) => u32::from_str_radix(hex, 16),
                    None => text.parse::<u32>(),
                }
                .with_context(|| format!("{text} is not a class id"))?,
            ),
        };
        let data = mesh::read_blob(&cli.archive, name)?;
        return report_nodes(&data, name, only, cli.payload.then_some(cli.payload_bytes));
    }

    if let Some(name) = &cli.collision {
        let (nodes, label) = collision::load(&cli.archive, name)?;
        println!("{label}: {} collision node(s)", nodes.len());
        report_collision(&nodes);

        let style = if cli.solid {
            collision::Style::Solid
        } else {
            collision::Style::Wireframe
        };
        let soup = collision::build_model(&label, &nodes, style, cli.cage);
        println!(
            "  drawing {} triangle(s) as {}",
            soup.indices.len() / 3,
            if cli.solid { "solid" } else { "outlines" }
        );

        // A `.vex` can carry collision without a `WO Track` node, in which case
        // the overlay is unavailable but the collision view itself is fine.
        // Warn and draw what there is rather than failing the whole command.
        let ribbon = match (cli.with_spline, track::load(&cli.archive, name)) {
            (false, _) => None,
            (true, Ok((ai, _))) => Some(track::build_model(&label, &ai)),
            (true, Err(e)) => {
                warn!("--with-spline: {e:#}");
                None
            }
        };

        let model = if let Some(ribbon) = ribbon {
            if let (Some(soup), Some(spline)) = (
                collision::bounds_of(&nodes, collision::is_collidable),
                collision::bounds_of_model(&ribbon),
            ) {
                // A sanity check, not a proof of enclosure: on a looping track
                // both boxes are the whole envelope, so this catches a spline
                // that lands in a different place or a different scale, and
                // nothing finer.
                //
                // Deliberately against *all* collidable geometry rather than
                // walls alone, and deliberately loose on y. Walls are vertical
                // strips whose box is only as tall as the wall, while the ribbon
                // raises the racing line and corridor by `track::HOVER_LIFT`, so
                // a wall-only y comparison reports a failure on correct data.
                let slack = [1.0, oag_vex::track::HOVER_LIFT * 4.0, 1.0];
                let within = (0..3).all(|i| {
                    soup.0[i] - slack[i] <= spline.0[i] && spline.1[i] <= soup.1[i] + slack[i]
                });
                println!(
                    "  collidable box {:?}..{:?}\n  spline box     {:?}..{:?}\n  \
                     spline AABB is {} the collidable AABB (x/z exact, y with hover slack)",
                    soup.0,
                    soup.1,
                    spline.0,
                    spline.1,
                    if within { "within" } else { "NOT within" }
                );
            }
            mesh::merge(&label, vec![soup, ribbon])
        } else {
            soup
        };

        // Tested on the merged model rather than on `nodes`, because
        // `--with-spline` can supply geometry a file with no collision class of
        // its own still has. Every `.vex` on the Pure disc reaches this, and so
        // does any Pulse file that is not a track - see
        // `docs/formats/pure-status.md`. An error rather than a blank frame: the
        // render path tolerates an empty model, but a screenshot of nothing is
        // not what the command was asked for, and a scripted caller wants the
        // non-zero exit.
        if model.vertices.is_empty() || model.indices.is_empty() {
            let cage_would_help =
                !cli.cage && nodes.iter().any(|n| !collision::is_collidable(n.kind));
            anyhow::bail!(
                "{label}: nothing to draw - no collision geometry{}",
                if cage_would_help {
                    ", only cage nodes (pass --cage to see them)"
                } else {
                    ""
                }
            );
        }

        // Tracks are flat and wide, so look down at them rather than along.
        let pitch = cli.pitch.unwrap_or(1.15);
        if let Some(path) = &cli.screenshot {
            mesh_render::capture_from(
                &model,
                path,
                1280,
                960,
                cli.yaw,
                pitch,
                cli.anisotropy,
                cli.anim_seconds,
            )?;
            println!("wrote {}", path.display());
            return Ok(());
        }
        println!("arrows orbit, +/- (or PageUp/PageDown) zoom, escape quits");
        orbit::run(model, cli.yaw, pitch, cli.anisotropy)?;
        return Ok(());
    }

    if let Some(name) = &cli.track {
        let (ai, label) = track::load(&cli.archive, name)?;
        println!(
            "{label}: version {:#x}, {} paths, {} junctions, {} control points",
            ai.version,
            ai.paths.len(),
            ai.junctions.len(),
            ai.point_count()
        );
        for (i, path) in ai.paths.iter().enumerate() {
            println!(
                "  path {i}: {} points, longest gap {:.2}, junctions {:?} -> {:?}",
                path.points.len(),
                path.max_spacing,
                path.entry,
                path.exit
            );
        }
        let model = track::build_model(&label, &ai);
        // Tracks are flat and wide, so look down at them rather than along.
        let pitch = cli.pitch.unwrap_or(1.15);
        if let Some(path) = &cli.screenshot {
            mesh_render::capture_from(
                &model,
                path,
                1280,
                960,
                cli.yaw,
                pitch,
                cli.anisotropy,
                cli.anim_seconds,
            )?;
            println!("wrote {}", path.display());
            return Ok(());
        }
        println!("arrows orbit, +/- (or PageUp/PageDown) zoom, escape quits");
        orbit::run(model, cli.yaw, pitch, cli.anisotropy)?;
        return Ok(());
    }

    if let Some(name) = &cli.sky {
        let external = cli
            .textures
            .as_deref()
            .map(|entry| load_texture_set(&cli.archive, entry))
            .transpose()?;
        if let Some(set) = &external {
            println!(
                "texture set: {} of {} entries decoded",
                set.decoded_count(),
                set.entry_count()
            );
        }
        let data = mesh::read_blob(&cli.archive, name)?;
        let sky = mesh::build_sky(name, &data, external.as_ref())?;
        let model = inspect(sky, &data, &cli);
        println!(
            "{}: {} sky node(s), {} vertices, {} triangles, radius {:.2}",
            model.label,
            model.mesh_count,
            model.vertices.len(),
            model.indices.len() / 3,
            model.radius
        );
        if model.mesh_count == 0 {
            println!("no Skycube node; this file authors no sky");
            return Ok(());
        }
        // Inside a sky cube looking out, which is where the camera always is.
        // The orbit viewer's default framing puts the camera outside the
        // bounding sphere, and from there a sky is a small box with its back
        // faces towards you.
        let pitch = cli.pitch.unwrap_or(0.0);
        if let Some(path) = &cli.screenshot {
            mesh_render::capture_from(
                &model,
                path,
                960,
                720,
                cli.yaw,
                pitch,
                cli.anisotropy,
                cli.anim_seconds,
            )?;
            println!("wrote {}", path.display());
            return Ok(());
        }
        println!("arrows orbit, +/- (or PageUp/PageDown) zoom, escape quits");
        orbit::run(model, cli.yaw, pitch, cli.anisotropy)?;
        return Ok(());
    }

    if let Some(name) = &cli.pads {
        let external = cli
            .textures
            .as_deref()
            .map(|entry| load_texture_set(&cli.archive, entry))
            .transpose()?;
        if let Some(set) = &external {
            println!(
                "texture set: {} of {} entries decoded",
                set.decoded_count(),
                set.entry_count()
            );
        }
        let data = mesh::read_blob(&cli.archive, name)?;
        let pads = mesh::build_pads(name, &data, external.as_ref())?;
        let model = inspect(pads, &data, &cli);
        println!(
            "{}: {} Speedup Pad node(s), {} vertices, {} triangles, radius {:.2}",
            model.label,
            model.mesh_count,
            model.vertices.len(),
            model.indices.len() / 3,
            model.radius
        );
        if model.mesh_count == 0 {
            println!("no Speedup Pad node; this file authors no pads");
            return Ok(());
        }
        // Pads ring a circuit rather than filling a volume, so the informative
        // view is from above: looking along the ground puts them all edge-on to
        // each other.
        let pitch = cli.pitch.unwrap_or(1.4);
        if let Some(path) = &cli.screenshot {
            mesh_render::capture_from(
                &model,
                path,
                960,
                720,
                cli.yaw,
                pitch,
                cli.anisotropy,
                cli.anim_seconds,
            )?;
            println!("wrote {}", path.display());
            return Ok(());
        }
        println!("arrows orbit, +/- (or PageUp/PageDown) zoom, escape quits");
        orbit::run(model, cli.yaw, pitch, cli.anisotropy)?;
        return Ok(());
    }

    if let Some(name) = &cli.mesh {
        let external = cli
            .textures
            .as_deref()
            .map(|entry| load_texture_set(&cli.archive, entry))
            .transpose()?;
        if let Some(set) = &external {
            println!(
                "texture set: {} of {} entries decoded",
                set.decoded_count(),
                set.entry_count()
            );
        }
        let data = mesh::read_blob(&cli.archive, name)?;
        let ps3 = mesh::rcs::scene_from(&cli.archive, name, &data)?
            .inspect(|(_, report)| println!("{}", report.describe()));
        let model = match ps3 {
            Some((model, _)) => ps3_mesh::with_pads(&cli.archive, name, &data, model),
            None => mesh::build_with_textures(name, &data, external.as_ref())?,
        };
        // No race camera switches a viewed model: it shows the finest
        // `LodGroup` tier, what the original shows up close.
        let mut model = model;
        model.keep_nearest();
        let model = inspect(model, &data, &cli);
        println!(
            "{}: {} meshes, {} vertices, {} triangles, radius {:.2}",
            model.label,
            model.mesh_count,
            model.vertices.len(),
            model.indices.len() / 3,
            model.radius
        );
        let pitch = cli.pitch.unwrap_or(0.35);
        if let Some(path) = &cli.screenshot {
            mesh_render::capture_from(
                &model,
                path,
                960,
                720,
                cli.yaw,
                pitch,
                cli.anisotropy,
                cli.anim_seconds,
            )?;
            println!("wrote {}", path.display());
            return Ok(());
        }
        println!("arrows orbit, +/- (or PageUp/PageDown) zoom, escape quits");
        orbit::run(model, cli.yaw, pitch, cli.anisotropy)?;
        return Ok(());
    }

    let assets = assets::load(&cli.archive, cli.names.as_deref())?;
    println!("{} texture(s) loaded from {}", assets.len(), cli.archive);

    if let Some(path) = cli.screenshot {
        let asset = assets
            .get(cli.index)
            .with_context(|| format!("index {} of {}", cli.index, assets.len()))?;
        offscreen::capture(asset, &path)?;
        println!(
            "wrote {} ({}x{})",
            path.display(),
            asset.width,
            asset.height
        );
        return Ok(());
    }

    println!("left/right to change, escape to quit");

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App {
        assets,
        current: 0,
        state: None,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

struct App {
    assets: Vec<assets::Asset>,
    current: usize,
    state: Option<Renderer>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("oag-view")
            .with_inner_size(winit::dpi::LogicalSize::new(960, 640));

        match event_loop
            .create_window(attributes)
            .context("creating the window")
            .and_then(|window| {
                // Texture/mesh browsing is keyboard-only; the cursor has
                // nothing to click on.
                window.set_cursor_visible(false);
                Renderer::new(Arc::new(window))
            }) {
            Ok(mut renderer) => {
                renderer.show(&self.assets[self.current]);
                self.state = Some(renderer);
            }
            Err(e) => {
                eprintln!("error: {e:#}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(renderer) = self.state.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                renderer.resize(size.width, size.height);
                renderer.window.request_redraw();
            }

            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let count = self.assets.len();
                let step = match event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        event_loop.exit();
                        return;
                    }
                    // Wrapping both ways, so browsing never dead-ends.
                    Key::Named(NamedKey::ArrowRight) => 1,
                    Key::Named(NamedKey::ArrowLeft) => count - 1,
                    _ => return,
                };

                self.current = (self.current + step) % count;
                renderer.show(&self.assets[self.current]);
                renderer.window.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                if let Err(e) = renderer.render() {
                    eprintln!("render error: {e:#}");
                    event_loop.exit();
                }
            }

            _ => {}
        }
    }
}

/// Uniforms shared with `shader.wesl`.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    scale: [f32; 2],
    surface: [f32; 2],
}

struct Renderer {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,
    bind_group: Option<wgpu::BindGroup>,
    /// Aspect ratio of the current texture, for letterboxing.
    aspect: f32,
}

impl Renderer {
    fn new(window: Arc<Window>) -> Result<Self> {
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .context("creating the surface")?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .context("no suitable GPU adapter (is a Vulkan driver installed?)")?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &oag_mesh::mesh_render::device_descriptor("oag-view", &adapter),
        ))
        .context("requesting the device")?;

        let size = window.inner_size();
        // The viewer draws through the same pipeline the game does, so it
        // works in the same space: gamma, with nothing encoding on write. See
        // [ADR-0020](../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md).
        // A viewer left on an sRGB surface would show every mesh differently
        // from the game and stop being usable as a reference, which is the
        // whole point of it.
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .context("surface is not supported by this adapter")?;
        config.format = config.format.remove_srgb_suffix();
        config.view_formats = vec![config.format];
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewer"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!(concat!(env!("OUT_DIR"), "/shader.wgsl")).into(),
            ),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewer"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("viewer"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("viewer"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(config.format.into())],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // Nearest filtering: these are small pixel-art textures, and smoothing
        // them would hide exactly the decoding errors this tool exists to find.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("viewer"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("viewer uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            window,
            device,
            queue,
            surface,
            config,
            pipeline,
            layout,
            sampler,
            uniform_buffer,
            bind_group: None,
            aspect: 1.0,
        })
    }

    /// Uploads an asset and makes it current.
    fn show(&mut self, asset: &assets::Asset) {
        let size = wgpu::Extent3d {
            width: asset.width,
            height: asset.height,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("asset"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Raw, and for the same reason as before by a shorter route: the
            // palettes hold non-linear colour, and the surface no longer
            // encodes ([ADR-0020](../../../docs/architecture/adr/0020-gamma-authoritative-colour-space.md)),
            // so handing the sampler's output straight to the screen shows the
            // palette's own bytes. Declaring it sRGB here would linearise on
            // read with nothing to encode on write, which is the washed-out
            // case that comment used to be about.
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            texture.as_image_copy(),
            &asset.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(asset.width * 4),
                rows_per_image: Some(asset.height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.bind_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("asset"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        }));

        self.aspect = asset.width as f32 / asset.height as f32;
        self.window.set_title(&format!(
            "oag-view - {}  {}x{}",
            asset.label, asset.width, asset.height
        ));
        println!("{}  {}x{}", asset.label, asset.width, asset.height);
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    fn render(&mut self) -> Result<()> {
        let Some(bind_group) = &self.bind_group else {
            return Ok(());
        };

        // Letterbox: shrink along whichever axis would otherwise stretch, so
        // the texture keeps its proportions whatever the window is doing.
        let surface_aspect = self.config.width as f32 / self.config.height as f32;
        let scale = if surface_aspect > self.aspect {
            [self.aspect / surface_aspect, 1.0]
        } else {
            [1.0, surface_aspect / self.aspect]
        };

        let uniforms = Uniforms {
            scale,
            surface: [self.config.width as f32, self.config.height as f32],
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // A frame can legitimately be unavailable: the surface may be occluded,
        // outdated after a resize, or timed out. None of those are errors, so
        // skip the frame rather than tearing the window down.
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            other => {
                eprintln!("skipping frame: {other:?}");
                return Ok(());
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);
        Ok(())
    }
}
