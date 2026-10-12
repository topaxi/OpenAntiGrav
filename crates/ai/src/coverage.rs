//! Which speed classes a controller is known to fly.
//!
//! AI opponents exist only for a class something defines how to fly: this
//! project's driver for the classes its sweeps verify, the original's driver
//! for the classes its own title's AI tables author. A race on any other class
//! fields no opponents rather than a driver nobody checked there.
//!
//! The set is data on the controller, so a later pilot-file `classes` key or a
//! new sweep widens it here and nowhere else.

/// The classes this project's driver is verified on, by the names the discs
/// spell: Pulse's four and Pure's `VECTOR`. `SUPERPHANTOM` (2048) is not here:
/// no sweep has flown this driver on it.
pub const PROJECT_DRIVER_CLASSES: &[&str] = &["VECTOR", "VENOM", "FLASH", "RAPIER", "PHANTOM"];

/// Whether this project's driver covers `class` (case-insensitive).
#[must_use]
pub fn project_driver_covers(class: &str) -> bool {
    PROJECT_DRIVER_CLASSES
        .iter()
        .any(|covered| covered.eq_ignore_ascii_case(class))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_project_driver_covers_the_swept_classes_and_not_super_phantom() {
        for class in ["venom", "FLASH", "Rapier", "phantom", "vector"] {
            assert!(project_driver_covers(class), "{class}");
        }
        assert!(!project_driver_covers("SUPERPHANTOM"));
        assert!(!project_driver_covers("nonsense"));
    }
}
