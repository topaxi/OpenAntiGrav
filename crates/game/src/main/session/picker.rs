//! The race box's flow: RACE page -> Track Select -> Ship Select -> race.
//!
//! The same three-page shape Pulse's own `Single Player` -> `Track Creation`
//! -> `Team Selection` -> `Launch Game` takes (`docs/formats/race-setup.md`),
//! with the RACE page standing in for `Single Player`. The START row opens
//! the track picker where it used to launch outright; each picker writes the
//! same `race.*` setting its RACE-page row does, so the row, the picker and
//! the race that launches all read one value; and a title whose front end
//! authors neither screen (`Shell::track_select` is `None`) launches from
//! START exactly as before.
//!
//! **Zone skips the ship picker.** The mode forces the shared Zone hull
//! whatever team is set - see `oag_game::race::ship_entry_name` - and the
//! RACE page already greys its TEAM row for the same reason, so a picker
//! there would offer a choice the race ignores.

use std::sync::{Arc, Mutex};

use log::{info, warn};
use oag_game::catalogue;
use oag_ui::picker::{self, Details, Entry, Event, Kind, Picker};

use crate::picker_stage::{Distances, LiveryAxis, PickerStage, PreviewSource};
use crate::session::menus::variant_choices;
use crate::stage::Stage;

use super::Session;

impl Session {
    /// Opens Track Select over the menus, on the circuit `race.track` names.
    ///
    /// `false` when this title has no such screen, which is the caller's cue
    /// to launch straight away.
    pub(crate) fn open_track_picker(&mut self) -> bool {
        let Some(shell) = self.shell.as_ref() else {
            return false;
        };
        let Some(layout) = shell.track_select.clone() else {
            return false;
        };
        let mode = self.race_mode();
        let title = shell.title;
        let zone = mode == oag_race::Mode::Zone;
        let (entries, sources): (Vec<Entry>, Vec<PreviewSource>) = shell
            .tracks_for(mode)
            .iter()
            .map(|(track, label)| {
                (
                    Entry {
                        id: track.id.clone(),
                        label: label.clone(),
                        details: Details::Track {
                            info: self.track_info(title.name, track, mode),
                        },
                    },
                    PreviewSource::Track {
                        location: track.location.clone(),
                        reversed: track.reversed,
                        zone,
                    },
                )
            })
            .unzip();
        let model = Picker::new(
            Kind::Track,
            entries,
            Some(self.settings.race.track.as_str()),
            None,
        );
        // The setting follows the screen from the moment it opens: a stored
        // circuit this mode's list does not hold - a race circuit after MODE
        // moved to Zone - lands the screen on its first entry, and that is
        // what a confirm without moving launches. The RACE page's own TRACK
        // row used to settle this; it is not on the page any more on a title
        // with these screens (`menu::Definition::drop_rows_picked_on_screen`).
        if let Some(entry) = model.selected()
            && entry.id != self.settings.race.track
        {
            self.settings.race.track = entry.id.clone();
        }
        // The lap lengths, measured off each circuit's own file on a worker
        // - the selected circuit first, then the rest of the list in order
        // - and copied onto the panel as they land. A 4 MB read and a
        // spline parse per circuit is too slow for the frame thread and
        // too cheap to cache on disk.
        let mut order: Vec<(String, String)> = shell
            .tracks_for(mode)
            .iter()
            .map(|(track, _)| (track.id.clone(), track.entry_name()))
            .collect();
        let selected = model.index().min(order.len().saturating_sub(1));
        if selected < order.len() {
            order.rotate_left(selected);
        }
        let distances = self.spawn_distance_worker(order);
        self.open_picker(model, layout, LiveryAxis::Variant, sources, distances)
    }

    /// Reads every circuit in `order` on its own thread and measures its lap
    /// (see `oag_game::race::circuit_length`) into the map the picker reads
    /// each tick. A circuit that will not read or measure is logged and its
    /// row keeps its dash.
    fn spawn_distance_worker(&self, order: Vec<(String, String)>) -> Option<Distances> {
        let options = self.race_options.as_ref()?;
        let source = options.source.clone();
        let dlc = options.dlc.clone();
        let distances: Distances = Arc::new(Mutex::new(std::collections::HashMap::new()));
        let sink = Arc::clone(&distances);
        let spawned = std::thread::Builder::new()
            .name("circuit-lengths".into())
            .spawn(move || {
                let (packs, pure_packs, _) = oag_game::dlc::packs_from_defaults(
                    &dlc,
                    &oag_game::boot::default_dlc_cache_dir(),
                );
                let mut archives = match oag_game::title::open_source(&source, packs, pure_packs) {
                    Ok(opened) => opened.archives,
                    Err(error) => {
                        warn!("cannot open {source} to measure its circuits: {error:#}");
                        return;
                    }
                };
                for (id, entry) in order {
                    let measured = archives
                        .read_name(&entry)
                        .map_err(anyhow::Error::from)
                        .and_then(|blob| oag_game::race::circuit_length(&blob));
                    match measured {
                        Ok(length) => {
                            if let Ok(mut map) = sink.lock() {
                                map.insert(id, length);
                            }
                        }
                        Err(error) => warn!("{entry}: {error:#} - no distance for {id}"),
                    }
                }
                info!("circuit lengths measured");
            });
        match spawned {
            Ok(_) => Some(distances),
            Err(error) => {
                warn!("cannot start the circuit-length worker: {error}");
                None
            }
        }
    }

    /// Opens Ship Select over the menus, on the team `race.team` names.
    pub(crate) fn open_ship_picker(&mut self) -> bool {
        let Some(shell) = self.shell.as_ref() else {
            return false;
        };
        let Some(layout) = shell.ship_select.clone() else {
            return false;
        };
        let title = shell.title;
        let (entries, sources): (Vec<Entry>, Vec<PreviewSource>) = shell
            .teams
            .iter()
            .map(|choice| {
                let details = shell
                    .team_details
                    .iter()
                    .find(|team| team.id == choice.value);
                // The livery row: the team's own skins where it declares
                // any - the original's `Classic`/`Alternative`, `Classic`
                // being the disc's own string id for the baseline paint -
                // and the title's variant table otherwise.
                let skins: Vec<(String, String)> = details
                    .map(|team| {
                        team.skins
                            .iter()
                            .map(|skin| (skin.name.clone(), skin.location.clone()))
                            .collect()
                    })
                    .unwrap_or_default();
                let variants: Vec<(String, String)> =
                    if skins.is_empty() {
                        variant_choices(title, &choice.value)
                            .into_iter()
                            .map(|variant| (variant.value, variant.label))
                            .collect()
                    } else {
                        std::iter::once((
                            String::new(),
                            shell
                                .strings
                                .get_or_id(catalogue::BASELINE_SKIN)
                                .to_string(),
                        ))
                        .chain(skins.iter().map(|(name, _)| {
                            (name.clone(), shell.strings.get_or_id(name).to_string())
                        }))
                        .collect()
                    };
                let stats = details
                    .map(|team| team.variant_stats(variants.iter().map(|(id, _)| id.as_str())))
                    .unwrap_or_default();
                (
                    Entry {
                        id: choice.value.clone(),
                        label: choice.label.clone(),
                        details: Details::Ship {
                            rating: details.and_then(|team| team.rating).map(|rating| {
                                picker::Rating {
                                    speed: rating.speed,
                                    thrust: rating.thrust,
                                    handling: rating.handling,
                                    shield: rating.shield,
                                }
                            }),
                            variants,
                            stats,
                        },
                    },
                    PreviewSource::Ship {
                        location: details.map_or_else(
                            || format!(r"Data\Ships\{}", choice.value),
                            |team| team.location.clone(),
                        ),
                        skins,
                    },
                )
            })
            .unzip();
        // One axis per screen, decided by the selected team's own entry so
        // the row and the setting it writes agree: every Pulse team
        // declares skins, no HD or 2048 team does. Keyed on the skins
        // themselves, not on the row's first id being empty - HD's own
        // variant table opens on the classic hull's empty suffix too, which
        // read as a skin row and sent `_c1`/`_n1` to `race.skin`.
        let selected = entries
            .iter()
            .position(|entry| entry.id == self.settings.race.team)
            .unwrap_or(0);
        let axis = match sources.get(selected) {
            Some(PreviewSource::Ship { skins, .. }) if !skins.is_empty() => LiveryAxis::Skin,
            _ => LiveryAxis::Variant,
        };
        let livery = match axis {
            LiveryAxis::Skin => self.settings.race.skin.as_str(),
            LiveryAxis::Variant => self.settings.race.variant.as_str(),
        };
        let model = Picker::new(
            Kind::Ship,
            entries,
            Some(self.settings.race.team.as_str()),
            Some(livery),
        );
        // HD's screen lays the teams out across - see
        // `Picker::with_entries_across`.
        let model = if layout.hd.is_some() {
            model.with_entries_across()
        } else {
            model
        };
        // As `open_track_picker` does for the circuit: the screen's own
        // selection is the setting, from the first frame.
        if let Some(entry) = model.selected()
            && entry.id != self.settings.race.team
        {
            self.settings.race.team = entry.id.clone();
            self.resupply_race_variant();
        }
        self.open_picker(model, layout, axis, sources, None)
    }

    /// The three info rows for a circuit: distance, lap record, race
    /// record. The records come off this build's own store, keyed the way
    /// the RECORDS page keys them; the distance is **not known here** - the
    /// original reads it off the loaded circuit, and this screen has not
    /// loaded one - so it draws the same dash an unset record does rather
    /// than a number nothing measured.
    fn track_info(
        &self,
        title: &str,
        track: &catalogue::Track,
        mode: oag_race::Mode,
    ) -> [String; 3] {
        let key = oag_game::records::Key::new(
            title,
            Some(&track.entry_name()),
            mode.name(),
            self.settings.race.class.trim(),
        );
        let record = self.records.get(&key);
        [
            "-".to_string(),
            oag_game::scoreboard::record_table_value(record, false),
            oag_game::scoreboard::record_table_value(record, true),
        ]
    }

    fn open_picker(
        &mut self,
        model: Picker,
        layout: picker::Layout,
        livery_axis: LiveryAxis,
        sources: Vec<PreviewSource>,
        distances: Option<Distances>,
    ) -> bool {
        let Some(options) = self.race_options.as_ref() else {
            return false;
        };
        let (packs, pure_packs, problems) = oag_game::dlc::packs_from_defaults(
            &options.dlc,
            &oag_game::boot::default_dlc_cache_dir(),
        );
        for problem in problems {
            warn!("{problem}");
        }
        let archives = match oag_game::title::open_source(&options.source, packs, pure_packs) {
            Ok(opened) => opened.archives,
            Err(error) => {
                warn!(
                    "cannot open {} for the selection screen's previews: {error:#}",
                    options.source
                );
                return false;
            }
        };
        let base = self
            .shell
            .as_ref()
            .map(|shell| shell.sprites.clone())
            .unwrap_or_default();
        let Stage::Menu(stage) = &mut self.stage else {
            return false;
        };
        let previews = crate::picker_stage::Previews {
            meshes: self
                .shell
                .as_ref()
                .and_then(|shell| shell.title.front_end)
                .is_some_and(|front_end| front_end.preview_meshes),
            globals: self
                .shell
                .as_ref()
                .map_or_else(Vec::new, |shell| shell.globals.clone()),
            strings: self
                .shell
                .as_ref()
                .map_or_else(oag_ui::language::StringTable::default, |shell| {
                    shell.strings.clone()
                }),
        };
        let mut picker = PickerStage::new(
            model,
            layout,
            livery_axis,
            sources,
            archives,
            distances,
            self.anisotropy,
            base,
            previews,
        );
        picker.refresh_preview(&self.gpu);
        picker.refresh_info();
        stage.picker = Some(picker);
        true
    }

    /// One tick of an open picker: its input, and what came of it.
    pub(crate) fn tick_picker(&mut self, pointer: &oag_ui::pointer::Pointer) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(picker) = stage.picker.as_mut() else {
            return;
        };
        picker.refresh_info();
        let mut events = picker.model.update(self.controls.buttons_mut());
        events.extend(super::pointer::picker_pointer(stage, pointer));
        for event in events {
            self.handle_picker(event);
        }
    }

    pub(crate) fn handle_picker(&mut self, event: Event) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(picker) = stage.picker.as_mut() else {
            return;
        };
        let kind = picker.model.kind();
        let layout_is_hd = picker.layout.hd.is_some();
        match (kind, event) {
            (Kind::Track, Event::Moved) => {
                if let Some(entry) = picker.model.selected() {
                    self.settings.race.track = entry.id.clone();
                }
                picker.refresh_preview(&self.gpu);
            }
            (Kind::Ship, Event::Moved) => {
                if let Some(entry) = picker.model.selected() {
                    let team = entry.id.clone();
                    let axis = picker.livery_axis;
                    picker.refresh_preview(&self.gpu);
                    self.settings.race.team = team;
                    match axis {
                        LiveryAxis::Skin => self.settings.race.skin = String::new(),
                        LiveryAxis::Variant => self.settings.race.variant = String::new(),
                    }
                    self.resupply_race_variant();
                }
            }
            (_, Event::VariantChanged) => {
                if let Some((id, _)) = picker.model.variant() {
                    let id = id.clone();
                    match picker.livery_axis {
                        LiveryAxis::Skin => self.settings.race.skin = id,
                        LiveryAxis::Variant => self.settings.race.variant = id,
                    }
                }
                picker.refresh_preview(&self.gpu);
            }
            (Kind::Track, Event::Confirmed) => {
                stage.picker = None;
                let zone = self.race_mode() == oag_race::Mode::Zone;
                if zone || !self.open_ship_picker() {
                    self.launch_from_settings();
                }
            }
            (Kind::Ship, Event::Confirmed) => {
                stage.picker = None;
                // A campaign cell already built `self.race_options` in full
                // (`Session::launch_campaign_cell`) - `launch_from_settings`
                // would overwrite the cell's own track/mode/class with
                // whatever the RACE page's own settings happen to hold, so
                // a campaign launch takes its own tail instead, which copies
                // only the team this screen just picked.
                if self.campaign_cell.is_some() {
                    self.launch_campaign_race();
                } else {
                    self.launch_from_settings();
                }
            }
            (Kind::Track, Event::Back) => stage.picker = None,
            (Kind::Ship, Event::Back) => {
                stage.picker = None;
                // Backing out of `Team Selection` on a campaign launch has
                // no `Track Select` to return to - a cell picks its own
                // track outright - so this reopens `Grid Selection` instead
                // and drops the abandoned cell rather than let it leak into
                // whatever race is launched next.
                let difficulty = self.campaign_difficulty.take();
                self.campaign_ai_skill_scale = None;
                // Wipeout HD/Fury's `TeamRedirectBack` authors no `goto`, so
                // its Back returns to the screen `Team Selection` came from:
                // `Cell Selection`, on the same cell and rung. Pulse keeps
                // its earlier `Grid Selection` - nothing measured there.
                let hd = layout_is_hd;
                match self.campaign_cell.take() {
                    Some(cell) if hd => self.reopen_cell_selection(&cell.name, difficulty),
                    Some(_) => self.open_campaign(),
                    None => {
                        self.open_track_picker();
                    }
                }
            }
        }
    }
}
