//! What the race on screen tells a screenshot of itself (`crate::shot_meta`): taken when a
//! shot is asked for, because an accumulated shot moves the race and puts it back before it
//! writes.
//!
//! The input stream is the race's own recording cut at the tick the frame is drawn at. In photo
//! mode a rewind sets the live recording aside and moves the race on screen through its past, so
//! the rewind holds the run and the shot is cut at the tick the race stands at.

use crate::shot_meta::{self, Meta};
use oag_replay::ShotRecord;

use super::Session;
use crate::app::stage::Stage;

/// A shot's description and stream, held until its pixels are read back.
#[derive(Debug)]
pub(crate) struct Pending {
    pub(crate) meta: Box<Meta>,
    pub(crate) record: Option<Box<ShotRecord>>,
}

impl Pending {
    /// Writes `pixels` to `path` with this description, the size filled in.
    pub(crate) fn write(
        mut self,
        path: &std::path::Path,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> anyhow::Result<()> {
        self.meta.shot.width = width;
        self.meta.shot.height = height;
        if let Some(race) = self.meta.race.as_ref() {
            self.meta.reproduce.flags = shot_meta::flags(
                race,
                self.meta.camera.as_ref(),
                self.meta.craft.as_ref(),
                self.meta.picture.as_ref(),
                (width, height),
            );
        }
        crate::capture::write_png_with(
            path,
            width,
            height,
            pixels,
            &self.meta,
            self.record.as_deref(),
        )
    }
}

impl Session {
    /// The description and stream of the frame the race on screen is about to be drawn as, or
    /// `None` when no race is on screen. `source` says how the frame was asked for.
    pub(crate) fn shot_pending(&self, source: &str) -> Option<Pending> {
        let Stage::Race(stage) = self.stack.top() else {
            return None;
        };
        let shell = self.shell.as_ref()?;
        let race = &stage.race;
        let rewind = self.photo.as_ref().and_then(|open| open.rewind.as_ref());
        let watching = self
            .photo
            .as_ref()
            .is_some_and(|open| open.origin() == super::photo::Origin::Viewer);
        let (record, why) = if watching {
            (
                None,
                "the race on screen is a replay being watched; its file is the stream",
            )
        } else {
            let run = rewind.map(|rewind| rewind.recording());
            match run {
                Some(None) => (None, "this race was not recorded"),
                run => match race.shot_record(run.flatten()) {
                    Some(record) => (Some(record), ""),
                    None if race.recording() => (
                        None,
                        "the race ran on past the end of its recorded input (the finish)",
                    ),
                    None => (None, "this race was not recorded"),
                },
            }
        };
        let header = &stage.replay_header;
        let tick = race.core.sim.world.tick;
        let hash = race.core.sim.state_hash();
        let setup = record
            .as_ref()
            .and_then(|record| record.replay.header.setup.as_ref());
        let race_info = shot_meta::Race {
            track: setup
                .and_then(|setup| setup.track.clone())
                .unwrap_or_else(|| header.track.clone()),
            mode: header.mode.clone(),
            class: header.class.clone(),
            team: header.team.clone(),
            seed: shot_meta::hex(header.seed),
        };
        let slot = race
            .photo()
            .map_or_else(|| race.core.player_slot(), |photo| photo.subject());
        let body = &race.core.sim.world.ships[slot].physics.body;
        let craft = shot_meta::Craft {
            position: [body.position.x, body.position.y, body.position.z],
            rotation: [
                body.orientation.x,
                body.orientation.y,
                body.orientation.z,
                body.orientation.w,
            ],
        };
        let camera = race.photo().zip(race.photo_camera()).map(|(photo, pose)| {
            shot_meta::camera_of(&format!("{:?}", photo.lens()).to_lowercase(), &pose)
        });
        let picture = race
            .photo()
            .map(|photo| shot_meta::Picture::of(photo.effects(), None));
        // A photo is drawn at the output's size with no upscaler (`RenderProfile::for_a_photo`),
        // so that is what its `[render]` text says it was drawn with. `--from-png` draws it that way too;
        // `reproduce.flags` carry no render flags.
        let redrawn = source == "photo mode" && self.photo_redraws();
        let render = self
            .settings
            .render_profiles
            .get(&crate::settings::profile_key(shell.title, shell.platform))
            .map(|profile| {
                if redrawn {
                    // The output's size is not a choice the player made.
                    crate::settings::RenderProfile {
                        render_scale_chosen: false,
                        ..profile.for_a_photo()
                    }
                } else {
                    profile.clone()
                }
            })
            .and_then(|profile| toml::Value::try_from(profile).ok())
            .and_then(|value| match value {
                toml::Value::Table(table) => Some(table),
                _ => None,
            });
        let mods = shell
            .mods
            .iter()
            .filter(|listing| listing.error.is_none() && listing.enabled.unwrap_or(false))
            .map(|listing| shot_meta::Mod {
                id: listing.id.clone(),
                version: listing.version.clone(),
                sha256: race
                    .core
                    .sim
                    .mods_used()
                    .iter()
                    .find(|used| used.id == listing.id)
                    .map(|used| used.sha256.clone())
                    .unwrap_or_default(),
            })
            .collect();
        let meta = Meta {
            format: shot_meta::FORMAT,
            shot: shot_meta::Shot {
                tick,
                state_hash: shot_meta::hex(hash),
                source: source.to_string(),
                stream: record.is_some(),
                no_stream_because: why.to_string(),
                ..shot_meta::Shot::default()
            },
            title: shot_meta::Title {
                name: shell.title.name.to_string(),
                serial: shell.serial.clone(),
                platform: shell.platform.to_string(),
            },
            race: Some(race_info),
            camera,
            picture,
            craft: Some(craft),
            render,
            gpu: Some(self.gpu.describe()),
            mods,
            build: shot_meta::Build::this(),
            reproduce: shot_meta::Reproduce {
                exact: "oag-game --from-png <this file>".to_string(),
                flags: Vec::new(),
            },
        };
        Some(Pending {
            meta: Box::new(meta),
            record: record.map(Box::new),
        })
    }
}
