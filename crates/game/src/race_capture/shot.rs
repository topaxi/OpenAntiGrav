//! A headless race screenshot's description and input stream (`crate::shot_meta`).
//!
//! The capture arms a recording before its first tick unless a ghost recording already did, so
//! the frame it writes carries the stream that was fed to the race: the held buttons and input
//! script of the run, or nothing held under the autopilot, whose flag the launch table records.

use oag_raceplay::{Launch, Loaded, Race};

use crate::shot_meta::{self, Meta};

/// What a capture knows of its race that the race itself does not hold.
pub(super) struct Facts {
    restage: Option<std::sync::Arc<oag_replay::ShotRecord>>,
    title: &'static str,
    platform: String,
    circuit: String,
    header: oag_replay::Header,
}

impl Facts {
    /// Read off the loaded race before the loader's parts are taken apart.
    pub(super) fn of(loaded: &Loaded, options: &super::CaptureOptions) -> Self {
        let mut launch = Launch::from_table(&std::collections::BTreeMap::new())
            .expect("an empty table is the default launch");
        launch.difficulty = loaded.setup.difficulty;
        launch.autopilot = options.autopilot;
        launch.pilot_assist = options.pilot_assist;
        let team = loaded
            .setup
            .request
            .as_ref()
            .and_then(|request| request.team.clone())
            .unwrap_or_default();
        let mut header = oag_replay::Header::new(
            loaded.title.name,
            loaded.circuit.clone(),
            loaded.setup.mode.name(),
            loaded.setup.class.clone(),
            team,
            loaded.setup.seed,
        );
        header.options = launch.to_table();
        Self {
            restage: options.restage.clone(),
            title: loaded.title.name,
            platform: loaded.platform.to_string(),
            circuit: loaded.circuit.clone(),
            header,
        }
    }

    /// Starts the recording the frame's stream is cut from, unless one is running.
    pub(super) fn arm(&self, race: &mut Race) {
        if let Some(record) = &self.restage {
            if let Err(why) = oag_raceplay::replay::apply_recorded(race, &record.replay.header) {
                log::warn!("--from-png: {why:#}");
            }
            let found = race.sim.state_hash();
            if found != record.replay.initial_hash {
                println!(
                    "from-png: the rebuilt race differs before tick 0 ({:#018x}, the shot began at {:#018x})",
                    found, record.replay.initial_hash
                );
            }
        }
        if !race.recording() {
            race.start_recording(self.header.clone());
        }
    }

    /// Writes the frame with its description.
    pub(super) fn write(
        &self,
        path: &std::path::Path,
        size: (u32, u32),
        pixels: &[u8],
        race: &Race,
        adapter: &wgpu::AdapterInfo,
    ) -> anyhow::Result<()> {
        use anyhow::Context as _;
        let record = race.shot_record(None);
        let tick = race.sim.world.tick;
        if let Some(want) = &self.restage {
            let found = race.sim.state_hash();
            println!(
                "from-png: state hash at tick {tick}: {} (recorded {:#018x} at tick {}, found {:#018x})",
                if found == want.hash && tick == want.tick {
                    "MATCH"
                } else {
                    "MISMATCH"
                },
                want.hash,
                want.tick,
                found
            );
        }
        let slot = race.player_slot();
        let body = &race.sim.world.ships[slot].physics.body;
        let race_info = shot_meta::Race {
            track: self.circuit.clone(),
            mode: self.header.mode.clone(),
            class: self.header.class.clone(),
            team: self.header.team.clone(),
            seed: shot_meta::hex(self.header.seed),
        };
        let craft = shot_meta::Craft {
            position: [body.position.x, body.position.y, body.position.z],
            rotation: [
                body.orientation.x,
                body.orientation.y,
                body.orientation.z,
                body.orientation.w,
            ],
        };
        let meta = Meta {
            format: shot_meta::FORMAT,
            shot: shot_meta::Shot {
                tick,
                state_hash: shot_meta::hex(race.sim.state_hash()),
                width: size.0,
                height: size.1,
                source: "screenshot".to_string(),
                stream: record.is_some(),
                no_stream_because: if record.is_some() {
                    String::new()
                } else {
                    "this race records no input stream (a simulation mod runs in it)".to_string()
                },
            },
            title: shot_meta::Title {
                name: self.title.to_string(),
                serial: None,
                platform: self.platform.clone(),
            },
            race: Some(race_info),
            craft: Some(craft),
            gpu: Some(shot_meta::Gpu {
                adapter: adapter.name.clone(),
                backend: format!("{:?}", adapter.backend),
                driver: adapter.driver.clone(),
                driver_info: adapter.driver_info.clone(),
            }),
            build: shot_meta::Build::this(),
            reproduce: shot_meta::Reproduce {
                exact: "oag-game --from-png <this file>".to_string(),
                flags: shot_meta::own_flags(std::env::args().skip(1)),
            },
            ..Meta::default()
        };
        let png = shot_meta::encode(size.0, size.1, pixels, &meta, record.as_ref());
        std::fs::write(path, png).with_context(|| format!("writing {}", path.display()))?;
        println!("wrote {} ({}x{})", path.display(), size.0, size.1);
        Ok(())
    }
}
