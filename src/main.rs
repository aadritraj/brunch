fn main() {
    match brunch::ui::run() {
        Ok(Some(appid)) => println!("Selection: {appid}"),
        Ok(None) => {}
        Err(error) => eprintln!("failed to start launcher: {error}"),
    }
}
