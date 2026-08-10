//! What the PS2 pressing's `Skin.xml` really does to the PSP's coordinates.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # Why this file exists
//!
//! `oag_game::frontend::Space::PS2` says the PS2 front end authors in a 640x448
//! grid. That conclusion is right, and the sentence that used to justify it was
//! not: it said the PS2 layout *is* the PSP's scaled by exactly the resolution
//! ratio, at confidence 95, on the strength of three samples and the file's two
//! extremes. Extremes cannot see an exception in the middle, which is the same
//! way the identical claim about `Arcade_HUD.xml` came to be wrong and had to be
//! corrected twice.
//!
//! So the measurement lives here instead of in a doc comment, coordinate by
//! coordinate, and the exceptions are named rather than counted. A count alone
//! would be a change-detector; the lists below say *which* coordinates break the
//! rule, and the day one of them starts scaling, this fails and points at it.
//!
//! # The finding worth carrying away
//!
//! **`x` and `y` do not mean one thing in this XML.** On most elements they are
//! a position in the grid, and those scale. On `<Key>` inside an `<Animation>`
//! they are a *relative* offset - how far the animation travels - and those are
//! byte-identical across the two consoles because a nudge of 24 pixels is
//! authored against the widget, not against the screen. `Arcade_HUD.xml`'s
//! `TimeDiffIcon` is the same shape in the HUD.
//!
//! Anything sweeping a PS2 layout for out-of-screen widgets has to know which
//! kind of `x` it is holding before it can check it against a screen at all.
//! This is what `oag_game::hud::inside_screen`'s doc comment warns about, and
//! this file is the second, independent instance of it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oag_formats::fexml;

/// The USA PSP pressing - **the one the PS2 layout was derived from**, which
/// [`the_two_psp_pressings_differ_in_exactly_one_coordinate`] is here to show.
const PSP_USA: &str = "data/images/pulse-psp-usa.chd";
/// The EU PSP pressing.
const PSP_EU: &str = "data/images/pulse-psp-eu.chd";
/// The PS2 pressing, which is a EU one - the only one this project has.
const PS2: &str = "data/images/pulse-ps2-eu.chd";

/// Every `x`/`y` in one `Skin.xml`, keyed by where in the tree it sits.
type Coords = BTreeMap<String, f32>;

fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(name);
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// Reads and expands a disc's front-end root.
///
/// The PS2 pressing stores the same entry unshortened, so the dictionary step is
/// conditional exactly as it is in `oag_game::boot`.
fn skin(name: &str) -> Option<Coords> {
    let path = image(name)?;
    let mut archives = oag_pulse::open(&path.display().to_string())
        .unwrap_or_else(|e| panic!("{name} opens as Pulse: {e}"));
    let blob = archives
        .read_name(oag_pulse::names::FRONTEND_ROOT)
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    let xml = if fexml::is_fexml(&blob) {
        fexml::expand(&blob).expect("expanding the front-end XML")
    } else {
        String::from_utf8(blob).expect("the front-end XML is text")
    };

    // Walked from the root's children rather than from the root, which is the
    // parser's synthetic `#document` and would prefix every key with itself.
    let mut out = Coords::new();
    let mut seen = BTreeMap::new();
    for child in &fexml::parse(&xml).children {
        walk(child, "", &mut out, &mut seen);
    }
    assert!(!out.is_empty(), "{name}: Skin.xml carries no coordinates");
    Some(out)
}

/// A node's own segment of the key: its `name` attribute where it has one,
/// because that is what survives a re-authoring, and its element name where it
/// does not.
fn segment(node: &fexml::Node) -> String {
    node.attr("name").map_or_else(
        || node.name.clone(),
        |name| format!("{}[{name}]", node.name),
    )
}

/// Collects `path -> value` for every parseable `x` and `y` in the tree.
///
/// Siblings that produce the same segment get a `#n` suffix from the second one
/// on, so the two `<Key>`s of an animation stay distinguishable without making
/// every other key noisier to read.
fn walk(node: &fexml::Node, path: &str, out: &mut Coords, seen: &mut BTreeMap<String, usize>) {
    let mut here = format!("{path}/{}", segment(node));
    let count = seen.entry(here.clone()).or_insert(0);
    if *count > 0 {
        here = format!("{here}#{count}");
    }
    *count += 1;

    for axis in ["x", "y"] {
        if let Some(value) = node.attr(axis).and_then(|v| v.parse::<f32>().ok()) {
            out.insert(format!("{here}@{axis}"), value);
        }
    }

    let mut children = BTreeMap::new();
    for child in &node.children {
        walk(child, &here, out, &mut children);
    }
}

/// What the grid ratio predicts a PSP coordinate becomes on the PS2.
fn predicted(key: &str, value: f32) -> f32 {
    let (psp, ps2) = (
        oag_game::frontend::Space::PSP.size,
        oag_game::frontend::Space::PS2.size,
    );
    if key.ends_with("@x") {
        value * ps2.0 / psp.0
    } else {
        value * ps2.1 / psp.1
    }
}

/// The coordinates present on both discs, split into those the grid ratio
/// predicts to within a pixel and those it does not.
fn split(psp: &Coords, ps2: &Coords) -> (Vec<String>, Vec<String>) {
    let (mut scaled, mut not) = (Vec::new(), Vec::new());
    for (key, value) in psp {
        let Some(actual) = ps2.get(key) else { continue };
        let want = predicted(key, *value);
        if (want - actual).abs() <= 1.0 {
            scaled.push(key.clone());
        } else {
            not.push(format!("{key}: {value} -> {actual}, predicted {want:.0}"));
        }
    }
    (scaled, not)
}

/// The measurement the `Space::PS2` doc comment used to assert in prose.
///
/// The grid conclusion survives it: 30 of the 43 coordinates the two files share
/// land within a pixel of the ratio, including both of the file's extremes. What
/// does not survive is "the PS2 layout is the PSP's, scaled" as a statement
/// about the file - 13 coordinates say otherwise, and they fall into three
/// groups rather than one.
///
/// **No mechanism is claimed for the ten repositions.** The obvious guess - a
/// PAL title-safe inset - does not fit them: `NavigationController`'s row moves
/// *in* from the right while `Language Selection`'s confirm prompt moves *out*
/// from the left, and the second one also drops to `y=400` where the ratio
/// predicts 296. Naming one cause over two behaviours is the over-read this
/// whole file is a correction for, so the groups are recorded and the cause is
/// left open.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and pulse-ps2-eu.chd"]
fn the_ps2_grid_scales_most_of_skin_xml_and_these_are_the_exceptions() {
    let (Some(psp), Some(ps2)) = (skin(PSP_USA), skin(PS2)) else {
        return;
    };

    let (scaled, not) = split(&psp, &ps2);
    println!("scaled: {}", scaled.len());
    for line in &not {
        println!("not scaled: {line}");
    }

    // **The split that makes this file's header claim an assertion rather than
    // an observation.** An `<Animation>`'s `<Key>` carries a travel, not a
    // place, so it has no business scaling with the screen - and it does not.
    // Every other exception is a widget that was put somewhere else by hand.
    let (offsets, repositions): (Vec<&String>, Vec<&String>) = not
        .iter()
        .partition(|line| line.contains("/Animation/Key@"));

    assert_eq!(
        offsets,
        [
            "/Screen/Screen[Top FE Screen]/Screen[FE Screen]/Screen/Screen/Animation/Key@y: -24 -> -24, predicted -40",
            "/Screen/Screen[Top FE Screen]/Screen[FE Screen]/Screen/Screen/Screen/Viewport#1/Animation/Key@y: -14 -> -14, predicted -23",
            "/Screen/Screen[Top FE Screen]/Screen[FE Screen]/Screen/Screen/Screen/Viewport/Animation/Key@y: 14 -> 14, predicted 23",
        ],
        "an animation's travel is authored against the widget, so it is the same \
         on both consoles - this is the set that was"
    );

    assert_eq!(
        repositions,
        [
            // **The language screen's confirm prompt was re-placed**, both axes,
            // both widgets, and outward rather than inward on `x`.
            "/Screen/Screen[Language Selection]/Text[ControlTextConfirmButton]/Values@x: 21 -> 48, predicted 28",
            "/Screen/Screen[Language Selection]/Text[ControlTextConfirmButton]/Values@y: 180 -> 400, predicted 296",
            "/Screen/Screen[Language Selection]/Text[ControlTextConfirm]/Values@x: 37 -> 80, predicted 49",
            "/Screen/Screen[Language Selection]/Text[ControlTextConfirm]/Values@y: 183 -> 402, predicted 301",
            // **The navigation bar's four prompts moved in from the right, and
            // only on `x`.** Their `y` is in `scaled` above, at 252 -> 415, and
            // the button-to-label gap goes 20 -> 27, which is the ratio. So the
            // row was scaled and then pulled left as a group, rather than not
            // scaled at all.
            "/Screen/Screen[Top FE Screen]/Screen[FE Screen]/Screen/NavigationController[NavigationController]/Text[ControlTextBackButton]/Values@x: 415 -> 513, predicted 553",
            "/Screen/Screen[Top FE Screen]/Screen[FE Screen]/Screen/NavigationController[NavigationController]/Text[ControlTextBack]/Values@x: 435 -> 540, predicted 580",
            "/Screen/Screen[Top FE Screen]/Screen[FE Screen]/Screen/NavigationController[NavigationController]/Text[ControlTextConfirmButton]/Values@x: 348 -> 394, predicted 464",
            "/Screen/Screen[Top FE Screen]/Screen[FE Screen]/Screen/NavigationController[NavigationController]/Text[ControlTextConfirm]/Values@x: 368 -> 421, predicted 491",
            // **Two near-misses at the bottom of the screen**, 5 and 12 pixels
            // short of the prediction. Small enough to be a hand adjustment and
            // too large to be rounding, which is why the tolerance above is a
            // pixel rather than something generous enough to absorb them.
            "/Screen/Screen[Top FE Screen]/Screen[FE Screen]/Screen/Screen/Screen/TextInfo/Values@y: 235 -> 392, predicted 387",
            "/Screen/Screen[Top FE Screen]/Screen[FE Screen]/Screen[Show Logo]/Viewport/Text[USLegalText]/Values@y: 250 -> 400, predicted 412",
        ],
        "the set of widgets the PS2 pressing re-placed by hand has moved"
    );

    assert_eq!(
        scaled.len(),
        30,
        "the coordinates that do scale: {scaled:#?}"
    );

    // The two extremes the `Space::PS2` doc cites, still checked, but now as two
    // members of a measured set rather than as the whole argument.
    let logo = "/Screen/Screen[Top FE Screen]/Screen[FE Screen]/Screen[Show Logo]/Text/Values";
    assert_eq!(
        (psp[&format!("{logo}@x")], ps2[&format!("{logo}@x")]),
        (460.0, 613.0)
    );
    assert_eq!(
        (psp[&format!("{logo}@y")], ps2[&format!("{logo}@y")]),
        (220.0, 362.0)
    );
}

/// Which PSP pressing the PS2 layout was scaled from, and how we can tell.
///
/// The two PSP discs agree on all 41 coordinates their `Skin.xml`s share except
/// one: the `Show Logo` screen's PRESS START text sits at `y=220` on the USA
/// disc and `y=230` on the EU one. The PS2 disc - itself a EU pressing - has
/// 362, and **362/220 is the grid ratio to within half a percent while 362/230
/// misses it by 4%**.
///
/// So the PS2 front end was scaled from the USA-era layout, not from the EU PSP
/// release that shipped alongside it. The practical consequence is that a
/// EU-to-EU comparison, which is the natural one to reach for when the only PS2
/// disc here is a EU one, shows a discrepancy that is nothing to do with the
/// console. The `Space::PS2` doc comment cited 230 for exactly that reason and
/// was wrong about that one sample for exactly that reason.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and pulse-psp-eu.chd"]
fn the_two_psp_pressings_differ_in_exactly_one_coordinate() {
    let (Some(usa), Some(eu)) = (skin(PSP_USA), skin(PSP_EU)) else {
        return;
    };

    let shared: Vec<&String> = usa.keys().filter(|key| eu.contains_key(*key)).collect();
    assert!(
        shared.len() > 30,
        "the pressings share almost nothing, so the comparison below proves \
         nothing: {} keys",
        shared.len()
    );

    let differ: Vec<String> = shared
        .iter()
        .filter(|key| (usa[**key] - eu[**key]).abs() > 0.01)
        .map(|key| format!("{key}: usa {} vs eu {}", usa[*key], eu[*key]))
        .collect();

    assert_eq!(
        differ,
        [
            "/Screen/Screen[Top FE Screen]/Screen[FE Screen]/Screen[Show Logo]/Text/Values@y: usa 220 vs eu 230"
        ],
        "the pressings' layouts diverge somewhere new"
    );
}
