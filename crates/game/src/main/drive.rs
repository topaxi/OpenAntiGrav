//! `--drive`: the real session, with no window, told what to do by a script and reporting what it
//! is doing as JSON lines.
//!
//! **The same code path as the window.** [`App::open_with`] builds the same `Session` the event
//! loop would, on a device with no surface (`Gpu::headless`); this module then calls
//! `Session::frame` in a loop, one tick a frame (`Dev::lockstep`), tapping buttons through the
//! same `Controls` the keyboard reaches. A menu launch therefore goes through `launch_race`,
//! the loading stage and the race build exactly as a pad's does. Nothing here reads a picture:
//! it builds an `oag_game::drive::State` from the stage on screen after every frame, and the
//! script's conditions and `select` rule (`oag_game::drive`) decide from that.
//!
//! No list of pages is kept here. A screen is drivable by reporting its rows in [`screen`]; one
//! that reports none (a prompt, the campaign grid) shows up in the state with its `screen` kind
//! and an empty row list, and a script can wait for it and `press` through it.

use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use oag_game::drive::{Focus, Next, Opt, Resolved, Row, Screen, Script, Select, SetChoice, State, Step};
use oag_gameplay::input::Button;
use oag_ui::menu::{Entry, Value};

use crate::app::App;
use crate::gpu::Gpu;
use crate::menu_stage::MenuStage;
use crate::session::Session;
use crate::stage::Stage;

/// The most wall-clock time one step may spend, loading screens included. **Chosen.** A Pulse PSP
/// race measured 8 s to build in a release build; ticks on a loading screen are not counted
/// against `within`, so this is what bounds a load that never ends.
const WALL: Duration = Duration::from_secs(300);

/// Nap taken on a loading screen, so it is not spun through at thousands of frames a second.
/// Its ticks are not counted against a step's `within`.
const LOADING_NAP: Duration = Duration::from_millis(2);

/// How long a frame spent *waiting* takes: one tick of real time, as in a window. Threads the
/// menus start (the custom-track check, a picker's distance worker) run on the wall clock, so a
/// wait that spun through its ticks in a millisecond would time out before they could answer.
const WAIT_FRAME: Duration = Duration::from_micros(16_667);

/// Runs `script` against a headless session.
///
/// # Errors
///
/// The device or session will not open, a step times out, or a row a step names is not there.
/// The message carries the step, its line and the last state reported.
pub(crate) fn run(app: &mut App, script: Script, state_log: Option<&Path>) -> Result<()> {
    if app.launcher.is_some() {
        bail!("--drive: no disc image was chosen and the search path holds several; pass --source <image>");
    }
    let size = app.settings.display.window_size;
    let gpu = Gpu::headless(
        winit::dpi::PhysicalSize::new(size.width, size.height),
        &app.settings.graphics.renderer,
    )?;
    let session = app
        .open_with(gpu)?
        .context("nothing to drive: the run has no source and no race")?;
    let out: Box<dyn Write> = match state_log {
        Some(path) => Box::new(std::io::BufWriter::new(
            std::fs::File::create(path).with_context(|| format!("--state-log {}", path.display()))?,
        )),
        None => Box::new(std::io::stderr()),
    };
    let mut driver = Driver {
        session,
        out,
        last: State::default(),
        down: None,
        spent: 0,
        started: Instant::now(),
    };
    driver.observe()?;
    for (number, (line, step)) in script.steps.iter().enumerate() {
        driver.reset_budget();
        if let Err(why) = driver.exec(step) {
            driver.session.finish_endrace_prefetch();
            oag_game::records::finish_background_saves();
            driver.out.flush().ok();
            bail!(
                "drive: step {} (line {line}): {step}: {why}\n  last state: {}",
                number + 1,
                driver.last.to_line()
            );
        }
        if matches!(step, Step::Quit) {
            break;
        }
    }
    driver.session.finish_endrace_prefetch();
    driver.session.keep_campaign_position();
    oag_game::records::finish_background_saves();
    driver.out.flush()?;
    Ok(())
}

struct Driver {
    session: Session,
    out: Box<dyn Write>,
    last: State,
    /// The button held down since the previous frame, released at the start of the next.
    down: Option<Button>,
    /// Ticks spent outside a loading screen on the step running.
    spent: u32,
    started: Instant,
}

impl Driver {
    fn reset_budget(&mut self) {
        self.spent = 0;
        self.started = Instant::now();
    }

    /// One frame, with `press` held through its tick. Reports the state if it changed.
    fn frame(&mut self, press: Option<Button>) -> Result<()> {
        if let Some(button) = self.down.take() {
            self.session.controls.press_button(button, false);
        }
        if let Some(button) = press {
            self.session.controls.press_button(button, true);
            self.down = Some(button);
        }
        self.session.frame()?;
        if self.session.quit {
            bail!("the game quit (the last state is below)");
        }
        self.observe()?;
        if self.last.stage == "loading" {
            std::thread::sleep(LOADING_NAP);
        }
        Ok(())
    }

    /// A frame spent waiting for something to happen: [`Self::frame`], held to real time.
    fn wait_frame(&mut self) -> Result<()> {
        let began = Instant::now();
        self.frame(None)?;
        if self.last.stage != "loading" {
            std::thread::sleep(WAIT_FRAME.saturating_sub(began.elapsed()));
        }
        Ok(())
    }

    /// Spends a tick of the running step's budget, failing when it is gone. Time on a loading
    /// screen is bounded by the wall clock alone.
    fn spend(&mut self, within: u32) -> Result<()> {
        if self.started.elapsed() > WALL {
            bail!("timed out after {} s of wall clock", WALL.as_secs());
        }
        if self.last.stage != "loading" {
            self.spent += 1;
            if self.spent > within {
                bail!("timed out after {within} ticks");
            }
        }
        Ok(())
    }

    fn observe(&mut self) -> Result<()> {
        let state = state_of(&self.session);
        if !state.same_as(&self.last) || self.last.stage.is_empty() {
            writeln!(self.out, "{}", state.to_line())?;
            self.out.flush()?;
        }
        self.last = state;
        Ok(())
    }

    /// The screen that takes input, or why there is none.
    fn screen(&self) -> Result<&Screen> {
        self.last
            .screen
            .as_ref()
            .ok_or_else(|| anyhow!("no screen takes input on stage {:?}", self.last.stage))
    }

    /// Waits out a transition, then taps `button`: held through one frame, let go on the next.
    fn tap(&mut self, button: Button, within: u32) -> Result<()> {
        while self.last.screen.as_ref().is_some_and(|s| s.busy) {
            self.spend(within)?;
            self.wait_frame()?;
        }
        self.frame(Some(button))?;
        self.frame(None)
    }

    fn exec(&mut self, step: &Step) -> Result<()> {
        match step {
            Step::Boot { within } => {
                // Start skips the logo and the intro; the language list confirms on Cross.
                let mut taps = [Button::Start, Button::Cross].into_iter().cycle();
                while self.last.stage != "menu" {
                    self.spend(*within)?;
                    if self.last.stage == "frontend" {
                        let button = taps.next().unwrap_or(Button::Start);
                        self.tap(button, *within)?;
                    } else {
                        self.wait_frame()?;
                    }
                }
            }
            Step::Until { conds, within } => {
                while !self.last.satisfies(conds) {
                    self.spend(*within)?;
                    self.wait_frame()?;
                }
            }
            Step::Expect { conds } => {
                if !self.last.satisfies(conds) {
                    bail!("does not hold");
                }
            }
            Step::Select { target, within } => self.select(target, *within)?,
            Step::Set { setting, value, within } => {
                self.select(setting, *within)?;
                let mut set = SetChoice::new(setting.clone(), value.clone());
                loop {
                    match set.next(self.screen()?).map_err(|why| anyhow!(why))? {
                        Next::Done => break,
                        Next::Press(button) => {
                            self.spend(*within)?;
                            self.tap(button, *within)?;
                        }
                        Next::Wait(reason) => {
                            self.spend(*within).map_err(|e| anyhow!("{e}; {reason}"))?;
                            self.wait_frame()?;
                        }
                    }
                }
            }
            Step::Press(buttons) => {
                for button in buttons {
                    self.tap(*button, oag_game::drive::script::DEFAULT_WITHIN)?;
                }
            }
            Step::Ticks(n) => {
                for _ in 0..*n {
                    self.frame(None)?;
                }
            }
            Step::Screenshot(path) => self.screenshot(path)?,
            Step::Quit => {}
        }
        Ok(())
    }

    fn select(&mut self, target: &str, within: u32) -> Result<()> {
        let mut select = Select::new(target);
        loop {
            while self.last.screen.as_ref().is_some_and(|s| s.busy) {
                self.spend(within)?;
                self.wait_frame()?;
            }
            // A page that has not opened yet is waited for, not failed: `select` straight after a
            // `press cross` that opens it.
            if self.screen().is_err() {
                self.spend(within)?;
                self.wait_frame()?;
                continue;
            }
            match select.next(self.screen()?).map_err(|why| anyhow!(why))? {
                Next::Done => return Ok(()),
                Next::Press(button) => {
                    self.spend(within)?;
                    self.tap(button, within)?;
                }
                Next::Wait(_) => self.wait_frame()?,
            }
        }
    }

    /// Draws the next frame into the offscreen target and writes it as a PNG.
    fn screenshot(&mut self, path: &Path) -> Result<()> {
        self.session.gpu.output.draw_next = true;
        self.frame(None)?;
        let output = &self.session.gpu.output;
        let texture = output.offscreen().context("this session has no offscreen target")?;
        let (width, height) = output.size();
        let device = &self.session.gpu.device;
        let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("drive screenshot"),
        });
        let mut pixels =
            oag_game::capture::read_back(device, &self.session.gpu.queue, encoder, texture, width, height)?;
        if matches!(
            texture.format(),
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            for pixel in pixels.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
            }
        }
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        oag_game::capture::write_png(path, width, height, &pixels)
    }
}

fn stage_name(stage: &Stage) -> &'static str {
    match stage {
        Stage::Launcher(_) => "launcher",
        Stage::Loading(_) => "loading",
        Stage::Frontend(_) => "frontend",
        Stage::Menu(_) => "menu",
        Stage::Race(_) => "race",
    }
}

/// Everything the game reports after a frame.
fn state_of(session: &Session) -> State {
    let mut state = State {
        tick: session.clock.tick(),
        stage: stage_name(&session.stage).to_string(),
        ..State::default()
    };
    match &session.stage {
        Stage::Frontend(stage) => {
            state.frontend = stage.frontend.machine().current().map(str::to_string);
        }
        Stage::Menu(stage) => {
            state.page = Some(stage.menu.page().id.clone());
            state.screen = Some(screen(stage));
        }
        _ => {}
    }
    // The race a launch handed the loader: from the race's own loading screen on. Before that
    // `race_options` is what the menus last wrote, which a pick still open may not have reached.
    if matches!(&session.stage, Stage::Race(_))
        || matches!(&session.stage, Stage::Loading(stage) if stage.race.is_some())
    {
        state.race = resolved(session);
    }
    state
}

/// The race the loader was handed; its `title` once it is loaded.
fn resolved(session: &Session) -> Option<Resolved> {
    let options = session.race_options.as_ref()?;
    Some(Resolved {
        source: options.source.clone(),
        craft_source: options.craft_source.clone(),
        title: matches!(session.stage, Stage::Race(_))
            .then(|| session.shell.as_ref().map(|shell| shell.title.name.to_string()))
            .flatten(),
        track: options.track.clone(),
        track_file: options
            .foreign_track
            .as_ref()
            .map(|path| path.display().to_string()),
        team: options.team.clone(),
        variant: options.hull_variant.clone(),
        mode: options.mode.name().to_string(),
    })
}

/// The screen that has the tick: an overlay if one is open, else the menu page.
fn screen(stage: &MenuStage) -> Screen {
    let overlay = |kind: &str| Screen {
        kind: kind.to_string(),
        id: kind.to_string(),
        busy: stage.change.is_some(),
        ..Screen::default()
    };
    if stage.prompt.is_some() {
        return overlay("prompt");
    }
    if let Some(picker) = &stage.picker {
        return picker_screen(&picker.model);
    }
    if let Some(campaign) = &stage.campaign {
        return campaign_screen(campaign);
    }
    if stage.pure_chain.is_some() {
        return overlay("pure-chain");
    }
    if stage.join.is_some() || stage.lan.is_some() {
        return overlay("join");
    }
    if stage.layout.is_some() {
        return overlay("window-layout");
    }
    if stage.assign.is_some() {
        return overlay("seat-assign");
    }
    page_screen(stage)
}

/// The campaign's screen: which of its levels is open, and the grid or cell under the cursor.
fn campaign_screen(campaign: &crate::campaign_stage::CampaignStage) -> Screen {
    let (id, index, name) = campaign.drive_focus();
    Screen {
        kind: "campaign".to_string(),
        id: id.to_string(),
        focus: Some(Focus {
            index,
            id: Some(name.clone()),
            label: name,
            setting: None,
        }),
        ..Screen::default()
    }
}

fn picker_screen(picker: &oag_ui_screens::picker::Picker) -> Screen {
    use oag_ui_screens::picker::Kind;
    let rows: Vec<Row> = picker
        .entries()
        .iter()
        .enumerate()
        .map(|(index, entry)| Row {
            index,
            id: Some(entry.id.clone()),
            label: entry.label.clone(),
            kind: "entry",
            enabled: true,
            ..Row::default()
        })
        .collect();
    let index = picker.index();
    Screen {
        kind: "picker".into(),
        id: match picker.kind() {
            Kind::Track => "track-select",
            Kind::Ship => "ship-select",
        }
        .into(),
        title: String::new(),
        focus: rows.get(index).map(|row| Focus {
            index,
            id: row.id.clone(),
            label: row.label.clone(),
            setting: None,
        }),
        step: Some(if picker.entries_across() {
            ("right", "left")
        } else {
            ("down", "up")
        }),
        rows,
        busy: false,
    }
}

fn page_screen(stage: &MenuStage) -> Screen {
    let menu = &stage.menu;
    let page = menu.page();
    let rows: Vec<Row> = page
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let (kind, setting, value, values) = match entry {
                Entry::Submenu { .. } => ("submenu", None, None, Vec::new()),
                Entry::Run { .. } => ("action", None, None, Vec::new()),
                Entry::Back { .. } => ("back", None, None, Vec::new()),
                Entry::Binding { .. } => ("binding", None, None, Vec::new()),
                Entry::Choice {
                    setting,
                    values,
                    current,
                    ..
                } => (
                    "choice",
                    Some(setting.clone()),
                    values.get(*current).map(|choice| choice.value.clone()),
                    values
                        .iter()
                        .map(|choice| Opt {
                            value: choice.value.clone(),
                            label: choice.label.clone(),
                            unavailable: choice.unavailable.clone(),
                        })
                        .collect(),
                ),
                Entry::Toggle { setting, .. } => (
                    "toggle",
                    Some(setting.clone()),
                    entry.value().map(|v| match v {
                        Value::Flag(true) => "on".to_string(),
                        Value::Flag(false) => "off".to_string(),
                        Value::Text(text) => text,
                    }),
                    Vec::new(),
                ),
            };
            Row {
                index,
                action: match entry {
                    Entry::Run { action, .. } => Some(action.name().to_string()),
                    _ => None,
                },
                id: page.meta.get(index).and_then(|meta| meta.id.clone()),
                label: entry.label().to_string(),
                kind,
                setting,
                value,
                values,
                enabled: !menu.is_disabled(entry),
            }
        })
        .collect();
    let index = menu.selected();
    Screen {
        kind: "page".into(),
        id: page.id.clone(),
        title: page.title.clone(),
        focus: rows.get(index).map(|row| Focus {
            index,
            id: row.id.clone(),
            label: row.label.clone(),
            setting: row.setting.clone(),
        }),
        step: None,
        rows,
        busy: stage.change.is_some(),
    }
}
