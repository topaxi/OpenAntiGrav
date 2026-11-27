//! A menu tree adjusted for the booted title and the build: rows a title has no use for
//! dropped, rows relabelled or renamed, the campaign picker added.

use super::{Action, Definition, Entry, Page, RowMeta, StringTable};
use crate::menu::{
    CAMPAIGN_EDITOR_ROW, CAMPAIGN_PICKER_PAGE, CUSTOM_CAMPAIGN_ROW, ORIGINAL_CAMPAIGN_ROW,
};

impl Definition {
    /// Drops the RACE page's VARIANT row when `title` has no variant axis at
    /// all - `oag_title::race::RaceDefaults::has_team_variants` - rather than
    /// leaving it to draw permanently empty and unusable.
    ///
    /// **Dropped from the definition itself, not greyed and not skipped at
    /// render time.** [`Menu::is_disabled`](crate::menu::Menu::is_disabled)'s own
    /// docs are deliberate about never hiding a row: greying beats hiding
    /// because a row that vanishes mid-session leaves the player unable to
    /// find what took it away. Neither concern applies here - a title is
    /// fixed for the life of a boot, so this runs once before the row is ever
    /// drawn, and the row does not exist on Wipeout Pure on *any* team, not
    /// merely the one currently held, the way the `disabled_by` machinery's
    /// "not now" is scoped to answer. **Wipeout Pulse was believed to be the
    /// same and is not** - see `oag_title::race::HullVariant`'s own doc
    /// comment - so this only drops the row on Pure now. Row count, the
    /// scroll window and `Menu::held`'s page scan all read [`Page::entries`]
    /// directly, so removing the entry here - once, before any of the three
    /// ever sees it - is what keeps all three in agreement; skipping it only
    /// at draw time would leave a gap in the strip with the cursor still
    /// able to land on it.
    ///
    /// A `variants_of` row is untouched: its title can be any title's, not
    /// the booted one, so it keeps offering real variants even in a
    /// Pure-booted session.
    pub fn drop_unavailable_race_variant(&mut self, title: &oag_title::Title) {
        if title.race.has_team_variants() {
            return;
        }
        if let Some(page) = self.pages.iter_mut().find(|page| page.id == "race") {
            page.retain_rows(|entry| entry.setting() != Some("race.variant"));
        }
    }

    /// Drops every row that edits `setting`, on every page.
    fn drop_setting_row(&mut self, setting: &str) {
        for page in &mut self.pages {
            page.retain_rows(|entry| entry.setting() != Some(setting));
        }
    }

    /// Drops the BOOST ZOOM row on a title that cannot borrow HD's ring
    /// (`oag_title::race::RaceDefaults::borrows_zoom_ring`): HD draws its own, always,
    /// and 2048 and Omega are unread. Dropped for the reason
    /// [`Self::drop_unavailable_race_variant`] drops its row: a title is fixed for the
    /// life of a boot, so this runs once, before the row is drawn.
    pub fn drop_unavailable_boost_zoom(&mut self, title: &oag_title::Title) {
        if title.race.borrows_zoom_ring {
            return;
        }
        self.drop_setting_row("graphics.boost_zoom");
    }

    /// Drops the SCENE PRECISION row on a title whose scene does not store encoded values
    /// (`oag_title::Looks::blends_encoded`): every title but Wipeout HD, which draw into a
    /// gamma target or a float one of their own, so the row would change nothing. Same
    /// once-before-read timing as [`Self::drop_unavailable_boost_zoom`].
    pub fn drop_unavailable_scene_precision(&mut self, title: &oag_title::Title) {
        if title.looks.blends_encoded.applies_everywhere() {
            return;
        }
        self.drop_setting_row("graphics.scene_precision");
    }

    /// Drops the CIRCUIT LIGHTING row on a title whose circuits are not shaded from their own
    /// materials (`oag_title::Looks::material_render_state` unread): Pulse and Pure, whose
    /// circuits have no authored rig, specular or normal map for the row to drop. Same
    /// once-before-read timing as [`Self::drop_unavailable_boost_zoom`].
    pub fn drop_unavailable_circuit_lighting(&mut self, title: &oag_title::Title) {
        if title.looks.material_render_state.applies_everywhere() {
            return;
        }
        self.drop_setting_row("graphics.circuit_lighting");
    }

    /// Drops the REFLECTION RESOLUTION row on a title that draws no screen-space glass or sea
    /// target (`oag_title::Looks::screen_space_target`): Pulse and Pure, so the row would change
    /// nothing. Same once-before-read timing as [`Self::drop_unavailable_boost_zoom`].
    pub fn drop_unavailable_reflection_resolution(&mut self, title: &oag_title::Title) {
        if title.looks.screen_space_target.applies_everywhere() {
            return;
        }
        self.drop_setting_row("graphics.reflection_resolution");
    }

    /// Drops the TRAIL HITS row on a title whose original plays the trail-hit burst by itself
    /// (`oag_title::Effects::plays_trail_hits_natively`): Wipeout HD alone. 2048 and Omega author
    /// the burst but never fire it, so they keep the row, as Pulse and Pure do. Same
    /// once-before-read timing as [`Self::drop_unavailable_boost_zoom`].
    pub fn drop_unavailable_trail_hits(&mut self, title: &oag_title::Title) {
        if !title.effects.plays_trail_hits_natively() {
            return;
        }
        self.drop_setting_row("graphics.trail_hits");
    }

    /// Drops the WINGTIP CONTRAILS row on a title that carries them itself
    /// (`oag_title::Title::wingtip`): HD, 2048 and Omega. Pulse and Pure keep it. Same
    /// once-before-read timing as [`Self::drop_unavailable_boost_zoom`].
    pub fn drop_unavailable_wingtip_contrails(&mut self, title: &oag_title::Title) {
        if title.wingtip.is_none() {
            return;
        }
        self.drop_setting_row("graphics.wingtip_contrails");
    }

    /// Drops the OWN SHIELD row on a title whose original draws the HD-style shield itself
    /// (`oag_title::Looks::own_shield` unread): HD, 2048 and Omega. Pulse and Pure keep it. Same
    /// once-before-read timing as [`Self::drop_unavailable_boost_zoom`].
    pub fn drop_unavailable_own_shield(&mut self, title: &oag_title::Title) {
        if title.looks.own_shield.applies_everywhere() {
            return;
        }
        self.drop_setting_row("graphics.own_shield");
    }

    /// Drops the OWN ROCKETS row on a title it is not offered on
    /// (`oag_title::Looks::own_rockets` unread): HD, 2048 and Omega. Pulse and Pure keep it. Same
    /// once-before-read timing as [`Self::drop_unavailable_boost_zoom`].
    pub fn drop_unavailable_own_rockets(&mut self, title: &oag_title::Title) {
        if title.looks.own_rockets.applies_everywhere() {
            return;
        }
        self.drop_setting_row("graphics.own_rockets");
    }

    /// Drops the HUD STYLE row on a title whose disc authors no `HUD Style` list
    /// (`oag_title::HudLayouts::skins`): Pulse, Pure and 2048. Same once-before-read
    /// timing as [`Self::drop_unavailable_boost_zoom`].
    pub fn drop_unavailable_hud_style(&mut self, title: &oag_title::Title) {
        if !title.hud.skins.is_empty() {
            return;
        }
        self.drop_setting_row("display.hud_style");
    }

    /// Narrows the DYNAMIC HUD row to `default` and `concave` on a title whose options list no
    /// such setting (`oag_title::HudArt::dynamic_hud`: every title but Pulse): Pulse's warp is
    /// offered on every HUD, its `flat` and `off` only where the disc authors them. Same
    /// once-before-read timing as [`Self::drop_unavailable_boost_zoom`].
    pub fn drop_unavailable_dynamic_hud(&mut self, title: &oag_title::Title) {
        if title.hud_art.dynamic_hud.is_some() {
            return;
        }
        for page in &mut self.pages {
            for entry in &mut page.entries {
                if let Entry::Choice {
                    setting, values, ..
                } = entry
                    && setting == "display.dynamic_hud"
                {
                    values.retain(|choice| choice.value == "default" || choice.value == "concave");
                }
            }
        }
    }

    /// Drops the PLAYER NAME and PLAYER TAG rows a title does not use
    /// (`oag_title::profile::Profile`): 2048 and Omega keep neither, HD only the name. Same
    /// once-before-read timing as [`Self::drop_unavailable_boost_zoom`].
    pub fn drop_unavailable_player_rows(&mut self, title: &oag_title::Title) {
        let profile = title.profile;
        for page in &mut self.pages {
            page.retain_rows(|entry| match entry {
                Entry::Run {
                    action: Action::EditPlayerName,
                    ..
                } => profile.asks_for_name(),
                Entry::Run {
                    action: Action::EditPlayerTag,
                    ..
                } => profile.asks_for_tag(),
                _ => true,
            });
        }
    }

    /// Drops the GHOST SHIPS VISIBLE, AUTO LOAD GHOST and MULTIPLAYER TAGS rows a title's own
    /// options list lacks (`oag_title::race::RaceDefaults::game_options`): Pulse's list has the
    /// first on both pressings and the others on the PSP's, so it offers all three. Same once-before-read
    /// timing as [`Self::drop_unavailable_boost_zoom`].
    pub fn drop_unavailable_game_options(&mut self, title: &oag_title::Title) {
        let offered = title.race.game_options;
        for page in &mut self.pages {
            page.retain_rows(|entry| match entry.setting() {
                Some("display.ghost_ship_visible") => offered.ship_visible,
                Some("display.auto_load_ghost") => offered.auto_load,
                Some("display.multiplayer_tags") => offered.multiplayer_tags,
                _ => true,
            });
        }
    }

    /// Drops the TRAIL ORDER row on a title whose engine ribbon is not the PSP
    /// preset (`oag_title::exhaust::Exhaust::is_psp_preset`): HD/Fury's tube keeps
    /// its own order whatever the setting holds, and Omega and 2048 reuse it. Same
    /// once-before-read timing as [`Self::drop_unavailable_boost_zoom`].
    pub fn drop_unavailable_trail_order(&mut self, title: &oag_title::Title) {
        if title.exhaust.is_psp_preset() {
            return;
        }
        self.drop_setting_row("graphics.trail_order");
    }

    /// Drops the SCENERY SORT row on a title whose circuit is not the PSP preset's: HD/Fury
    /// keeps its own blended order, and Omega and 2048 reuse it. The same titles as
    /// [`Self::drop_unavailable_trail_order`].
    pub fn drop_unavailable_scenery_sort(&mut self, title: &oag_title::Title) {
        if title.exhaust.is_psp_preset() {
            return;
        }
        self.drop_setting_row("graphics.scenery_sort");
    }

    /// Drops the TRAIL LENGTH row on a title whose engine ribbon is not the PSP preset: HD/Fury
    /// draws its own 54-sample tube, and Omega and 2048 reuse it. The same titles as
    /// [`Self::drop_unavailable_trail_order`].
    pub fn drop_unavailable_trail_length(&mut self, title: &oag_title::Title) {
        if title.exhaust.is_psp_preset() {
            return;
        }
        self.drop_setting_row("graphics.trail_length");
    }

    /// Drops the WEATHER row on a title whose weather is not played
    /// (`oag_title::weather::Weather::is_played`): every title but Pulse. Same
    /// once-before-read timing as [`Self::drop_unavailable_boost_zoom`].
    pub fn drop_unavailable_weather(&mut self, title: &oag_title::Title) {
        if title.weather.is_played() {
            return;
        }
        self.drop_setting_row("graphics.weather");
    }

    /// Drops the RUMBLE row's `original` value on a title whose original plays
    /// no pattern rumble (`oag_title::rumble::Rumble::is_played`): Pulse, Pure
    /// and 2048. Same once-before-read timing as [`Self::drop_unavailable_boost_zoom`].
    pub fn drop_unavailable_rumble_original(&mut self, title: &oag_title::Title) {
        if title.rumble.is_played() {
            return;
        }
        for page in &mut self.pages {
            for entry in &mut page.entries {
                if let Entry::Choice {
                    setting, values, ..
                } = entry
                    && setting == "controls.rumble"
                {
                    values.retain(|choice| choice.value != "original");
                }
            }
        }
    }

    /// Drops the TARGET FPS row's `original` value on a title with no width throttle of its
    /// own (`oag_title::Title::width_throttle`): every title but Wipeout HD. The row keeps
    /// its other values. Same once-before-read timing as [`Self::drop_unavailable_boost_zoom`].
    pub fn drop_unavailable_width_throttle(&mut self, title: &oag_title::Title) {
        if title.width_throttle.is_some() {
            return;
        }
        for page in &mut self.pages {
            for entry in &mut page.entries {
                if let Entry::Choice {
                    setting, values, ..
                } = entry
                    && setting == "graphics.target_fps"
                {
                    values.retain(|choice| choice.value != "original");
                }
            }
        }
    }

    /// Drops the values of the WATER row that `offered` refuses, and the row itself when
    /// `changes_picture` is false (nothing it could hold would show). The row's own order is
    /// kept, and a value is asked about by its stored spelling. Same once-before-read timing as
    /// [`Self::drop_unavailable_boost_zoom`].
    pub fn drop_unavailable_water(
        &mut self,
        offered: &dyn Fn(&str) -> bool,
        changes_picture: bool,
    ) {
        for page in &mut self.pages {
            for entry in &mut page.entries {
                if let Entry::Choice {
                    setting, values, ..
                } = entry
                    && setting == "graphics.water"
                {
                    values.retain(|choice| offered(&choice.value));
                }
            }
            if !changes_picture {
                page.retain_rows(|entry| {
                    !matches!(entry, Entry::Choice { setting, .. } if setting == "graphics.water")
                });
            }
        }
    }

    /// Drops every QUIT row, for a platform whose apps are not quit from
    /// inside (Android: the player leaves with the system's home or app
    /// switcher, and the app is suspended, not ended). Same once-before-read
    /// timing as [`Self::drop_unavailable_race_variant`].
    pub fn drop_quit(&mut self) {
        for page in &mut self.pages {
            page.retain_rows(|entry| {
                !matches!(
                    entry,
                    Entry::Run {
                        action: Action::Quit,
                        ..
                    }
                )
            });
        }
    }

    /// Changes what the row editing `setting` draws for its option `value`,
    /// leaving the stored value alone (the web build reads `borderless` as
    /// FULLSCREEN). Same once-before-read timing as [`Self::drop_quit`].
    pub fn relabel_choice(&mut self, setting: &str, value: &str, label: &str) {
        for page in &mut self.pages {
            for entry in &mut page.entries {
                if let Entry::Choice {
                    setting: key,
                    values,
                    ..
                } = entry
                    && key == setting
                {
                    for choice in values.iter_mut().filter(|c| c.value == value) {
                        choice.label = label.to_string();
                    }
                }
            }
        }
    }

    /// [`Self::relabel_choice`] with the words `strings` has under `id`; the
    /// label stays when the table has none.
    pub fn relabel_choice_from(
        &mut self,
        setting: &str,
        value: &str,
        strings: &StringTable,
        id: &str,
    ) {
        if let Some(label) = strings.get(id) {
            self.relabel_choice(setting, value, label);
        }
    }

    /// Words the row that fires `action` as `label`, with `subtitle` under it
    /// while it is selected, for a title whose original names that row
    /// differently (Wipeout Pure's `SINGLE PLAYER` where this project's tree
    /// says `RACE CAMPAIGN`). The row keeps its place and its action. Same
    /// once-before-read timing as [`Self::drop_quit`].
    pub fn relabel_action(&mut self, action: Action, label: &str, subtitle: Option<&str>) {
        for page in &mut self.pages {
            for (entry, line) in page.entries.iter_mut().zip(&mut page.subtitles) {
                if let Entry::Run {
                    action: found,
                    label: shown,
                    ..
                } = entry
                    && *found == action
                {
                    *shown = label.to_string();
                    *line = subtitle.map(str::to_string);
                }
            }
        }
    }

    /// Words the campaign row as the title's original does
    /// ([`oag_title::Campaign::main_row`]); a title that names none keeps
    /// `RACE CAMPAIGN`. The words are the project's own, looked up in `strings`
    /// and falling back to the English the title carries.
    pub fn rename_campaign_row(&mut self, title: &oag_title::Title, strings: &StringTable) {
        let Some(row) = title.campaign.main_row else {
            return;
        };
        let say = |wording: oag_title::Wording| {
            strings
                .get_or_fallback(wording.id, wording.text)
                .to_string()
        };
        self.relabel_action(
            Action::OpenCampaign,
            &say(row.label),
            Some(&say(row.subtitle)),
        );
    }

    /// Turns the campaign row into a page that picks between the disc's own
    /// campaign and each mod campaign in `rows` (`(key, name, description)`),
    /// ADR-0077. The page's first row is the original campaign, worded as the
    /// row was; each other row fires [`Action::OpenCustomCampaign`] and carries
    /// `CUSTOM_CAMPAIGN_ROW` + its key as its id. `editor` (`(label, subtitle)`)
    /// adds a last row firing [`Action::OpenCampaignEditor`], and makes the page
    /// exist with no mod campaign. With no `rows` and no `editor`, or no
    /// campaign row in the tree, nothing changes. Run once, after the title's
    /// own trimming and [`Self::rename_campaign_row`].
    pub fn add_campaign_picker(
        &mut self,
        title: &str,
        rows: &[(String, String, String)],
        editor: Option<(&str, &str)>,
    ) {
        if rows.is_empty() && editor.is_none() {
            return;
        }
        let Some((page, row)) = self.pages.iter().enumerate().find_map(|(p, page)| {
            page.entries
                .iter()
                .position(|entry| {
                    matches!(
                        entry,
                        Entry::Run {
                            action: Action::OpenCampaign,
                            ..
                        }
                    )
                })
                .map(|row| (p, row))
        }) else {
            return;
        };
        let Entry::Run { label, .. } = self.pages[page].entries[row].clone() else {
            return;
        };
        let subtitle = self.pages[page].subtitles[row].clone();
        let mut picker = Page {
            id: CAMPAIGN_PICKER_PAGE.to_string(),
            title: title.to_string(),
            entries: vec![Entry::Run {
                label: label.clone(),
                action: Action::OpenCampaign,
                inputs: None,
            }],
            subtitles: vec![subtitle],
            meta: vec![RowMeta {
                id: Some(ORIGINAL_CAMPAIGN_ROW.to_string()),
                parent: None,
            }],
            ..Page::default()
        };
        for (key, name, description) in rows {
            picker.entries.push(Entry::Run {
                label: name.clone(),
                action: Action::OpenCustomCampaign,
                inputs: None,
            });
            picker
                .subtitles
                .push((!description.is_empty()).then(|| description.clone()));
            picker.meta.push(RowMeta {
                id: Some(format!("{CUSTOM_CAMPAIGN_ROW}{key}")),
                parent: None,
            });
        }
        if let Some((name, description)) = editor {
            picker.entries.push(Entry::Run {
                label: name.to_string(),
                action: Action::OpenCampaignEditor,
                inputs: None,
            });
            picker.subtitles.push(Some(description.to_string()));
            picker.meta.push(RowMeta {
                id: Some(CAMPAIGN_EDITOR_ROW.to_string()),
                parent: None,
            });
        }
        let target = self.pages.len();
        self.pages.push(picker);
        self.pages[page].entries[row] = Entry::Submenu { label, target };
    }

    /// Words the MULTIPLAYER row as the title's original does
    /// ([`oag_title::MainMenu::multiplayer`]); a title that names none keeps the project's
    /// `MULTIPLAYER`. The words are the project's own, looked up in `strings` and falling back
    /// to the English the title carries, the way [`Self::rename_campaign_row`] does.
    pub fn rename_multiplayer_row(&mut self, title: &oag_title::Title, strings: &StringTable) {
        let Some(row) = title.main_menu.multiplayer else {
            return;
        };
        let say = |wording: oag_title::Wording| {
            strings
                .get_or_fallback(wording.id, wording.text)
                .to_string()
        };
        let Some(page) = self.pages.iter().position(|page| page.id == "multiplayer") else {
            return;
        };
        for page_entries in &mut self.pages {
            for (entry, line) in page_entries
                .entries
                .iter_mut()
                .zip(&mut page_entries.subtitles)
            {
                if let Entry::Submenu { target, label } = entry
                    && *target == page
                {
                    *label = say(row.label);
                    *line = Some(say(row.subtitle));
                }
            }
        }
    }

    /// Drops every row that edits one of `settings`, for a platform that has
    /// no such thing (the web build has no monitor to pick and one present
    /// mode). Same once-before-read timing as [`Self::drop_quit`].
    pub fn drop_settings(&mut self, settings: &[&str]) {
        for page in &mut self.pages {
            page.retain_rows(|entry| entry.setting().is_none_or(|key| !settings.contains(&key)));
        }
    }

    /// Drops the CONTROLS page's TOUCH CONTROLS row, for a machine with no
    /// touchscreen to set up. The page stays defined but unreachable. Same
    /// once-before-read timing as [`Self::drop_quit`].
    pub fn drop_touch_controls(&mut self) {
        let Some(touch) = self
            .pages
            .iter()
            .position(|page| page.id == "touch_controls")
        else {
            return;
        };
        for page in &mut self.pages {
            page.retain_rows(
                |entry| !matches!(entry, Entry::Submenu { target, .. } if *target == touch),
            );
        }
    }

    /// Drops the RACE page's TEAM, VARIANT and TRACK rows on a title whose
    /// front end authors its own selection screens
    /// ([`oag_title::FrontEnd::race_box`]) - Wipeout Pulse, on both
    /// pressings - where START opens Track Select and Ship Select and every
    /// one of the three is picked there, with its preview, its ratings and
    /// its livery row. The rows would be the same choice twice, and the
    /// original's own `Single Player` page carries none of them
    /// (`docs/formats/race-setup.md`). Dropped from the definition for the
    /// reason [`Self::drop_unavailable_race_variant`] drops its row: once,
    /// before anything reads the page, so the row count, the scroll window
    /// and the cursor all agree.
    ///
    /// A title with no such screens - Pure, 2048 today - keeps all
    /// three, and START launches off the rows as before. A mod's
    /// two-title race page and RECORDS keep their own rows (`mod.<id>.*`) either
    /// way: the first mixes titles and the second browses records, neither of
    /// which the selection screens do.
    pub fn drop_rows_picked_on_screen(&mut self, title: &oag_title::Title) {
        // The KILLS and WEAPONS rows offer the lists of the race box's
        // `Single Player` screen, which only a title with a
        // `FrontEnd::race_setup` file has: elsewhere the row would draw
        // permanently unusable, the reason the VARIANT row is dropped too.

        let reads_setup = title
            .front_end
            .is_some_and(|front_end| front_end.race_setup.is_some());
        if !reads_setup && let Some(page) = self.pages.iter_mut().find(|page| page.id == "race") {
            page.retain_rows(|entry| {
                !matches!(entry.setting(), Some("race.kill_target" | "race.weapons"))
            });
        }
        // The TARGET row offers a score Eliminator's own slider: a title whose Eliminator ends
        // on kills has the KILLS row (or none), and a score title with no slider has no steps.
        let scores = matches!(
            title.eliminator.value,
            oag_title::eliminator::EliminatorRule::Score(oag_title::eliminator::ScoreRule {
                target: oag_title::eliminator::ScoreTarget::Slider(_),
                ..
            })
        );
        if !scores && let Some(page) = self.pages.iter_mut().find(|page| page.id == "race") {
            page.retain_rows(|entry| entry.setting() != Some("race.score_target"));
        }
        // Both screens, for a title that authors them in files of its own
        // (Wipeout HD/Fury): with only one of the two, the other pick would
        // have no screen to be made on and its row stays.
        let picks_on_screen = title.front_end.is_some_and(|front_end| {
            front_end.race_box.is_some()
                || (front_end.track_select.is_some() && front_end.team_select.is_some())
        });
        if !picks_on_screen {
            return;
        }
        if let Some(page) = self.pages.iter_mut().find(|page| page.id == "race") {
            page.retain_rows(|entry| {
                !matches!(
                    entry.setting(),
                    Some("race.team" | "race.variant" | "race.track")
                )
            });
        }
    }
}
