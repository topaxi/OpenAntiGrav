//! The on-screen racing controls, as a draw list.
//!
//! **Plain shapes, chosen not measured.** No title ships a touch scheme, so
//! there is no original art to play. The project's button-prompt glyphs
//! (`oag_ui::prompt`) are PromptFont codepoints drawn by the text pass for a
//! pad's face buttons; none of them is a thrust, shield or pause symbol, so
//! each control carries a vector glyph built from the same generated
//! textures as its body (see [`art`] and [`shape`]): chevrons, a shield, a crosshair, side arrows, pause
//! bars. The only words are GO and the L and R of the airbrakes: the other glyphs
//! (crosshair, shield, pause bars, eye) say which button they are alone.
//!
//! The look follows the common phone overlay: round translucent buttons with
//! a thin soft outline and a dark low-opacity body (so the white glyph reads
//! on HD's bright track), a pressed state that is brighter but never
//! opaque, and a stick that is a ring with a knob where the thumb landed.
//! Where the layout is and what it presses is [`oag_input::touch`]'s; this
//! only draws what it is told.
//!
//! Drawn in the overlay's grid (see `oag_present::perf::grid`), whose height
//! is fixed, so `scale` is grid units per window pixel.

pub mod art;
pub mod preview;
mod shape;

use oag_input::touch::{self, Control, GoZone, Scheme, Setup, Touches};
use oag_ui::frontend::{Align, Draw};

use art::Art;
use shape::Rounded;

/// Colours, all translucent. A dark body keeps the white glyph legible over
/// HD's bright track; the held state is a cyan wash well short of opaque.
/// Chosen, not measured.
const BODY: [f32; 4] = [0.0, 0.0, 0.0, 0.28];
const BODY_HELD: [f32; 4] = [0.2, 0.8, 1.0, 0.40];
const EDGE: [f32; 4] = [1.0, 1.0, 1.0, 0.60];
const EDGE_HELD: [f32; 4] = [0.7, 1.0, 1.0, 0.80];
const GLYPH: [f32; 4] = [1.0, 1.0, 1.0, 0.85];
const DIVIDER: [f32; 4] = [1.0, 1.0, 1.0, 0.30];
/// The lit brake segment's colour; its alpha runs from `.0` at a light pull
/// to `.1` at a full one.
const ZONE_LIT: [f32; 4] = [1.0, 0.75, 0.2, 0.0];
const ZONE_ALPHA: (f32, f32) = (0.22, 0.60);
/// The share of the corner a feather-light pull already lights; the lit
/// patch grows from here to the whole corner at a full pull.
const ZONE_MIN_REACH: f32 = 0.4;
const STICK_BODY: [f32; 4] = [1.0, 1.0, 1.0, 0.08];
const STICK_EDGE: [f32; 4] = [1.0, 1.0, 1.0, 0.35];
const STICK_KNOB: [f32; 4] = [1.0, 1.0, 1.0, 0.40];

/// Label scale: the overlay's built-in glyphs are already small.
const LABEL_SCALE: f32 = 1.0;
/// Approximate cap height of the built-in face, in grid units.
const LABEL_HEIGHT: f32 = 8.0;
/// GO's corner radius as a share of its side.
const GO_ROUND: f32 = 0.30;
/// A quarter turn, for the side arrows.
const QUARTER: f32 = std::f32::consts::FRAC_PI_2;

/// Whether this machine has a touchscreen to need the overlay: Android, or
/// any platform with `OAG_TOUCH_CONTROLS` set, which is how a desktop run
/// shows it (chosen, not measured: a desktop with no touchscreen gains
/// nothing from the page, so it is offered only on request).
#[must_use]
pub fn available() -> bool {
    cfg!(target_os = "android") || std::env::var_os("OAG_TOUCH_CONTROLS").is_some()
}

fn scaled(mut color: [f32; 4], opacity: f32) -> [f32; 4] {
    color[3] *= opacity;
    color
}

fn text(x: f32, y: f32, word: &str, opacity: f32) -> Draw {
    Draw::Text {
        x,
        y,
        scale: LABEL_SCALE,
        color: scaled(GLYPH, opacity),
        border: None,
        align: Align::Centre,
        text: word.to_string(),
        wrap_width: None,
    }
}

fn fill(rect: [f32; 4], color: [f32; 4]) -> Draw {
    Draw::Fill { rect, color }
}

/// GO's brake corners: the pulled one lit inside GO's own outline, growing
/// and brightening with the pull, a faint divider either side of the dead
/// centre, and the `L` and `R` letters.
fn go_zones(art: &Art, touches: &Touches, body: Rounded, opacity: f32, out: &mut Vec<Draw>) {
    let [x, y, w, h] = body.rect;
    let side = w * touch::GO_ZONE_SIDE;
    let top = y + h * (1.0 - touch::GO_ZONE_BOTTOM);
    let band = y + h - top;
    let inner = body.inset(body.border());
    for (zone, cell_x, word) in [(GoZone::Left, x, "L"), (GoZone::Right, x + w - side, "R")] {
        if touches.zone_down(zone) {
            let pull = touches.zone_strength(zone);
            let reach = ZONE_MIN_REACH + (1.0 - ZONE_MIN_REACH) * pull;
            let (lit_w, lit_h) = (side * reach, band * reach);
            let lit_x = if zone == GoZone::Left {
                cell_x
            } else {
                cell_x + side - lit_w
            };
            let mut colour = ZONE_LIT;
            colour[3] = ZONE_ALPHA.0 + (ZONE_ALPHA.1 - ZONE_ALPHA.0) * pull;
            shape::fill(
                art,
                inner,
                Some([lit_x, top + band - lit_h, lit_w, lit_h]),
                scaled(colour, opacity),
                out,
            );
        }
        out.push(text(
            cell_x + side / 2.0,
            top + (band - LABEL_HEIGHT) / 2.0,
            word,
            opacity,
        ));
    }
    for line_x in [x + side, x + w - side] {
        out.push(fill(
            [line_x, top + 2.0, 0.5, band - 5.0],
            scaled(DIVIDER, opacity),
        ));
    }
}

/// Easy's bar under GO with the zone off: one pill the width of GO, a
/// divider down the middle, and each half labelled; a held half is tinted
/// inside the pill's own outline.
fn split_bar(
    art: &Art,
    touches: &Touches,
    size: (f32, f32),
    scale: f32,
    opacity: f32,
    out: &mut Vec<Draw>,
) {
    let setup = Setup::new(Scheme::Easy, false);
    let rect_of = |control: Control| {
        touch::layout_for(size, setup)
            .into_iter()
            .find(|(c, _)| *c == control)
            .map(|(_, r)| [r.x * scale, r.y * scale, r.w * scale, r.h * scale])
    };
    let (Some(brake), Some(absorb)) = (rect_of(Control::Brake), rect_of(Control::Absorb)) else {
        return;
    };
    let whole = [brake[0], brake[1], brake[2] + absorb[2], brake[3]];
    let body = Rounded {
        rect: whole,
        radius: whole[3] / 2.0,
    };
    shape::button(art, body, scaled(EDGE, opacity), scaled(BODY, opacity), out);
    let inner = body.inset(body.border());
    for (control, half) in [(Control::Brake, brake), (Control::Absorb, absorb)] {
        if touches.is_down(control) {
            shape::fill(art, inner, Some(half), scaled(BODY_HELD, opacity), out);
        }
        out.push(text(
            half[0] + half[2] / 2.0,
            half[1] + (half[3] - LABEL_HEIGHT) / 2.0,
            control.label(),
            opacity,
        ));
    }
    out.push(fill(
        [absorb[0] - 0.5, whole[1] + 2.0, 1.0, whole[3] - 4.0],
        scaled(EDGE, opacity),
    ));
}

/// Easy's BRAKE band across GO's bottom: lit inside GO's own outline from
/// the bottom up as the pull rises, with its word.
fn go_band(art: &Art, touches: &Touches, body: Rounded, opacity: f32, out: &mut Vec<Draw>) {
    let [x, y, w, h] = body.rect;
    let band = h * touch::EASY_BAND;
    let top = y + h - band;
    let inner = body.inset(body.border());
    let pull = touches.brake_pull();
    if pull > 0.0 {
        let lit_h = band * (ZONE_MIN_REACH + (1.0 - ZONE_MIN_REACH) * pull);
        let mut colour = ZONE_LIT;
        colour[3] = ZONE_ALPHA.0 + (ZONE_ALPHA.1 - ZONE_ALPHA.0) * pull;
        shape::fill(
            art,
            inner,
            Some([x, y + h - lit_h, w, lit_h]),
            scaled(colour, opacity),
            out,
        );
    }
    out.push(fill([x + 4.0, top, w - 8.0, 0.5], scaled(DIVIDER, opacity)));
    out.push(text(
        x + w / 2.0,
        top + (band - LABEL_HEIGHT) / 2.0,
        "BRAKE",
        opacity,
    ));
}

/// The glyph for `control`, centred in `rect`. GO's stays in its upper part
/// so the lower band keeps its brake corners; only GO and the airbrakes
/// carry a word.
fn glyph(
    art: &Art,
    control: Control,
    rect: [f32; 4],
    paused: bool,
    opacity: f32,
    out: &mut Vec<Draw>,
) {
    let [x, y, w, h] = rect;
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let u = w.min(h);
    let ink = scaled(GLYPH, opacity);
    let circle = |cx: f32, cy: f32, r: f32| Rounded {
        rect: [cx - r, cy - r, 2.0 * r, 2.0 * r],
        radius: r,
    };
    match control {
        Control::Accelerate => {
            let g = u * 0.30;
            let top = y + h * 0.07;
            for k in 0..2 {
                shape::sprite(
                    [cx - g / 2.0, top + k as f32 * g * 0.62, g, g * 0.7],
                    art.triangle,
                    0.0,
                    ink,
                    out,
                );
            }
        }
        Control::Fire => {
            shape::ring(art, circle(cx, cy, u * 0.25), ink, out);
            shape::fill(art, circle(cx, cy, u * 0.06), None, ink, out);
        }
        Control::Brake => out.push(text(cx, cy - LABEL_HEIGHT / 2.0, "BRAKE", opacity)),
        Control::Absorb => {
            let g = u * 0.40;
            shape::sprite(
                [cx - g / 2.0, cy - g * 0.55, g, g * 1.1],
                art.shield,
                0.0,
                ink,
                out,
            );
        }
        Control::AirbrakeLeft | Control::AirbrakeRight => {
            let right = control == Control::AirbrakeRight;
            let g = u * 0.34;
            shape::sprite(
                [cx - g / 2.0, cy - g * 0.6 - 2.0, g, g],
                art.triangle,
                if right { QUARTER } else { -QUARTER },
                ink,
                out,
            );
            out.push(text(
                cx,
                cy + u * 0.18,
                if right { "R" } else { "L" },
                opacity,
            ));
        }
        Control::Pause if paused => {
            let g = u * 0.34;
            shape::sprite(
                [cx - g / 2.0, cy - g / 2.0, g, g],
                art.triangle,
                QUARTER,
                ink,
                out,
            );
        }
        Control::Pause => {
            let (bar_w, bar_h) = (u * 0.10, u * 0.40);
            for dx in [-1.6, 0.6] {
                out.push(fill([cx + dx * bar_w, cy - bar_h / 2.0, bar_w, bar_h], ink));
            }
        }
        Control::Camera => {
            let (bw, bh, t) = (u * 0.56, u * 0.38, 1.2);
            let (bx, by) = (cx - bw / 2.0, cy - bh / 2.0);
            for rect in [
                [bx, by, bw, t],
                [bx, by + bh - t, bw, t],
                [bx, by + t, t, bh - 2.0 * t],
                [bx + bw - t, by + t, t, bh - 2.0 * t],
            ] {
                out.push(fill(rect, ink));
            }
            shape::ring(art, circle(cx, cy, u * 0.10), ink, out);
        }
    }
}

/// A held-finger pose to draw in a capture, since a headless run has no
/// fingers: `--touch-overlay`. Same [`Touches`] a real finger would build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Demo {
    /// Nothing held.
    Idle,
    /// Nothing held, with `touch_go_zones` off: GO is thrust alone and the
    /// separate airbrake buttons are back.
    IdleButtons,
    /// A finger well into GO's bottom-left corner: nearly a full brake.
    GoLeft,
    /// A finger partway into GO's bottom-right corner: a partial brake.
    GoRight,
    /// A finger in GO's centre, with another on the floating stick pushed
    /// right.
    StickAndGo,
    /// Easy, nothing held, GO's BRAKE band on.
    EasyIdle,
    /// Easy, nothing held, GO's zone off: the bar below GO is BRAKE and
    /// ABSORB side by side.
    EasyZonesOff,
    /// Easy: the stick dragged left past full lock into the brake rim, and a
    /// finger deep in GO's BRAKE band.
    EasyBrake,
    /// Easy with the zone off: a finger that began on GO slid onto the bar's
    /// BRAKE half, the stick steering right.
    EasyBarBrake,
    /// Standard: a finger that began on GO slid up onto FIRE.
    GoFire,
    /// Standard: a finger that began on GO dragged well past its bottom-left
    /// corner: the brake holds full.
    GoLeftPast,
}

impl Demo {
    /// The pose named on the command line.
    ///
    /// # Errors
    /// Names the accepted values when `text` is none of them.
    pub fn parse(text: &str) -> Result<Self, String> {
        match text {
            "idle" => Ok(Self::Idle),
            "idle-buttons" => Ok(Self::IdleButtons),
            "go-left" => Ok(Self::GoLeft),
            "go-right" => Ok(Self::GoRight),
            "stick-go" => Ok(Self::StickAndGo),
            "easy-idle" => Ok(Self::EasyIdle),
            "easy-zones-off" => Ok(Self::EasyZonesOff),
            "easy-brake" => Ok(Self::EasyBrake),
            "easy-bar-brake" => Ok(Self::EasyBarBrake),
            "go-fire" => Ok(Self::GoFire),
            "go-left-past" => Ok(Self::GoLeftPast),
            other => Err(format!(
                "unknown touch pose {other:?}: idle, idle-buttons, go-left, go-right, stick-go, \
                 go-fire, go-left-past, easy-idle, easy-zones-off, easy-brake or easy-bar-brake"
            )),
        }
    }

    /// The scheme and zone setting this pose is drawn with.
    #[must_use]
    pub fn setup(self) -> Setup {
        match self {
            Self::IdleButtons => Setup::new(Scheme::Standard, false),
            Self::EasyIdle | Self::EasyBrake => Setup::new(Scheme::Easy, true),
            Self::EasyZonesOff | Self::EasyBarBrake => Setup::new(Scheme::Easy, false),
            _ => Setup::new(Scheme::Standard, true),
        }
    }

    /// The fingers for a window of `size` pixels.
    #[must_use]
    pub fn touches(self, size: (f32, f32)) -> Touches {
        let setup = self.setup();
        let mut touches = Touches::default();
        touches.set_setup(setup, size);
        let go = touch::go_rect(size);
        let at = |fx: f32, fy: f32| (go.x + go.w * fx, go.y + go.h * fy);
        let of = |control: Control| {
            touch::layout_for(size, setup)
                .into_iter()
                .find(|(c, _)| *c == control)
                .map_or((0.0, 0.0), |(_, r)| r.centre())
        };
        let drag = |touches: &mut Touches, dx: f32, dy: f32| {
            let origin = (size.0 * 0.14, size.1 * 0.62);
            touches.down(2, origin, size);
            touches.moved(2, (origin.0 + size.1 * dx, origin.1 + size.1 * dy), size);
        };
        match self {
            Self::Idle | Self::IdleButtons | Self::EasyIdle | Self::EasyZonesOff => {}
            Self::GoLeft => touches.down(1, at(0.03, 0.97), size),
            Self::GoRight => touches.down(1, at(0.88, 0.88), size),
            Self::StickAndGo => {
                touches.down(1, at(0.5, 0.35), size);
                drag(&mut touches, 0.10, -0.03);
            }
            Self::EasyBrake => {
                touches.down(1, at(0.5, 0.88), size);
                drag(&mut touches, -0.18, -0.02);
            }
            Self::EasyBarBrake => {
                touches.down(1, at(0.5, 0.5), size);
                touches.moved(1, of(Control::Brake), size);
                drag(&mut touches, 0.07, 0.0);
            }
            Self::GoLeftPast => {
                touches.down(1, at(0.5, 0.5), size);
                touches.moved(1, at(-0.45, 1.25), size);
            }
            Self::GoFire => {
                touches.down(1, at(0.5, 0.5), size);
                touches.moved(1, of(Control::Fire), size);
            }
        }
        touches
    }
}

/// Draws `demo` over what `view` already holds, in a window of `size`
/// pixels: the capture-side twin of the window's overlay pass.
///
/// # Errors
/// When the overlay's pipelines will not build.
pub fn draw_pose(
    demo: Demo,
    gpu: (&wgpu::Device, &wgpu::Queue, wgpu::TextureFormat),
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    size: (u32, u32),
) -> anyhow::Result<()> {
    let px = (size.0 as f32, size.1 as f32);
    overlay_pass(gpu, encoder, view, size, |art, space| {
        draw(
            art,
            &demo.touches(px),
            px,
            space.size.1 / px.1.max(1.0),
            false,
            demo.setup(),
            1.0,
        )
    })
}

/// The TOUCH CONTROLS page's preview over what `view` already holds: the
/// capture-side twin of the window's overlay pass, which draws the same
/// [`preview::for_page`] list.
///
/// # Errors
/// When the overlay's pipelines will not build.
pub fn draw_preview(
    controls: &crate::settings::Controls,
    gpu: (&wgpu::Device, &wgpu::Queue, wgpu::TextureFormat),
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    size: (u32, u32),
) -> anyhow::Result<()> {
    overlay_pass(gpu, encoder, view, size, |art, space| {
        preview::draw(
            art,
            space,
            controls.touch_setup(),
            controls.touch_alpha(),
            0.0,
        )
    })
}

/// One overlay-renderer pass of whatever `build` makes from the touch art
/// and the window's grid.
fn overlay_pass(
    (device, queue, format): (&wgpu::Device, &wgpu::Queue, wgpu::TextureFormat),
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    size: (u32, u32),
    build: impl FnOnce(&Art, oag_display::space::Space) -> Vec<Draw>,
) -> anyhow::Result<()> {
    let px = (size.0 as f32, size.1 as f32);
    let space = oag_present::perf::grid(size);
    let sheet = crate::cursor::sheet(crate::cursor::LAUNCHER);
    let mut renderer = crate::render::Renderer::new(
        device,
        queue,
        format,
        None,
        oag_ui::font::Atlas::build(),
        &sheet,
    )?;
    renderer.set_space(space);
    let list = Art::from_sheet(&sheet).map_or_else(Vec::new, |art| build(&art, space));
    renderer.overlay(device, queue, encoder, view, &list, (0.0, 0.0, px.0, px.1));
    Ok(())
}

/// Every control and the stick, for a window of `size` pixels, in a grid
/// `scale` units per pixel. While `paused` the pause button is a play
/// triangle; `setup` is the scheme and whether GO carries its brake zone(s); `opacity`
/// scales every alpha (1.0 is the design). `art` is where the generated
/// textures sit in the overlay's sprite sheet.
#[must_use]
pub fn draw(
    art: &Art,
    touches: &Touches,
    size: (f32, f32),
    scale: f32,
    paused: bool,
    setup: Setup,
    opacity: f32,
) -> Vec<Draw> {
    let (zones, easy) = (setup.zones, setup.scheme == Scheme::Easy);
    let mut out = Vec::new();
    let split = easy && !zones;
    for (control, rect) in touch::layout_for(size, setup) {
        if split && matches!(control, Control::Brake | Control::Absorb) {
            continue;
        }
        let held = touches.is_down(control);
        let rect = [
            rect.x * scale,
            rect.y * scale,
            rect.w * scale,
            rect.h * scale,
        ];
        let radius = if control == Control::Accelerate {
            rect[2] * GO_ROUND
        } else {
            rect[2].min(rect[3]) / 2.0
        };
        let body = Rounded { rect, radius };
        let (edge, fill_colour) = if held {
            (EDGE_HELD, BODY_HELD)
        } else {
            (EDGE, BODY)
        };
        shape::button(
            art,
            body,
            scaled(edge, opacity),
            scaled(fill_colour, opacity),
            &mut out,
        );
        glyph(art, control, rect, paused, opacity, &mut out);
        if control == Control::Accelerate {
            let at = match (zones, easy) {
                (true, true) => 0.51,
                (true, false) => 0.55,
                _ => 0.62,
            };
            out.push(text(
                rect[0] + rect[2] / 2.0,
                rect[1] + rect[3] * at,
                "GO",
                opacity,
            ));
            if zones && easy {
                go_band(art, touches, body, opacity, &mut out);
            } else if zones {
                go_zones(art, touches, body, opacity, &mut out);
            }
        }
    }
    if split {
        split_bar(art, touches, size, scale, opacity, &mut out);
    }
    if let Some((origin, at)) = touches.stick() {
        let radius = size.1 * setup.stick_radius() * scale;
        let o = (origin.0 * scale, origin.1 * scale);
        let base = Rounded {
            rect: [o.0 - radius, o.1 - radius, 2.0 * radius, 2.0 * radius],
            radius,
        };
        shape::button(
            art,
            base,
            scaled(STICK_EDGE, opacity),
            scaled(STICK_BODY, opacity),
            &mut out,
        );
        let dx = (at.0 - origin.0) * scale;
        let dy = (at.1 - origin.1) * scale;
        let reach = (dx * dx + dy * dy).sqrt();
        let k = if reach > radius { radius / reach } else { 1.0 };
        let knob = radius * 0.38;
        let mut knob_colour = STICK_KNOB;
        if easy {
            let u = (at.0 - origin.0) / (size.1 * setup.stick_radius());
            let brake = touch::easy_curve(u).1;
            let guide = radius * touch::EASY_STEER_FULL;
            shape::ring(
                art,
                Rounded {
                    rect: [o.0 - guide, o.1 - guide, 2.0 * guide, 2.0 * guide],
                    radius: guide,
                },
                scaled(STICK_EDGE, opacity),
                &mut out,
            );
            for k in 0..3 {
                knob_colour[k] += (ZONE_LIT[k] - knob_colour[k]) * brake;
            }
            knob_colour[3] += (ZONE_ALPHA.1 - knob_colour[3]) * brake;
        }
        shape::fill(
            art,
            Rounded {
                rect: [
                    o.0 + dx * k - knob,
                    o.1 + dy * k - knob,
                    2.0 * knob,
                    2.0 * knob,
                ],
                radius: knob,
            },
            None,
            scaled(knob_colour, opacity),
            &mut out,
        );
    }
    out
}

#[cfg(test)]
mod tests;
