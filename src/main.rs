fn main() {
    if let Err(error) = brunch::directories::AppDirectories::initialize() {
        // there is no config, so fallback to the default state (there is none for now)
        eprintln!("warning: could not initialize app directories: {error}; using default config");
    }

    match brunch::ui::run() {
        Ok(()) => {}
        Err(error) => eprintln!("failed to start launcher: {error}"),
    }
}
