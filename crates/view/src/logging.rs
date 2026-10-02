//! The logger `oag-view` installs.

/// Installs the sink every `log` call in this run ends up in.
///
/// `warn` globally with our own crates at `debug`, and a format carrying the
/// level and nothing else - both match `oag-game`'s `init_logging`, whose doc
/// comment gives the reasoning. The global `warn` matters most here: at `info`
/// wgpu and naga narrate every pipeline they build, and this tool's reports go
/// to stdout where a reader wants them undisturbed. `calloop=error` comes from
/// there too, and for the same reason: this tool opens a winit window, so it
/// gets the same Wayland key-repeat noise a race does.
pub fn init() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn,oag=info,calloop=error"),
    )
    .format_timestamp(None)
    .format_target(false)
    .init();
}
