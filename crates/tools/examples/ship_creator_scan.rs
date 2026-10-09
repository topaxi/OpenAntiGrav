//! Scratch probe (a scratch directory, not kept): for every `SP.xml` event
//! instance, dump `M_PPLAYERSHIPMODELDATA`/`M_PGRIDSHIPMODELDATA`/
//! `M_PPLAYERSHIPCREATORPARAMS`/`M_PGRIDSHIPCREATORPARAMS` so authored (non
//! -empty) craft references can be found and joined against `WOShipModelData`
//! instance names.
//!
//! `cargo run -p oag-tools --example ship_creator_scan -- <base data.psarc>`

fn main() {
    let mut args = std::env::args().skip(1);
    let psarc_path = args
        .next()
        .expect("usage: ship_creator_scan <base data.psarc>");

    let mut archive = oag_assets::psarc::Archive::open(&psarc_path)
        .unwrap_or_else(|e| panic!("open {psarc_path}: {e}"));

    let sp_bytes = archive
        .read_path(r"Data\xml\SP.xml")
        .or_else(|_| archive.read_path("data/xml/SP.xml"))
        .expect("read SP.xml");
    let sp_xml = String::from_utf8(sp_bytes).expect("SP.xml is not UTF-8");
    let sp_doc = oag_tables::mjolnir::parse(&sp_xml);

    println!("== WOShipModelData instances (typedef 520725191) ==");
    for inst in sp_doc.by_typedef(520725191) {
        let team = inst.field("M_TEAM").and_then(|f| f.value()).unwrap_or("");
        let livery = inst.field("M_LIVERY").and_then(|f| f.value()).unwrap_or("");
        println!(
            "  id={} name={:?} team={:?} livery={:?}",
            inst.instance_id, inst.name, team, livery
        );
    }

    let event_typedefs = [-1915183557i64, -1353052320, 1311982788, 1018671239];
    let mut total_events = 0usize;
    let mut forced_player = 0usize;
    let mut restricted_by_flag = 0usize;
    let mut any_player_creator = 0usize;
    let mut any_grid_creator = 0usize;
    println!("\n== Events with non-empty ship model data / creator params ==");
    for inst in &sp_doc.instances {
        if event_typedefs.contains(&inst.typedef_id) {
            total_events += 1;
            if inst
                .field("M_PPLAYERSHIPMODELDATA")
                .and_then(|f| f.reference())
                .is_some()
            {
                forced_player += 1;
            }
            let flags = [
                "M_BPREVENTCOMBATSHIPS",
                "M_BPREVENTAGILITYSHIPS",
                "M_BPREVENTSPEEDSHIPS",
                "M_BPREVENTPROTOSHIPS",
            ];
            if flags
                .iter()
                .any(|f| inst.field(f).and_then(|f| f.bool()) == Some(true))
            {
                restricted_by_flag += 1;
            }
            if inst
                .field("M_PPLAYERSHIPCREATORPARAMS")
                .and_then(|f| f.reference())
                .is_some()
            {
                any_player_creator += 1;
            }
            if inst
                .field("M_PGRIDSHIPCREATORPARAMS")
                .map(|f| !f.references().is_empty())
                .unwrap_or(false)
            {
                any_grid_creator += 1;
            }
        }
        if !event_typedefs.contains(&inst.typedef_id) {
            continue;
        }
        let player_model = inst
            .field("M_PPLAYERSHIPMODELDATA")
            .and_then(|f| f.reference());
        let grid_model = inst
            .field("M_PGRIDSHIPMODELDATA")
            .map(|f| f.references())
            .unwrap_or_default();
        let player_creator = inst
            .field("M_PPLAYERSHIPCREATORPARAMS")
            .and_then(|f| f.reference());
        let grid_creator = inst
            .field("M_PGRIDSHIPCREATORPARAMS")
            .map(|f| f.references())
            .unwrap_or_default();
        let prevent_combat = inst.field("M_BPREVENTCOMBATSHIPS").and_then(|f| f.bool());
        let prevent_agility = inst.field("M_BPREVENTAGILITYSHIPS").and_then(|f| f.bool());
        let prevent_speed = inst.field("M_BPREVENTSPEEDSHIPS").and_then(|f| f.bool());
        let prevent_proto = inst.field("M_BPREVENTPROTOSHIPS").and_then(|f| f.bool());

        let interesting = player_model.is_some()
            || !grid_model.is_empty()
            || player_creator.is_some()
            || !grid_creator.is_empty()
            || prevent_combat == Some(true)
            || prevent_agility == Some(true)
            || prevent_speed == Some(true)
            || prevent_proto == Some(true);

        if interesting {
            println!(
                "{} (typedef {}): player_model={:?} grid_model={:?} player_creator={:?} grid_creator={:?} prevent(combat={:?},agility={:?},speed={:?},proto={:?})",
                inst.name,
                inst.typedef_id,
                player_model.map(|r| r.instance_id),
                grid_model.iter().map(|r| r.instance_id).collect::<Vec<_>>(),
                player_creator.map(|r| r.instance_id),
                grid_creator
                    .iter()
                    .map(|r| r.instance_id)
                    .collect::<Vec<_>>(),
                prevent_combat,
                prevent_agility,
                prevent_speed,
                prevent_proto,
            );
        }
    }

    println!(
        "\n== Summary: {total_events} total events, {forced_player} force a specific player craft (M_PPLAYERSHIPMODELDATA), {restricted_by_flag} restrict player craft by category flag, {any_player_creator} author M_PPLAYERSHIPCREATORPARAMS, {any_grid_creator} author M_PGRIDSHIPCREATORPARAMS =="
    );

    println!("\n== All '* Ship Challenge' events, every field ==");
    for inst in &sp_doc.instances {
        if !inst.name.contains("Ship Challenge") {
            continue;
        }
        println!(
            "-- {} (id={}, typedef={}) --",
            inst.name, inst.instance_id, inst.typedef_id
        );
        for f in &inst.fields {
            let vals: Vec<&str> = f.values.iter().map(|v| v.value.as_str()).collect();
            if vals.iter().all(|v| v.is_empty()) {
                continue;
            }
            println!("    {} (type={}): {:?}", f.tag, f.type_name, vals);
        }
    }
}
