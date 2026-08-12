use std::{env, fs};

fn main() {
    match env::args().nth(1).as_deref() {
        Some("--exit-42") => std::process::exit(42),
        Some("--abort") => std::process::abort(),
        _ => {}
    }
    let capture = env::var("TASKSEAL_CAPTURE_PATH").expect("capture path is required");
    let args = env::args().skip(1).collect::<Vec<_>>().join("\0");
    fs::write(capture, args).expect("capture must be writable");
}
