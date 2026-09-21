//! Scratch probe (not wired into any doc yet): dump every `SP.xml` event's
//! `M_X`/`M_Y`/`M_BUTTONSHAPE`/`M_CANVASTWEAK_X`/`M_CANVASTWEAK_Y`, and every
//! `FE3DCanvas` `<CanvasLabel>`'s own `x`/`y`/`linkedevent`, so the two
//! number spaces can be correlated against each other and against a
//! candidate affine fit, without a live emulator or a Ghidra session.
//!
//! `cargo run -p oag-tools --example campaign_map_dump -- <base data.psarc>`

fn main() {
    let mut args = std::env::args().skip(1);
    let psarc_path = args
        .next()
        .expect("usage: campaign_map_dump <base data.psarc>");

    let mut archive = oag_assets::psarc::Archive::open(&psarc_path)
        .unwrap_or_else(|e| panic!("open {psarc_path}: {e}"));

    let sp_bytes = archive
        .read_path(r"Data\xml\SP.xml")
        .or_else(|_| archive.read_path("data/xml/SP.xml"))
        .expect("read SP.xml");
    let sp_xml = String::from_utf8(sp_bytes).expect("SP.xml is not UTF-8");
    let sp_doc = oag_tables::mjolnir::parse(&sp_xml);

    println!(
        "== SP.xml events: instance_id, name, M_X, M_Y, M_BUTTONSHAPE, M_CANVASTWEAK_X, M_CANVASTWEAK_Y =="
    );
    let event_typedefs = [
        oag_tables::mjolnir::campaign::typedef::RACE_A,
        oag_tables::mjolnir::campaign::typedef::RACE_B,
        oag_tables::mjolnir::campaign::typedef::ELIMINATION,
        oag_tables::mjolnir::campaign::typedef::ZONE,
    ];
    let mut rows = Vec::new();
    for instance in &sp_doc.instances {
        if !event_typedefs.contains(&instance.typedef_id) {
            continue;
        }
        let x = instance.field("M_X").and_then(|f| f.int());
        let y = instance.field("M_Y").and_then(|f| f.int());
        let shape = instance
            .field("M_BUTTONSHAPE")
            .and_then(|f| f.value())
            .unwrap_or("");
        let tweak_x = instance.field("M_CANVASTWEAK_X").and_then(|f| f.int());
        let tweak_y = instance.field("M_CANVASTWEAK_Y").and_then(|f| f.int());
        println!(
            "{}\t{:?}\tx={:?}\ty={:?}\tshape={:?}\ttweak_x={:?}\ttweak_y={:?}",
            instance.instance_id, instance.name, x, y, shape, tweak_x, tweak_y
        );
        rows.push((instance.name.clone(), x, y));
    }

    let x_vals: Vec<i64> = rows.iter().filter_map(|r| r.1).collect();
    let y_vals: Vec<i64> = rows.iter().filter_map(|r| r.2).collect();
    println!(
        "\nM_X range: {:?}..{:?} ({} events with M_X)",
        x_vals.iter().min(),
        x_vals.iter().max(),
        x_vals.len()
    );
    println!(
        "M_Y range: {:?}..{:?} ({} events with M_Y)",
        y_vals.iter().min(),
        y_vals.iter().max(),
        y_vals.len()
    );

    let def_bytes = archive
        .read_path("data/plugins/frontend/NEWGUI/Definition.xml")
        .expect("read Definition.xml");
    let def_xml = String::from_utf8(def_bytes).expect("Definition.xml is not UTF-8");
    let def_doc = oag_tables::fexml::parse(&def_xml);

    println!("\n== FE3DCanvas CanvasLabel: x, y, linkedevent, matched SP.xml (M_X, M_Y) ==");
    for canvas in find_all(&def_doc, "FE3DCanvas") {
        for label in canvas.children_named("CanvasLabel") {
            let Some(linked) = label.value("linkedevent").filter(|v| !v.is_empty()) else {
                continue;
            };
            let x: Option<f64> = label.value("x").and_then(|v| v.trim().parse().ok());
            let y: Option<f64> = label.value("y").and_then(|v| v.trim().parse().ok());
            let matched = rows.iter().find(|(name, ..)| name == linked);
            println!(
                "label x={x:?} y={y:?} linkedevent={linked:?} -> SP.xml {:?}",
                matched
            );
        }
    }
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
