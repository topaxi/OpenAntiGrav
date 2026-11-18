//! A headless race screenshot's description and input stream (`crate::shot_meta`).
//!
//! The capture arms a recording before its first tick unless a ghost recording already did, so
//! the frame it writes carries the stream that was fed to the race: the held buttons and input
//! script of the run, or nothing held under the autopilot, whose flag the launch table records.

use oag_raceplay::{Launch, Loaded, Race};

use crate::shot_meta::{self, Meta};

/// What a capture knows of its race that the race itself does not hold.
pub(super) struct Facts {
    launch: Launch,
    context: crate::shot_meta::Context,
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
            launch,
            context: options.shot.clone(),
            title: loaded.title.name,
            platform: loaded.platform.to_string(),
            circuit: loaded.circuit.clone(),
            header,
        }
    }

    /// What the original opponent driver was asked, which the header must carry for a rebuild
    /// (the viewer, `--from-png`'s verification) to fly the same opponents.
    pub(super) fn note_ai(&mut self, original: Option<oag_raceplay::OriginalAi>) {
        self.launch.original_ai = original;
        self.header.options = self.launch.to_table();
    }

    /// Starts the recording the frame's stream is cut from, unless one is running.
    ///
    /// # Errors
    ///
    /// A shot being made again (`--from-png`) whose simulation modules this machine does not
    /// have, or has another build of: the race would not be the shot's, and the error names the
    /// module.
    pub(super) fn arm(&self, race: &mut Race) -> anyhow::Result<()> {
        if let Some(record) = &self.context.restage {
            oag_raceplay::extensions::check_recorded(
                &record.replay.header.mods,
                race.core.sim.mods_used(),
            )
            .map_err(|why| anyhow::anyhow!("--from-png cannot make this shot again: {why}"))?;
            if let Err(why) = oag_raceplay::replay::apply_recorded(race, &record.replay.header) {
                log::warn!("--from-png: {why:#}");
            }
            let found = race.core.sim.state_hash();
            if found != record.replay.initial_hash {
                println!(
                    "from-png: the rebuilt race differs before tick 0 ({:#018x}, the shot began at {:#018x})",
                    found, record.replay.initial_hash
                );
            }
        }
        if !race.recording() {
            race.start_recording(self.header.clone());
            // The recording installs the run's own best lap as a drawn ghost once a lap
            // beats it; a capture that asked for no ghost draws none, as it did unrecorded.
            if race.ghost().is_none() {
                race.set_ghost_visible(false);
            }
        }
        Ok(())
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
        let tick = race.core.sim.world.tick;
        if let Some(want) = &self.context.restage {
            let found = race.core.sim.state_hash();
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
        let body = &race.core.sim.world.ships[slot].physics.body;
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
                state_hash: shot_meta::hex(race.core.sim.state_hash()),
                width: size.0,
                height: size.1,
                source: "screenshot".to_string(),
                stream: record.is_some(),
                no_stream_because: if record.is_some() {
                    String::new()
                } else {
                    "this race was not recorded".to_string()
                },
            },
            title: shot_meta::Title {
                name: self.title.to_string(),
                serial: None,
                platform: self.platform.clone(),
            },
            race: Some(race_info),
            craft: Some(craft),
            render: self.context.render.clone(),
            mods: race
                .core
                .sim
                .mods_used()
                .iter()
                .map(|used| shot_meta::Mod {
                    id: used.id.clone(),
                    version: used.version.clone(),
                    sha256: used.sha256.clone(),
                })
                .collect(),
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
