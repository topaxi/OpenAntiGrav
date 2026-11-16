//! `--probe-pad`: prints each pad's button events with the value `[controls.music]` takes, so a
//! control can be found without a window.

use std::io::Write;

use crate::music::PadControl;
use crate::pad::{is_real_pad, map_button};

/// One line for a pad event, or `None` for events a settings file cannot bind (axes, drops).
#[must_use]
pub fn describe(name: &str, event: &gilrs::EventType) -> Option<String> {
    match *event {
        gilrs::EventType::ButtonPressed(button, code) => {
            Some(button_line(name, "pressed", button, code.into_u32()))
        }
        gilrs::EventType::ButtonReleased(button, code) => {
            Some(button_line(name, "released", button, code.into_u32()))
        }
        gilrs::EventType::Connected => Some(format!("{name}: connected")),
        gilrs::EventType::Disconnected => Some(format!("{name}: disconnected")),
        _ => None,
    }
}

fn button_line(name: &str, verb: &str, button: gilrs::Button, code: u32) -> String {
    let value = PadControl::describe(button, code);
    let note = if map_button(button).is_some() {
        " (a game button: cannot be bound)"
    } else {
        ""
    };
    format!("{name}: {verb} {value} (code {code}){note}")
}

/// Prints pad events to `out` until `stop` returns true, polling every 5 ms.
///
/// # Errors
/// The pad subsystem cannot be opened, or `out` fails.
pub fn run(out: &mut impl Write, mut stop: impl FnMut() -> bool) -> Result<(), String> {
    let mut gilrs = gilrs::Gilrs::new().map_err(|e| format!("no gamepad support: {e}"))?;
    for (_, pad) in gilrs.gamepads().filter(|(_, pad)| is_real_pad(pad)) {
        writeln!(out, "{}: present", pad.name()).map_err(|e| e.to_string())?;
    }
    while !stop() {
        while let Some(event) = gilrs.next_event() {
            let pad = gilrs.gamepad(event.id);
            if !is_real_pad(&pad) {
                continue;
            }
            let name = pad.name().to_string();
            if let Some(line) = describe(&name, &event.event) {
                writeln!(out, "{line}").map_err(|e| e.to_string())?;
                out.flush().map_err(|e| e.to_string())?;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_prints_the_value_a_settings_file_takes() {
        let line = button_line("pad", "pressed", gilrs::Button::RightThumb, 318);
        assert!(line.contains("pressed RightThumb (code 318)"), "{line}");
        assert!(!line.contains("game button"), "{line}");
        let line = button_line("pad", "pressed", gilrs::Button::South, 304);
        assert!(line.contains("game button"), "{line}");
        let line = button_line("pad", "released", gilrs::Button::Unknown, 300);
        assert!(line.contains("released raw:300"), "{line}");
        assert!(describe("pad", &gilrs::EventType::Dropped).is_none());
    }
}
