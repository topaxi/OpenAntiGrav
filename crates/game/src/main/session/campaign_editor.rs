//! Opens and drives the campaign editor over the Race Campaign's screens
//! (`oag_game::campaign::editor`, `docs/formats/custom-campaign.md`).
//!
//! It is entered from the campaign picker's NEW CAMPAIGN row
//! ([`oag_ui::menu::Action::OpenCampaignEditor`]), and takes the screen, the pad and the
//! pointer while it is open: the pad moves the cursor over the title's hex slots, CROSS
//! places an event and opens its fields, LEFT and RIGHT step a field's value, TRIANGLE
//! clears a slot, SQUARE names the campaign and START saves it; the pointer hovers and
//! clicks the same hexes and rows. A save is written off the render thread as a fresh
//! folder mod under the user mods directory, the menus are rebuilt the way a MODS page
//! switch rebuilds them, and the picker lists the campaign.

use log::{debug, error, info};

use oag_core::buttons::Button;
use oag_game::campaign::editor::{Authored, Choices, Editor, Field, Pick};
use oag_ui::menu::nav::Nav;
use oag_ui_screens::campaign::editor::{self as view, Event};
use oag_ui_screens::prompt;

use crate::campaign_editor_stage::EditorScreen;
use crate::overlay::{Prompt, Purpose};
use crate::stage::Stage;

use super::Session;

/// The longest campaign name the keyboard takes.
const NAME_LENGTH: usize = 24;

/// What one tick of pad input asked for.
#[derive(Default)]
struct Pad {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    cross: bool,
    circle: bool,
    triangle: bool,
    square: bool,
    start: bool,
}

impl Session {
    /// The picker's NEW CAMPAIGN row: the campaign screens, opened on an empty editor.
    pub(crate) fn open_campaign_editor(&mut self) {
        self.custom_campaign = None;
        self.open_campaign_screens(true);
    }

    /// The editor over the screens just loaded: choices from the booted title and disc.
    pub(super) fn editor_for(
        &self,
        slots: Vec<(u32, u32)>,
        disc: &[oag_tables::race_campaign::Grid],
    ) -> Option<EditorScreen> {
        let shell = self.shell.as_ref()?;
        let pick = |value: &str, label: &str| Pick {
            value: value.to_string(),
            label: label.to_string(),
        };
        let choices = Choices {
            tracks: shell
                .tracks
                .iter()
                .map(|(track, label)| pick(&track.id, label))
                .collect(),
            teams: shell
                .teams
                .iter()
                .map(|team| pick(&team.value, &team.label))
                .collect(),
            classes: shell
                .title
                .race
                .speed_classes
                .map_or(&[][..], |classes| classes.names)
                .iter()
                .map(|name| Pick::plain(name))
                .collect(),
            slots,
            title: shell.title.slug(),
            authored: disc
                .iter()
                .flat_map(|grid| &grid.cells)
                .filter_map(Authored::of)
                .collect(),
        };
        let strings = &shell.strings;
        let say = |id: &str, english: &str| strings.get(id).unwrap_or(english).to_string();
        let name = say("OAG_EDITOR_DEFAULT_NAME", "MY CAMPAIGN");
        Some(EditorScreen::new(Editor::new(choices, &name).with_words(&say)))
    }

    fn word(&self, id: &str, english: &str) -> String {
        self.shell
            .as_ref()
            .and_then(|shell| shell.strings.get(id))
            .unwrap_or(english)
            .to_string()
    }

    /// One tick of the open editor: a save that finished, the pad, then the pointer.
    pub(crate) fn tick_campaign_editor(&mut self, pointer: &oag_ui::pointer::Pointer) {
        self.poll_editor_save();
        let buttons = self.controls.buttons_mut();
        let pad = Pad {
            up: buttons.take(Button::Up),
            down: buttons.take(Button::Down),
            left: buttons.take(Button::Left),
            right: buttons.take(Button::Right),
            cross: buttons.take(Button::Cross),
            circle: buttons.take(Button::Circle),
            triangle: buttons.take(Button::Triangle),
            square: buttons.take(Button::Square),
            start: buttons.take(Button::Start),
        };
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(campaign) = stage.campaign.as_mut() else {
            return;
        };
        let events = {
            let Some(screen) = campaign.editor.as_ref() else {
                return;
            };
            let shown = screen.shown();
            let view = shown.view(&screen.pulse, screen.model.cursor());
            let targets = view::targets(
                &view,
                campaign.cell_layout(),
                &|src| campaign.sprites.get(src),
                &|src| campaign.sprites.opaque_extent(src),
            );
            view::pointer(&view, pointer, &targets)
        };
        let Some(screen) = campaign.editor.as_mut() else {
            return;
        };
        let mut nav = None;
        let (mut save, mut rename, mut leave) = (false, false, false);
        let model = &mut screen.model;
        if !screen.status.is_empty() && (pad.up || pad.down || pad.left || pad.right || pad.cross) {
            screen.status.clear();
        }
        if model.editing() {
            if pad.up {
                model.move_field(-1);
                nav = Some(Nav::Moved(None));
            }
            if pad.down {
                model.move_field(1);
                nav = Some(Nav::Moved(None));
            }
            if pad.left {
                model.cycle(-1);
                nav = Some(Nav::Stepped(None));
            }
            if pad.right || pad.cross {
                model.cycle(1);
                nav = Some(Nav::Stepped(None));
            }
            if pad.circle {
                model.back();
                nav = Some(Nav::Decline);
            }
        } else {
            for (pressed, dx, dy) in [
                (pad.up, 0, -1),
                (pad.down, 0, 1),
                (pad.left, -1, 0),
                (pad.right, 1, 0),
            ] {
                if pressed {
                    let before = model.cursor();
                    model.step(dx, dy);
                    if model.cursor() != before {
                        nav = Some(Nav::Moved(None));
                    }
                }
            }
            if pad.cross {
                model.confirm();
                nav = Some(Nav::Accept);
            }
            leave = pad.circle;
        }
        if pad.triangle && model.has(model.cursor()) {
            model.clear();
            nav = Some(Nav::Decline);
        }
        rename |= pad.square;
        save |= pad.start;
        for event in events {
            match event {
                Event::Hover(at) | Event::Select(at) => {
                    if model.cursor() != at {
                        model.select(at);
                        nav = Some(Nav::Moved(None));
                    }
                }
                Event::Confirm => {
                    model.confirm();
                    nav = Some(Nav::Accept);
                }
                Event::Field(index) => {
                    model.begin_edit();
                    model.select_field(Field::ALL[index]);
                }
                Event::Row(index) => {
                    let fields = if model.has(model.cursor()) { Field::ALL.len() } else { 0 };
                    if index < fields {
                        model.begin_edit();
                        model.select_field(Field::ALL[index]);
                        model.cycle(1);
                        nav = Some(Nav::Stepped(None));
                    } else if index == fields {
                        rename = true;
                    } else {
                        save = true;
                    }
                }
                Event::Back => {
                    if model.back() {
                        nav = Some(Nav::Decline);
                    } else {
                        leave = true;
                    }
                }
            }
        }
        self.play_navs(nav);
        if rename {
            self.editor_rename();
        } else if save {
            self.editor_save();
        } else if leave {
            self.editor_back();
        }
    }

    /// Back: closes the field panel, else leaves the editor for the picker. What is
    /// placed and not saved is dropped.
    pub(crate) fn editor_back(&mut self) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(campaign) = stage.campaign.as_mut() else {
            return;
        };
        if campaign.editor.as_mut().is_some_and(|screen| screen.model.back()) {
            self.play_navs(Some(Nav::Decline));
            return;
        }
        debug!("campaign editor closed without saving");
        stage.campaign = None;
        self.play_navs(Some(Nav::Decline));
    }

    /// SAVE: writes the campaign as a folder mod, off the render thread.
    fn editor_save(&mut self) {
        let empty = self.word("OAG_EDITOR_NEEDS_EVENT", "PLACE AN EVENT FIRST");
        let failed = self.word("OAG_EDITOR_SAVE_FAILED", "COULD NOT SAVE THE CAMPAIGN");
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(screen) = stage.campaign.as_mut().and_then(|c| c.editor.as_mut()) else {
            return;
        };
        if screen.saving.is_some() {
            return;
        }
        if screen.model.count() == 0 {
            screen.status = empty;
            return;
        }
        let Some(root) = oag_mod::user_mods_dir() else {
            error!("cannot save the campaign: no data directory on this platform");
            screen.status = failed;
            return;
        };
        let model = screen.model.clone();
        screen.saving = Some(oag_thread::Task::spawn("campaign-save", move || {
            model.save(&root)
        }));
        self.play_navs(Some(Nav::Accept));
    }

    /// A save that finished: the editor closes on the picker, which lists the campaign.
    fn poll_editor_save(&mut self) {
        let failed = self.word("OAG_EDITOR_SAVE_FAILED", "COULD NOT SAVE THE CAMPAIGN");
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(screen) = stage.campaign.as_mut().and_then(|c| c.editor.as_mut()) else {
            return;
        };
        let Some(task) = screen.saving.as_mut().filter(|task| task.is_finished()) else {
            return;
        };
        let result = task.join();
        screen.saving = None;
        match result {
            Some(Ok(Ok(folder))) => {
                info!("campaign saved to {}", folder.display());
                stage.campaign = None;
                self.rebuild_menus(None);
            }
            Some(Ok(Err(why))) => {
                error!("cannot save the campaign: {why}");
                screen.status = failed;
            }
            _ => {
                error!("cannot save the campaign: the writer stopped");
                screen.status = failed;
            }
        }
    }

    /// SQUARE or the NAME row: the on-screen keyboard on the campaign's name.
    fn editor_rename(&mut self) {
        let labels = prompt::Labels {
            title: self.word("OAG_EDITOR_NAME_TITLE", "NAME YOUR CAMPAIGN"),
            delete: self.word("OAG_KEYBOARD_DELETE", "DEL"),
            accept: self.word("OAG_KEYBOARD_ACCEPT", "OK"),
            hint: self.word(
                "OAG_KEYBOARD_HINT",
                "CROSS TYPE   SQUARE DELETE   START ACCEPT   CIRCLE CANCEL",
            ),
        };
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        let Some(screen) = stage.campaign.as_ref().and_then(|c| c.editor.as_ref()) else {
            return;
        };
        let keyboard = prompt::Keyboard::new(labels, screen.model.name(), NAME_LENGTH);
        stage.prompt = Some(Prompt::typing(Purpose::CampaignName, keyboard));
    }

    /// The name keyboard was accepted: an empty answer keeps the name.
    pub(crate) fn campaign_named(&mut self, text: &str) {
        let Stage::Menu(stage) = &mut self.stage else {
            return;
        };
        if let Some(screen) = stage.campaign.as_mut().and_then(|c| c.editor.as_mut())
            && !text.trim().is_empty()
        {
            screen.model.set_name(text);
        }
    }
}
