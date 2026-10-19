//! What a phone needs around the windowed game, for every mobile OS at once.
//!
//! A phone differs from a desktop in three ways that are not the game's
//! business: there is no command line and no working directory worth the name,
//! the app's files live where the OS says (not under `$XDG_*`), and the window
//! system's event loop has to be built the OS's way. This crate holds those
//! three and nothing else; the game keeps no `cfg(target_os)` for them.
//!
//! **The entry points stay in `oag-game`.** The windowed `App` lives in the
//! binary's own modules and no crate may depend on `oag-game` (rule 2 of
//! `just check-deps`), so a crate below it cannot call `run`. `oag-game`'s
//! `oag_android` example and its iOS binary are therefore the thin per-OS
//! entries, and each calls into here first. See ADR-0065.
//!
//! The suspend/resume and surface-recreation half of the lifecycle is not here:
//! it is winit's `Suspended`/`Resumed` events, handled in the game's `App`
//! identically on every platform (`suspended`/`resumed`), so there is nothing
//! per-OS to hold.

#[cfg(target_os = "android")]
pub mod android;
#[cfg(target_os = "ios")]
pub mod ios;

/// Whether this build targets a phone or tablet: the touch overlay is on by
/// default and a way to quit is not offered (the OS owns the app's lifetime).
/// A constant of the build, not a probe of the machine.
pub const MOBILE: bool = cfg!(any(target_os = "android", target_os = "ios"));

#[cfg(test)]
mod tests {
    /// Android refuses every socket to an app without `INTERNET`: LAN JOIN GAME's host
    /// browser failed to open and the player landed back on the LAN page.
    #[test]
    fn the_apk_manifest_asks_for_the_network() {
        let script = include_str!("../../../scripts/build-apk.sh");
        assert!(script.contains(r#"<uses-permission android:name="android.permission.INTERNET" />"#));
    }
}
