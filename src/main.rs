fn main() {
    match brunch::ui::run() {
        Ok(()) => {}
        Err(error) => eprintln!("failed to start launcher: {error}"),
    }
}
