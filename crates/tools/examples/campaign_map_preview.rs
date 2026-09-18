//! Scratch probe: composite Wipeout 2048's `FE3DCanvas` hotspot icons onto a
//! blank 960x544 canvas, at each `<CanvasLabel>`'s authored `x`/`y`.
//!
//! `cargo run -p oag-tools --example campaign_map_preview -- <base data.psarc> <out.png>`
//!
//! Each `CanvasLabel`'s `u`/`v` is a top-left anchor into `canvasTexture.gxt`,
//! confirmed by cropping at `(u*2048, v*2048)` for the `Trophy-*` labels and
//! landing exactly on the trophy-cup glyph (see `docs/formats/2048-frontend.md`).
//! What `width`/`height` mean is not confirmed - several labels share
//! identical `u`/`v`/`width`/`height` while linking different events, so
//! those two fields select an icon rather than size a per-label crop, and a
//! naive `width * k` box does not bound the glyph cleanly at any `k` tried.
//! This probe sidesteps that by trimming to the actual non-background pixels
//! near the anchor instead of trusting an unconfirmed formula - real data
//! deciding the crop, not an invented number.

const CANVAS_WIDTH: u32 = 960;
const CANVAS_HEIGHT: u32 = 544;
const SEARCH_WINDOW: u32 = 220;

fn main() {
    let mut args = std::env::args().skip(1);
    let psarc_path = args
        .next()
        .expect("usage: campaign_map_preview <base data.psarc> <out.png>");
    let out_path = args
        .next()
        .expect("usage: campaign_map_preview <base data.psarc> <out.png>");

    let mut archive = oag_assets::psarc::Archive::open(&psarc_path)
        .unwrap_or_else(|e| panic!("open {psarc_path}: {e}"));

    let xml_bytes = archive
        .read_path("data/plugins/frontend/NEWGUI/Definition.xml")
        .expect("read Definition.xml");
    let xml = String::from_utf8(xml_bytes).expect("Definition.xml is not UTF-8");
    let doc = oag_tables::fexml::parse(&xml);

    let atlas_bytes = archive
        .read_path("data/FE/NewImages/canvasTexture.gxt")
        .expect("read canvasTexture.gxt");
    let atlas_gxt = oag_texture::gxt::Gxt::parse(&atlas_bytes).expect("parse canvasTexture.gxt");
    let atlas_texture = atlas_gxt.only().expect("one texture in canvasTexture.gxt");
    let (atlas_w, atlas_h) = atlas_texture.level_size(0);
    let atlas_rgba: Vec<[u8; 4]> = atlas_texture
        .to_rgba(&atlas_bytes)
        .expect("decode canvasTexture.gxt");

    let mut canvas = vec![[0xff_u8, 0xff, 0xff, 0xff]; (CANVAS_WIDTH * CANVAS_HEIGHT) as usize];

    let mut placed = 0usize;
    for canvas_3d in find_all(&doc, "FE3DCanvas") {
        for label in canvas_3d.children_named("CanvasLabel") {
            let (Some(x), Some(y), Some(u), Some(v)) = (
                parse_attr(label.value("x")),
                parse_attr(label.value("y")),
                parse_attr::<f64>(label.value("u")),
                parse_attr::<f64>(label.value("v")),
            ) else {
                continue;
            };

            let anchor_x = (u * f64::from(atlas_w)).round() as u32;
            let anchor_y = (v * f64::from(atlas_h)).round() as u32;
            let Some((sx, sy, sw, sh)) = trim_to_content(
                &atlas_rgba,
                atlas_w,
                atlas_h,
                anchor_x,
                anchor_y,
                SEARCH_WINDOW,
            ) else {
                continue;
            };

            blit(
                &atlas_rgba,
                atlas_w,
                sx,
                sy,
                sw,
                sh,
                &mut canvas,
                CANVAS_WIDTH,
                CANVAS_HEIGHT,
                x,
                y,
            );
            placed += 1;
        }
    }

    let flat: Vec<u8> = canvas.into_iter().flatten().collect();
    let png = oag_texture::png::encode_rgba(CANVAS_WIDTH, CANVAS_HEIGHT, &flat);
    std::fs::write(&out_path, png).unwrap_or_else(|e| panic!("write {out_path}: {e}"));
    println!(
        "{out_path}: {placed} CanvasLabel icons placed on a {CANVAS_WIDTH}x{CANVAS_HEIGHT} canvas"
    );
}

fn find_all<'a>(node: &'a oag_tables::fexml::Node, name: &str) -> Vec<&'a oag_tables::fexml::Node> {
    let mut out = Vec::new();
    if node.name.eq_ignore_ascii_case(name) {
        out.push(node);
    }
    for child in &node.children {
        out.extend(find_all(child, name));
    }
    out
}

fn parse_attr<T: std::str::FromStr>(value: Option<&str>) -> Option<T> {
    value.and_then(|v| v.trim().parse().ok())
}

/// Whether a texel counts as background: opaque near-white, or fully
/// transparent. Everything else is "content".
fn is_background(px: [u8; 4]) -> bool {
    px[3] < 8 || (px[0] > 240 && px[1] > 240 && px[2] > 240)
}

/// Finds the tight bounding box of non-background texels reachable from
/// `(anchor_x, anchor_y)` within a `window`x`window` search box, scanning
/// row/column emptiness rather than flood-filling - cheap, and the atlas
/// packs icons with enough whitespace between them that a bounding-box trim
/// does not bleed into a neighbour (checked visually per icon this probe
/// placed).
fn trim_to_content(
    rgba: &[[u8; 4]],
    atlas_w: u32,
    atlas_h: u32,
    anchor_x: u32,
    anchor_y: u32,
    window: u32,
) -> Option<(u32, u32, u32, u32)> {
    let x0 = anchor_x.min(atlas_w.saturating_sub(1));
    let y0 = anchor_y.min(atlas_h.saturating_sub(1));
    let x1 = (x0 + window).min(atlas_w);
    let y1 = (y0 + window).min(atlas_h);

    let at = |x: u32, y: u32| rgba[(y * atlas_w + x) as usize];

    let (mut min_x, mut min_y) = (x1, y1);
    let (mut max_x, mut max_y) = (x0, y0);
    let mut found = false;

    for y in y0..y1 {
        for x in x0..x1 {
            if !is_background(at(x, y)) {
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if !found {
        return None;
    }
    Some((min_x, min_y, max_x - min_x + 1, max_y - min_y + 1))
}

#[allow(clippy::too_many_arguments)]
fn blit(
    src: &[[u8; 4]],
    src_w: u32,
    sx: u32,
    sy: u32,
    sw: u32,
    sh: u32,
    dst: &mut [[u8; 4]],
    dst_w: u32,
    dst_h: u32,
    dx: u32,
    dy: u32,
) {
    for row in 0..sh {
        let dst_y = dy + row;
        if dst_y >= dst_h {
            break;
        }
        for col in 0..sw {
            let dst_x = dx + col;
            if dst_x >= dst_w {
                break;
            }
            let px = src[((sy + row) * src_w + sx + col) as usize];
            if is_background(px) {
                continue;
            }
            dst[(dst_y * dst_w + dst_x) as usize] = px;
        }
    }
}
