//! The music wave on the campaign map: the front end's half.
//!
//! The music path hands over the playing track's band levels each frame
//! ([`Frontend::set_music_levels`]); every 60 Hz step the wave
//! ([`super::canvas_wave`]) is advanced around the current event, and the
//! raised cells are drawn as triangles ([`super::canvas_mesh`]) over the ground
//! and under the tiles. With no levels (a title or disc without the spectrum
//! sidecar) the map is the static picture.
//!
//! **The current event** is the first open one the map shows, the event the
//! original's `TouchCampaign_Update` centres the wave on (the one the campaign
//! is up to: `canvas-music-wave.md`, "Where it runs"); the selected event
//! stands in when none is open.

use super::canvas_camera::{self, Camera, TILE_HEIGHT};
use super::canvas_cells;
use super::canvas_lattice;
use super::canvas_mesh::{self, Placed, Scene};
use super::canvas_wave::{self, Wave};
use super::*;

/// The levels' band the map follows: band 1, the second lowest of 12 (the bass).
pub const BAND: usize = 1;

impl Frontend {
    /// The playing track's band levels this frame, or `None` with no spectrum.
    /// An output of the music path, never simulation state.
    pub fn set_music_levels(&mut self, levels: Option<[f32; 12]>) {
        self.campaign.band1 = levels.map(|levels| levels[BAND]);
    }

    /// The event the wave is centred on.
    fn wave_centre(&self) -> Option<(f32, f32)> {
        let events = &self.campaign.events;
        let current = (0..events.len())
            .find(|&at| {
                !self.campaign.hidden(at) && self.campaign.state_of(at) == ProgressState::Open
            })
            .or_else(|| {
                (self.campaign.selected < events.len()).then_some(self.campaign.selected)
            })?;
        let top = canvas_camera::tile_top(CampaignMap::cell(&events[current]));
        Some((top[0], top[1]))
    }

    /// Advances the wave by `dt` seconds: whole 60 Hz steps, the original's
    /// frames. Off the map, or with no levels, it is cleared.
    pub(super) fn step_wave(&mut self, dt: f64) {
        let Some(band1) = self.campaign.band1.filter(|_| self.on_open_map()) else {
            self.campaign.wave = Wave::default();
            self.campaign.wave_owed = 0.0;
            return;
        };
        let centre = self.wave_centre();
        let priorities = canvas_cells::priorities(&self.canvas_buttons());
        let priority = |cell: (i32, i32)| priorities.get(&cell).copied().unwrap_or(0);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a tick is a fraction of a second"
        )]
        let owed = self.campaign.wave_owed + dt as f32;
        // A stalled frame does not replay a second of steps.
        let steps = ((owed / canvas_wave::STEP) as u32).min(4);
        self.campaign.wave_owed = if steps == 4 {
            0.0
        } else {
            owed - steps as f32 * canvas_wave::STEP
        };
        for _ in 0..steps {
            self.campaign.wave.step(band1, centre, &priority, &[]);
        }
    }

    /// The raised cells, far to near, over the ground cells.
    pub(super) fn draw_wave(&self, camera: &Camera, out: &mut Vec<Draw>) {
        let raised: Vec<(i32, i32)> = self
            .campaign
            .wave
            .raised()
            .filter(|&(_, height)| height > 0.5 / 255.0)
            .map(|(cell, _)| cell)
            .collect();
        if raised.is_empty() {
            return;
        }
        let buttons = self.canvas_buttons();
        let priorities = canvas_cells::priorities(&buttons);
        let priority = |cell: (i32, i32)| priorities.get(&cell).copied().unwrap_or(0);
        let fills: std::collections::BTreeMap<(i32, i32), u8> = canvas_cells::resolve(&buttons)
            .into_iter()
            .map(|painted| (painted.cell, painted.fill))
            .collect();
        let height_of = |cell: (i32, i32)| match priority(cell) {
            7 => TILE_HEIGHT,
            6 => 0.0,
            _ => self.campaign.wave.height(cell),
        };
        let skip = |cell: (i32, i32)| priority(cell) >= canvas_wave::STILL_PRIORITY;
        let lattice = self.placed(canvas_lattice::TILE);
        let copies = self.lattice_copies();
        let place = |cell: (i32, i32), ground: [[f32; 2]; 3]| -> Option<Placed> {
            let fill = fills.get(&cell).copied().unwrap_or(0);
            let sheet = |picture: &super::placement::Placed, at: [f32; 2]| {
                let rect = picture.sheet_rect([at[0], at[1], 0.0, 0.0]);
                [rect[0], rect[1]]
            };
            if fill == 0 {
                // Plain ground shows the lattice picture: the copy that holds
                // the whole triangle, read at the triangle's own place.
                let picture = lattice.as_ref()?;
                let (w, h) = (picture.width as f32, picture.height as f32);
                return copies.iter().find_map(|&(x, y)| {
                    let local = ground.map(|corner| [corner[0] - x, corner[1] - y]);
                    local
                        .iter()
                        .all(|p| (2.0..=w - 2.0).contains(&p[0]) && (2.0..=h - 2.0).contains(&p[1]))
                        .then(|| Placed {
                            at: ground,
                            uv: local.map(|p| sheet(picture, p)),
                        })
                });
            }
            // A ring cell shows its own baked picture, whose anchor is the
            // cell's centre.
            let up = canvas_cells::is_up(cell);
            let picture = self.placed(canvas_cells::name(fill, up))?;
            let anchor = canvas_cells::layout(up).anchor;
            let origin = canvas_cells::origin(camera, cell);
            let origin = [origin.0.round(), origin.1.round()];
            let centre = camera.project([
                cell.0 as f32 * canvas_lattice::CELL_DX,
                cell.1 as f32 * canvas_lattice::CELL_DY,
                0.0,
            ]);
            let in_picture = ground.map(|corner| {
                [
                    anchor.0 + corner[0] - centre.0,
                    anchor.1 + corner[1] - centre.1,
                ]
            });
            Some(Placed {
                at: in_picture.map(|p| [origin[0] + p[0], origin[1] + p[1]]),
                uv: in_picture.map(|p| sheet(&picture, p)),
            })
        };
        let (width, height) = self.space.size;
        let scene = Scene {
            camera,
            scale: (
                width / canvas_camera::VIEWPORT.0,
                height / canvas_camera::VIEWPORT.1,
            ),
            height_of: &height_of,
            skip: &skip,
            place: &place,
        };
        let triangles = canvas_mesh::triangles(&scene, &raised);
        if !triangles.is_empty() {
            out.push(Draw::Triangles(triangles));
        }
    }
}
